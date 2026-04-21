use alloy_primitives::{Address, B256, U256};
use revm::{
    interpreter::Interpreter,
    primitives::{AccountInfo, Bytecode},
    Database, EvmContext, Inspector,
};
use std::collections::HashMap;
use tracing::warn;

/// The final state comparison report.
#[derive(Debug, Clone)]
pub struct DivergenceReport {
    pub divergent_slots: Vec<(U256, U256, U256)>,
    pub total_dust_wei: U256,
    pub is_asymmetric: bool,
}

/// 1. The Observer
#[derive(Debug, Default)]
pub struct StateRecorder {
    pub recorded_state: HashMap<U256, U256>,
}

impl<DB: Database> Inspector<DB> for StateRecorder {
    fn step(&mut self, interp: &mut Interpreter, _context: &mut EvmContext<DB>) {
        let opcode = interp.current_opcode();
        if opcode == 0x55 {
            if let (Ok(slot), Ok(value)) = (interp.stack.peek(0), interp.stack.peek(1)) {
                self.recorded_state.insert(slot, value);
            }
        }
    }
}

/// 2. The God-Tier Wrapper
pub struct ChaosDatabase<DB> {
    pub inner: DB,
    pub oracle_slot: U256,
    pub chaotic_price: U256,
}

impl<DB: Database> Database for ChaosDatabase<DB> {
    type Error = DB::Error;

    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        self.inner.basic(address)
    }

    fn code_by_hash(&mut self, hash: B256) -> Result<Bytecode, Self::Error> {
        self.inner.code_by_hash(hash)
    }

    fn storage(&mut self, address: Address, slot: U256) -> Result<U256, Self::Error> {
        if slot == self.oracle_slot {
            warn!(target: "dss", "🎯 DATABASE STORAGE HIJACKED! Injecting chaotic price.");
            return Ok(self.chaotic_price);
        }
        self.inner.storage(address, slot)
    }

    // THE FINAL BOSS: The last required trait method
    fn block_hash(&mut self, block_number: u64) -> Result<B256, Self::Error> {
        self.inner.block_hash(block_number)
    }
}

/// 3. The Engine
pub struct DivergenceEngine;

impl DivergenceEngine {
    pub fn diff_states(
        honest_state: &HashMap<U256, U256>, 
        chaotic_state: &HashMap<U256, U256>
    ) -> DivergenceReport {
        let mut divergent_slots = Vec::new();
        let mut total_dust = U256::ZERO;

        for (slot, honest_val) in honest_state {
            if let Some(chaotic_val) = chaotic_state.get(slot) {
                if honest_val != chaotic_val {
                    let diff = if honest_val > chaotic_val {
                        honest_val - chaotic_val
                    } else {
                        chaotic_val - honest_val
                    };
                    divergent_slots.push((*slot, *honest_val, *chaotic_val));
                    total_dust += diff;
                }
            }
        }

        DivergenceReport {
            divergent_slots,
            total_dust_wei: total_dust,
            is_asymmetric: false,
        }
    }

    pub fn check_symmetry(positive_report: &DivergenceReport, negative_report: &DivergenceReport) -> bool {
        let pos_dust = positive_report.total_dust_wei;
        let neg_dust = negative_report.total_dust_wei;

        let is_asymmetric = pos_dust > neg_dust * U256::from(2); 

        if is_asymmetric {
            warn!(target: "dss", pos_dust = %pos_dust, neg_dust = %neg_dust, "🚨 ASYMMETRY DETECTED!");
        }

        is_asymmetric
    }
}
