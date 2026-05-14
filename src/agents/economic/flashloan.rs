use alloy::primitives::U256;

const FLASH_LOAN_FEE_BPS: u64 = 9; // Uniswap V2: 0.09% = 9 bps

#[derive(Debug, Clone)]
pub struct FlashLoanPool {
    pub pool_address: String,
    pub max_borrow: U256,
    pub fee_bps: u64,
}

impl FlashLoanPool {
    pub fn new(pool_address: String, reserve: U256) -> Self {
        Self {
            pool_address,
            max_borrow: reserve,
            fee_bps: FLASH_LOAN_FEE_BPS,
        }
    }

    /// Total cost to borrow: amount + fee
    pub fn repay_amount(&self, borrow: U256) -> U256 {
        borrow + (borrow * U256::from(self.fee_bps)) / U256::from(10000u64)
    }

    /// Maximum profitable borrow given available capital and fee
    pub fn optimal_borrow(&self, capital: U256) -> U256 {
        capital.min(self.max_borrow)
    }
}

/// Result of a flash loan simulation
#[derive(Debug, Clone)]
pub struct FlashLoanSimulation {
    pub borrow_amount: U256,
    pub repay_amount: U256,
    pub fee: U256,
    pub is_feasible: bool,
}

/// Simulate a flash loan: can we borrow, manipulate, exploit, and repay?
pub fn simulate_flash_loan(
    pool: &FlashLoanPool,
    capital: U256,
    exploit_gain: U256,
) -> FlashLoanSimulation {
    let borrow = pool.optimal_borrow(capital);
    let repay = pool.repay_amount(borrow);
    let fee = repay.saturating_sub(borrow);

    FlashLoanSimulation {
        borrow_amount: borrow,
        repay_amount: repay,
        fee,
        is_feasible: exploit_gain >= repay,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flash_loan_pool_repay() {
        let pool = FlashLoanPool::new(
            "0x0".to_string(),
            U256::from(1_000_000_000_000_000_000_000u128),
        );
        let repay = pool.repay_amount(U256::from(1_000_000_000_000_000_000u128));
        assert!(
            repay > U256::from(1_000_000_000_000_000_000u128),
            "repay > borrow"
        );
    }

    #[test]
    fn test_flash_loan_simulation_profitable() {
        let pool = FlashLoanPool::new(
            "0x0".to_string(),
            U256::from(1_000_000_000_000_000_000_000u128),
        );
        let result = simulate_flash_loan(
            &pool,
            U256::from(100_000_000_000_000_000_000u128),
            U256::from(200_000_000_000_000_000_000u128),
        );
        assert!(result.is_feasible, "gain > repay should be feasible");
    }

    #[test]
    fn test_flash_loan_simulation_unprofitable() {
        let pool = FlashLoanPool::new("0x0".to_string(), U256::from(1_000_000_000_000_000_000u128));
        let result = simulate_flash_loan(
            &pool,
            U256::from(100_000_000_000_000_000_000u128),
            U256::ZERO,
        );
        assert!(!result.is_feasible, "no gain should not be feasible");
    }

    #[test]
    fn test_optimal_borrow_capped() {
        let pool = FlashLoanPool::new("0x0".to_string(), U256::from(100u64));
        assert_eq!(pool.optimal_borrow(U256::from(1000u64)), U256::from(100u64));
    }

    #[test]
    fn test_fee_calculation() {
        let pool = FlashLoanPool::new("0x0".to_string(), U256::MAX);
        let repay = pool.repay_amount(U256::from(1_000_000_000_000_000_000_000u128));
        let fee = repay - U256::from(1_000_000_000_000_000_000_000u128);
        assert_eq!(
            fee,
            U256::from(900_000_000_000_000_000u128),
            "0.09% of 1000 ETH = 0.9 ETH"
        );
    }
}
