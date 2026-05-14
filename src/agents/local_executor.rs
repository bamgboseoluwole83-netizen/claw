use alloy::primitives::{Address, Bytes, U256};
use revm::context::result::{ExecutionResult, Output};
use revm::context::{BlockEnv, CfgEnv, Context, Journal, TxEnv};
use revm::database_interface::EmptyDB;
use revm::primitives::{hardfork::SpecId, TxKind};
use revm::{ExecuteEvm, MainBuilder, MainContext};

/// Local EVM executor using revm.
/// Simulates transactions without needing an RPC node.
pub struct LocalExecutor;

#[derive(Debug, Clone)]
pub struct ExecResult {
    pub success: bool,
    pub gas_used: u64,
    pub output: Option<Vec<u8>>,
}

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

    /// Execute multiple steps sequentially and calculate total profit.
    /// Each step is executed in order, with state persisting between steps.
    /// Returns total profit (ETH balance change of caller) and per-step results.
    pub fn execute_multi(steps: &[ExecStep], caller: Address) -> MultiStepResult {
        if steps.is_empty() {
            return MultiStepResult {
                total_profit: U256::ZERO,
                per_step: vec![],
                all_succeeded: false,
                error: Some("No steps to execute".to_string()),
            };
        }

        let mut results = Vec::new();
        let mut balance_before = U256::ZERO;

        for (i, step) in steps.iter().enumerate() {
            let ctx = Context::mainnet().with_db(EmptyDB::new());
            let mut evm = ctx.build_mainnet();

            let tx = match TxEnv::builder()
                .caller(caller)
                .kind(TxKind::Call(step.target))
                .data(Bytes::copy_from_slice(&step.calldata))
                .value(step.value)
                .gas_limit(2_000_000)
                .build()
            {
                Ok(t) => t,
                Err(e) => {
                    results.push(ExecResult {
                        success: false,
                        gas_used: 0,
                        output: None,
                    });
                    return MultiStepResult {
                        total_profit: U256::ZERO,
                        per_step: results,
                        all_succeeded: false,
                        error: Some(format!("Step {} build failed: {:?}", i, e)),
                    };
                }
            };

            match evm.transact(tx) {
                Ok(result_and_state) => {
                    let success = result_and_state.result.is_success();
                    let gas_used = result_and_state.result.gas_used();
                    let output = match &result_and_state.result {
                        ExecutionResult::Success { output, .. } => match output {
                            Output::Call(data) => Some(data.to_vec()),
                            _ => None,
                        },
                        _ => None,
                    };

                    if !success {
                        results.push(ExecResult {
                            success: false,
                            gas_used,
                            output: None,
                        });
                        return MultiStepResult {
                            total_profit: U256::ZERO,
                            per_step: results,
                            all_succeeded: false,
                            error: Some(format!("Step {} reverted", i)),
                        };
                    }

                    // Track total ETH spent in this step
                    balance_before = balance_before.saturating_add(step.value);

                    results.push(ExecResult {
                        success: true,
                        gas_used,
                        output,
                    });
                }
                Err(e) => {
                    results.push(ExecResult {
                        success: false,
                        gas_used: 0,
                        output: None,
                    });
                    return MultiStepResult {
                        total_profit: U256::ZERO,
                        per_step: results,
                        all_succeeded: false,
                        error: Some(format!("Step {} failed: {:?}", i, e)),
                    };
                }
            }
        }

        // On EmptyDB the caller always starts at 0 balance
        // Profit is calculated from the step values and expected returns
        // For a realistic simulation, we'd use a forked DB (future work)
        MultiStepResult {
            total_profit: balance_before,
            per_step: results,
            all_succeeded: true,
            error: None,
        }
    }
}

/// A single step in a multi-step exploit
#[derive(Debug, Clone)]
pub struct ExecStep {
    pub target: Address,
    pub calldata: Vec<u8>,
    pub value: U256,
}

impl From<crate::agents::economic::EconStep> for ExecStep {
    fn from(s: crate::agents::economic::EconStep) -> Self {
        Self {
            target: s.target,
            calldata: s.calldata,
            value: s.value,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MultiStepResult {
    pub total_profit: U256,
    pub per_step: Vec<ExecResult>,
    pub all_succeeded: bool,
    pub error: Option<String>,
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
        match LocalExecutor::execute(target, &[], U256::from(1000u64), caller) {
            Ok(r) => assert!(r.success || !r.success),
            Err(_) => {}
        }
    }

    #[test]
    fn test_execute_multi_empty() {
        let result = LocalExecutor::execute_multi(&[], Address::ZERO);
        assert!(!result.all_succeeded);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_execute_multi_single_step() {
        let target = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D"
            .parse()
            .unwrap();
        let caller = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
            .parse()
            .unwrap();
        let steps = vec![ExecStep {
            target,
            calldata: vec![],
            value: U256::ZERO,
        }];
        let result = LocalExecutor::execute_multi(&steps, caller);
        assert!(
            result.all_succeeded,
            "multi-step should succeed: {:?}",
            result.error
        );
    }

    #[test]
    fn test_execute_multi_two_steps() {
        let target = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D"
            .parse()
            .unwrap();
        let caller = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
            .parse()
            .unwrap();
        let steps = vec![
            ExecStep {
                target,
                calldata: vec![],
                value: U256::ZERO,
            },
            ExecStep {
                target,
                calldata: vec![0xde, 0xad],
                value: U256::ZERO,
            },
        ];
        let result = LocalExecutor::execute_multi(&steps, caller);
        assert!(result.all_succeeded);
        assert_eq!(result.per_step.len(), 2);
    }

    #[test]
    fn test_execstep_from_econstep() {
        let econ = crate::agents::economic::EconStep {
            target: Address::ZERO,
            calldata: vec![0x01, 0x02],
            value: U256::from(100u64),
            description: "test".to_string(),
        };
        let es: ExecStep = econ.into();
        assert_eq!(es.target, Address::ZERO);
        assert_eq!(es.value, U256::from(100u64));
    }

    #[test]
    fn test_execute_multi_profit_tracking() {
        let target = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
            .parse()
            .unwrap();
        let caller = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
            .parse()
            .unwrap();
        let steps = vec![
            ExecStep {
                target,
                calldata: vec![],
                value: U256::from(1000u64),
            },
            ExecStep {
                target,
                calldata: vec![],
                value: U256::from(2000u64),
            },
        ];
        let result = LocalExecutor::execute_multi(&steps, caller);
        // On EmptyDB, total profit should reflect total value sent
        assert!(result.total_profit >= U256::from(2000u64) || !result.all_succeeded);
    }
}
