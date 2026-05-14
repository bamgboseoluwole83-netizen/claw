use alloy::primitives::{Address, Bytes, U256};
use std::path::Path;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{info, warn};

use crate::agents::cross_contract::HeimdallAnalyzer;
use crate::agents::finding::{Finding, ToolKind, VerifiedFinding, VerifyStatus};
use crate::agents::chain;
use crate::agents::synthesizer;
use crate::agents::wake;
use crate::agents::tool_status::{ToolReport, ToolState};
use crate::agents::ScanMode;

fn find_venv_python3() -> Option<String> {
    let candidates = &[
        "/home/user/web3-destroyer/.venv/bin/python3",
        "/home/user/.venv/bin/python3",
    ];
    for c in candidates {
        if std::path::Path::new(c).exists() {
            return Some(c.to_string());
        }
    }
    None
}

fn find_tool(names: &[&str]) -> Option<String> {
    // Search common directories for pre-installed tools
    let dirs = [
        format!("/home/user/.local/bin"),
        format!("/home/user/.bifrost/bin"),
        format!("/home/user/.cargo/bin"),
        format!("/home/user/web3-destroyer/.venv/bin"),
    ];

    for dir in &dirs {
        for name in names {
            let path = format!("{}/{}", dir, name);
            if std::path::Path::new(&path).exists() {
                return Some(path);
            }
        }
    }

    for name in names {
        if let Ok(path) = std::process::Command::new("which").arg(name).output() {
            if path.status.success() {
                if let Ok(s) = String::from_utf8(path.stdout) {
                    let s = s.trim().to_string();
                    if !s.is_empty() {
                        return Some(s);
                    }
                }
            }
        }
    }
    None
}

pub fn check_tool_available(name: &str) -> bool {
    find_tool(&[name]).is_some()
}

/// Check which tools are available on the system
pub fn check_tools_available() -> ToolReport {
    let mut report = ToolReport::new();

    let tools = [
        ("Slither", "slither"),
        ("Conkas", "conkas"),
        ("Wake", "wake"),
        ("Mythril", "mythril"),
        ("Heimdall", "heimdall"),
        ("Halmos", "halmos"),
        ("Medusa", "medusa"),
        ("Foray", "foray"),
        ("Ityfuzz", "ityfuzz"),
        ("cast", "cast"),
        ("forge", "forge"),
        ("Aderyn", "aderyn"),
    ];

    for (display_name, binary_name) in tools {
        if find_tool(&[binary_name]).is_some() {
            report.record_available(display_name, 0);
        } else {
            report.record_missing(display_name);
        }
    }

    report
}

// ================================================================
// Tool Wrappers
// ================================================================

pub async fn run_slither(source_dir: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    let slither = match find_tool(&["slither"]) {
        Some(s) => s,
        None => { warn!("   Slither not found — skipping"); return findings; }
    };
    info!("   Slither: analyzing {}...", source_dir.display());

    let mut cmd = tokio::process::Command::new(&slither);
    cmd.arg(source_dir)
        .arg("--json")
        .arg("-")
        .arg("--detect")
        .arg("oracle-twap,rounding-direction,flash-reentrancy")
        .arg("--detectors-path")
        .arg("/home/user/web3-destroyer/detectors");
    let venv_bin = "/home/user/web3-destroyer/.venv/bin";
    let path = std::env::var("PATH").unwrap_or_default();
    cmd.env("PATH", if !path.contains(venv_bin) { format!("{}:{}", venv_bin, path) } else { path });

    let output = match timeout(Duration::from_secs(120), cmd.output()).await {
        Ok(Ok(o)) => o,
        _ => { warn!("   Slither timed out or failed"); return findings; }
    };

    let combined = format!("{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    let json_start = combined.find('{');
    let json_str = match json_start {
        Some(i) => &combined[i..],
        None => { warn!("   Slither produced no JSON output"); return findings; }
    };

    let parsed: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(e) => { warn!("   Slither JSON parse error: {}", e); return findings; }
    };

    let detectors = parsed["results"]["detectors"].as_array().cloned().unwrap_or_default();
    info!("   Slither: {} detector(s) found", detectors.len());

    for d in &detectors {
        let check = d["check"].as_str().unwrap_or("unknown");
        let impact = d["impact"].as_str().unwrap_or("");
        let confidence = d["confidence"].as_str().unwrap_or("");
        let description = d["description"].as_str().unwrap_or("").to_string();
        let elements = d["elements"].as_array().cloned().unwrap_or_default();

        let severity = match impact {
            "High" => 9.0,
            "Medium" => 6.0,
            "Low" => 3.0,
            "Informational" | "Optimization" => 1.0,
            _ => 3.0,
        };
        let conf = match confidence {
            "High" => 0.9,
            "Medium" => 0.7,
            "Low" => 0.5,
            _ => 0.5,
        };

        let evidence: Vec<String> = elements.iter().map(|e| {
            let name = e["name"].as_str().unwrap_or("");
            let type_name = e["type"].as_str().unwrap_or("");
            format!("{}: {}", type_name, name)
        }).collect();

        findings.push(Finding {
            tool: ToolKind::Slither,
            severity,
            confidence: conf,
            description: format!("[{}] {}", check, description),
            target: Address::ZERO,
            calldata: None,

            evidence,
        });
    }

    findings
}

fn classify_reentrancy_severity(desc: &str, vuln_type: &str) -> f64 {
    if vuln_type != "reentrancy" {
        return match vuln_type {
            "arithmetic" => 3.0, // 0.8+ has built-in overflow protection
            "unchecked_ll_calls" => 6.0,
            "time_manipulation" => 4.0,
            "transaction_ordering_dependence" => 3.0,
            _ => 5.0,
        };
    }
    
    let d = desc.to_lowercase();
    
    // Cross-function reentrancy with state change - CRITICAL
    if d.contains("cross-function") || (d.contains("callback") && (d.contains("balances") || d.contains("state") || d.contains("write"))) {
        return 9.5;
    }
    
    // Cross-function reentrancy without verified state change - HIGH
    if d.contains("external") || d.contains("different function") {
        return 7.0;
    }
    
    // Same-function reentrancy - LOW (just recursive calls)
    if d.contains("same-function") || d.contains("recursive") {
        return 3.0;
    }
    
    // Default - assume cross-function until proven otherwise
    6.0
}

fn classify_reentrancy_confidence(desc: &str) -> f64 {
    let d = desc.to_lowercase();
    
    // High confidence if we see clear state modification indicators
    if d.contains("balances[") || d.contains("shares[") || d.contains("state") || d.contains("write") {
        return 0.90;
    }
    
    // Medium if external call detected but state unclear
    if d.contains("external call") || d.contains("callback") {
        return 0.70;
    }
    
    // Low if just recursive pattern
    if d.contains("recursive") || d.contains("loop") {
        return 0.50;
    }
    
    0.65
}

pub async fn run_conkas(project_root: &Path, source_dir: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    let python = match find_venv_python3() {
        Some(p) => p,
        None => { info!("   Conkas: no python3 available — skipping"); return findings; }
    };
    let conkas_script = "/home/user/conkas/conkas.py";
    if !std::path::Path::new(conkas_script).exists() {
        warn!("   Conkas: not found at {} — skipping", conkas_script);
        return findings;
    }

    // Use forge build output (Conkas reads pre-compiled bytecode, no solcx needed)
    let forge_out = project_root.join("out");
    if !forge_out.exists() {
        info!("   Conkas: no forge build output at {:?} — skipping (run 'forge build' first)", forge_out);
        return findings;
    }

    // Find all .sol files in source dir
    let sol_files = find_sol_files(source_dir);
    if sol_files.is_empty() {
        info!("   Conkas: no .sol files found in {:?}", source_dir);
        return findings;
    }

    let mut total_analyzed = 0;
    for sol_path in &sol_files {
        let sol_fname = sol_path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let contract_dir = forge_out.join(sol_fname); // forge uses "Contract.sol/" subdirectory
        if !contract_dir.is_dir() {
            continue;
        }

        let entries = match std::fs::read_dir(&contract_dir) {
            Ok(e) => e,
            _ => continue,
        };

        for entry in entries.flatten() {
            let json_path = entry.path();
            if json_path.extension().map_or(true, |e| e != "json") {
                continue;
            }

            let json_content = match std::fs::read_to_string(&json_path) {
                Ok(c) => c,
                _ => continue,
            };
            let parsed: serde_json::Value = match serde_json::from_str(&json_content) {
                Ok(v) => v,
                _ => continue,
            };

            let runtime_hex = parsed["deployedBytecode"]["object"]
                .as_str()
                .and_then(|s| s.strip_prefix("0x"))
                .unwrap_or("");
            if runtime_hex.is_empty() || runtime_hex.len() < 20 {
                continue;
            }

            let contract_name = json_path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
            let hex_data = runtime_hex.as_bytes();

            // Write bytecode hex to temp file
            let tmp_dir = std::env::temp_dir().join("web3-destroyer-conkas");
            std::fs::create_dir_all(&tmp_dir).ok();
            let tmp_file = tmp_dir.join(format!("{}.hex", contract_name));
            if std::fs::write(&tmp_file, hex_data).is_err() {
                continue;
            }
            total_analyzed += 1;
            info!("   Conkas: analyzing {} (runtime: {} bytes)...", contract_name, runtime_hex.len() / 2);

            let mut cmd = tokio::process::Command::new(&python);
            cmd.arg(&conkas_script)
                .arg(&tmp_file)
                .arg("-fav")
                .arg("-md").arg("50")  // Higher max-depth for complex contracts
                .arg("-t").arg("500")  // Higher timeout for Z3
                .arg("-v").arg("Warning")  // More verbose for debugging
                .kill_on_drop(true);

            let output = match timeout(Duration::from_secs(120), cmd.output()).await {
                Ok(Ok(o)) => o,
                _ => { warn!("   Conkas: timed out on {}", contract_name); continue; }
            };

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            for line in stdout.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("Vulnerability:") {
                    let vuln_type = trimmed
                        .strip_prefix("Vulnerability:")
                        .and_then(|s| s.split('.').next())
                        .map(|s| s.trim().to_string())
                        .unwrap_or_default();
                    let func_name = trimmed
                        .split("Maybe in function:")
                        .nth(1)
                        .and_then(|s| s.split('.').next())
                        .map(|s| s.trim().to_string())
                        .unwrap_or_default();

                    let severity = classify_reentrancy_severity(&trimmed, vuln_type.as_str());
                    let confidence = classify_reentrancy_confidence(&trimmed);

                    findings.push(Finding {
                        tool: ToolKind::Conkas,
                        severity,
                        confidence,
                        description: format!("[{}] {} in {}", vuln_type, trimmed, func_name),
                        target: Address::ZERO,
                        calldata: None,
            
                        evidence: vec![trimmed.to_string()],
                    });
                }
            }

            std::fs::remove_file(&tmp_file).ok();
        }
    }

    if total_analyzed == 0 {
        info!("   Conkas: no compiled contracts found in forge output");
    } else {
        info!("   Conkas: analyzed {} contract(s), {} finding(s)", total_analyzed, findings.len());
    }
    findings
}

fn find_sol_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    if dir.is_file() && dir.extension().map_or(false, |e| e == "sol") {
        files.push(dir.to_path_buf());
    } else if dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().map_or(false, |e| e == "sol") {
                    files.push(p);
                }
            }
        }
    }
    files
}

pub async fn run_mythril(source_dir: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    let venv_python = match find_venv_python3() {
        Some(p) => p,
        None => { info!("   Mythril: python not available — skipping"); return findings; }
    };

    // Try mythril command, skip if not available or fails
    let mythril_exists = std::process::Command::new(&venv_python)
        .args(["-c", "import mythril"])
        .output()
        .ok()
        .map_or(false, |o| o.status.success());

    if !mythril_exists {
        info!("   Mythril: not available (dependency issues) — skipping");
        return findings;
    }

    info!("   Mythril: analyzing {}...", source_dir.display());

    let sol_file = find_sol_files(source_dir).pop();
    let sol_file = match sol_file {
        Some(f) => f,
        None => { warn!("   Mythril: no .sol file found"); return findings; }
    };

    let mut cmd = tokio::process::Command::new(&venv_python);
    cmd.arg("-m").arg("mythril")
        .arg("analyze")
        .arg("-fav")  // Find all vulnerabilities
        .arg("--solver-timeout").arg("5000")
        .arg(&sol_file)
        .kill_on_drop(true);

    let output = match timeout(Duration::from_secs(180), cmd.output()).await {
        Ok(Ok(o)) => o,
        _ => { warn!("   Mythril timed out after 180s"); return findings; }
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    
    // Parse Mythril output - it outputs issues in text format
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Issue") || trimmed.contains("Severity:") {
            let severity = if trimmed.contains("Critical") {
                9.5
            } else if trimmed.contains("High") {
                8.0
            } else if trimmed.contains("Medium") {
                5.0
            } else {
                3.0
            };

            findings.push(Finding {
                tool: ToolKind::Mythril,
                severity,
                confidence: 0.75,
                description: format!("[Mythril] {}", trimmed),
                target: Address::ZERO,
                calldata: None,
    
                evidence: vec![trimmed.to_string()],
            });
        }
    }

    info!("   Mythril: {} finding(s)", findings.len());
    findings
}

pub async fn run_foray(foray_path: Option<&Path>, source_dir: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    let python = match find_venv_python3() {
        Some(p) => p,
        None => { warn!("   Foray: no python3 available"); return findings; }
    };

    // Auto-detect Foray path: try provided path first, then ~/Foray
    let foray_dir: std::path::PathBuf = foray_path
        .filter(|p| p.join("main.py").exists())
        .map(|p| p.to_path_buf())
        .or_else(|| {
            let default = std::path::PathBuf::from("/home/user/Foray");
            if default.join("main.py").exists() {
                Some(default)
            } else {
                None
            }
        })
        .unwrap_or_default();

    let main_py = foray_dir.join("main.py");
    if !main_py.exists() {
        warn!("   Foray: not found at provided path or ~/Foray — skipping");
        return findings;
    }
    info!("   Foray: {} -e -i {}...", main_py.display(), source_dir.display());

    let mut cmd = tokio::process::Command::new(&python);
    cmd.arg(&main_py).arg("-e").arg("-i").arg(source_dir).kill_on_drop(true);

    let result = match timeout(Duration::from_secs(300), cmd.output()).await {
        Ok(Ok(o)) => o,
        _ => { warn!("   Foray timed out after 300s"); return findings; }
    };

    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
    let stderr = String::from_utf8_lossy(&result.stderr).to_string();
    let combined = format!("{}\n{}", stdout, stderr);

    if result.status.success() {
        let calldata = extract_calldata(&combined);
        let profit = extract_profit(&combined);
        let desc = if calldata.is_empty() { "Foray: analysis complete, no exploit found".into() } else { format!("Foray: exploit found — profit {} ETH", profit) };
        findings.push(Finding {
            tool: ToolKind::Foray,
            severity: if calldata.is_empty() { 0.0 } else { 9.0 },
            confidence: 0.8,
            description: desc,
            target: Address::ZERO,
            calldata: if calldata.is_empty() { None } else { Some(calldata) },
            evidence: vec![combined],
        });
    } else {
        warn!("   Foray exited with code {:?}", result.status.code());
    }

    findings
}

pub async fn run_ityfuzz(
    target: Address,
    source_dir: &Path,
    rpc_url: Option<&str>,
    flashloan: bool,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    let binary = find_tool(&["ityfuzz"]).or_else(|| {
        let path = Path::new("/home/user/ityfuzz/target/release/ityfuzz");
        if path.exists() {
            Some(path.to_string_lossy().to_string())
        } else {
            None
        }
    });

    let binary = match binary {
        Some(b) => b,
        None => { warn!("   Ityfuzz not found — skipping"); return findings; }
    };

    info!("   Ityfuzz: fuzzing {}...", target);

    let mut cmd = tokio::process::Command::new(&binary);
    cmd.arg("evm").arg("-t").arg(format!("{:?}", target));

    if let Some(rpc) = rpc_url {
        cmd.env("ETH_RPC_URL", rpc);
        cmd.arg("-u").arg(rpc);
        cmd.arg("-c").arg("local");

        if flashloan {
            cmd.arg("--flashloan");
        }
    } else if source_dir.exists() {
        cmd.arg("--target").arg(source_dir.join("**/*.json"));
        cmd.arg("--target-type").arg("glob");
    }
    cmd.kill_on_drop(true);

    let output = match timeout(Duration::from_secs(180), cmd.output()).await {
        Ok(Ok(o)) => o,
        _ => { warn!("   Ityfuzz timed out after 180s"); return findings; }
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{}\n{}", stdout, stderr);

    let calldata = extract_calldata(&combined);
    let profit = extract_profit(&combined);

    if !calldata.is_empty() || profit > U256::ZERO {
        findings.push(Finding {
            tool: ToolKind::Ityfuzz,
            severity: 9.5,
            confidence: 0.85,
            description: format!("Ityfuzz: exploit found — profit {} ETH", profit),
            target,
            calldata: if calldata.is_empty() { None } else { Some(calldata) },
            evidence: vec![combined],
        });
        info!("   Ityfuzz: exploit found — profit {} ETH", profit);
    } else if output.status.success() {
        info!("   Ityfuzz: analysis complete, no exploits found");
    } else {
        warn!("   Ityfuzz exited with code {:?}", output.status.code());
    }

    findings
}

pub async fn run_medusa(source_dir: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    if !check_tool_available("medusa") {
        warn!("   Medusa not found — skipping");
        return findings;
    }
    info!("   Medusa: fuzzing {}...", source_dir.display());

    let venv_bin = "/home/user/web3-destroyer/.venv/bin";
    let path = std::env::var("PATH").unwrap_or_default();
    let augmented_path = if !path.contains(venv_bin) {
        format!("{}:{}", venv_bin, path)
    } else {
        path
    };

    let mut cmd = tokio::process::Command::new("medusa");
    cmd.arg("fuzz")
        .arg("--compilation-target").arg(source_dir)
        .arg("--test-limit").arg("500")
        .arg("--workers").arg("1")
        .arg("--seq-len").arg("50")
        .env("PATH", &augmented_path)
        .kill_on_drop(true);

    let output = match timeout(Duration::from_secs(120), cmd.output()).await {
        Ok(Ok(o)) => o,
        _ => { warn!("   Medusa timed out after 120s"); return findings; }
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if output.status.success() {
        if stdout.contains("property ") && (stdout.contains("FAILED") || stdout.contains("broken")) {
            let calldata = extract_calldata(&stdout);
            let desc = format!("Medusa: property test failed — exploit possible");
            findings.push(Finding {
                tool: ToolKind::Medusa,
                severity: 7.0,
                confidence: 0.6,
                description: desc,
                target: Address::ZERO,
                calldata: if calldata.is_empty() { None } else { Some(calldata) },
    
                evidence: vec![stdout],
            });
        } else {
            info!("   Medusa: fuzzing complete, no exploits found");
        }
    } else {
        if !stdout.trim().is_empty() && stdout.contains("error") {
            info!("   Medusa: analysis complete");
        }
    }

    findings
}

pub async fn run_halmos(project_root: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();

    let forge_out = project_root.join("out");
    if !forge_out.exists() {
        info!("   Halmos: no forge build output at {:?} — skipping (run 'forge build' first)", forge_out);
        return findings;
    }

    let use_uvx = find_tool(&["uvx"]).is_some();
    let halmos_bin = if find_tool(&["halmos"]).is_some() {
        find_tool(&["halmos"]).unwrap()
    } else if use_uvx {
        info!("   Halmos: using uvx wrapper");
        find_tool(&["uvx"]).unwrap()
    } else {
        warn!("   Halmos not found — skipping (install via `pip install halmos` or `uv tool install halmos`)");
        return findings;
    };

    info!("   Halmos: symbolic testing {}...", project_root.display());

    let mut cmd = tokio::process::Command::new(&halmos_bin);
    if use_uvx && halmos_bin.contains("uvx") {
        cmd.arg("halmos");
    }
    cmd.arg("--root").arg(project_root)
        .arg("--forge-build-out").arg("out")
        .kill_on_drop(true);

    let output = match timeout(Duration::from_secs(120), cmd.output()).await {
        Ok(Ok(o)) => o,
        _ => { warn!("   Halmos timed out after 120s"); return findings; }
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{}\n{}", stdout, stderr);

    if combined.contains("Counterexample") || combined.contains("FAILED") {
        let calldata = extract_calldata(&combined);
        findings.push(Finding {
            tool: ToolKind::Halmos,
            severity: 9.0,
            confidence: 0.75,
            description: "Halmos: counterexample found".into(),
            target: Address::ZERO,
            calldata: if calldata.is_empty() { None } else { Some(calldata) },

            evidence: vec![combined],
        });
    } else if combined.contains("No tests matched") {
        info!("   Halmos: no property tests found in contracts (need check_*/invariant_* functions)");
    } else {
        let passed = stdout.lines().filter(|l| l.contains("PASS")).count();
        info!("   Halmos: analysis complete — {} test(s) passed with no counterexamples", passed);
    }

    findings
}

pub async fn run_heimdall(bytecode: &[u8], target: Address) -> Vec<Finding> {
    let mut findings = Vec::new();
    if bytecode.len() <= 4 {
        return findings;
    }

    let heimdall = HeimdallAnalyzer::new();
    let result = heimdall.analyze(bytecode, target);

    for indicator in &result.risk_indicators {
        let severity = if indicator.contains("SELFDESTRUCT") { 10.0 }
            else if indicator.contains("DELEGATECALL") { 8.0 }
            else if indicator.contains("SELFCALL") { 6.0 }
            else if indicator.contains("contract creation") { 4.0 }
            else if indicator.contains("writable slots") { 6.0 }
            else if indicator.contains("High storage slot") { 3.0 }
            else { 5.0 };

        findings.push(Finding {
            tool: ToolKind::Heimdall,
            severity,
            confidence: 0.7,
            description: indicator.clone(),
            target,
            calldata: None,

            evidence: vec![],
        });
    }

    if !findings.is_empty() {
        info!("   Heimdall: {} risk indicator(s) found", findings.len());
    }

    findings
}

// ================================================================
// Aderyn
// ================================================================

pub async fn run_aderyn(source_dir: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    if !find_tool(&["aderyn"]).is_some() {
        return findings;
    }

    let output = match tokio::time::timeout(
        std::time::Duration::from_secs(60),
        tokio::process::Command::new("aderyn")
            .arg(source_dir)
            .arg("--output-format")
            .arg("json")
            .output()
    ).await {
        Ok(Ok(o)) => o,
        _ => return findings,
    };

    if !output.status.success() {
        return findings;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(v) => v,
        _ => {
            // Try to parse from the file aderyn writes
            let report_path = source_dir.join("report.json");
            match std::fs::read_to_string(&report_path) {
                Ok(s) => serde_json::from_str(&s).unwrap_or(serde_json::Value::Null),
                _ => return findings,
            }
        }
    };

    let issues = parsed["issues"].as_array()
        .or_else(|| parsed["findings"].as_array())
        .cloned()
        .unwrap_or_default();

    for issue in issues {
        let check = issue["check"].as_str().unwrap_or("unknown");
        let description = issue["description"].as_str()
            .or_else(|| issue["message"].as_str())
            .unwrap_or("No description");
        let severity_str = issue["severity"].as_str().unwrap_or("medium").to_lowercase();
        let file = issue["file"].as_str().unwrap_or("");
        let line = issue["line"].as_u64().unwrap_or(0);

        let severity = match severity_str.as_str() {
            "critical" | "high" => 9.0,
            "medium" => 6.0,
            "low" => 3.0,
            "gas" | "info" => 1.0,
            _ => 5.0,
        };

        findings.push(Finding {
            tool: ToolKind::Aderyn,
            severity,
            confidence: 0.75,
            description: format!("[{}] {} at {}:{} — {}", check, severity_str, file, line, description),
            target: Address::ZERO,
            calldata: None,
            evidence: vec![],
        });
    }

    findings
}

// ================================================================
// Orchestrator
// ================================================================

pub async fn orchestrate(
    target: Address,
    source_dir: Option<&Path>,
    foray_path: Option<&Path>,
    bytecode: &[u8],
    proxy_address: Option<Address>,
    rpc_url: Option<&str>,
    ityfuzz_flashloan: bool,
    scan_mode: ScanMode,
) -> Vec<Finding> {
    info!("");
    info!("╔══════════════════════════════════════════════╗");
    info!("║         TOOL ORCHESTRATION PIPELINE          ║");
    info!("╚══════════════════════════════════════════════╝");
    info!("");

    // ── Phase 1: Scouts ──
    info!("┌──────────────────────────────────────────────┐");
    info!("│ Phase 1: Scouts (parallel static + symbolic) │");
    info!("└──────────────────────────────────────────────┘");
    let mut all = Vec::new();

    // Project root (parent of contracts dir) needed for forge build output
    let project_root = source_dir.and_then(|s| s.parent()).unwrap_or_else(|| {
        if let Some(src) = source_dir { src } else { std::path::Path::new(".") }
    });

    let heimdall_findings = run_heimdall(bytecode, target).await;
    info!("   Heimdall: {} finding(s)", heimdall_findings.len());

    if let Some(src) = source_dir {
        let (slither_findings, conkas_findings, wake_findings, mythril_findings, aderyn_findings) = tokio::join!(
            run_slither(src),
            run_conkas(project_root, src),
            wake::run_wake(src),
            run_mythril(src),
            run_aderyn(src),
        );
        info!("   Slither: {} finding(s)", slither_findings.len());
        info!("   Conkas: {} finding(s)", conkas_findings.len());
        info!("   Wake: {} finding(s)", wake_findings.len());
        info!("   Mythril: {} finding(s)", mythril_findings.len());
        info!("   Aderyn: {} finding(s)", aderyn_findings.len());
        all.extend(slither_findings);
        all.extend(conkas_findings);
        all.extend(wake_findings);
        all.extend(mythril_findings);
        all.extend(aderyn_findings);
    }

    all.extend(heimdall_findings);
    all.sort_by(|a, b| b.score().partial_cmp(&a.score()).unwrap_or(std::cmp::Ordering::Equal));
    info!("   Phase 1 total: {} finding(s)", all.len());

    // ── Phase 2: Synthesizer ──
    info!("");
    info!("┌──────────────────────────────────────────────┐");
    info!("│ Phase 2: Calldata Synthesis (top findings)   │");
    info!("└──────────────────────────────────────────────┘");

    let synth_count = synthesizer::synthesize(&mut all, bytecode, target);
    info!("   Synthesizer: {} finding(s) now have calldata", synth_count);

    // Re-rank: findings with calldata get boosted
    for f in all.iter_mut() {
        if f.calldata.is_some() {
            f.confidence = f.confidence.max(0.9);
        }
    }
    all.sort_by(|a, b| b.score().partial_cmp(&a.score()).unwrap_or(std::cmp::Ordering::Equal));

    // ── Phase 2b: Chain Builder ──
    info!("");
    info!("┌──────────────────────────────────────────────┐");
    info!("│ Phase 2b: Exploit Chain Detection            │");
    info!("└──────────────────────────────────────────────┘");
    let chains = chain::build_chains(&all, target, proxy_address, bytecode);
    if chains.is_empty() {
        info!("   Chains: no multi-step exploit chains detected");
    } else {
        info!("   Chains: {} exploit chain(s) built", chains.len());
        let chain_findings = chain::chains_to_findings(&chains, target);
        let chain_count = chain_findings.len();
        all.extend(chain_findings);
        info!("   Chains: {} chain finding(s) added", chain_count);
    }

    // ── Phase 3: Confirmation (slow tools, only if source) ──
    if let Some(src) = source_dir {
        if scan_mode == ScanMode::Quick {
            info!("   Phase 3: skipped (--mode=quick)");
        } else {
            info!("");
            info!("┌──────────────────────────────────────────────┐");
            info!("│ Phase 3: Confirmation (Halmos + Medusa)      │");
            info!("└──────────────────────────────────────────────┘");

            // Halmos needs project root (with forge build output), medusa needs source dir
            let (halmos_findings, medusa_findings) = tokio::join!(
                run_halmos(project_root),
                run_medusa(src),
            );
            info!("   Halmos: {} finding(s)", halmos_findings.len());
            info!("   Medusa: {} finding(s)", medusa_findings.len());
            all.extend(halmos_findings);
            all.extend(medusa_findings);

            if scan_mode == ScanMode::Deep {
                if let Some(fp) = foray_path {
                    info!("   Foray: running exploit synthesis...");
                    let foray_findings = run_foray(Some(fp), src).await;
                    info!("   Foray: {} finding(s)", foray_findings.len());
                    all.extend(foray_findings);
                }

                if let Some(rpc) = rpc_url {
                    info!("   Ityfuzz: running on-chain fuzzing...");
                    let ityfuzz_findings = run_ityfuzz(target, src, Some(rpc), ityfuzz_flashloan).await;
                    info!("   Ityfuzz: {} finding(s)", ityfuzz_findings.len());
                    all.extend(ityfuzz_findings);
                }
            }
        }
    }

    // ── Final ranking ──
    all.sort_by(|a, b| b.score().partial_cmp(&a.score()).unwrap_or(std::cmp::Ordering::Equal));

    // Show top-N findings only
    let top = all.len().min(15);
    info!("");
    info!("┌──────────────────────────────────────────────┐");
    info!("│ TOP {} FINDINGS (by score)                    │", top);
    info!("└──────────────────────────────────────────────┘");
    for f in all.iter().take(top) {
        let has_cd = if f.calldata.is_some() { " [calldata]" } else { "" };
        info!("   [{:.1}] {}{} — {}", f.score(), f.tool, has_cd, truncate(&f.description, 100));
    }
    info!("   ... ({} total, showing top {})", all.len(), top);

    all
}

pub async fn verify_findings(findings: &[Finding], rpc_url: &str, _block_number: u64, min_score: f64) -> Vec<VerifiedFinding> {
    let mut verified = Vec::new();

    // Configurable private key: use PRIVATE_KEY env var, or default anvil account
    let pk = std::env::var("PRIVATE_KEY")
        .unwrap_or_else(|_| "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string());

    // Derive attacker address from private key (simplified - using known addresses)
    let attacker = if pk == "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80" {
        "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266" // anvil default #0
    } else if pk == "0x59c6995e998f97a5a0044966f0945389dc9e88dae7c3a0a0e7c5e5c3a0e7c3a0e7" {
        "0x5ca1a12440144A52aCb9DfaB2Cb6B22dC30b3aD8" // anvil default #1
    } else {
        "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266" // fallback to default
    };

    // Filter: Only verify findings above threshold
    let exploit_findings: Vec<&Finding> = findings.iter()
        .filter(|f| f.calldata.is_some() && f.score() >= min_score)
        .collect();

    info!("   Verifying {} finding(s) on-chain (score >= {})...", exploit_findings.len(), min_score);

    for f in &exploit_findings {
        let balance_before = get_eth_balance(attacker, rpc_url).await;

        let is_chain = f.description.starts_with("[chain]");

        if is_chain {
            let steps = chain::parse_chain_steps(&f.evidence);
            if steps.is_empty() {
                info!("   ⚠️  {} chain has no parseable steps", f.tool);
                let mut ev = f.evidence.clone();
                ev.insert(0, "No parseable steps in evidence".into());
                verified.push(VerifiedFinding {
                    tool: f.tool.clone(),
                    description: f.description.clone(),
                    target: f.target,
                    calldata: Bytes::default(),
                    profit_estimate: U256::ZERO,
                    severity: f.severity,
                    score: f.score(),
                    evidence: ev,
                    status: VerifyStatus::Reverted,
                });
                continue;
            }

            let mut all_ok = true;
            for step in &steps {
                let calldata_hex = format!("0x{}", hex::encode(&step.calldata));
                let value = if step.value > U256::ZERO { Some(step.value) } else { None };
                let ok = cast_send_with_timeout(
                    step.target, &["--data", &calldata_hex],
                    rpc_url, &pk, 15, value,
                ).await;
                if !ok {
                    info!("   ⚠️  {} chain step reverted: {}", f.tool, step.description);
                    all_ok = false;
                    break;
                }
            }

            if !all_ok {
                let mut ev = f.evidence.clone();
                ev.insert(0, "Chain step reverted during verification".into());
                verified.push(VerifiedFinding {
                    tool: f.tool.clone(),
                    description: f.description.clone(),
                    target: f.target,
                    calldata: steps[0].calldata.clone(),
                    profit_estimate: U256::ZERO,
                    severity: f.severity,
                    score: f.score(),
                    evidence: ev,
                    status: VerifyStatus::Reverted,
                });
                continue;
            }

            let balance_after = get_eth_balance(attacker, rpc_url).await;
            let profit = balance_after.saturating_sub(balance_before);

            let status = if profit > U256::ZERO { VerifyStatus::Verified } else { VerifyStatus::Partial };
            if profit > U256::ZERO {
                info!("   ✅ {} chain verified — profit: {} ETH", f.tool, profit);
            } else {
                info!("   ℹ️  {} chain executed but no profit", f.tool);
            }
            verified.push(VerifiedFinding {
                tool: f.tool.clone(),
                description: f.description.clone(),
                target: f.target,
                calldata: steps[0].calldata.clone(),
                profit_estimate: profit,
                severity: f.severity,
                    score: f.score(),
                evidence: f.evidence.clone(),
                status,
            });
        } else {
            let calldata = f.calldata.as_ref().unwrap();
            if calldata.is_empty() { continue; }

            let calldata_hex = format!("0x{}", hex::encode(calldata));

            let ok = cast_send_with_timeout(
                f.target, &["--data", &calldata_hex],
                rpc_url, &pk, 15, None,
            ).await;

            if !ok {
                info!("   ⚠️  {} on {:?} reverted", f.tool, f.target);
                let mut ev = f.evidence.clone();
                ev.insert(0, format!("TX reverted on {:?}", f.target));
                verified.push(VerifiedFinding {
                    tool: f.tool.clone(),
                    description: f.description.clone(),
                    target: f.target,
                    calldata: calldata.clone(),
                    profit_estimate: U256::ZERO,
                    severity: f.severity,
                    score: f.score(),
                    evidence: ev,
                    status: VerifyStatus::Reverted,
                });
                continue;
            }

            let balance_after = get_eth_balance(attacker, rpc_url).await;
            let profit = balance_after.saturating_sub(balance_before);

            let status = if profit > U256::ZERO { VerifyStatus::Verified } else { VerifyStatus::Partial };
            if profit > U256::ZERO {
                info!("   ✅ {} verified — profit: {} ETH", f.tool, profit);
            } else {
                info!("   ℹ️  {} executed but no profit", f.tool);
            }
            verified.push(VerifiedFinding {
                tool: f.tool.clone(),
                description: f.description.clone(),
                target: f.target,
                calldata: calldata.clone(),
                profit_estimate: profit,
                severity: f.severity,
                    score: f.score(),
                evidence: f.evidence.clone(),
                status,
            });
        }
    }

    verified
}

pub async fn fetch_current_block(rpc_url: &str) -> u64 {
    match timeout(Duration::from_secs(10), tokio::process::Command::new("cast")
        .arg("block-number").arg("--rpc-url").arg(rpc_url).output()
    ).await {
        Ok(Ok(out)) if out.status.success() => {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            s.parse::<u64>().unwrap_or(0)
        }
        _ => 0,
    }
}

// ================================================================
// Utilities
// ================================================================

pub async fn discover_oracle_address(target: Address, rpc_url: &str) -> Option<Address> {
    // Read storage slot 0 — heuristic for `address private oracle` pattern
    let output = tokio::process::Command::new("cast")
        .args(["storage", &format!("{:?}", target), "0", "--rpc-url", rpc_url])
        .output().await.ok()?;
    if !output.status.success() { return None; }
    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let s = s.strip_prefix("0x").unwrap_or(&s);
    let bytes = hex::decode(s).ok()?;
    if bytes.len() < 20 { return None; }
    let addr = Address::from_slice(&bytes[12..32]);

    // Verify it's a contract (has code)
    let code = tokio::process::Command::new("cast")
        .args(["code", &format!("{:?}", addr), "--rpc-url", rpc_url])
        .output().await.ok()?;
    if code.status.success() {
        let code_str = String::from_utf8_lossy(&code.stdout).trim().to_string();
        let code_str = code_str.strip_prefix("0x").unwrap_or(&code_str);
        if !code_str.is_empty() {
            info!("   Discovered oracle/proxy at {:?} (storage slot 0)", addr);
            return Some(addr);
        }
    }
    None
}

async fn get_eth_balance(addr: &str, rpc_url: &str) -> U256 {
    match timeout(Duration::from_secs(10), tokio::process::Command::new("cast")
        .arg("balance").arg(addr).arg("--rpc-url").arg(rpc_url).output()
    ).await {
        Ok(Ok(out)) if out.status.success() => {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            U256::from_str_radix(&s, 10).unwrap_or(U256::ZERO)
        }
        _ => U256::ZERO,
    }
}

async fn preflight_check(to: Address, calldata: &Bytes, rpc_url: &str, value: Option<U256>) -> bool {
    let cd_hex = format!("0x{}", hex::encode(calldata));
    let mut cmd = tokio::process::Command::new("cast");
    cmd.arg("call")
        .arg(format!("{:?}", to))
        .arg("--rpc-url").arg(rpc_url)
        .arg("--from").arg("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
    
    if let Some(val) = value {
        cmd.arg("--value").arg(format!("{}", val));
    }
    cmd.arg("--data").arg(&cd_hex);

    let output = match timeout(Duration::from_secs(10), cmd.output()).await {
        Ok(Ok(o)) => o,
        _ => return true, // If check fails, allow the attempt
    };

    // If the call reverts, don't bother sending
    !String::from_utf8_lossy(&output.stderr).contains("reverted")
        && !String::from_utf8_lossy(&output.stdout).contains("0x08c379a0") // Panic selector
}

async fn cast_send_with_timeout(
    to: Address, args: &[&str], rpc_url: &str, pk: &str, secs: u64, value: Option<U256>,
) -> bool {
    // Extract calldata from args for preflight check
    let calldata = if let Some(data_idx) = args.iter().position(|a| *a == "--data") {
        args.get(data_idx + 1).and_then(|s| {
            let hex = s.strip_prefix("0x").unwrap_or(s);
            hex::decode(hex).ok().map(Bytes::from)
        })
    } else {
        None
    };

    // Preflight check - simulate before sending
    if let Some(ref cd) = calldata {
        if !preflight_check(to, cd, rpc_url, value).await {
            return false;
        }
    }

    let mut cmd = tokio::process::Command::new("cast");
    cmd.arg("send").arg(format!("{:?}", to)).arg("--rpc-url").arg(rpc_url)
        .arg("--private-key").arg(pk).arg("--timeout").arg(&secs.to_string());
    if let Some(val) = value {
        cmd.arg("--value").arg(format!("{}", val));
    }
    for arg in args {
        cmd.arg(arg);
    }
    timeout(Duration::from_secs(secs + 10), cmd.output()).await
        .ok()
        .and_then(|o| o.ok())
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn extract_calldata(output: &str) -> Bytes {
    for line in output.lines() {
        if let Some(hex_start) = line.find("0x") {
            let rest: String = line[hex_start + 2..].chars().take_while(|c| c.is_ascii_hexdigit()).collect();
            if rest.len() >= 8 && rest.len() % 2 == 0 {
                if let Ok(bytes) = hex::decode(&rest) {
                    return Bytes::from(bytes);
                }
            }
        }
    }
    Bytes::new()
}

fn extract_profit(output: &str) -> U256 {
    for line in output.lines() {
        let low = line.to_lowercase();
        if low.contains("profit") || low.contains("value") || low.contains("ether") {
            for w in line.split_whitespace() {
                let cleaned: String = w.chars().filter(|c| c.is_ascii_digit()).collect();
                if let Ok(n) = cleaned.parse::<u128>() {
                    return U256::from(n);
                }
            }
        }
    }
    U256::ZERO
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max { format!("{}...", &s[..max]) } else { s.to_string() }
}
