use alloy_primitives::{Address, U256};
use revm::db::{CacheDB, EmptyDB};

#[derive(Debug, Clone, Default)]
pub struct RevenueReport {
    pub attack: String,
    pub attacker: Address,
    pub target: Address,
    pub eth_gained: U256,
    pub tokens_gained: Vec<(Address, U256)>,
    pub gas_cost_wei: U256,
    pub net_profit_wei: U256,
    pub viability: f64,
}

pub struct RevenueCalc;

impl RevenueCalc {
    pub fn compute(
        before: &CacheDB<EmptyDB>,
        after: &CacheDB<EmptyDB>,
        attacker: Address,
        target: Address,
        gas_used: u64,
        gas_price: U256,
        description: &str,
    ) -> RevenueReport {
        let before_bal = before
            .accounts
            .get(&attacker)
            .map(|a| a.info.balance)
            .unwrap_or(U256::ZERO);
        let after_bal = after
            .accounts
            .get(&attacker)
            .map(|a| a.info.balance)
            .unwrap_or(U256::ZERO);
        let eth_gained = after_bal.saturating_sub(before_bal);

        let tokens_gained = Vec::new();

        let gas_cost = U256::from(gas_used) * gas_price;
        let net = eth_gained.saturating_sub(gas_cost);
        let viability = if gas_cost.is_zero() {
            0.0
        } else {
            let eth_u128: u128 = eth_gained.to();
            let gas_u128: u128 = gas_cost.to();
            eth_u128 as f64 / gas_u128 as f64
        };

        RevenueReport {
            attack: description.to_string(),
            attacker,
            target,
            eth_gained,
            tokens_gained,
            gas_cost_wei: gas_cost,
            net_profit_wei: net,
            viability,
        }
    }
}
