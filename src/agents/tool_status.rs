//! Tool availability and status tracking
//!
//! This module provides tool health monitoring for the pipeline.
//! It tracks which tools were attempted, succeeded, failed, or were unavailable.

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolState {
    /// Tool was found and executed successfully
    Available,
    /// Tool was found but execution failed
    Failed,
    /// Tool binary not found in PATH
    Missing,
    /// Tool timed out during execution
    TimedOut,
    /// Tool execution was skipped (e.g., no source directory)
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStatus {
    pub name: String,
    pub state: ToolState,
    pub finding_count: usize,
    pub error_message: Option<String>,
}

impl ToolStatus {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            state: ToolState::Skipped,
            finding_count: 0,
            error_message: None,
        }
    }

    pub fn available(findings: usize) -> Self {
        Self {
            name: String::new(),
            state: ToolState::Available,
            finding_count: findings,
            error_message: None,
        }
    }

    pub fn failed(msg: &str) -> Self {
        Self {
            name: String::new(),
            state: ToolState::Failed,
            finding_count: 0,
            error_message: Some(msg.to_string()),
        }
    }

    pub fn missing() -> Self {
        Self {
            name: String::new(),
            state: ToolState::Missing,
            finding_count: 0,
            error_message: None,
        }
    }

    pub fn timed_out() -> Self {
        Self {
            name: String::new(),
            state: ToolState::TimedOut,
            finding_count: 0,
            error_message: Some("Execution timed out".to_string()),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ToolReport {
    pub tools: Vec<ToolStatus>,
}

impl ToolReport {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    pub fn record(&mut self, name: &str, status: ToolStatus) {
        // Update existing or add new
        if let Some(existing) = self.tools.iter_mut().find(|t| t.name == name) {
            *existing = status;
        } else {
            let mut s = status;
            s.name = name.to_string();
            self.tools.push(s);
        }
    }

    pub fn record_available(&mut self, name: &str, findings: usize) {
        self.record(name, ToolStatus::available(findings));
    }

    pub fn record_missing(&mut self, name: &str) {
        self.record(name, ToolStatus::missing());
    }

    pub fn record_failed(&mut self, name: &str, error: &str) {
        self.record(name, ToolStatus::failed(error));
    }

    pub fn record_timed_out(&mut self, name: &str) {
        self.record(name, ToolStatus::timed_out());
    }

    pub fn record_skipped(&mut self, name: &str) {
        self.record(name, ToolStatus::new(name));
    }

    pub fn summary(&self) -> String {
        let mut lines = Vec::new();
        lines.push("═══════════════════════════════════════════════".to_string());
        lines.push("  Tool Availability Report".to_string());
        lines.push("═══════════════════════════════════════════════".to_string());

        let mut available_count = 0;
        let mut missing_count = 0;
        let mut failed_count = 0;
        let mut timed_out_count = 0;
        let mut skipped_count = 0;

        for tool in &self.tools {
            match tool.state {
                ToolState::Available => {
                    available_count += 1;
                    lines.push(format!(
                        "  ✅ {} — {} finding(s)",
                        tool.name, tool.finding_count
                    ));
                }
                ToolState::Missing => {
                    missing_count += 1;
                    lines.push(format!(
                        "  ❌ {} — NOT FOUND (install to enable)",
                        tool.name
                    ));
                }
                ToolState::Failed => {
                    failed_count += 1;
                    let err = tool.error_message.as_deref().unwrap_or("unknown error");
                    lines.push(format!("  ⚠️  {} — FAILED ({})", tool.name, err));
                }
                ToolState::TimedOut => {
                    timed_out_count += 1;
                    lines.push(format!("  ⏱️  {} — TIMED OUT", tool.name));
                }
                ToolState::Skipped => {
                    skipped_count += 1;
                    lines.push(format!("  ➖ {} — SKIPPED (no input provided)", tool.name));
                }
            }
        }

        lines.push("".to_string());
        lines.push(format!(
            "  Total: {} ✅  {} ❌  {} ⚠️  {} ⏱️  {} ➖",
            available_count, missing_count, failed_count, timed_out_count, skipped_count
        ));

        if missing_count > 0 || failed_count > 0 {
            lines.push("".to_string());
            lines.push("  ⚠️  Some tools unavailable — results may be incomplete".to_string());
        }

        lines.join("\n")
    }

    pub fn has_failures(&self) -> bool {
        self.tools
            .iter()
            .any(|t| t.state == ToolState::Failed || t.state == ToolState::Missing)
    }

    pub fn available_tools(&self) -> Vec<&str> {
        self.tools
            .iter()
            .filter(|t| t.state == ToolState::Available)
            .map(|t| t.name.as_str())
            .collect()
    }

    pub fn missing_tools(&self) -> Vec<&str> {
        self.tools
            .iter()
            .filter(|t| t.state == ToolState::Missing)
            .map(|t| t.name.as_str())
            .collect()
    }
}

impl fmt::Display for ToolState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolState::Available => write!(f, "available"),
            ToolState::Failed => write!(f, "failed"),
            ToolState::Missing => write!(f, "missing"),
            ToolState::TimedOut => write!(f, "timed out"),
            ToolState::Skipped => write!(f, "skipped"),
        }
    }
}

/// List of all tools that can be run by the pipeline
pub const ALL_TOOLS: &[&str] = &[
    "Slither", "Conkas", "Wake", "Mythril", "Heimdall", "Halmos", "Medusa", "Foray", "Ityfuzz",
    "cast", "forge",
];

/// Check which tools are available on the system
pub fn check_all_tools() -> ToolReport {
    let mut report = ToolReport::new();

    for tool in ALL_TOOLS {
        if which::which(tool).is_ok() {
            report.record_available(tool, 0);
        } else {
            report.record_missing(tool);
        }
    }

    report
}
