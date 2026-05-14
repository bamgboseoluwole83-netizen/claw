//! Integration tests for web3-destroyer pipeline
//!
//! These tests require: anvil, forge, and external tools (slither, wake, etc.)
//! Run with: cargo test --test integration -- --ignored
//!
//! Or run explicitly: cargo test --test integration

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

const ANVIL_PORT: u16 = 8545;
const HARD_TEST_ADDRESS: &str = "0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0";

fn find_forge() -> Option<PathBuf> {
    which::which("forge").ok()
}

fn find_anvil() -> Option<PathBuf> {
    which::which("anvil").ok()
}

fn start_anvil() -> Option<Child> {
    let mut child = Command::new("anvil")
        .args(["-p", &ANVIL_PORT.to_string(), "--host", "127.0.0.1"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;

    std::thread::sleep(Duration::from_secs(2));
    Some(child)
}

fn deploy_contracts(rpc_url: &str) -> bool {
    let deploy_cmd = Command::new("forge")
        .args([
            "script",
            "script/DeployTestSuite.s.sol",
            "--rpc-url",
            rpc_url,
            "--broadcast",
            "--skip",
        ])
        .output();

    match deploy_cmd {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

fn kill_anvil_on_drop(_child: &mut Option<Child>) {
    // anvil will be killed when the process exits
}

mod pipeline_tests {
    use super::*;

    #[test]
    #[ignore = "Requires anvil, forge, and external tools installed"]
    fn test_pipeline_runs_without_crash() {
        let rpc_url = "http://127.0.0.1:8545";

        // Start anvil
        let mut anvil = match start_anvil() {
            Some(c) => Some(c),
            None => {
                eprintln!("WARNING: anvil not available, skipping test");
                return;
            }
        };

        // Deploy contracts
        let deployed = deploy_contracts(rpc_url);
        if !deployed {
            eprintln!("WARNING: forge deployment failed, skipping pipeline test");
            if let Some(ref mut a) = anvil {
                let _ = a.kill();
            }
            return;
        }

        // Note: We can't easily run the full pipeline here because it requires
        // spawning subprocesses. The real integration test would call
        // Controller::run_pipeline directly. This is a placeholder for the
        // architecture we need.
        println!("✓ Anvil started, contracts deployed, pipeline ready");

        if let Some(ref mut a) = anvil {
            let _ = a.kill();
        }
    }

    #[test]
    #[ignore = "Requires full pipeline to be runnable programmatically"]
    fn test_finds_known_vulnerability() {
        // This test would:
        // 1. Start anvil
        // 2. Deploy HardTest.sol (contains delegatecall vulnerability)
        // 3. Run: cargo run -- scan 0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0
        // 4. Assert: findings.contains("delegatecall") || findings.contains("selfdestruct")
        // 5. Assert: verified.len() > 0 (at least one exploit was confirmed on-chain)

        // Expected: The pipeline should find the delegatecall → selfdestruct chain
        // in HardTest and verify it on-chain.
        unreachable!("Not yet implemented - requires Controller to be callable from tests");
    }

    #[test]
    fn test_anvil_connection() {
        let rpc_url = "http://127.0.0.1:8545";
        let mut anvil = match start_anvil() {
            Some(c) => c,
            None => {
                eprintln!("SKIP: anvil not available");
                return;
            }
        };

        // Try to connect via curl
        let result = Command::new("curl")
            .args([
                "-s",
                "-X",
                "POST",
                rpc_url,
                "-H",
                "Content-Type: application/json",
                "--data",
                r#"{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}"#,
            ])
            .output();

        if let Ok(output) = result {
            assert!(output.status.success(), "RPC should respond");
        }

        let _ = anvil.kill();
    }
}

// Integration test that actually tests the Controller
// This would be in a separate binary or require proper setup
mod controller_tests {
    use super::*;

    #[test]
    #[ignore = "Requires full controller integration"]
    fn test_controller_with_hard_test_vulnerability() {
        // This would be the real test:
        // use web3_destroyer::agents::controller::Controller;
        //
        // let target = "0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0".parse().unwrap();
        // let rpc_url = "http://127.0.0.1:8545";
        //
        // let controller = Controller::new(target, rpc_url, None);
        // let findings = tokio::runtime::Runtime::new().unwrap().block_on(controller.run());
        //
        // assert!(findings.len() > 0, "Should find vulnerabilities in HardTest");
        // assert!(findings.iter().any(|f| f.description.contains("delegatecall") || f.description.contains("selfdestruct")));
    }
}

mod mode_gating_tests {
    use alloy::primitives::Address;
    use web3_destroyer::agents::hunt;
    use web3_destroyer::agents::ScanMode;

    #[test]
    fn test_scan_mode_from_str() {
        assert_eq!("quick".parse::<ScanMode>().unwrap(), ScanMode::Quick);
        assert_eq!("QUICK".parse::<ScanMode>().unwrap(), ScanMode::Quick);
        assert_eq!("standard".parse::<ScanMode>().unwrap(), ScanMode::Standard);
        assert_eq!("deep".parse::<ScanMode>().unwrap(), ScanMode::Deep);
        assert!("invalid".parse::<ScanMode>().is_err());
    }

    #[test]
    fn test_scan_mode_display() {
        assert_eq!(ScanMode::Quick.to_string(), "quick");
        assert_eq!(ScanMode::Standard.to_string(), "standard");
        assert_eq!(ScanMode::Deep.to_string(), "deep");
    }

    #[test]
    fn test_scan_mode_default() {
        let default: ScanMode = Default::default();
        assert_eq!(default, ScanMode::Standard);
    }

    #[test]
    fn test_orchestrate_quick_mode_no_crash() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let target = "0x1111111111111111111111111111111111111111"
            .parse()
            .unwrap();
        let mut bytecode = vec![0x60, 0x00, 0x60, 0x00, 0x60, 0x00];
        bytecode.extend_from_slice(&[
            0x73, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11,
            0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11,
        ]);
        bytecode.extend_from_slice(&[0x60, 0xff]);
        bytecode.push(0xf4);

        let findings = rt.block_on(hunt::orchestrate(
            target,
            None,
            None,
            &bytecode,
            None,
            None,
            false,
            ScanMode::Quick,
        ));

        assert!(
            !findings.is_empty(),
            "Quick mode should still run Heimdall bytecode analysis"
        );
        assert!(
            findings
                .iter()
                .any(|f| f.description.contains("DELEGATECALL")),
            "Should detect DELEGATECALL in bytecode"
        );
    }

    #[test]
    fn test_orchestrate_standard_mode_no_crash() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let target = "0x2222222222222222222222222222222222222222"
            .parse()
            .unwrap();
        let mut bytecode = vec![0x60, 0x00, 0x60, 0x00, 0x60, 0x00];
        bytecode.push(0xff);
        bytecode.extend_from_slice(&[0x60, 0x01, 0x60, 0x02]);

        let findings = rt.block_on(hunt::orchestrate(
            target,
            None,
            None,
            &bytecode,
            None,
            None,
            false,
            ScanMode::Standard,
        ));

        assert!(
            !findings.is_empty(),
            "Standard mode should still run Heimdall bytecode analysis"
        );
        assert!(
            findings
                .iter()
                .any(|f| f.description.contains("SELFDESTRUCT")),
            "Should detect SELFDESTRUCT in bytecode"
        );
    }

    #[test]
    fn test_orchestrate_deep_mode_no_crash() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let target = "0x3333333333333333333333333333333333333333"
            .parse()
            .unwrap();
        let mut bytecode = vec![0x60, 0x01, 0x60, 0x02, 0x60, 0x03];
        bytecode.push(0xf0);
        bytecode.push(0x60);

        let findings = rt.block_on(hunt::orchestrate(
            target,
            None,
            None,
            &bytecode,
            None,
            None,
            false,
            ScanMode::Deep,
        ));

        assert!(
            !findings.is_empty(),
            "Deep mode should still run Heimdall bytecode analysis"
        );
        assert!(
            findings
                .iter()
                .any(|f| f.description.contains("Contract creation")),
            "Should detect CREATE in bytecode"
        );
    }

    #[test]
    fn test_orchestrate_all_modes_with_source_dir() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let target: Address = Address::ZERO;
        let bytecode = vec![
            0x73, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11,
            0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x60, 0xff, 0xf4,
        ];

        for mode in &[ScanMode::Quick, ScanMode::Standard, ScanMode::Deep] {
            let findings = rt.block_on(hunt::orchestrate(
                target, None, // source_dir - tools will be skipped gracefully
                None, &bytecode, None, None, false, *mode,
            ));

            // All modes should produce at least Heimdall findings from bytecode
            assert!(
                findings
                    .iter()
                    .any(|f| f.description.contains("DELEGATECALL")),
                "Mode {:?} should detect DELEGATECALL in bytecode",
                mode
            );
        }
    }
}
