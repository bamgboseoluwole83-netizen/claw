use alloy_primitives::{Address, B256, U256};
use revm::Database;
// FIX: Removed "use std::collections::HashMap;" because we use the StateSnapshot type alias now

#[derive(Debug, Clone)]
pub struct DivergenceReport {
    pub divergent_slots: Vec<(Address, U256, U256, U256)>, // (Address, Slot, HonestVal, ChaosVal)
    pub total_dust_wei: U256,
}

pub struct ChaosDatabase<DB> {
    pub inner: DB,
    pub flip_mask: U256, 
}

impl<DB: Database> Database for ChaosDatabase<DB> {
    type Error = DB::Error;

    fn basic(&mut self, address: Address) -> Result<Option<revm::primitives::AccountInfo>, Self::Error> { 
        self.inner.basic(address) 
    }

    fn code_by_hash(&mut self, hash: B256) -> Result<revm::primitives::Bytecode, Self::Error> { 
        self.inner.code_by_hash(hash) 
    }

    fn storage(&mut self, address: Address, slot: U256) -> Result<U256, Self::Error> {
        let real_val = self.inner.storage(address, slot)?;
        if real_val != U256::ZERO {
            return Ok(real_val ^ self.flip_mask);
        }
        Ok(real_val)
    }

    fn block_hash(&mut self, block_number: u64) -> Result<B256, Self::Error> { 
        self.inner.block_hash(block_number) 
    }
}

pub struct DivergenceEngine;

impl DivergenceEngine {
    pub fn diff_states(
        honest_state: &crate::agents::execution_agent::StateSnapshot,
        chaos_state: &crate::agents::execution_agent::StateSnapshot,
    ) -> DivergenceReport {
        let mut slots = Vec::new();
        let mut dust = U256::ZERO;
        
        for (addr, honest_slots) in honest_state {
            if let Some(chaos_slots) = chaos_state.get(addr) {
                for (slot, h_val) in honest_slots {
                    if let Some(c_val) = chaos_slots.get(slot) {
                        if h_val != c_val {
                            let d = if *h_val > *c_val { *h_val - *c_val } else { *c_val - *h_val };
                            slots.push((*addr, *slot, *h_val, *c_val));
                            dust += d;
                        }
                    }
                }
            }
        }
        
        DivergenceReport { 
            divergent_slots: slots, 
            total_dust_wei: dust, 
        }
    }
}