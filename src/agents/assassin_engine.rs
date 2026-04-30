use alloy_primitives::U256;
use crate::agents::symbolic_stack::SymbolicValue;

#[derive(Debug, Clone)]
pub enum SymbolicConstraint { IsZero(SymbolicValue), NonZero(SymbolicValue) }

#[derive(Debug, Clone)]
pub struct AssassinationPlan {
    pub vuln_class: String,
    pub chaos_delta: u64,
    pub is_high_confidence: bool,
    pub oracle_pc: Option<usize>,
    pub math_pc: Option<usize>,
    pub sstore_pc: Option<usize>,
    pub estimated_max_drain_per_tx: U256,
    pub symbolic_formula: Option<SymbolicValue>,
    pub vulnerable_function_selector: Option<[u8; 4]>,
    pub path_constraints: Vec<SymbolicConstraint>,
}

pub struct AssassinEngine;
