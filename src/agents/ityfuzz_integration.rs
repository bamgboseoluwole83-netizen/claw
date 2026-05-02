use std::process::Command;
use std::fs;
use alloy_primitives::U256;
use tracing::info;

/// Run ityfuzz on the oracle bytecode and ABI.
/// Returns a list of (description, profit) for each invariant break found.
pub fn run_ityfuzz(bytecode: &[u8], abi_json: &str, oracle_addr: &str) -> Vec<(String, U256)> {
    let dir = tempfile::tempdir().expect("tempdir");
    let bc_path = dir.path().join("target.bin");
    let abi_path = dir.path().join("target.abi");
    fs::write(&bc_path, bytecode).unwrap();
    fs::write(&abi_path, abi_json).unwrap();

    let output = Command::new("ityfuzz")
        .args([
            "evm",
            "--bytecode-file", bc_path.to_str().unwrap(),
            "--abi-file", abi_path.to_str().unwrap(),
            "--rpc-url", "https://mainnet.base.org",
            "--sequences", "1000",
        ])
        .output()
        .expect("ityfuzz failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    info!("ityfuzz output: {}", stdout);

    let mut exploits = Vec::new();
    for line in stdout.lines() {
        if line.contains("invariant broken") {
            // Parse actual profit later – for now mark as zero for manual verification
            let profit = U256::ZERO;
            exploits.push((line.to_string(), profit));
        }
    }
    exploits
}
