use alloy_primitives::{Address, Bytes, U256};
use crate::agents::simulator::Simulator;
use crate::agents::forker::ForkerAgent;
use revm::Database;
use tracing::info;

#[derive(Debug)]
pub struct ReentrancyReport {
    pub pool: Address,
    pub lender: Address,
    pub profit: U256,
    pub calldata: Bytes,
}

pub struct ReentrancyDetector;

impl ReentrancyDetector {
    pub async fn hunt(
        forker: &ForkerAgent,
        _sim: &Simulator,
        pool: Address,
        lender: Address,
        attacker: Address,
        remove_liquidity_calldata: Bytes,
        borrow_calldata: Bytes,
    ) -> Option<ReentrancyReport> {
        let mut db = forker
            .build_cross_contract_db(pool, vec![lender])
            .await
            .ok()?;

        // ---------- Initial state ----------
        // Pool price = 1 ETH
        let price_slot = U256::from(0);
        db.insert_account_storage(pool, price_slot, U256::from(1_000_000_000_000_000_000u128))
            .ok()?;

        // Collateral mapping (slot 0) – attacker has 1 ETH deposited
        let coll_base_slot = U256::from(0);
        let attacker_coll_slot = mapping_slot(attacker, coll_base_slot);
        db.insert_account_storage(lender, attacker_coll_slot, U256::from(1_000_000_000_000_000_000u128))
            .ok()?;

        // Loans mapping (slot 1) – initially zero
        let loan_base_slot = U256::from(1);
        let attacker_loan_slot = mapping_slot(attacker, loan_base_slot);
        db.insert_account_storage(lender, attacker_loan_slot, U256::ZERO)
            .ok()?;

        // 🔥 FUND LENDX WITH 100 ETH so transfer() succeeds
        db.insert_account_info(lender, revm::primitives::AccountInfo {
            balance: U256::from(100_000_000_000_000_000_000u128), // 100 ETH
            nonce: 1,
            code: db.accounts.get(&lender).and_then(|a| a.info.code.clone()),
            code_hash: Default::default(),
        });

        // ---------- Execute the reentrancy ----------
        Simulator::execute_reentrancy_sequence(
            &mut db,
            attacker,
            pool,
            remove_liquidity_calldata,
            lender,
            borrow_calldata.clone(),
        )?;

        // ---------- Check post‑attack state ----------
        let true_price = db.storage(pool, price_slot).ok()?;
        let loan_amount = db.storage(lender, attacker_loan_slot).ok()?;
        let collateral = db.storage(lender, attacker_coll_slot).ok()?;

        let scale = U256::from(10).pow(U256::from(18));
        let safe_max = collateral
            .checked_mul(true_price)
            .and_then(|v| v.checked_div(scale))
            .unwrap_or(U256::ZERO);

        if loan_amount > safe_max && safe_max > U256::ZERO {
            let profit = loan_amount - safe_max;
            info!(
                "💥 Read‑Only Reentrancy: loan={}, safe_max={}, profit={}",
                loan_amount, safe_max, profit
            );
            return Some(ReentrancyReport {
                pool,
                lender,
                profit,
                calldata: borrow_calldata,
            });
        }
        None
    }
}

fn mapping_slot(user: Address, base_slot: U256) -> U256 {
    let mut preimage = [0u8; 64];
    preimage[12..32].copy_from_slice(user.as_ref());
    preimage[32..64].copy_from_slice(&base_slot.to_be_bytes::<32>());
    let key = alloy_primitives::keccak256(preimage);
    U256::from_be_bytes(key.0)
}