use alloy_primitives::{Address, U256};
use revm::{Database};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct DivergenceReport {
    pub divergent_slots: HashMap<(Address, U256), U256>,
    pub total_dss_wei: U256,
}

pub struct ChaosDatabase<DB> {
    pub inner: DB,
    pub flip_mask: U256,
}

impl<DB: Database> Database for ChaosDatabase<DB> {
    type Error = DB::Error;
    fn basic(&mut self, address: Address) -> Result<Option<revm::primitives::AccountInfo>, Self::Error> { self.inner.basic(address) }
    fn code_by_hash(&mut self, hash: U256) -> Result<revm::primitives::Bytecode, Self::Error> { self.inner.code_by_hash(hash) }
    fn storage(&mut self, address: Address, slot: U256) -> Result<U256, Self::Error> {
        let real_val = self.inner.storage(address, slot)?;
        if real_val != U256::ZERO {
            tracing::warn!(target: "dss", "TARGET STORAGE HIJACKED! Flipping bits on Slot {:?}", slot);
            return Ok(real_val ^ self.flip_mask);
        }
        Ok(real_val)
    }
    fn block_hash(&mut self, block_number: u64) -> Result<U256, Self::Error> { self.inner.block_hash(block_number) }
}