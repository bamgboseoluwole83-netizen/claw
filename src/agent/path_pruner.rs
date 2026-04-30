use std::collections::HashMap; // FIX: Added this import
use alloy_primitives::{Address, U256};
use revm::{Database, EvmBuilder, primitives::{SpecId, TransactTo}};
use crate::agents::execution_agent::StateSnapshot;
use tracing::info;

pub struct PathSniper;

impl PathSniper {
    pub fn filter_dead_payloads<DB: Database + Clone>(
        base_db: &DB,
        caller: Address,
        target: Address,
        payloads: Vec<Vec<u8>>,
    ) -> Vec<Vec<u8>> {
        let mut live_payloads = Vec::new();

        for calldata in payloads {
            let db = base_db.clone(); 
            let mut evm = EvmBuilder::default()
                .with_spec_id(SpecId::CANCUN)
                .with_db(db) 
                .build();
            
            evm.tx_mut().caller = caller;
            evm.tx_mut().transact_to = TransactTo::Call(target);
            evm.tx_mut().data = calldata.clone().into();
            evm.tx_mut().gas_limit = 10_000_000;

            if let Ok(res_and_state) = evm.transact() {
                let mut changes = StateSnapshot::new();
                for (addr, account) in res_and_state.state.iter() {
                    let mutated_slots: HashMap<U256, U256> = account.storage.iter()
                        .filter(|(_, slot)| slot.original_value != slot.present_value)
                        .map(|(slot, evm_slot)| (*slot, evm_slot.present_value))
                        .collect();
                        
                    if !mutated_slots.is_empty() {
                        changes.insert(*addr, mutated_slots);
                    }
                }

                if changes.is_empty() {
                    info!(target: "sniper", "PRUNED: Payload does not mutate state.");
                } else {
                    info!(target: "sniper", "LIVE: Payload mutates {} contracts.", changes.len());
                    live_payloads.push(calldata);
                }
            }
        }
        live_payloads
    }
}