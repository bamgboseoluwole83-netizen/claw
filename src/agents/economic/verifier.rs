use alloy::primitives::{Address, U256};
use serde_json::Value;

use crate::agents::economic::EconomicFinding;

/// Simulate an exploit on-chain using eth_call to verify profit
pub struct ExploitVerifier {
    rpc_url: String,
}

impl ExploitVerifier {
    pub fn new(rpc_url: &str) -> Self {
        Self {
            rpc_url: rpc_url.to_string(),
        }
    }

    /// Simulate all steps of an exploit via eth_call
    pub async fn verify(&self, finding: &EconomicFinding) -> VerificationResult {
        if finding.steps.is_empty() {
            return VerificationResult::unverifiable("No steps to simulate");
        }

        // Get balances before simulation
        let pre_balance = self
            .eth_get_balance(finding.target)
            .await
            .unwrap_or(U256::ZERO);

        // Try simulating each step
        let mut total_gas: u64 = 0;
        for step in &finding.steps {
            match self.simulate_step(step.target, &step.calldata, step.value).await {
                Ok(gas) => total_gas += gas,
                Err(e) => {
                    return VerificationResult::failed(
                        &finding.strategy,
                        &format!("Step simulation failed: {}", e),
                    );
                }
            }
        }

        // Get balances after simulation
        let post_balance = self
            .eth_get_balance(finding.target)
            .await
            .unwrap_or(U256::ZERO);

        let actual_profit = post_balance.saturating_sub(pre_balance);
        let expected_profit = finding.profit_estimate;

        if actual_profit >= expected_profit {
            VerificationResult::verified(finding.strategy.clone(), actual_profit, total_gas)
        } else if !actual_profit.is_zero() {
            VerificationResult::partial(
                finding.strategy.clone(),
                actual_profit,
                expected_profit,
            )
        } else {
            VerificationResult::failed(
                &finding.strategy,
                "No profit detected in simulation",
            )
        }
    }

    /// Simulate a single step via eth_call
    async fn simulate_step(
        &self,
        to: Address,
        calldata: &[u8],
        value: U256,
    ) -> Result<u64, String> {
        let client = reqwest::Client::new();

        let tx = serde_json::json!({
            "from": "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
            "to": hex::encode(to),
            "data": hex::encode(calldata),
            "value": format!("0x{:x}", value),
            "gas": "0x186a0", // 100000
        });

        let params = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_call",
            "params": [tx, "latest"]
        });

        let resp = client
            .post(&self.rpc_url)
            .json(&params)
            .send()
            .await
            .map_err(|e| format!("RPC request failed: {}", e))?;

        let body: Value = resp
            .json()
            .await
            .map_err(|e| format!("RPC parse failed: {}", e))?;

        if let Some(error) = body["error"].as_object() {
            return Err(format!("Simulation reverted: {}", error["message"]));
        }

        Ok(100_000) // estimated gas used
    }

    async fn eth_get_balance(&self, addr: Address) -> Result<U256, String> {
        let client = reqwest::Client::new();

        let params = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_getBalance",
            "params": [hex::encode(addr), "latest"]
        });

        let resp = client
            .post(&self.rpc_url)
            .json(&params)
            .send()
            .await
            .map_err(|e| format!("RPC failed: {}", e))?;

        let body: Value = resp
            .json()
            .await
            .map_err(|e| format!("Parse failed: {}", e))?;

        let result = body["result"]
            .as_str()
            .ok_or("No balance result")?;

        let hex_str = result.trim_start_matches("0x");
        let bytes = hex::decode(hex_str).map_err(|e| format!("Hex decode: {}", e))?;

        let mut arr = [0u8; 32];
        let start = 32usize.saturating_sub(bytes.len());
        arr[start..].copy_from_slice(&bytes);
        Ok(U256::from_be_bytes(arr))
    }
}

#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub strategy: String,
    pub verified: bool,
    pub actual_profit: U256,
    pub expected_profit: U256,
    pub gas_used: u64,
    pub error: Option<String>,
}

impl VerificationResult {
    pub fn verified(strategy: String, profit: U256, gas: u64) -> Self {
        Self {
            strategy,
            verified: true,
            actual_profit: profit,
            expected_profit: profit,
            gas_used: gas,
            error: None,
        }
    }

    pub fn partial(strategy: String, actual: U256, expected: U256) -> Self {
        Self {
            strategy,
            verified: false,
            actual_profit: actual,
            expected_profit: expected,
            gas_used: 0,
            error: Some(format!(
                "Partial profit: {} vs expected {}",
                crate::agents::economic::u256_to_f64(actual),
                crate::agents::economic::u256_to_f64(expected),
            )),
        }
    }

    pub fn failed(strategy: &str, error: &str) -> Self {
        Self {
            strategy: strategy.to_string(),
            verified: false,
            actual_profit: U256::ZERO,
            expected_profit: U256::ZERO,
            gas_used: 0,
            error: Some(error.to_string()),
        }
    }

    pub fn unverifiable(reason: &str) -> Self {
        Self {
            strategy: String::new(),
            verified: false,
            actual_profit: U256::ZERO,
            expected_profit: U256::ZERO,
            gas_used: 0,
            error: Some(reason.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::economic::{EconStep, EconomicFinding};

    fn sample_finding() -> EconomicFinding {
        EconomicFinding {
            strategy: "test".to_string(),
            target: Address::ZERO,
            profit_estimate: U256::from(1_000_000_000_000_000_000u128),
            steps: vec![EconStep {
                target: Address::ZERO,
                calldata: vec![0xde, 0xad],
                value: U256::ZERO,
                description: "test step".to_string(),
            }],
            confidence: 0.5,
            description: "test".to_string(),
        }
    }

    #[test]
    fn test_verification_result_verified() {
        let r = VerificationResult::verified("test".to_string(), U256::from(100u64), 50000);
        assert!(r.verified);
        assert!(r.error.is_none());
    }

    #[test]
    fn test_verification_result_failed() {
        let r = VerificationResult::failed("test", "reverted");
        assert!(!r.verified);
        assert_eq!(r.error.unwrap(), "reverted");
    }

    #[test]
    fn test_verification_result_partial() {
        let r = VerificationResult::partial("test".to_string(), U256::from(50u64), U256::from(100u64));
        assert!(!r.verified);
    }

    #[test]
    fn test_verification_result_unverifiable() {
        let r = VerificationResult::unverifiable("no steps");
        assert!(!r.verified);
    }

    #[test]
    fn test_empty_steps_unverifiable() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let verifier = ExploitVerifier::new("http://localhost:8545");
        let mut finding = sample_finding();
        finding.steps.clear();
        let result = rt.block_on(verifier.verify(&finding));
        assert!(!result.verified);
    }
}
