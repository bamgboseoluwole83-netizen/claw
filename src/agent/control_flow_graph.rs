use petgraph::graph::{DiGraph, NodeIndex};
use crate::agents::disassembler::DisassemblyResult;
use tracing::info;

/// Represents a chunk of EVM logic between boundaries
#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub start_pc: usize,
    pub end_pc: usize,
    pub is_terminal: bool, // Does this block end in a JUMP or STOP/RETURN?
}

/// The type of arrow connecting two blocks
#[derive(Debug, Clone)]
pub enum EdgeType {
    Linear,       // Just flows to the next line
    Conditional,  // JUMPI (If true go here, if false go next)
}

/// The True Map of the contract
pub struct ControlFlowGraph {
    pub graph: DiGraph<BasicBlock, EdgeType>,
    pub entry_node: NodeIndex,
}

impl ControlFlowGraph {
    /// Builds a directional graph from the raw disassembly
    pub fn build(disassembly: &DisassemblyResult, bytecode_len: usize) -> Self {
        info!(target: "cfg", "🏗️  Building True Control Flow Graph...");
        
        let mut graph = DiGraph::new();
        let mut boundaries = disassembly.jumpdest_offsets.clone();
        
        if boundaries.is_empty() { boundaries.push(0); }
        boundaries.push(bytecode_len);
        boundaries.sort_unstable();
        boundaries.dedup();

        let mut node_indices = Vec::new();

        // 1. Create all nodes (Basic Blocks)
        for i in 0..boundaries.len() {
            let start = boundaries[i];
            let end = if i + 1 < boundaries.len() { boundaries[i + 1] } else { bytecode_len };
            
            // If the last instruction in this range is a JUMP, it's a terminal block
            let is_terminal = disassembly.unconditional_jumps.iter().any(|&pc| pc >= start && pc < end);

            let block = BasicBlock { start_pc: start, end_pc: end, is_terminal };
            let idx = graph.add_node(block);
            node_indices.push(idx);
        }

        let entry_node = node_indices[0];

        // 2. Draw the arrows (Edges)
        for i in 0..node_indices.len() {
            let current_idx = node_indices[i];
            let current_block = &graph[current_idx];

            // Check if this block contains a JUMPI
            let has_jumpi = disassembly.conditional_jumps.iter().any(|&pc| pc >= current_block.start_pc && pc < current_block.end_pc);

            if current_block.is_terminal {
                // Unconditional JUMP. Flow stops here. Do NOT connect to next block.
                // (This is what isolates WETH's deposit() from withdraw()!)
                continue;
            }

            if i + 1 < node_indices.len() {
                let next_idx = node_indices[i + 1];
                
                if has_jumpi {
                    // It's an IF statement. Draw a conditional arrow to the next block.
                    graph.add_edge(current_idx, next_idx, EdgeType::Conditional);
                } else {
                    // Normal linear flow.
                    graph.add_edge(current_idx, next_idx, EdgeType::Linear);
                }
            }
        }

        info!(target: "cfg", "✅ Graph built: {} blocks, {} edges.", graph.node_count(), graph.edge_count());
        Self { graph, entry_node }
    }
}