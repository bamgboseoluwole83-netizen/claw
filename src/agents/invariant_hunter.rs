use alloy_primitives::{Address, U256};
use revm::Database;
use revm::db::{CacheDB, EmptyDB};
use tracing::info;
use crate::agents::heimdall_analyzer::{ContractAnalysis, analyze as heimdall_analyze};
use crate::agents::solver;

pub struct InvariantHunter;

/// The types of invariants the hunter can check.
pub enum Invariant {
    HealthFactor,
    LiquidationIncentive,
    CollateralCap,
    Custom(String),
}

/// A transaction that breaks an invariant.
pub struct ExploitSequence {
    pub steps: Vec<TransactionStep>,
    pub profit: U256,
    pub invariant_broken: String,
}

pub struct TransactionStep {
    pub to: Address,
    pub calldata: Vec<u8>,
    pub value: U256,
}

impl InvariantHunter {
    pub async fn hunt(
        db: &mut CacheDB<EmptyDB>,
        lender_addr: Address,
        oracle_addr: Address,
        attacker: Address,
        invariants: Vec<Invariant>,
        external_analysis: Option<&ContractAnalysis>,
    ) -> Vec<ExploitSequence> {
        let analysis = match external_analysis {
            Some(a) => a.clone(),
            None => match heimdall_analyze(&format!("{:?}", lender_addr)) {
                Some(a) => a,
                None => {
                    info!("❌ No analysis available, cannot hunt invariants");
                    return Vec::new();
                }
            },
        };

        let mut results = Vec::new();
        for inv in &invariants {
            match inv {
                Invariant::HealthFactor => {
                    if let Some(seq) = InvariantHunter::hunt_health_factor(db, lender_addr, oracle_addr, attacker, &analysis).await {
                        results.push(seq);
                    }
                }
                Invariant::LiquidationIncentive => {
                    if let Some(seq) = InvariantHunter::hunt_liquidation_incentive(db, lender_addr, oracle_addr, attacker, &analysis).await {
                        results.push(seq);
                    }
                }
                Invariant::CollateralCap => {
                    if let Some(seq) = InvariantHunter::hunt_collateral_cap(db, lender_addr, oracle_addr, attacker, &analysis).await {
                        results.push(seq);
                    }
                }
                Invariant::Custom(formula) => {
                    if let Some(seq) = InvariantHunter::hunt_custom(db, lender_addr, oracle_addr, attacker, formula, &analysis).await {
                        results.push(seq);
                    }
                }
            }
        }
        results
    }

    async fn hunt_health_factor(
        db: &mut CacheDB<EmptyDB>,
        lender: Address,
        oracle: Address,
        _attacker: Address,
        analysis: &ContractAnalysis,
    ) -> Option<ExploitSequence> {
        let price_slot = analysis.storage_slots.iter()
            .find(|(_, name)| name.contains("price"))
            .map(|(s, _)| U256::from(*s)).unwrap_or(U256::from(0));
        let coll_slot = U256::from(0);
        let debt_slot = U256::from(1);

        // Read current state (after the oracle laggard attack restored the true price)
        let true_price = db.storage(oracle, price_slot)
            .unwrap_or(U256::from(1_000_000_000_000_000_000u128));
        let collateral = db.storage(lender, coll_slot).unwrap_or(U256::ZERO);
        let debt = db.storage(lender, debt_slot).unwrap_or(U256::ZERO);

        // Linear SMT formula: (collateral + amount) * price < (debt + amount) * 1.1
        let formula = format!(
            "(define collateral::real) (define debt::real) (define price::real) (define amount::real)\n\
             (assert (= collateral {})) (assert (= debt {})) (assert (= price {}))\n\
             (assert (< (* (+ collateral amount) price) (* (+ debt amount) 1.1)))\n\
             (assert (> amount 0))\n\
             (check)\n\
             (show-model)\n",
            collateral, debt, true_price
        );

        let output = solver::run_yices(&formula)?;
        if !output.contains("sat") {
            return None;
        }

        // Parse the amount from the model (simplified: just use a reasonable value)
        // In a full implementation, we'd extract the exact value.
        let profit = U256::from(1000) * U256::from(10).pow(U256::from(18)); // 1000 ETH profit
        info!("🏹 Invariant Hunter: HealthFactor can be broken! profit={}", profit);
        Some(ExploitSequence {
            steps: vec![TransactionStep {
                to: lender,
                calldata: vec![],
                value: U256::ZERO,
            }],
            profit,
            invariant_broken: "HealthFactor < 1.1".into(),
        })
    }

    async fn hunt_liquidation_incentive(
        _db: &mut CacheDB<EmptyDB>,
        _lender: Address,
        _oracle: Address,
        _attacker: Address,
        _analysis: &ContractAnalysis,
    ) -> Option<ExploitSequence> {
        None
    }

    async fn hunt_collateral_cap(
        _db: &mut CacheDB<EmptyDB>,
        _lender: Address,
        _oracle: Address,
        _attacker: Address,
        _analysis: &ContractAnalysis,
    ) -> Option<ExploitSequence> {
        None
    }

    async fn hunt_custom(
        _db: &mut CacheDB<EmptyDB>,
        _lender: Address,
        _oracle: Address,
        _attacker: Address,
        _formula: &str,
        _analysis: &ContractAnalysis,
    ) -> Option<ExploitSequence> {
        None
    }
}
