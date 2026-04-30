use alloy_primitives::U256;
use petgraph::visit::EdgeRef;
use tracing::info;

use crate::agents::disassembler::DisassemblyResult;
use crate::agents::control_flow_graph::{ControlFlowGraph, EdgeType};
use crate::agents::symbolic_stack::{SymbolicStack, SymbolicValue, ArithmeticOp};
use crate::agents::abi_router::FunctionBlock;



#[derive(Debug, Clone)]
pub enum SymbolicConstraint {
    IsZero(SymbolicValue),
    NonZero(SymbolicValue),
}

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

impl AssassinEngine {
    /// Analyze a single function using CFG trace first, then linear fallback.
    pub fn profile_function(
        bytecode: &[u8],
        disassembly: &DisassemblyResult,
        cfg: &ControlFlowGraph,
        func: &FunctionBlock,
    ) -> Option<AssassinationPlan> {
        if let Some(node) = find_node_for_pc(cfg, func.start_pc) {
            let stack = SymbolicStack::new();
            let result = Self::trace_path_dfs(
                bytecode, disassembly, cfg, node, stack, 0, vec![],
            );
            if result.is_some() {
                return result;
            }
        }
        info!(target: "assassin", "⚡ Linear fallback for {:?}", func.selector);
        Self::linear_scan_function(bytecode, func.start_pc, func.end_pc)
    }

    fn trace_path_dfs(
        bytecode: &[u8],
        disassembly: &DisassemblyResult,
        cfg: &ControlFlowGraph,
        current_node: petgraph::graph::NodeIndex,
        mut stack: SymbolicStack,
        depth: usize,
        constraints: Vec<SymbolicConstraint>,
    ) -> Option<AssassinationPlan> {
        if depth > 40 { return None; }

        let block = &cfg.graph[current_node];
        let mut pc = block.start_pc;
        let mut last_math_pc = None;

        while pc < block.end_pc && pc < bytecode.len() {
            let op = bytecode[pc];
            match op {
                0x54 => {
                    stack.push(SymbolicValue::Sload(U256::ZERO));
                    pc += 1;
                }
                0x60..=0x7f => {
                    let push_size = (op - 0x60 + 1) as usize;
                    let mut val_bytes = [0u8; 32];
                    if pc + 1 + push_size <= bytecode.len() {
                        val_bytes[(32 - push_size)..].copy_from_slice(&bytecode[pc+1..pc+1+push_size]);
                        stack.push(SymbolicValue::Constant(U256::from_be_bytes(val_bytes)));
                    }
                    pc += 1 + push_size;      // correct advancement
                    continue;                  // skip the pc+=1 at bottom
                }
                0x37 => {
                    let _ = stack.pop();
                    stack.push(SymbolicValue::Calldata { offset: 4 });
                    pc += 1;
                }
                0x01 | 0x02 | 0x03 | 0x04 | 0x06 => {
                    last_math_pc = Some(pc);
                    let b = stack.pop().unwrap_or(SymbolicValue::Unknown);
                    let a = stack.pop().unwrap_or(SymbolicValue::Unknown);
                    let op = match op {
                        0x01 => ArithmeticOp::Add,
                        0x02 => ArithmeticOp::Mul,
                        0x03 => ArithmeticOp::Sub,
                        0x04 => ArithmeticOp::Div,
                        0x06 => ArithmeticOp::Mod,
                        _ => unreachable!(),
                    };
                    stack.push(SymbolicValue::Arithmetic {
                        op,
                        left: Box::new(a),
                        right: Box::new(b),
                    });
                    pc += 1;
                }
                0x55 => {
                    let _slot = stack.pop();
                    if let Some(stored_value) = stack.pop() {
                        if stored_value.contains_div() && stored_value.has_calldata() && stored_value.has_sload() {
                            let drain = Self::calculate_max_drain(&stored_value);
                            let delta = drain.as_limbs()[0];
                            return Some(AssassinationPlan {
                                vuln_class: "Data-Flow Precision Loss".to_string(),
                                chaos_delta: delta,
                                is_high_confidence: true,
                                oracle_pc: None,
                                math_pc: last_math_pc,
                                sstore_pc: Some(pc),
                                estimated_max_drain_per_tx: drain,
                                symbolic_formula: Some(stored_value),
                                vulnerable_function_selector: None,
                                path_constraints: constraints.clone(),
                            });
                        }
                        if stored_value.contains_div() && stored_value.has_calldata() {
                            return Some(AssassinationPlan {
                                vuln_class: "Potential Precision Loss".to_string(),
                                chaos_delta: 0,
                                is_high_confidence: false,
                                oracle_pc: None,
                                math_pc: last_math_pc,
                                sstore_pc: Some(pc),
                                estimated_max_drain_per_tx: U256::ZERO,
                                symbolic_formula: Some(stored_value),
                                vulnerable_function_selector: None,
                                path_constraints: constraints.clone(),
                            });
                        }
                    }
                    pc += 1;
                }
                0x90..=0x9f => { stack.swap((op - 0x90 + 1) as usize); pc += 1; }
                0x80..=0x8f => { stack.dup((op - 0x80 + 1) as usize); pc += 1; }
                0x50 => { let _ = stack.pop(); pc += 1; }
                _ => { pc += 1; }
            }
        }

        if block.is_terminal { return None; }

        for edge in cfg.graph.edges(current_node) {
            let target = edge.target();
            let edge_type = edge.weight();
            let mut new_constraints = constraints.clone();
            if let EdgeType::Conditional = edge_type {
                let mut fork = stack.clone();
                let cond = fork.pop().unwrap_or(SymbolicValue::Unknown);
                new_constraints.push(SymbolicConstraint::NonZero(cond));
            }
            if let Some(plan) = Self::trace_path_dfs(
                bytecode, disassembly, cfg, target,
                stack.clone(), depth + 1, new_constraints,
            ) {
                return Some(plan);
            }
        }
        None
    }

    fn linear_scan_function(
        bytecode: &[u8],
        start_pc: usize,
        end_pc: usize,
    ) -> Option<AssassinationPlan> {
        let mut stack = SymbolicStack::new();
        let mut last_math_pc = None;
        let mut pc = start_pc;

        while pc < end_pc && pc < bytecode.len() {
            let op = bytecode[pc];
            match op {
                0x54 => {
                    stack.push(SymbolicValue::Sload(U256::ZERO));
                    pc += 1;
                }
                0x60..=0x7f => {
                    let push_size = (op - 0x60 + 1) as usize;
                    let mut val_bytes = [0u8; 32];
                    if pc + 1 + push_size <= bytecode.len() {
                        val_bytes[(32 - push_size)..].copy_from_slice(&bytecode[pc+1..pc+1+push_size]);
                        stack.push(SymbolicValue::Constant(U256::from_be_bytes(val_bytes)));
                    }
                    pc += 1 + push_size;
                    continue;
                }
                0x37 => {
                    let _ = stack.pop();
                    stack.push(SymbolicValue::Calldata { offset: 4 });
                    pc += 1;
                }
                0x01 | 0x02 | 0x03 | 0x04 | 0x06 => {
                    last_math_pc = Some(pc);
                    let b = stack.pop().unwrap_or(SymbolicValue::Unknown);
                    let a = stack.pop().unwrap_or(SymbolicValue::Unknown);
                    let op = match op {
                        0x01 => ArithmeticOp::Add,
                        0x02 => ArithmeticOp::Mul,
                        0x03 => ArithmeticOp::Sub,
                        0x04 => ArithmeticOp::Div,
                        0x06 => ArithmeticOp::Mod,
                        _ => unreachable!(),
                    };
                    stack.push(SymbolicValue::Arithmetic {
                        op,
                        left: Box::new(a),
                        right: Box::new(b),
                    });
                    pc += 1;
                }
                0x55 => {
                    let _slot = stack.pop();
                    if let Some(stored_value) = stack.pop() {
                        if stored_value.contains_div() && stored_value.has_calldata() && stored_value.has_sload() {
                            let drain = Self::calculate_max_drain(&stored_value);
                            let delta = drain.as_limbs()[0];
                            return Some(AssassinationPlan {
                                vuln_class: "Data-Flow Precision Loss".to_string(),
                                chaos_delta: delta,
                                is_high_confidence: true,
                                oracle_pc: None,
                                math_pc: last_math_pc,
                                sstore_pc: Some(pc),
                                estimated_max_drain_per_tx: drain,
                                symbolic_formula: Some(stored_value),
                                vulnerable_function_selector: None,
                                path_constraints: vec![],
                            });
                        }
                        if stored_value.contains_div() && stored_value.has_calldata() {
                            return Some(AssassinationPlan {
                                vuln_class: "Potential Precision Loss".to_string(),
                                chaos_delta: 0,
                                is_high_confidence: false,
                                oracle_pc: None,
                                math_pc: last_math_pc,
                                sstore_pc: Some(pc),
                                estimated_max_drain_per_tx: U256::ZERO,
                                symbolic_formula: Some(stored_value),
                                vulnerable_function_selector: None,
                                path_constraints: vec![],
                            });
                        }
                    }
                    pc += 1;
                }
                0x90..=0x9f => { stack.swap((op - 0x90 + 1) as usize); pc += 1; }
                0x80..=0x8f => { stack.dup((op - 0x80 + 1) as usize); pc += 1; }
                0x50 => { let _ = stack.pop(); pc += 1; }
                _ => { pc += 1; }
            }
        }
        None
    }

    fn calculate_max_drain(val: &SymbolicValue) -> U256 {
        if let Some(divisor) = val.extract_divisor() {
            if divisor > U256::ZERO { divisor - U256::from(1) } else { U256::ZERO }
        } else {
            U256::ZERO
        }
    }
}

fn find_node_for_pc(cfg: &ControlFlowGraph, pc: usize) -> Option<petgraph::graph::NodeIndex> {
    for node in cfg.graph.node_indices() {
        let block = &cfg.graph[node];
        if pc >= block.start_pc && pc < block.end_pc {
            return Some(node);
        }
    }
    None
}