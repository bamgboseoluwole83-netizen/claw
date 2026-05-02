use alloy_primitives::{Address, U256};
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct ContractTrace {
    pub storage_writes: Vec<StorageWrite>,
    pub external_calls: Vec<ExternalCall>,
    pub layout: HashMap<U256, U256>,
    pub created_contracts: Vec<Address>,
}

#[derive(Debug, Clone)]
pub struct StorageWrite {
    pub slot: U256,
    pub value: U256,
    pub pc: usize,
    pub caller_checked: bool,
}

#[derive(Debug, Clone)]
pub struct ExternalCall {
    pub opcode: u8,
    pub target: Address,
    pub selector: [u8; 4],
    pub pc: usize,
}

pub async fn trace_contract(
    _forker: &crate::agents::forker::ForkerAgent,
    _target: Address,
) -> Option<ContractTrace> {
    Some(ContractTrace::default())
}
