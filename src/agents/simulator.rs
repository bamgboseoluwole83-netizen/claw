use alloy_primitives::{Address, Bytes, U256};
use revm::db::{CacheDB, EmptyDB};
use revm::primitives::{ExecutionResult, TransactTo, SpecId};
use revm::Evm;

pub struct Simulator;

impl Simulator {
    pub fn new() -> Self { Self }

    /// Execute a single call, modifying the DB in place.
    pub fn execute_call(
        db: &mut CacheDB<EmptyDB>,
        caller: Address,
        to: Address,
        calldata: Bytes,
        value: U256,
    ) -> Option<ExecutionResult> {
        // Build EVM with a reference to the database.
        let mut evm = Evm::builder()
            .with_db(db)
            .with_spec_id(SpecId::LATEST)
            .build();

        *evm.tx_mut() = revm::primitives::TxEnv {
            caller,
            transact_to: TransactTo::Call(to),
            data: calldata,
            value,
            ..Default::default()
        };

        evm.transact_commit().ok()
    }

    /// Execute a synthetic reentrancy attack.
    /// 1. Call first_target with first_calldata on the given DB.
    /// 2. Immediately call second_target with second_calldata on the SAME dirty DB.
    /// Returns the two results if both succeeded.
    pub fn execute_reentrancy_sequence(
        db: &mut CacheDB<EmptyDB>,
        caller: Address,
        first_target: Address,
        first_calldata: Bytes,
        second_target: Address,
        second_calldata: Bytes,
    ) -> Option<(ExecutionResult, ExecutionResult)> {
        let res1 = Self::execute_call(db, caller, first_target, first_calldata, U256::ZERO)?;
        let res2 = Self::execute_call(db, caller, second_target, second_calldata, U256::ZERO)?;
        Some((res1, res2))
    }
}
