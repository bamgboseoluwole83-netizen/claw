pub mod amm;
pub mod flashloan;
pub mod graph;
pub mod pool_state;
pub mod strategies;
pub mod verifier;

use alloy::primitives::{Address, U256};
use std::collections::HashMap;

use crate::agents::finding::{Finding, ToolKind};

use self::graph::ContractGraph;
use self::pool_state::PoolState;
use self::strategies::exploit_strategies;
use self::verifier::ExploitVerifier;

/// Convert U256 low 128 bits to f64 for display / comparison
pub fn u256_to_f64(u: U256) -> f64 {
    let bytes = u.to_be_bytes::<32>();
    let low = u128::from_be_bytes(bytes[16..32].try_into().unwrap_or([0u8; 16]));
    low as f64
}

/// Extract low u64 from U256
pub fn u256_low_u64(u: U256) -> u64 {
    let bytes = u.to_be_bytes::<32>();
    u64::from_be_bytes(bytes[24..32].try_into().unwrap_or([0u8; 8]))
}

pub struct EconomicSimulator {
    rpc_url: String,
    pools: HashMap<Address, PoolState>,
    graph: ContractGraph,
}

impl EconomicSimulator {
    pub fn new(rpc_url: &str) -> Self {
        Self {
            rpc_url: rpc_url.to_string(),
            pools: HashMap::new(),
            graph: ContractGraph::new(),
        }
    }

    pub async fn analyze(
        &mut self,
        target: Address,
        proxy_address: Option<Address>,
        bytecode: &[u8],
    ) -> Vec<EconomicFinding> {
        let mut findings = Vec::new();

        // Build contract graph from bytecode
        self.graph = ContractGraph::from_bytecode(target, bytecode);
        self.graph.expand(&self.rpc_url).await;

        // Discover pools from graph + bytecode
        let all_candidates = self.discover_all_pools(bytecode);
        for &pool_addr in &all_candidates {
            if !self.pools.contains_key(&pool_addr) {
                if let Ok(state) = PoolState::fetch(&self.rpc_url, pool_addr).await {
                    if state.is_valid() {
                        self.pools.insert(pool_addr, state);
                    }
                }
            }
        }

        // Also try to find pools via the graph's recursive discovery
        let graph_pools = self.graph.find_pools(&self.rpc_url).await;
        for pool in graph_pools {
            if !self.pools.contains_key(&pool.address) {
                self.pools.insert(pool.address, pool);
            }
        }

        if self.pools.is_empty() {
            return findings;
        }

        let proxy = proxy_address.unwrap_or(target);
        for strategy in exploit_strategies() {
            if let Some(result) =
                (strategy.evaluate)(target, proxy, &self.pools, &self.graph)
            {
                findings.push(result);
            }
        }

        // Phase 2b: Verify findings on-chain
        if !findings.is_empty() {
            let verifier = ExploitVerifier::new(&self.rpc_url);
            let mut verified_findings = Vec::new();
            for finding in &findings {
                let v_result = verifier.verify(finding).await;
                if v_result.verified {
                    tracing::info!(
                        "   ✅ Verified economic exploit: {} — profit: {:.6} ETH (gas: {})",
                        finding.strategy,
                        crate::agents::economic::u256_to_f64(v_result.actual_profit) / 1e18,
                        v_result.gas_used,
                    );
                    verified_findings.push(finding.clone());
                } else if v_result.error.as_deref() == Some("No steps to simulate") {
                    // Can't simulate (missing calldata) — keep finding with lowered confidence
                    let mut partial = finding.clone();
                    partial.confidence *= 0.5;
                    verified_findings.push(partial);
                } else {
                    tracing::info!(
                        "   ❌ Unverified economic exploit: {} — {}",
                        finding.strategy,
                        v_result.error.unwrap_or_default(),
                    );
                }
            }
            return verified_findings;
        }

        findings
    }

    fn discover_all_pools(&self, bytecode: &[u8]) -> Vec<Address> {
        let mut candidates = Vec::new();
        let mut i = 0;
        while i < bytecode.len() {
            if bytecode[i] == 0x73 {
                if i + 1 + 20 <= bytecode.len() {
                    let addr = Address::from_slice(&bytecode[i + 1..i + 1 + 20]);
                    if !candidates.contains(&addr) && !addr.is_zero() {
                        candidates.push(addr);
                    }
                }
            }
            i += 1;
        }
        candidates.truncate(30);
        candidates
    }
}

#[derive(Debug, Clone)]
pub struct EconomicFinding {
    pub strategy: String,
    pub target: Address,
    pub profit_estimate: U256,
    pub steps: Vec<EconStep>,
    pub confidence: f64,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct EconStep {
    pub target: Address,
    pub calldata: Vec<u8>,
    pub value: U256,
    pub description: String,
}

impl EconomicFinding {
    pub fn to_finding(&self) -> Finding {
        Finding {
            tool: ToolKind::Economic,
            severity: 9.0,
            confidence: self.confidence,
            description: format!(
                "[economic] {} — profit: {:.6} ETH, {} steps",
                self.description,
                u256_to_f64(self.profit_estimate) / 1e18,
                self.steps.len()
            ),
            target: self.target,
            calldata: self
                .steps
                .first()
                .map(|s| alloy::primitives::Bytes::from(s.calldata.clone())),
            evidence: self
                .steps
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    format!(
                        "STEP{}:{}:{}:{}",
                        i,
                        hex::encode(s.target.as_slice()),
                        hex::encode(&s.calldata),
                        s.description
                    )
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u256_to_f64() {
        let u = U256::from(1_000_000_000_000_000_000u128);
        let f = u256_to_f64(u);
        assert!((f - 1e18).abs() < 1.0, "should convert U256 to f64");
    }

    #[test]
    fn test_u256_low_u64() {
        let u = U256::from(42u64);
        assert_eq!(u256_low_u64(u), 42u64);
    }

    #[test]
    fn test_discover_pools_empty_bytecode() {
        let sim = EconomicSimulator::new("http://localhost:8545");
        assert!(sim.discover_all_pools(&[]).is_empty());
    }

    #[test]
    fn test_discover_pools_with_address() {
        let sim = EconomicSimulator::new("http://localhost:8545");
        let mut bytecode = vec![0x73u8];
        bytecode.extend(std::iter::repeat(0x11u8).take(20));
        bytecode.push(0x00);
        let pools = sim.discover_all_pools(&bytecode);
        assert_eq!(pools.len(), 1);
    }

    #[test]
    fn test_economic_finding_to_finding() {
        let finding = EconomicFinding {
            strategy: "test".to_string(),
            target: Address::ZERO,
            profit_estimate: U256::from(1_000_000_000_000_000_000u128),
            steps: vec![EconStep {
                target: Address::ZERO,
                calldata: vec![0xde, 0xad],
                value: U256::ZERO,
                description: "step 1".to_string(),
            }],
            confidence: 0.5,
            description: "test exploit".to_string(),
        };
        let f = finding.to_finding();
        assert_eq!(f.tool, ToolKind::Economic);
        assert!(f.description.contains("test exploit"));
    }

    #[test]
    fn test_discover_pools_no_duplicates() {
        let sim = EconomicSimulator::new("http://localhost:8545");
        let mut bytecode = Vec::new();
        bytecode.push(0x73);
        bytecode.extend(std::iter::repeat(0x11u8).take(20));
        bytecode.push(0x73);
        bytecode.extend(std::iter::repeat(0x11u8).take(20));
        let pools = sim.discover_all_pools(&bytecode);
        assert_eq!(pools.len(), 1, "should deduplicate addresses");
    }
}
