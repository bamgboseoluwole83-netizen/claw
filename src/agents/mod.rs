// This file tells Rust that all our agent files are part of the "agents" module.
// We declare them as "pub" so the Controller can access them.

pub mod discovery;
pub mod fetcher;
pub mod disassembler;
pub mod storage_collision;
pub mod access_control;
pub mod oracle_stale;
pub mod cross_ghost_reentrancy;
pub mod invariant_precision;
pub mod poc_simulator;
pub mod execution_agent;
pub mod reporter;
