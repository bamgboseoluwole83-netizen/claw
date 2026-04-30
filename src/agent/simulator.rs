use alloy_primitives::{Address, Bytes, U256};
use revm::db::{CacheDB, EmptyDB};
use revm::primitives::{ExecutionResult, TransactTo, SpecId};
use revm::Evm;

pub struct Simulator;

impl Simulator {
    pub fn new() -> Self { Self }

    /// Execute a single call, consuming the DB and returning the result.
    pub fn execute_call(
        mut db: CacheDB<EmptyDB>,
        caller: Address,
        to: Address,
        calldata: Bytes,
        value: U256,
    ) -> (CacheDB<EmptyDB>, Option<ExecutionResult>) {
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

        let result = evm.transact_commit().ok();
        let db = evm.into_db();
        (db, result)
    }

    /// Execute a synthetic reentrancy attack.
    /// 1. Execute `first_calldata` on `first_target` using the given DB.
    /// 2. Immediately execute `second_calldata` on `second_target` using the SAME DB
    ///    (which now contains the dirty state left by the first call).
    /// This models the attacker's fallback function calling the lending protocol while
    /// the pool's state is still unbalanced.
    pub fn execute_reentrancy_sequence(
        db: CacheDB<EmptyDB>,
        caller: Address,
        first_target: Address,
        first_calldata: Bytes,
        second_target: Address,
        second_calldata: Bytes,
    ) -> (CacheDB<EmptyDB>, Option<(ExecutionResult, ExecutionResult)>) {
        let (db_after_first, res1) = Self::execute_call(db, caller, first_target, first_calldata, U256::ZERO);
        let (db_final, res2) = Self::execute_call(db_after_first, caller, second_target, second_calldata, U256::ZERO);
        (db_final, res1.and_then(|r1| res2.map(|r2| (r1, r2))))
    }
}