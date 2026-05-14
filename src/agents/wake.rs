use std::path::Path;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{info, warn};

use alloy::primitives::Address;
use crate::agents::finding::{Finding, ToolKind};

pub async fn run_wake(source_dir: &Path) -> Vec<Finding> {
    // Try local first, then Docker
    let findings = run_wake_local(source_dir).await;
    if !findings.is_empty() || check_wake_available() {
        return findings;
    }

    let docker_findings = run_wake_docker(source_dir).await;
    if !docker_findings.is_empty() || docker_maybe_available() {
        return docker_findings;
    }

    warn!("   Wake not available. Install eth-wake or pull ackeeblockchain/wake Docker image.");
    Vec::new()
}

fn tool_maybe_available_locally() -> bool {
    let venv = "/home/user/web3-destroyer/.venv/bin/wake";
    std::path::Path::new(venv).exists()
        || std::process::Command::new("which")
            .arg("wake")
            .output()
            .ok()
            .map_or(false, |o| o.status.success())
}

fn docker_maybe_available() -> bool {
    let docker_ok = std::process::Command::new("which")
        .arg("docker")
        .output()
        .ok()
        .map_or(false, |o| o.status.success());

    if !docker_ok {
        return false;
    }

    let image_ok = std::process::Command::new("docker")
        .args(["image", "inspect", "ackeeblockchain/wake:latest"])
        .output()
        .ok()
        .map_or(false, |o| o.status.success());

    if !image_ok {
        info!("   Docker available but ackeeblockchain/wake:latest not pulled");
    }

    image_ok
}

async fn run_wake_local(source_dir: &Path) -> Vec<Finding> {
    let wake_bin = if std::path::Path::new("/home/user/web3-destroyer/.venv/bin/wake").exists() {
        "/home/user/web3-destroyer/.venv/bin/wake".to_string()
    } else if let Ok(path) = std::process::Command::new("which").arg("wake").output() {
        String::from_utf8_lossy(&path.stdout).trim().to_string()
    } else {
        warn!("   Wake not found — skipping local");
        return Vec::new();
    };

    if wake_bin.is_empty() {
        warn!("   Wake not found — skipping local");
        return Vec::new();
    }

    info!("   Wake (local): analyzing {}...", source_dir.display());

    let venv_bin = "/home/user/web3-destroyer/.venv/bin";
    let path = std::env::var("PATH").unwrap_or_default();
    let augmented_path = if !path.contains(venv_bin) {
        format!("{}:{}", venv_bin, path)
    } else {
        path
    };

    // Run wake detect directly on the source directory (no wake up needed)
    let mut detect = tokio::process::Command::new(&wake_bin);
    detect.args(["detect", "all", "--min-impact", "medium", "--min-confidence", "high"])
        .arg(source_dir)
        .env("PATH", &augmented_path)
        .kill_on_drop(true);

    let output = match timeout(Duration::from_secs(300), detect.output()).await {
        Ok(Ok(o)) => o,
        _ => { warn!("   Wake detect timed out"); return Vec::new(); }
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{}\n{}", stdout, stderr);

    let findings = parse_wake_output(&combined);
    info!("   Wake (local): {} finding(s)", findings.len());
    findings
}

async fn run_wake_docker(source_dir: &Path) -> Vec<Finding> {
    if !docker_maybe_available() {
        return Vec::new();
    }

    info!("   Wake (Docker): analyzing {}...", source_dir.display());

    let src_abs = source_dir.canonicalize().unwrap_or_else(|_| source_dir.to_path_buf());
    let src_str = src_abs.to_string_lossy().to_string();

    let mut cmd = tokio::process::Command::new("docker");
    cmd.args([
        "run", "--rm",
        "-v", &format!("{}:/share", src_str),
        "ackeeblockchain/wake:latest",
        "sh", "-c",
        &format!("wake up /share && wake detect all --min-impact medium --min-confidence high"),
    ]).kill_on_drop(true);

    let output = match timeout(Duration::from_secs(300), cmd.output()).await {
        Ok(Ok(o)) => o,
        _ => { warn!("   Wake Docker timed out"); return Vec::new(); }
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{}\n{}", stdout, stderr);

    let findings = parse_wake_output(&combined);
    info!("   Wake (Docker): {} finding(s)", findings.len());
    findings
}

fn parse_wake_output(output: &str) -> Vec<Finding> {
    let mut findings = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || !trimmed.starts_with('[') {
            continue;
        }

        // Expected format: [Impact] [Confidence] detector_name file:line message
        let parts: Vec<&str> = trimmed.splitn(5, ' ').collect();
        if parts.len() < 5 {
            continue;
        }

        let impact = parts[0].trim_matches(|c| c == '[' || c == ']');
        let confidence_label = parts[1].trim_matches(|c| c == '[' || c == ']');
        let detector = parts[2];
        let location = parts[3]; // file:line
        let message = parts[4]; // rest of message

        let (severity, vuln_class) = classify_wake_detector(detector);

        let confidence = match confidence_label.to_lowercase().as_str() {
            "critical" | "high" => 0.9,
            "medium" => 0.7,
            "low" => 0.5,
            _ => 0.6,
        };

        let final_severity = match impact.to_lowercase().as_str() {
            "critical" => severity.max(9.0),
            "high" => severity.max(7.0),
            "medium" => severity.max(5.0),
            "low" => severity.max(3.0),
            _ => severity,
        };

        findings.push(Finding {
            tool: ToolKind::Wake,
            severity: final_severity,
            confidence,
            description: format!("[{}] {} at {}: {}", vuln_class, detector, location, message),
            target: Address::ZERO,
            calldata: None,

            evidence: vec![trimmed.to_string()],
        });
    }

    findings
}

fn classify_wake_detector(detector: &str) -> (f64, &'static str) {
    match detector {
        "reentrancy" => (9.0, "Reentrancy"),
        "tx_origin" => (7.0, "AccessControl"),
        "unsafe_delegatecall" => (8.0, "AccessControl"),
        "unprotected_selfdestruct" => (10.0, "AccessControl"),
        "chainlink_deprecated_function" => (6.0, "OracleManipulation"),
        "balance_relied_on" => (5.0, "Validation"),
        "unsafe_erc20_call" => (6.0, "Validation"),
        "unchecked_return_value" => (5.0, "Validation"),
        _ => {
            if detector.contains("storage") || detector.contains("struct")
                || detector.contains("mapping") || detector.contains("array")
            {
                (3.0, "Storage")
            } else {
                (2.0, "CodeQuality")
            }
        }
    }
}

pub fn check_wake_available() -> bool {
    let venv = "/home/user/web3-destroyer/.venv/bin/wake";
    std::path::Path::new(venv).exists() || tool_maybe_available_locally() || docker_maybe_available()
}
