use alloy_primitives::{Address, B256, U256};
use revm::{
    interpreter::Interpreter,
    primitives::{AccountInfo, Bytecode},
    Database, EvmContext, Inspector,
};
use std::collections::HashMap;
use tracing::warn;

#[derive(Debug, Clone)]
pub struct DivergenceReport {
    pub divergent_slots: Vec<(U256, U256, U256)>,
    pub total_dust_wei: U256,
    pub is_asymmetric: bool,
}

#[derive(Debug, Default, Clone)]
pub struct StateRecorder {
    pub recorded_state: HashMap<U256, U256>,
}

impl<DB: Database> Inspector<DB> for StateRecorder {
    fn step(&mut self, interp: &mut Interpreter, _context: &mut EvmContext<DB>) {
        // X-RAY: Log the first 3 opcodes to prove the Inspector is alive
        if self.recorded_state.len() < 3 {
            warn!(target: "executor", "👁️ INSPECTOR ACTIVE - SEING OPCODE: {:?}", interp.current_opcode());
        }

        if interp.current_opcode() == 0x55 {
            warn!(target: "executor", "🚨 SSTORE OPCODE FOUND!");
            if let (Ok(slot), Ok(value)) = (interp.stack.peek(0), interp.stack.peek(1)) {
                self.recorded_state.insert(slot, value);
            }
        }
    }
}

pub struct ChaosDatabase<DB> {
    pub inner: DB,
    pub oracle_slot: U256,
    pub chaotic_price: U256,
}

impl<DB: Database> Database for ChaosDatabase<DB> {
    type Error = DB::Error;
    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> { self.inner.basic(address) }
    fn code_by_hash(&mut self, hash: B256) -> Result<Bytecode, Self::Error> { self.inner.code_by_hash(hash) }
    fn storage(&mut self, address: Address, slot: U256) -> Result<U256, Self::Error> {
        if slot == self.oracle_slot {
            warn!(target: "dss", "🎯 DATABASE STORAGE HIJACKED!");
            return Ok(self.chaotic_price);
        }
        self.inner.storage(address, slot)
    }
    fn block_hash(&mut self, block_number: u64) -> Result<B256, Self::Error> { self.inner.block_hash(block_number) }
}

pub struct DivergenceEngine;
impl DivergenceEngine {
    pub fn diff_states(honest: &HashMap<U256, U256>, chaotic: &HashMap<U256, U256>) -> DivergenceReport {
        let mut slots = Vec::new();
        let mut dust = U256::ZERO;
        for (s, h) in honest { if let Some(c) = chaotic.get(s) { if h != c { let d = if h > c { *h - *c } else { *c - *h }; slots.push((*s, *h, *c)); dust += d; }}}
        DivergenceReport { divergent_slots: slots, total_dust_wei: dust, is_asymmetric: false }
    }
    pub fn check_symmetry(pos: &DivergenceReport, neg: &DivergenceReport) -> bool {
        let asym = pos.total_dust_wei > neg.total_dust_wei * U256::from(2);
        if asym { warn!(target: "dss", "🚨 ASYMMETRY DETECTED!"); }
        asym
    }
}
