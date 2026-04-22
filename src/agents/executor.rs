use alloy_primitives::{Address, U256};
use revm::{Database, EvmBuilder, primitives::{SpecId, TransactTo}};
use crate::agents::cross_ghost_reentrancy::ChaosDatabase;
use tracing::info;
use std::collections::HashMap;

pub struct ExecutorAgent;

impl ExecutorAgent {
    pub fn execute<DB: Database>(
        chaos_db: ChaosDatabase<DB>,
        caller: Address,
        target: Address,
        calldata: Vec<u8>,
        value: U256,
    ) -> HashMap<U256, U256> {
        info!(target: "executor", "Spinning up REVM CPU...");


        let mut evm = EvmBuilder::default()
            .with_spec_id(SpecId::CANCUN)
            .with_db(chaos_db)
            .modify_tx_env(|tx| {
                tx.caller = caller;
                tx.transact_to = TransactTo::Call(target);
                tx.data = calldata.into();
                tx.value = value;
                tx.gas_limit = 1_000_000_u64;
                tx.nonce = Some(1);
                tx.gas_price = U256::from(1_000_000_000_u64);
            })
            .build();

        let result = evm.transact();
        
        if let Ok(result_and_state) = result {
            info!(target: "executor", "TX SUCCESS! Extracting state...");
            let mut target_storage = HashMap::new();
            
            // Get our specific contract from the REVM state map
            if let Some(account) = result_and_state.state.get(&target) {
                // Iterate over the contract's storage slots
                for (slot, val) in account.storage.iter() {
                    target_storage.insert(*slot, val.present_value);
                }
            }
            
            info!(target: "executor", "Extracted {} storage changes directly from REVM!", target_storage.len());
            return target_storage;
        }

        info!(target: "executor", "TX Reverted. No state changes.");
        HashMap::new()
    }
}  