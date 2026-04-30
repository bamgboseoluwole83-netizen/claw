use std::process::Command;
use std::fs;
use tracing::info;

/// Run the standalone ityfuzz CLI on bytecode and ABI.
/// Returns any exploit lines found.
pub fn run_ityfuzz_cli(bytecode: &[u8], abi_json: &str, oracle_addr: &str) -> Vec<String> {
    let dir = tempfile::tempdir().expect("tempdir");
    let bc_path = dir.path().join("target.bin");
    let abi_path = dir.path().join("target.abi");
    fs::write(&bc_path, bytecode).unwrap();
    fs::write(&abi_path, abi_json).unwrap();

    let output = Command::new("ityfuzz")
        .args([
            "evm",
            "--target", bc_path.to_str().unwrap(),
            "--abi", abi_path.to_str().unwrap(),
            "--oracle", oracle_addr,
            "--rpc-url", "https://mainnet.base.org",
            "--sequences", "1000",
        ])
        .output()
        .expect("ityfuzz failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    info!("ityfuzz output: {}", stdout);

    stdout.lines()
        .filter(|l| l.contains("invariant broken"))
        .map(|l| l.to_string())
        .collect()
}