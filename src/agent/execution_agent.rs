use revm::Evm;
use revm::primitives::{ExecutionResult, TransactTo, Bytes, SpecId};
use revm::db::CacheDB;
use revm::db::EmptyDB;
use alloy_primitives::{Address, U256};

pub struct ExecutionAgent;

impl ExecutionAgent {
    /// Simple transaction execution (returns only result).
    pub fn execute_transaction(
        db: CacheDB<EmptyDB>,
        caller: Address,
        to: Address,
        calldata: Bytes,
    ) -> Option<ExecutionResult> {
        let mut evm = Evm::builder()
            .with_db(db)
            .with_spec_id(SpecId::LATEST)
            .build();

        *evm.tx_mut() = revm::primitives::TxEnv {
            caller,
            transact_to: TransactTo::Call(to),
            data: calldata,
            value: U256::ZERO,
            ..Default::default()
        };

        match evm.transact_commit() {
            Ok(res) => Some(res),
            Err(_) => None,
        }
    }
}