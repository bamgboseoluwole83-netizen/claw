use std::path::PathBuf;
use std::sync::LazyLock;

use alloy::primitives::{Address, U256};

use crate::agents::economic::{EconStep, EconomicFinding};
use crate::agents::finding::VerifiedFinding;

const POC_TEMPLATE: &str = include_str!("../../templates/PocTemplate.t.sol");
static DOT: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::from("."));

pub struct PoCGenerator;

impl PoCGenerator {
    pub fn new() -> Self {
        Self
    }

    /// Generate a Foundry PoC test file from a verified economic finding
    pub fn generate_from_economic(
        &self,
        finding: &EconomicFinding,
        out_dir: &PathBuf,
        index: usize,
    ) -> std::io::Result<PoCFile> {
        let name = format!("E{:02}_{}", index, sanitize_name(&finding.strategy));
        let test_name = format!("test{}", capitalize(&name));
        let file_name = format!("{}.t.sol", name);
        let file_path = out_dir.join(&file_name);

        // Build step execution code
        let mut step_code = String::new();
        for (i, step) in finding.steps.iter().enumerate() {
            let calldata_hex = if step.calldata.is_empty() {
                "bytes(\"\")".to_string()
            } else {
                format!("hex\"{}\"", hex::encode(&step.calldata))
            };
            let val = format_value(step.value);
            let addr = fmt_addr(step.target);
            step_code.push_str(&format!(
                "        // Step {}: {}\n",
                i + 1,
                step.description,
            ));
            step_code.push_str(&format!(
                "        address target_{} = address({});\n",
                i, addr,
            ));
            step_code.push_str(&format!(
                "        bytes memory data_{} = {};\n",
                i, calldata_hex,
            ));
            if step.value > U256::ZERO {
                step_code.push_str(&format!("        deal(address(this), {});\n", val,));
                step_code.push_str(&format!(
                    "        (bool success_{},) = target_{}.call{{value: {}}}(data_{});\n",
                    i, i, val, i,
                ));
            } else {
                step_code.push_str(&format!(
                    "        (bool success_{},) = target_{}.call(data_{});\n",
                    i, i, i,
                ));
            }
            step_code.push_str(&format!(
                "        require(success_{}, \"Step {} failed: {}\");\n",
                i,
                i + 1,
                step.description,
            ));
            step_code.push_str(&format!(
                "        address target_{} = address({});\n",
                i, addr,
            ));
            step_code.push_str(&format!(
                "        bytes memory data_{} = {};\n",
                i, calldata_hex,
            ));
            step_code.push_str(&format!("        deal(address(this), {});\n", val,));
            step_code.push_str(&format!(
                "        (bool success_{},) = target_{}.call{{value: {}}}(data_{});\n",
                i, i, val, i,
            ));
            step_code.push_str(&format!(
                "        require(success_{}, \"Step {} failed: {}\");\n",
                i,
                i + 1,
                step.description,
            ));
        }

        // Build pre/post balance checks
        let mut pre_balance_code = String::new();
        let mut post_balance_code = String::new();
        for (i, step) in finding.steps.iter().enumerate() {
            if step.value > U256::ZERO {
                pre_balance_code.push_str(&format!(
                    "        uint256 pre_{} = address(this).balance;\n",
                    i
                ));
            }
        }
        for (i, step) in finding.steps.iter().enumerate() {
            if step.value > U256::ZERO {
                post_balance_code.push_str(&format!(
                    "        uint256 post_{} = address(this).balance;\n",
                    i
                ));
            }
        }

        let profit_eth = crate::agents::economic::u256_to_f64(finding.profit_estimate) / 1e18;

        let content = POC_TEMPLATE
            .replace("{{TEST_NAME}}", &test_name)
            .replace("{{TARGET_ADDRESS}}", &fmt_addr(finding.target))
            .replace("{{ATTACKER_ADDRESS}}", &fmt_addr(Address::ZERO))
            .replace("{{BLOCK_NUMBER}}", "latest")
            .replace("{{STEP_CODE}}", &step_code)
            .replace("{{PRE_BALANCE_CODE}}", &pre_balance_code)
            .replace("{{POST_BALANCE_CODE}}", &post_balance_code)
            .replace("{{PROFIT_ESTIMATE_ETH}}", &format!("{:.6}", profit_eth))
            .replace("{{STRATEGY}}", &finding.strategy)
            .replace("{{PROFIT_ASSERT}}", &format!("{:.0}", profit_eth * 1e18));

        std::fs::write(&file_path, &content)?;

        Ok(PoCFile {
            name: file_name,
            path: file_path,
            strategy: finding.strategy.clone(),
            profit_estimate: finding.profit_estimate,
            confidence: finding.confidence,
            test_result: None,
        })
    }

    /// Generate a PoC from a VerifiedFinding (simpler, single-step)
    pub fn generate_from_verified(
        &self,
        finding: &VerifiedFinding,
        out_dir: &PathBuf,
        index: usize,
    ) -> std::io::Result<PoCFile> {
        let name = format!("V{:02}_{}", index, sanitize_name(&finding.description));
        let test_name = format!("test{}", capitalize(&name));
        let file_name = format!("{}.t.sol", name);
        let file_path = out_dir.join(&file_name);

        let calldata_hex = if finding.calldata.is_empty() {
            "bytes(\"\")".to_string()
        } else {
            format!("hex\"{}\"", hex::encode(&finding.calldata))
        };

        let profit_eth = crate::agents::economic::u256_to_f64(finding.profit_estimate) / 1e18;

        let content = format!(
            r#"// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Test.sol";

contract {} is Test {{
    address constant TARGET = {};
    address constant ATTACKER = address(0xdead);

    function setUp() public {{
        vm.createSelectFork(
            vm.envString("DRPC_URL"),
            {}  // block number
        );
    }}

    function {}() public {{
        vm.startPrank(ATTACKER);

        uint256 pre = address(this).balance;
        bytes memory data = {};
        (bool success,) = TARGET.call(data);
        require(success, "exploit call failed");
        uint256 post = address(this).balance;

        vm.stopPrank();

        uint256 profit = post - pre;
        assertGt(profit, {} / 100, "profit below expected");
    }}
}}
"#,
            test_name,
            fmt_addr(finding.target),
            "block.number", // placeholder
            test_name,
            calldata_hex,
            format!("{:.0}", profit_eth * 1e18),
        );

        std::fs::write(&file_path, &content)?;

        Ok(PoCFile {
            name: file_name,
            path: file_path,
            strategy: finding.description.clone(),
            profit_estimate: finding.profit_estimate,
            confidence: finding.severity / 10.0,
            test_result: None,
        })
    }

    /// Generate PoC and run forge test against it
    pub fn generate_and_verify(
        &self,
        finding: &VerifiedFinding,
        out_dir: &PathBuf,
        index: usize,
    ) -> PoCFile {
        let mut poc = self
            .generate_from_verified(finding, out_dir, index)
            .unwrap_or_else(|e| PoCFile {
                name: format!("V{:02}_error", index),
                path: out_dir.join(format!("V{:02}_error.txt", index)),
                strategy: finding.description.clone(),
                profit_estimate: finding.profit_estimate,
                confidence: finding.severity / 10.0,
                test_result: Some(TestResult {
                    passed: false,
                    stdout: String::new(),
                    stderr: format!("Failed to write PoC file: {}", e),
                }),
            });

        // Run forge test on the generated PoC
        let result = self.run_forge_test(&poc);
        poc.test_result = Some(result);
        poc
    }

    /// Execute `forge test` on the PoC file and return the result
    pub fn run_forge_test(&self, poc: &PoCFile) -> TestResult {
        let parent = poc.path.parent().unwrap_or(&DOT);
        let test_contract = poc.name.replace(".t.sol", "");

        let output = match std::process::Command::new("forge")
            .arg("test")
            .arg("--match-contract")
            .arg(&test_contract)
            .arg("-vvv")
            .current_dir(parent)
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                return TestResult {
                    passed: false,
                    stdout: String::new(),
                    stderr: format!("Failed to run forge: {}", e),
                };
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let passed = output.status.success() && stdout.contains("[PASS]");

        TestResult {
            passed,
            stdout,
            stderr,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PoCFile {
    pub name: String,
    pub path: PathBuf,
    pub strategy: String,
    pub profit_estimate: U256,
    pub confidence: f64,
    pub test_result: Option<TestResult>,
}

#[derive(Debug, Clone)]
pub struct TestResult {
    pub passed: bool,
    pub stdout: String,
    pub stderr: String,
}

impl PoCFile {
    pub fn profit_eth(&self) -> f64 {
        crate::agents::economic::u256_to_f64(self.profit_estimate) / 1e18
    }
}

// ── Helpers ──

fn fmt_addr(addr: Address) -> String {
    format!("0x{}", hex::encode(addr.as_slice()))
}

fn format_value(v: U256) -> String {
    if v.is_zero() {
        "0".to_string()
    } else {
        format!("{}", v)
    }
}

fn sanitize_name(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().to_string() + c.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("oracle_manipulation"), "oracle_manipulation");
        assert_eq!(sanitize_name("test-strategy!"), "test_strategy");
    }

    #[test]
    fn test_capitalize() {
        assert_eq!(capitalize("hello"), "Hello");
        assert_eq!(capitalize("hello_world"), "Hello_world");
    }

    #[test]
    fn test_generate_economic_poc() {
        let dir = tempdir().unwrap();
        let gen = PoCGenerator::new();
        let finding = EconomicFinding {
            strategy: "test_strategy".to_string(),
            target: Address::ZERO,
            profit_estimate: U256::from(1_000_000_000_000_000_000u128),
            steps: vec![EconStep {
                target: Address::ZERO,
                calldata: vec![0xde, 0xad],
                value: U256::from(1e18 as u64),
                description: "deposit".to_string(),
            }],
            confidence: 0.5,
            description: "test".to_string(),
        };
        let result = gen.generate_from_economic(&finding, &dir.into_path(), 1);
        assert!(result.is_ok());
        let file = result.unwrap();
        assert!(file.path.exists());
        assert!(file.name.contains("test_strategy"));
    }
}
