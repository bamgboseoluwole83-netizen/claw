use alloy_primitives::{Address, B256, U256};
use revm::{Database, primitives::{AccountInfo, Bytecode}};
use std::collections::HashMap;
use tracing::warn;

#[derive(Debug, Clone)]
pub struct DivergenceReport {
    pub divergent_slots: Vec<(U256, U256, U256)>,
    pub total_dust_wei: U256,
    pub is_asymmetric: bool,
}

pub struct ChaosDatabase<DB> {
    pub inner: DB,
    pub flip_mask: U256, // XOR mask to flip specific bits
}

impl<DB: Database> Database for ChaosDatabase<DB> {
    type Error = DB::Error;
    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> { self.inner.basic(address) }
    fn code_by_hash(&mut self, hash: B256) -> Result<Bytecode, Self::Error> { self.inner.code_by_hash(hash) }
    fn storage(&mut self, address: Address, slot: U256) -> Result<U256, Self::Error> {
        let real_val = self.inner.storage(address, slot)?;
        if real_val != U256::ZERO {
            warn!(target: "dss", "TARGET STORAGE HIJACKED! Flipping bits on Slot {:?}", slot);
            // THE 0.1% MOVE: XOR the lower bits to break precision without killing core logic
            return Ok(real_val ^ self.flip_mask);
        }
        Ok(real_val)
    }
    fn block_hash(&mut self, block_number: u64) -> Result<B256, Self::Error> { self.inner.block_hash(block_number) }
}

pub struct DivergenceEngine;
impl DivergenceEngine {
    pub fn diff_states(honest: &HashMap<U256, U256>, chaotic: &HashMap<U256, U256>) -> DivergenceReport {
        let mut slots = Vec::new();
        let mut dust = U256::ZERO;
        for (s, h) in honest {
            if let Some(c) = chaotic.get(s) {
                if h != c {
                    let d = if h > c { *h - *c } else { *c - *h };
                    slots.push((*s, *h, *c));
                    dust += d;
                }
            }
        }
        DivergenceReport { divergent_slots: slots, total_dust_wei: dust, is_asymmetric: false }
    }
}
