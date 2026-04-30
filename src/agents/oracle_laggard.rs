use alloy_primitives::{Address, U256};
use alloy_sol_types::{sol, SolCall};
use revm::Database;
use revm::db::{CacheDB, EmptyDB};
use revm::primitives::{AccountInfo, TransactTo, SpecId};
use revm::Evm;
use tracing::info;
use crate::agents::heimdall_analyzer::ContractAnalysis;

sol! {
    function deposit() external payable;
    function borrow(uint256 amount) external;
}

/// Run the oracle laggard detection using heimdall‑discovered price slot.
/// If an analysis is provided, the price slot is taken from its storage layout
/// (looking for a variable that contains "price" or "rate"). Otherwise falls back to slot 0.
pub fn detect_oracle_laggard_heimdall(
    db: &mut CacheDB<EmptyDB>,
    lender_addr: Address,
    oracle_addr: Address,
    attacker: Address,
    analysis: Option<&ContractAnalysis>,
) -> Option<(U256, Vec<u8>)> {

    // 1. Determine the price slot from heimdall analysis (or default 0)
    let price_slot = if let Some(analysis) = analysis {
        // Look for a storage variable whose name hints at a price
        let slot = analysis.storage_slots.iter()
            .find(|(_, name)| name.contains("price") || name.contains("rate") || name.contains("Price") || name.contains("Rate"))
            .map(|(slot_num, _)| U256::from(*slot_num));
        if let Some(slot) = slot {
            info!("🧠 Using heimdall‑discovered price slot: {}", slot);
            slot
        } else {
            info!("⚠️  No price variable found in heimdall analysis, using default slot 0");
            U256::from(0)
        }
    } else {
        U256::from(0)
    };

    // 2. If heimdall also gave us a list of potential oracles, use the first one
    let target_oracle = if let Some(analysis) = analysis {
        if let Some(first) = analysis.potential_oracles.first() {
            info!("🧠 Using heimdall‑discovered oracle address: {:?}", first);
            *first
        } else {
            oracle_addr
        }
    } else {
        oracle_addr
    };

    // 3. Run the core detection with the discovered slot and oracle
    detect_oracle_laggard_core(db, target_oracle, lender_addr, attacker, price_slot)
}

/// Core detection function that uses an explicit price slot.
fn detect_oracle_laggard_core(
    db: &mut CacheDB<EmptyDB>,
    oracle_addr: Address,
    lender_addr: Address,
    attacker: Address,
    price_slot: U256,
) -> Option<(U256, Vec<u8>)> {
    let true_price  = U256::from(1_000_000_000_000_000_000u128); // 1 ETH
    let stale_price = U256::from(100_000_000_000_000_000_000u128); // 100 ETH

    // ---- 1. Set oracle to stale price ----
    db.insert_account_storage(oracle_addr, price_slot, stale_price).unwrap();

    // ---- 2. Fund lender and attacker ----
    let existing_lender_code = db.accounts.get(&lender_addr).and_then(|a| a.info.code.clone());
    db.insert_account_info(lender_addr, AccountInfo {
        balance: U256::from(1000_000_000_000_000_000_000u128),
        nonce: 1,
        code: existing_lender_code,
        code_hash: Default::default(),
    });

    let attacker_collateral = U256::from(1_000_000_000_000_000_000u128); // 1 ETH
    db.insert_account_info(attacker, AccountInfo {
        balance: attacker_collateral * U256::from(100),
        nonce: 0,
        code: None,
        code_hash: Default::default(),
    });

    // ---- 3. Attacker deposits collateral ----
    let deposit_calldata = depositCall::new(()).abi_encode();
    {
        let mut evm = Evm::builder()
            .with_db(&mut *db)
            .with_spec_id(SpecId::LATEST)
            .build();
        *evm.tx_mut() = revm::primitives::TxEnv {
            caller: attacker,
            transact_to: TransactTo::Call(lender_addr),
            data: alloy_primitives::Bytes::from(deposit_calldata),
            value: attacker_collateral,
            ..Default::default()
        };
        evm.transact_commit().ok();
    }

    // ---- 4. Borrow 50 ETH at stale price ----
    let borrow_amount = U256::from(50) * U256::from(10).pow(U256::from(18));
    let borrow_calldata = borrowCall::new((borrow_amount,)).abi_encode();
    {
        let mut evm = Evm::builder()
            .with_db(&mut *db)
            .with_spec_id(SpecId::LATEST)
            .build();
        *evm.tx_mut() = revm::primitives::TxEnv {
            caller: attacker,
            transact_to: TransactTo::Call(lender_addr),
            data: alloy_primitives::Bytes::from(borrow_calldata.clone()),
            value: U256::ZERO,
            ..Default::default()
        };
        let result = evm.transact_commit();
        let success = match result {
            Ok(res) => res.is_success(),
            _ => false,
        };
        info!("   Borrow at stale price: {}", success);
        if !success {
            return None;
        }
    }

    // ---- 5. Oracle updates to true price ----
    db.insert_account_storage(oracle_addr, price_slot, true_price).unwrap();

    // ---- 6. Check undercollateralization ----
    let coll_slot = mapping_slot(attacker, U256::from(0));
    let loan_slot = mapping_slot(attacker, U256::from(1));
    let collateral = db.storage(lender_addr, coll_slot).unwrap_or(U256::ZERO);
    let loan       = db.storage(lender_addr, loan_slot).unwrap_or(U256::ZERO);
    let safe_max   = collateral * true_price / U256::from(10).pow(U256::from(18));

    if loan > safe_max && safe_max > U256::ZERO {
        let profit = loan - safe_max;
        info!("💥 Oracle laggard detected! profit={}", profit);
        Some((profit, borrow_calldata))
    } else {
        info!("✅ No oracle laggard found.");
        None
    }
}

pub fn mapping_slot(user: Address, base_slot: U256) -> U256 {
    let mut preimage = [0u8; 64];
    preimage[12..32].copy_from_slice(user.as_ref());
    preimage[32..64].copy_from_slice(&base_slot.to_be_bytes::<32>());
    let key = alloy_primitives::keccak256(preimage);
    U256::from_be_bytes(key.0)
}