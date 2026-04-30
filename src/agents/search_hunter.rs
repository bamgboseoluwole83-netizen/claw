use alloy_primitives::{Address, U256};
use revm::Database;
use revm::db::{CacheDB, EmptyDB};
use tracing::info;
use crate::agents::heimdall_analyzer::{ContractAnalysis, analyze as heimdall_analyze};

pub struct SearchHunter;

impl SearchHunter {
    /// Dummy oracle discovery – Heimdall is used for oracle detection elsewhere.
    pub fn discover_oracle_via_trace(
        _db: &mut CacheDB<EmptyDB>,
        _lender: Address,
        _calldata: &[u8],
    ) -> Option<Address> {
        None
    }

    /// Pure‑Rust binary search that finds the exact borrow amount breaking Health Factor.
    /// No external solver needed.
    pub fn hunt_health_factor_proptest(
        db: &mut CacheDB<EmptyDB>,
        lender: Address,
        oracle: Address,
        _attacker: Address,
    ) -> Option<U256> {
        // 1. Load storage layout (default to mock slots if Heimdall fails)
        let analysis = heimdall_analyze(&format!("{:?}", lender))
            .unwrap_or_else(|| ContractAnalysis {
                storage_slots: vec![(0, "collateral".into()), (1, "loans".into())].into_iter().collect(),
                potential_oracles: vec![],
                function_selectors: vec![],
                pseudocode: String::new(),
            });

        let price_slot = analysis.storage_slots.iter()
            .find(|(_, name)| name.contains("price"))
            .map(|(s, _)| U256::from(*s)).unwrap_or(U256::from(0));
        let coll_slot = U256::from(0);
        let debt_slot = U256::from(1);

        let true_price = db.storage(oracle, price_slot)
            .unwrap_or(U256::from(1_000_000_000_000_000_000u128));
        let collateral = db.storage(lender, coll_slot).unwrap_or(U256::ZERO);
        let debt = db.storage(lender, debt_slot).unwrap_or(U256::ZERO);

        if collateral.is_zero() || true_price.is_zero() {
            return None;
        }

        let scale = U256::from(10).pow(U256::from(18));
        let target_hf = U256::from(11) * scale / U256::from(10); // 1.1 in fixed‑point

        let mut low = U256::ZERO;
        let mut high = (collateral * true_price) / scale; // max borrowable
        if high.is_zero() {
            return None;
        }

        let mut break_amount = high;
        for _ in 0..64 {
            let mid = (low + high) / U256::from(2);
            let new_debt = debt + mid;
            if new_debt.is_zero() {
                low = mid; continue;
            }
            let hf = (collateral * true_price) / new_debt;
            if hf < target_hf {
                break_amount = mid;
                high = mid;
            } else {
                low = mid;
            }
            if high == low || high - low <= U256::from(1) {
                break;
            }
        }

        if break_amount > U256::ZERO {
            info!("🏹 HealthFactor broken at borrow > {} (profit={})", break_amount, break_amount);
            Some(break_amount)
        } else {
            info!("✅ HealthFactor intact.");
            None
        }
    }
}
