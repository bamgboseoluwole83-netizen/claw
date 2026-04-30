use crate::agents::disassembler::DisassemblyResult;
use crate::agents::control_flow_graph::ControlFlowGraph;
use crate::agents::symbolic_stack::{SymbolicStack, SymbolicValue, is_precision_loss_formula};
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct AssassinationPlan {
    pub vuln_class: String,
    pub chaos_delta: u64,
    pub is_high_confidence: bool,
    pub oracle_pc: Option<usize>,
    pub math_pc: Option<usize>,
    pub sstore_pc: Option<usize>,
}

pub struct HeuristicEngine;

impl HeuristicEngine {
    pub fn profile_target(bytecode: &[u8], disassembly: &DisassemblyResult, cfg: &ControlFlowGraph) -> Option<AssassinationPlan> {
        info!(target: "heuristic", "🧠 Walking True CFG with Symbolic Tracker...");
        let start_node = cfg.entry_node;
        
        let initial_stack = SymbolicStack::new();
        
        if let Some(plan) = Self::trace_path_dfs(bytecode, disassembly, cfg, start_node, initial_stack) {
            return Some(plan);
        }

        info!(target: "heuristic", "Target anatomy clean. No data-connected flaw paths.");
        None
    }

    fn trace_path_dfs(
        bytecode: &[u8],
        disassembly: &DisassemblyResult,
        cfg: &ControlFlowGraph,
        current_node: petgraph::graph::NodeIndex,
        mut stack: SymbolicStack,
    ) -> Option<AssassinationPlan> {
        let block = &cfg.graph[current_node];
        let mut last_math_pc = None;

        let mut pc = block.start_pc;
        while pc < block.end_pc && pc < bytecode.len() {
            let op = bytecode[pc];

            match op {
                0x54 => {
                    stack.push(SymbolicValue::Sload(U256::ZERO)); 
                }
                
                0x60..=0x7f => {
                    let push_size = (op - 0x60 + 1) as usize;
                    let mut val_bytes = [0u8; 32];
                    let end = std::cmp::min(pc + 1 + push_size, bytecode.len());
                    val_bytes[(32 - push_size)..].copy_from_slice(&bytecode[pc + 1..end]);
                    stack.push(SymbolicValue::Constant(U256::from_be_bytes(val_bytes)));
                    pc += push_size; 
                }

                0x04 => {
                    last_math_pc = Some(pc);
                    let b = stack.pop().unwrap_or(SymbolicValue::Unknown);
                    let a = stack.pop().unwrap_or(SymbolicValue::Unknown);
                    stack.push(SymbolicValue::Div(Box::new(a), Box::new(b)));
                }

                0x06 => {
                    last_math_pc = Some(pc);
                    let _ = stack.pop();
                    let _ = stack.pop();
                    stack.push(SymbolicValue::Unknown); 
                }

                0x55 => {
                    let _slot = stack.pop(); 
                    if let Some(stored_value) = stack.pop() {
                        if is_precision_loss_formula(&stored_value) {
                            warn!(target: "heuristic", "🚨 SYMBOLIC LOCK: Data flows SLOAD -> DIV -> SSTORE at PC {}", pc);
                            return Some(AssassinationPlan {
                                vuln_class: "Data-Flow Precision Loss".to_string(),
                                chaos_delta: 1, 
                                is_high_confidence: true,
                                oracle_pc: None, 
                                math_pc: last_math_pc,
                                sstore_pc: Some(pc),
                            });
                        }
                    }
                }

                0x90..=0x9f => stack.swap((op - 0x90 + 1) as usize), 
                0x80..=0x8f => stack.dup((op - 0x80 + 1) as usize),  
                0x50 => { let _ = stack.pop(); }

                _ => {}
            }
            pc += 1;
        }

        if block.is_terminal {
            return None;
        }

        for edge in cfg.graph.edges(current_node) {
            let target_node = edge.target();
            if let Some(plan) = Self::trace_path_dfs(bytecode, disassembly, cfg, target_node, stack.clone()) {
                return Some(plan);
            }
        }

        None
    }
}