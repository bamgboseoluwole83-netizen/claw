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

        // ---------- Initial state setup ----------
        // 1. Pool virtual price = 1 ETH (slot 0)
        let price_slot = U256::from(0);
        db.insert_account_storage(pool, price_slot, U256::from(1_000_000_000_000_000_000u128))
            .ok()?;

        // 2. Attacker deposits 1 ETH collateral into LendX
        let collateral_base_slot = U256::from(0);            // collateral mapping base slot
        let attacker_collat_slot = mapping_slot(attacker, collateral_base_slot);
        db.insert_account_storage(lender, attacker_collat_slot, U256::from(1_000_000_000_000_000_000u128))
            .ok()?;

        // 3. Ensure the attacker’s loan is initially zero
        let loan_base_slot = U256::from(1);                  // loans mapping base slot
        let attacker_loan_slot = mapping_slot(attacker, loan_base_slot);
        db.insert_account_storage(lender, attacker_loan_slot, U256::ZERO)
            .ok()?;

        // ---------- Execute the reentrancy ----------
        Simulator::execute_reentrancy_sequence(
            &mut db,
            attacker,
            pool,
            remove_liquidity_calldata,
            lender,
            borrow_calldata.clone(),
        )?;

        // ---------- Check post‑execution state ----------
        // True pool price is back to 1 ETH (after the callback finished)
        let true_price = db.storage(pool, price_slot).ok()?;
        // Loan amount taken by the attacker
        let loan_amount = db.storage(lender, attacker_loan_slot).ok()?;
        // Attacker’s remaining collateral
        let collateral = db.storage(lender, attacker_collat_slot).ok()?;

        // Safe borrow limit: collateral * true_price / 1e18
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

/// Compute the storage key for a mapping `mapping(address => value)` at base slot.
fn mapping_slot(user: Address, base_slot: U256) -> U256 {
    let mut preimage = [0u8; 64];
    preimage[12..32].copy_from_slice(user.as_ref());
    preimage[32..64].copy_from_slice(&base_slot.to_be_bytes::<32>());
    let key = alloy_primitives::keccak256(preimage);
    U256::from_be_bytes(key.0)
}