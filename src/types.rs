use alloy_primitives::{Address, U256};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VulnType {
    AccessControl,
    OracleStale,
    StorageCollision,
    CrossContractLogicBomb,
    GhostReentrancy,
    InvariantBreak,
    PrecisionDust,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub vuln_type: VulnType,
    pub target_address: Address,
    pub block_found: u64,
    pub severity: String,
    pub math_proof: String,
    pub poc_calldata: Vec<u8>,
    pub dust_amount_per_loop: U256,
    pub profit_estimate_wei: U256,
}