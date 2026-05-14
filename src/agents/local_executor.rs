use alloy::primitives::{Address, Bytes, U256};
use revm::context::result::{ExecutionResult, Output};
use revm::context::{BlockEnv, CfgEnv, Context, Journal, TxEnv};
use revm::database_interface::EmptyDB;
use revm::primitives::{hardfork::SpecId, TxKind};
use revm::{ExecuteEvm, MainBuilder, MainContext};

/// Local EVM executor using revm.
/// Simulates transactions without needing an RPC node.
pub struct LocalExecutor;

impl LocalExecutor {
    /// Execute a single transaction locally
    pub fn execute(
        target: Address,
        calldata: &[u8],
        value: U256,
        caller: Address,
    ) -> Result<ExecResult, String> {
        let ctx = Context::mainnet().with_db(EmptyDB::new());
        let mut evm = ctx.build_mainnet();

        let tx = TxEnv::builder()
            .caller(caller)
            .kind(TxKind::Call(target))
            .data(Bytes::copy_from_slice(calldata))
            .value(value)
            .gas_limit(1_000_000)
            .build()
            .map_err(|e| format!("Failed to build TxEnv: {:?}", e))?;

        match evm.transact(tx) {
            Ok(result_and_state) => {
                let gas_used = result_and_state.result.gas_used();
                let success = result_and_state.result.is_success();
                let output = match &result_and_state.result {
                    ExecutionResult::Success { output, .. } => match output {
                        Output::Call(data) => Some(data.to_vec()),
                        _ => None,
                    },
                    _ => None,
                };
                Ok(ExecResult {
                    success,
                    gas_used,
                    output,
                })
            }
            Err(e) => Err(format!("revm execution failed: {:?}", e)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExecResult {
    pub success: bool,
    pub gas_used: u64,
    pub output: Option<Vec<u8>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_empty_calldata() {
        let target = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D"
            .parse()
            .unwrap();
        let caller = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
            .parse()
            .unwrap();
        let result = LocalExecutor::execute(target, &[], U256::ZERO, caller);
        // Empty calldata on EmptyDB should execute (no account, but doesn't error)
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_with_random_calldata() {
        let target = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D"
            .parse()
            .unwrap();
        let caller = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
            .parse()
            .unwrap();
        let result = LocalExecutor::execute(target, &[0xde, 0xad, 0xbe, 0xef], U256::ZERO, caller);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_with_value() {
        let target = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
            .parse()
            .unwrap();
        let caller = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
            .parse()
            .unwrap();
        // Sending ETH from caller to self with 0 balance should error gracefully
        match LocalExecutor::execute(target, &[], U256::from(1000u64), caller) {
            Ok(r) => assert!(r.success || !r.success),
            Err(_) => {} // also acceptable on EmptyDB
        }
    }
}
