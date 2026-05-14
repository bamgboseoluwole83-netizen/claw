use alloy::primitives::U256;

use crate::agents::economic::{u256_low_u64, u256_to_f64};

#[derive(Debug, Clone)]
pub struct AMModel {
    pub reserve0: U256,
    pub reserve1: U256,
}

impl AMModel {
    pub fn new(reserve0: U256, reserve1: U256) -> Self {
        Self { reserve0, reserve1 }
    }

    /// Constant product swap with 0.3% fee: amount_in → amount_out
    pub fn swap_output(&self, amount_in: U256) -> U256 {
        if amount_in.is_zero() || self.reserve0.is_zero() {
            return U256::ZERO;
        }
        let amount_in_with_fee = amount_in * U256::from(997u64);
        let numerator = amount_in_with_fee * self.reserve1;
        let denominator = self.reserve0 * U256::from(1000u64) + amount_in_with_fee;
        if denominator.is_zero() {
            return U256::ZERO;
        }
        numerator / denominator
    }

    /// Amount needed as input to get a target output
    pub fn input_for_output(&self, target_output: U256) -> U256 {
        if target_output.is_zero() || target_output >= self.reserve1 {
            return U256::from(1u64) << 255;
        }
        let numerator = self.reserve0 * target_output * U256::from(1000u64);
        let denominator = (self.reserve1 - target_output) * U256::from(997u64);
        if denominator.is_zero() {
            return U256::from(1u64) << 255;
        }
        numerator / denominator + U256::from(1u64)
    }

    /// Price impact ratio for a given swap (0.0 = none, 1.0 = 100% impact)
    pub fn price_impact(&self, amount_in: U256) -> f64 {
        if amount_in.is_zero() || self.reserve0.is_zero() || self.reserve1.is_zero() {
            return 0.0;
        }
        let ideal = (amount_in * self.reserve1) / self.reserve0;
        let actual = self.swap_output(amount_in);
        if ideal.is_zero() {
            return 0.0;
        }
        let scale = U256::from(1_000_000_000_000u64);
        let diff = ideal.saturating_sub(actual);
        let impact_scaled = (diff * scale) / ideal;
        let impact = u256_low_u64(impact_scaled) as f64 / 1_000_000_000_000.0;
        impact.clamp(0.0, 1.0)
    }

    /// Simulate reserves after swapping `amount_in` of token0
    pub fn after_swap(&self, amount_in: U256) -> (U256, U256) {
        let output = self.swap_output(amount_in);
        (
            self.reserve0 + amount_in,
            self.reserve1.saturating_sub(output),
        )
    }

    /// Swap token1 → token0 (reciprocal direction)
    pub fn swap_output_inverse(&self, amount_in: U256) -> U256 {
        if amount_in.is_zero() || self.reserve1.is_zero() {
            return U256::ZERO;
        }
        let amount_in_with_fee = amount_in * U256::from(997u64);
        let numerator = amount_in_with_fee * self.reserve0;
        let denominator = self.reserve1 * U256::from(1000u64) + amount_in_with_fee;
        if denominator.is_zero() {
            return U256::ZERO;
        }
        numerator / denominator
    }

    /// Spot price: reserve1 / reserve0
    pub fn spot_price(&self) -> f64 {
        if self.reserve0.is_zero() {
            return 0.0;
        }
        let r0 = u256_to_f64(self.reserve0);
        let r1 = u256_to_f64(self.reserve1);
        if r0 == 0.0 {
            return 0.0;
        }
        r1 / r0
    }

    /// Total value locked in the pool (in ETH equivalent)
    pub fn tvl(&self, token0_price_eth: f64, token1_price_eth: f64) -> f64 {
        let r0 = u256_to_f64(self.reserve0);
        let r1 = u256_to_f64(self.reserve1);
        r0 * token0_price_eth + r1 * token1_price_eth
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE: U256 = U256::from_limbs([1_000_000_000_000_000_000u64, 0, 0, 0]);

    fn pool() -> AMModel {
        AMModel::new(ONE * U256::from(1000u64), ONE * U256::from(1000u64))
    }

    #[test]
    fn test_swap_output_basic() {
        let model = pool();
        assert!(model.swap_output(ONE) > U256::ZERO);
    }

    #[test]
    fn test_swap_zero_input() {
        assert_eq!(pool().swap_output(U256::ZERO), U256::ZERO);
    }

    #[test]
    fn test_swap_zero_reserves() {
        assert_eq!(AMModel::new(U256::ZERO, ONE).swap_output(ONE), U256::ZERO);
    }

    #[test]
    fn test_swap_applies_fee() {
        let ideal = |a: U256| pool().swap_output(a);
        let no_fee = AMModel::new(
            pool().reserve0 * U256::from(1000u64),
            pool().reserve1 * U256::from(1000u64),
        );
        // no_fee has larger reserves so impact is lower → output is higher
        assert!(ideal(ONE * U256::from(10u64)) < no_fee.swap_output(ONE * U256::from(10u64)));
    }

    #[test]
    fn test_price_impact_small() {
        let deep = AMModel::new(
            ONE * U256::from(1_000_000u64),
            ONE * U256::from(1_000_000u64),
        );
        let impact = deep.price_impact(ONE);
        assert!(
            impact > 0.001,
            "impact should include 0.3% fee, got {}",
            impact
        );
        assert!(
            impact < 0.01,
            "impact should be modest for deep pool, got {}",
            impact
        );
    }

    #[test]
    fn test_price_impact_large() {
        let small = AMModel::new(ONE * U256::from(100u64), ONE * U256::from(100u64));
        assert!(small.price_impact(ONE * U256::from(50u64)) > 0.1);
    }

    #[test]
    fn test_input_for_output_roundtrip() {
        let model = pool();
        let output = model.swap_output(ONE);
        let input = model.input_for_output(output);
        assert!(input >= ONE, "roundtrip needs at least original input");
    }

    #[test]
    fn test_after_swap() {
        let model = pool();
        let (r0, r1) = model.after_swap(ONE);
        assert!(r0 > model.reserve0);
        assert!(r1 < model.reserve1);
    }

    #[test]
    fn test_spot_price() {
        let model = AMModel::new(ONE * U256::from(2u64), ONE * U256::from(4u64));
        assert!((model.spot_price() - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_swap_output_symmetry() {
        let model = pool();
        let out = model.swap_output(ONE);
        // The reciprocal swap should get back less than we put in (fee)
        let back = AMModel::new(model.reserve1, model.reserve0).swap_output(out);
        assert!(back < ONE);
    }

    #[test]
    fn test_large_swap_does_not_overflow() {
        let model = AMModel::new(ONE, ONE);
        let huge = U256::MAX >> 8;
        let output = model.swap_output(huge);
        assert!(output < model.reserve1);
    }

    #[test]
    fn test_swap_output_inverse_basic() {
        let model = pool();
        let output = model.swap_output_inverse(ONE);
        assert!(output > U256::ZERO, "inverse swap should produce output");
    }

    #[test]
    fn test_swap_output_inverse_zero() {
        assert_eq!(pool().swap_output_inverse(U256::ZERO), U256::ZERO);
    }

    #[test]
    fn test_swap_output_inverse_zero_reserves() {
        assert_eq!(
            AMModel::new(ONE, U256::ZERO).swap_output_inverse(ONE),
            U256::ZERO
        );
    }
}
