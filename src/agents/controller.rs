use alloy::primitives::{Address, U256};
use std::collections::HashSet;
use std::path::PathBuf;
use tracing::{info, warn};

use crate::agents::economic::EconomicSimulator;
use crate::agents::finding::{VerifiedFinding, VerifyStatus, deduplicate_findings};
use crate::agents::hunt;
use crate::agents::wake;
use crate::agents::tool_status::{ToolReport, ToolState};
use crate::agents::ScanMode;
use crate::report;

pub struct Controller {
    rpc_url: String,
    scan_mode: ScanMode,
    target_contracts: HashSet<Address>,
    target_source_dir: Option<PathBuf>,
    foray_path: Option<PathBuf>,
    proxy_override: Option<Address>,
    ityfuzz_flashloan: bool,
    block_number: Option<u64>,

    verified_exploits: Vec<VerifiedFinding>,
    all_findings: Vec<crate::agents::finding::Finding>,
    tool_report: ToolReport,
}

impl Controller {
    pub fn new(rpc_url: String, scan_mode: ScanMode) -> Self {
        Self {
            rpc_url,
            scan_mode,
            target_contracts: HashSet::new(),
            target_source_dir: None,
            foray_path: None,
            proxy_override: None,
            ityfuzz_flashloan: false,
            block_number: None,
            verified_exploits: Vec::new(),
            all_findings: Vec::new(),
            tool_report: ToolReport::new(),
        }
    }

    pub fn with_proxy(mut self, proxy: Address) -> Self {
        self.proxy_override = Some(proxy);
        self
    }

    pub fn with_source_dir(mut self, dir: PathBuf) -> Self {
        self.target_source_dir = Some(dir);
        self
    }

    pub fn with_foray_path(mut self, path: PathBuf) -> Self {
        self.foray_path = Some(path);
        self
    }

    pub fn with_block_number(mut self, block: u64) -> Self {
        self.block_number = Some(block);
        self
    }

    pub fn add_target(&mut self, address: Address) {
        self.target_contracts.insert(address);
    }

    pub async fn run_pipeline(&mut self) -> PipelineSummary {
        info!("");
        info!("══════════════════════════════════════════════");
        info!("  Web3 Destroyer — Tool Orchestration Pipeline");
        info!("══════════════════════════════════════════════");
        info!("");
        info!(" Target: {:?}", self.target_contracts.iter().next().copied().unwrap_or(Address::ZERO));
        info!("");

        // Tool availability check
        let tool_check = hunt::check_tools_available();
        for tool in &tool_check.tools {
            match tool.state {
                ToolState::Missing => {
                    info!("   ⚠️  {} not found — install to enable full analysis", tool.name);
                }
                _ => {}
            }
        }

        if !wake::check_wake_available() && self.target_source_dir.is_some() {
            info!("   ⚠️  Wake not available locally or via Docker — install eth-wake or pull docker image");
        }

        // Resolve block number
        let block_number = match self.block_number {
            Some(b) => {
                info!("   Using pinned block: {}", b);
                b
            }
            None => {
                let b = hunt::fetch_current_block(&self.rpc_url).await;
                info!("   Using current block: {}", b);
                b
            }
        };

        // Phase 1: Recon
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        info!(" Phase 1: Recon");
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        let bytecode = self.fetch_bytecode().await;
        info!("   Bytecode: {} bytes", bytecode.len());

        let target = self.target_contracts.iter().next().copied().unwrap_or(Address::ZERO);

        // Discover oracle/proxy address (heuristic: storage slot 0, or override)
        let proxy = if let Some(p) = self.proxy_override {
            info!("   Using proxy override: {:?}", p);
            Some(p)
        } else {
            let discovered = hunt::discover_oracle_address(target, &self.rpc_url).await;
            if let Some(p) = discovered {
                info!("   Discovered oracle proxy at {:?} (storage slot 0)", p);
            }
            discovered
        };

        // Phase 2: Tool Orchestration
        let findings = hunt::orchestrate(
            target,
            self.target_source_dir.as_deref(),
            self.foray_path.as_deref(),
            &bytecode,
            proxy,
            Some(&self.rpc_url),
            self.ityfuzz_flashloan,
            self.scan_mode,
        ).await;

        // Deduplicate findings from multiple tools
        let deduped = deduplicate_findings(&findings);
        info!("   {} findings ({} after deduplication)", findings.len(), deduped.len());

        // Phase 2b: Economic Exploit Simulation
        info!("");
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        info!(" Phase 2b: Economic Exploit Simulation");
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        let mut economic_sim = EconomicSimulator::new(&self.rpc_url);
        let economic_findings = economic_sim.analyze(target, proxy, &bytecode).await;
        let mut all_with_economic: Vec<_> = deduped;
        for ef in &economic_findings {
            info!("   💰 {} — profit: {:.6} ETH (confidence: {:.1})",
                ef.description,
                crate::agents::economic::u256_to_f64(ef.profit_estimate) / 1e18,
                ef.confidence);
            all_with_economic.push(ef.to_finding());
        }

        // Re-deduplicate with economic findings
        let deduped = deduplicate_findings(&all_with_economic);
        self.all_findings = deduped;

        info!("");
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        info!(" Phase 2: Verification");
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        let verified = hunt::verify_findings(&self.all_findings, &self.rpc_url, block_number, 5.0).await;
        self.verified_exploits = verified;

        info!("");
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        info!(" Phase 3: Report Generation");
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        self.write_poc_report(block_number);

        info!("");
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        info!(" Results");
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        let high_count = self.verified_exploits.iter().filter(|e| e.severity >= 10.0).count();
        let med_count = self.verified_exploits.iter().filter(|e| e.severity >= 5.0 && e.severity < 10.0).count();
        let profit_count = self.verified_exploits.iter().filter(|e| e.status == VerifyStatus::Verified).count();

        info!("   {} verified ({} high, {} medium, {} profitable)",
            self.verified_exploits.len(), high_count, med_count, profit_count);

        // Print tool availability report
        info!("");
        info!("{}", self.tool_report.summary());

        PipelineSummary {
            targets_scanned: self.target_contracts.len(),
            findings_total: self.all_findings.len(),
            verified_exploits: self.verified_exploits.len(),
            total_profit: self.verified_exploits.iter().fold(U256::ZERO, |a, e| a.saturating_add(e.profit_estimate)),
        }
    }

    async fn fetch_bytecode(&self) -> Vec<u8> {
        let target = self.target_contracts.iter().next().copied().unwrap_or(Address::ZERO);
        if target == Address::ZERO { return Vec::new(); }
        match tokio::time::timeout(
            std::time::Duration::from_secs(10),
            tokio::process::Command::new("cast")
                .arg("code").arg(format!("{:?}", target))
                .arg("--rpc-url").arg(&self.rpc_url)
                .output()
        ).await {
            Ok(Ok(out)) if out.status.success() => {
                let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let s = s.strip_prefix("0x").unwrap_or(&s);
                hex::decode(s).unwrap_or_default()
            }
            _ => Vec::new(),
        }
    }

    fn write_poc_report(&self, block_number: u64) {
        let (markdown, sol_code) = report::generate_report(
            self.target_contracts.iter().next().copied().unwrap_or(Address::ZERO),
            &self.rpc_url,
            block_number,
            &self.all_findings,
            &self.verified_exploits,
            self.target_source_dir.as_deref(),
        );

        if let Err(e) = std::fs::write("EXPLOIT_REPORT.md", &markdown) {
            warn!("   Failed to write EXPLOIT_REPORT.md: {}", e);
        } else {
            info!("   ✓ EXPLOIT_REPORT.md written");
        }

        if let Err(e) = std::fs::write("poc_found.t.sol", &sol_code) {
            warn!("   Failed to write poc_found.t.sol: {}", e);
        } else {
            info!("   ✓ poc_found.t.sol written");
        }
    }

    pub fn get_verified_exploits(&self) -> Vec<&VerifiedFinding> {
        let mut exploits: Vec<_> = self.verified_exploits.iter().collect();
        exploits.sort_by(|a, b| b.profit_estimate.cmp(&a.profit_estimate));
        exploits
    }
}

#[derive(Debug, Clone)]
pub struct PipelineSummary {
    pub targets_scanned: usize,
    pub findings_total: usize,
    pub verified_exploits: usize,
    pub total_profit: U256,
}

impl std::fmt::Display for PipelineSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Pipeline Summary:\n\
              Targets Scanned: {}\n\
              Total Findings: {}\n\
              Verified Exploits: {}\n\
              Total Profit: {}",
            self.targets_scanned,
            self.findings_total,
            self.verified_exploits,
            self.total_profit,
        )
    }
}
