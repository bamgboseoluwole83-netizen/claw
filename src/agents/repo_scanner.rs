use std::process::Command;
use tracing::info;

#[derive(Debug)]
pub struct ExtractedCall {
    pub file: String,
    pub contract: String,
    pub function: String,
    pub external_call: String,
    pub target_address: Option<String>,
}

pub fn scan_repo(repo_url: &str) -> Vec<ExtractedCall> {
    let dir = "temp_repo";
    // 1. Remove old clone if it exists
    let _ = std::fs::remove_dir_all(dir);
    let status = Command::new("git")
        .args(["clone", repo_url, dir])
        .status()
        .expect("git clone failed");
    if !status.success() {
        panic!("Clone failed");
    }

    // 2. Install npm dependencies if package.json exists
    let npm_install = Command::new("npm")
        .args(["install", "--prefix", dir])
        .status();
    if npm_install.is_ok() {
        info!("✅ npm dependencies installed");
    }

    // 3. Build (ignore failures)
    let build_status = Command::new("forge")
        .args(["build", "--root", dir])
        .status();
    if build_status.is_ok() && build_status.unwrap().success() {
        info!("✅ forge build succeeded");
    } else {
        info!("⚠️  forge build had issues – continuing anyway");
    }

    // 4. Extract external calls using forge inspect (JSON output)
    let output = Command::new("forge")
        .args(["inspect", "methods", "--root", dir, "--json"])
        .output()
        .unwrap_or_else(|_| {
            info!("⚠️  forge inspect failed, returning empty");
            std::process::Output {
                status: std::process::ExitStatus::default(),
                stdout: vec![],
                stderr: vec![],
            }
        });

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut calls = Vec::new();

    // Parse JSON output: each contract -> list of method signatures
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
        if let Some(obj) = json.as_object() {
            for (contract, methods) in obj {
                if let Some(methods_array) = methods.as_array() {
                    for method in methods_array {
                        if let Some(method_str) = method.as_str() {
                            if let Some(arrow_pos) = method_str.find(" -> ") {
                                let func = &method_str[..arrow_pos];
                                let call = &method_str[arrow_pos + 4..];
                                let addr = if let Some(addr_start) = call.find("0x") {
                                    let rest = &call[addr_start..];
                                    let addr_end = rest.find(|c: char| !c.is_ascii_hexdigit() && c != 'x').unwrap_or(rest.len());
                                    Some(rest[..addr_end].to_string())
                                } else {
                                    None
                                };
                                calls.push(ExtractedCall {
                                    file: String::new(),
                                    contract: contract.clone(),
                                    function: func.to_string(),
                                    external_call: call.to_string(),
                                    target_address: addr,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    info!("🔍 Repo scan found {} external calls", calls.len());
    calls
}
