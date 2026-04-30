use alloy_primitives::{Address, U256};
use serde::{Deserialize, Serialize};

/// The heart of the system. Every agent communicates using this enum.
/// Notice PrecisionDust is isolated — this triggers the private MEV path, not the bounty path.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VulnType {
    // Public Bounty Targets
    AccessControl,
    OracleStale,
    StorageCollision,
    CrossContractLogicBomb,
    GhostReentrancy,
    InvariantBreak,
    
    // Private Execution Target (The Wealth Plan)
    PrecisionDust,
}

/// A proven vulnerability found by the analysis agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub vuln_type: VulnType,
    pub target_address: Address,
    pub block_found: u64,
    pub severity: String, // "High", "Medium", "Low"
    
    // The Undeniable Proof Stack
    pub math_proof: String,        // The Wad/Ray breakdown
    pub poc_calldata: Vec<u8>,     // Raw bytes to trigger the bug
    
    // Specific to PrecisionDust (Private Path)
    pub dust_amount_per_loop: U256, // The microscopic amount drained per iteration
    pub profit_estimate_wei: U256,  // Total after 10,000 loops
}

/// The payload passed from agent to agent during analysis.
#[derive(Debug, Clone)]
pub struct AnalysisContext {
    pub bytecode: Vec<u8>,
    pub target_address: Address,
    pub block_number: u64,
    pub is_high_value: bool, // Flagged by Discovery agent
}

/// Configuration loaded from config.rs at startup.
#[derive(Debug, Clone)]
pub struct DestroyerConfig {
    pub drpc_url: String,
    pub flashbots_relay_url: String,
    pub private_key: String,
    pub balancer_vault: Address, // 0xBA12222222228d8Ba445958a75a0704d566BF2C8
}
