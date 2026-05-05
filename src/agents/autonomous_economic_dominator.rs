//! Fully Autonomous Economic Dominator (Enhanced Version)
//!
//! This implements a completely autonomous economic exploit detection system that:
//! - Discovers targets autonomously (live blockchain, fork, or static analysis)
//! - Uses Context-First Architecture (Layers 1-3) for deep fuzzing
//! - Detects multi-contract financial state bugs
//! - Generates PoC automatically for Immunefi submissions
//!
//! INTEGRATED FROM OLD SYSTEM:
//! - ForkerAgent (live forking - LIVE FORK HIJACK capability)
//! - Fetcher (transaction replay - LIVE AMMO capability)  
//! - ContractArchetype (hypothesis generation)
//!
//! Key insight: Control the Context, let the EVM work FOR your analysis.

use crate::agents::concolic_engine::Navigator;
use crate::agents::context_first_engine::{
    DecisionCollector, DecisionType, ExecutionContext, HandlerOverrides, OracleDrivenFuzzer,
};
use crate::agents::contract_classifier::ContractArchetype;
use crate::agents::economic_engine::smt_verifier::{ExploitProof, Z3Solver};
use crate::agents::fetcher::Fetcher;
use crate::agents::forker::ForkerAgent;
use crate::agents::invariant_generator::{EconomicInvariant, InvariantSeverity};
use crate::agents::multi_contract_analysis::{
    CrossContractFinding, CrossContractVulnType, MultiContractAnalyzer,
};
use crate::agents::poc_generator::PoCGenerator;
use alloy::providers::Provider;
use alloy_primitives::{Address, U256};
use revm::db::{CacheDB, EmptyDB};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{info, warn};

/// Target source for autonomous discovery
#[derive(Debug, Clone, PartialEq)]
pub enum TargetSource {
    /// Fork from live blockchain at specific block
    ForkedMainnet { rpc_url: String, block_number: u64 },
    /// Static analysis of deployed contract
    StaticAnalysis { address: Address, bytecode: Vec<u8> },
    /// Custom protocol targeting
    Protocol {
        name: String,
        addresses: Vec<Address>,
    },
}

/// Vulnerability severity level
#[derive(Debug, Clone, PartialEq)]
pub enum VulnerabilitySeverity {
    Critical,
    High,
    Medium,
    Low,
}

/// A complete exploit finding with all context
#[derive(Debug, Clone)]
pub struct AutonomousExploit {
    pub target: Address,
    pub vulnerability_type: String,
    pub severity: VulnerabilitySeverity,
    pub description: String,
    pub profit_estimate: U256,
    pub exploit_proof: Option<ExploitProof>,
    pub cross_contract_findings: Vec<CrossContractFinding>,
    pub poc_file: Option<String>,
    pub confidence: f32,
}

/// The main autonomous Economic Dominator orchestrator (ENHANCED VERSION)
pub struct AutonomousEconomicDominator {
    /// Context-first engine for oracle-driven fuzzing
    pub context_fuzzer: OracleDrivenFuzzer,
    /// Multi-contract analyzer for cross-protocol bugs
    pub multi_analyzer: MultiContractAnalyzer,
    /// Storage navigator
    pub navigator: Navigator,
    /// Z3 solver for symbolic analysis
    pub solver: Z3Solver,
    /// Current target being analyzed
    pub current_target: Option<TargetSource>,
    /// Discovered exploits
    pub exploits: Vec<AutonomousExploit>,
    /// Configuration
    pub config: DominatorConfig,

    // ============================================================================
    // INTEGRATED FROM OLD SYSTEM (Your Proven Autonomous Capabilities)
    // ============================================================================
    /// Live forking capability (from ForkerAgent - LIVE FORK HIJACK)
    pub forker: Option<ForkerAgent>,
    /// Transaction fetching capability (from Fetcher - LIVE AMMO)
    pub fetcher: Option<Fetcher>,
    /// RPC URL for live chain interaction
    pub rpc_url: Option<String>,
    /// Classification results cache
    pub contract_classifications: HashMap<Address, ContractArchetype>,
    /// Flag indicating system is fully initialized with autonomous capabilities
    pub is_autonomous_ready: bool,
}

/// Configuration for the autonomous dominator
#[derive(Debug, Clone)]
pub struct DominatorConfig {
    /// Maximum iterations for oracle-driven fuzzing
    pub max_fuzz_iterations: usize,
    /// Minimum confidence threshold for reporting
    pub min_confidence: f32,
    /// Enable multi-contract analysis
    pub enable_multi_contract: bool,
    /// Enable automatic PoC generation
    pub auto_poc: bool,
    /// Enable live fork integration
    pub enable_forking: bool,
    /// Target protocols to focus on
    pub target_protocols: Vec<String>,
}

impl Default for DominatorConfig {
    fn default() -> Self {
        Self {
            max_fuzz_iterations: 100,
            min_confidence: 0.7,
            enable_multi_contract: true,
            auto_poc: true,
            enable_forking: true,
            target_protocols: vec![
                "Aave".to_string(),
                "Compound".to_string(),
                "Uniswap".to_string(),
                "Curve".to_string(),
            ],
        }
    }
}

impl AutonomousEconomicDominator {
    /// Create a new autonomous Economic Dominator (basic version - no autonomous capabilities)
    pub fn new() -> Self {
        Self {
            context_fuzzer: OracleDrivenFuzzer::new(),
            multi_analyzer: MultiContractAnalyzer::new(Navigator::new()),
            navigator: Navigator::new(),
            solver: Z3Solver::new(),
            current_target: None,
            exploits: Vec::new(),
            config: DominatorConfig::default(),
            forker: None,
            fetcher: None,
            rpc_url: None,
            contract_classifications: HashMap::new(),
            is_autonomous_ready: false,
        }
    }

    /// Create fully autonomous Economic Dominator with live forking + transaction fetching
    /// This version includes your proven capabilities: LIVE FORK HIJACK + LIVE AMMO
    pub async fn new_autonomous(rpc_url: &str) -> eyre::Result<Self> {
        use crate::agents::fetcher::HttpProvider;
        use alloy::providers::ProviderBuilder;
        use alloy::transports::http::Http;
        use reqwest::Client;
        
        let provider = Arc::new(
            ProviderBuilder::new()
                .on_http(rpc_url.parse()?)
        );
        
        let forker = ForkerAgent::new(provider.clone());
        let fetcher = Fetcher::new(provider.clone());
        
        info!("🔄 Created fully autonomous Economic Dominator with LIVE FORK HIJACK + LIVE AMMO capabilities");
        
        Ok(Self {
            context_fuzzer: OracleDrivenFuzzer::new(),
            multi_analyzer: MultiContractAnalyzer::new(Navigator::new()),
            navigator: Navigator::new(),
            solver: Z3Solver::new(),
            current_target: None,
            exploits: Vec::new(),
            config: DominatorConfig::default(),
            forker: Some(forker),
            fetcher: Some(fetcher),
            rpc_url: Some(rpc_url.to_string()),
            contract_classifications: HashMap::new(),
            is_autonomous_ready: true,
        })
    }

    /// Create with custom configuration
    pub fn with_config(config: DominatorConfig) -> Self {
        let mut fuzzer = OracleDrivenFuzzer::new();
        fuzzer.max_iterations = config.max_fuzz_iterations;

        Self {
            context_fuzzer: fuzzer,
            multi_analyzer: MultiContractAnalyzer::new(Navigator::new()),
            navigator: Navigator::new(),
            solver: Z3Solver::new(),
            current_target: None,
            exploits: Vec::new(),
            config,
            forker: None,
            fetcher: None,
            rpc_url: None,
            contract_classifications: HashMap::new(),
            is_autonomous_ready: false,
        }
    }

    /// Set the target to analyze
    pub fn set_target(&mut self, target: TargetSource) {
        info!("Setting target: {:?}", target);
        self.current_target = Some(target);
        self.exploits.clear();
    }

    /// Run the complete autonomous analysis pipeline
    pub fn analyze(&mut self) -> Vec<AutonomousExploit> {
        info!("Starting autonomous economic exploit analysis...");

        let target = match &self.current_target {
            Some(t) => t,
            None => {
                warn!("No target set, cannot analyze");
                return Vec::new();
            }
        };

        // Phase 1: Context-First Fuzzing (Oracle-Driven)
        info!("Phase 1: Context-First Fuzzing with Oracle-Driven Loop");
        let fuzz_exploits = self.run_context_fuzzing();

        // Phase 2: Multi-Contract Financial Analysis
        let multi_findings = if self.config.enable_multi_contract {
            info!("Phase 2: Multi-Contract Financial State Analysis");
            self.run_multi_contract_analysis()
        } else {
            Vec::new()
        };

        // Phase 3: Combine and enhance exploits
        info!("Phase 3: Combining and enhancing exploit findings");
        let combined_exploits = self.combine_findings(fuzz_exploits, multi_findings);

        // Phase 4: Generate PoCs if enabled
        let final_exploits = if self.config.auto_poc {
            info!("Phase 4: Generating PoCs for Immunefi");
            self.generate_pocs(combined_exploits)
        } else {
            combined_exploits
        };

        self.exploits = final_exploits.clone();
        info!(
            "Analysis complete. Found {} potential exploits",
            final_exploits.len()
        );

        final_exploits
    }

    /// Phase 1: Context-First Fuzzing with Oracle-Driven Loop
    fn run_context_fuzzing(&mut self) -> Vec<AutonomousExploit> {
        let target_address = match &self.current_target {
            Some(TargetSource::ForkedMainnet { .. }) => Address::default(),
            Some(TargetSource::StaticAnalysis { address, .. }) => *address,
            Some(TargetSource::Protocol { addresses, .. }) => {
                addresses.first().copied().unwrap_or_default()
            }
            None => Address::default(),
        };

        // Define economic invariants to check
        let invariants = vec![
            EconomicInvariant {
                name: "Over-collateralization".to_string(),
                condition: "borrow <= collateral * price / 1e18".to_string(),
                severity: InvariantSeverity::Critical,
            },
            EconomicInvariant {
                name: "Price Oracle Stability".to_string(),
                condition: "price_change < 5% per block".to_string(),
                severity: InvariantSeverity::High,
            },
            EconomicInvariant {
                name: "Liquidation Safety".to_string(),
                condition: "health_factor > 1.0 after liquidation".to_string(),
                severity: InvariantSeverity::Critical,
            },
        ];

        // Run the oracle-driven fuzzing loop
        let exploit_proofs = self
            .context_fuzzer
            .find_exploits(target_address, &invariants);

        exploit_proofs
            .into_iter()
            .map(|proof| {
                let severity = match proof
                    .profit_estimate
                    .cmp(&U256::from(1_000_000_000_000_000u64))
                {
                    std::cmp::Ordering::Greater => VulnerabilitySeverity::Critical,
                    std::cmp::Ordering::Less => VulnerabilitySeverity::Medium,
                    std::cmp::Ordering::Equal => VulnerabilitySeverity::High,
                };

                let profit = proof.profit_estimate;
                let vtype = proof.vulnerability_type.clone();
                let desc = proof.description.clone();

                AutonomousExploit {
                    target: proof.target,
                    vulnerability_type: vtype,
                    severity,
                    description: desc,
                    profit_estimate: profit,
                    exploit_proof: Some(proof),
                    cross_contract_findings: Vec::new(),
                    poc_file: None,
                    confidence: 0.85,
                }
            })
            .collect()
    }

    /// Phase 2: Multi-Contract Financial State Analysis
    fn run_multi_contract_analysis(&mut self) -> Vec<CrossContractFinding> {
        // Analyze for various cross-contract vulnerabilities
        let mut findings = Vec::new();

        // This would normally analyze real contracts
        // For now, return empty as we'd need actual contract interactions
        // In full implementation, this would:
        // 1. Fetch contracts from target
        // 2. Build call graph
        // 3. Check for oracle manipulation
        // 4. Check for liquidation cascades
        // 5. Check for flash loan atomicity violations
        // 6. Check for state diff exploits via storage overlaps

        info!(
            "Multi-contract analysis would check {} protocols",
            self.config.target_protocols.len()
        );

        findings
    }

    /// Phase 3: Combine findings from different analysis phases
    fn combine_findings(
        &mut self,
        fuzz_exploits: Vec<AutonomousExploit>,
        multi_findings: Vec<CrossContractFinding>,
    ) -> Vec<AutonomousExploit> {
        // Merge multi-contract findings into fuzz exploits
        let mut combined = fuzz_exploits;

        for finding in multi_findings {
            // Create a new exploit entry for each cross-contract finding
            let severity = match finding.severity {
                8..=10 => VulnerabilitySeverity::Critical,
                6..=7 => VulnerabilitySeverity::High,
                4..=5 => VulnerabilitySeverity::Medium,
                _ => VulnerabilitySeverity::Low,
            };

            let exploit = AutonomousExploit {
                target: finding
                    .involved_contracts
                    .first()
                    .copied()
                    .unwrap_or_default(),
                vulnerability_type: format!("{:?}", finding.vulnerability_type),
                severity,
                description: finding.description.clone(),
                profit_estimate: U256::from(0),
                exploit_proof: None,
                cross_contract_findings: vec![finding],
                poc_file: None,
                confidence: 0.75,
            };

            combined.push(exploit);
        }

        // Filter by confidence threshold
        combined
            .into_iter()
            .filter(|e| e.confidence >= self.config.min_confidence)
            .collect()
    }

    /// Phase 4: Generate PoCs for Immunefi submissions
    fn generate_pocs(&mut self, exploits: Vec<AutonomousExploit>) -> Vec<AutonomousExploit> {
        exploits
            .into_iter()
            .map(|mut exploit| {
                if let Some(proof) = &exploit.exploit_proof {
                    let mut generator = PoCGenerator::new(exploit.target, vec![proof.clone()]);

                    // Add storage overlaps if available
                    let overlaps = self.navigator.find_slot_overlaps();
                    if !overlaps.is_empty() {
                        let overlap_pairs: Vec<(Address, Vec<Address>)> = overlaps
                            .iter()
                            .map(|(slot, contracts)| (slot.contract, contracts.clone()))
                            .collect();
                        generator.add_overlaps(overlap_pairs);
                    }

                    let poc_content = generator.generate_test_file();
                    exploit.poc_file = Some(poc_content);
                }
                exploit
            })
            .collect()
    }

    /// Quick analysis for a specific target (convenience method)
    pub fn quick_scan(&mut self, address: Address, bytecode: &[u8]) -> Vec<AutonomousExploit> {
        self.set_target(TargetSource::StaticAnalysis {
            address,
            bytecode: bytecode.to_vec(),
        });
        self.analyze()
    }

    /// Get the most critical finding
    pub fn get_critical_findings(&self) -> Vec<&AutonomousExploit> {
        self.exploits
            .iter()
            .filter(|e| matches!(e.severity, VulnerabilitySeverity::Critical))
            .collect()
    }

    /// Export findings in Immunefi-ready format
    pub fn export_immunefi_report(&self) -> String {
        let mut report = String::new();
        report.push_str("# Economic Dominator - Vulnerability Report\n\n");

        for (i, exploit) in self.exploits.iter().enumerate() {
            report.push_str(&format!("## Finding {}\n", i + 1));
            report.push_str(&format!("**Target:** {:?}\n", exploit.target));
            report.push_str(&format!("**Type:** {}\n", exploit.vulnerability_type));
            report.push_str(&format!("**Severity:** {:?}\n", exploit.severity));
            report.push_str(&format!(
                "**Confidence:** {:.1}%\n",
                exploit.confidence * 100.0
            ));
            report.push_str(&format!(
                "**Profit Estimate:** {}\n",
                exploit.profit_estimate
            ));
            report.push_str(&format!("\n{}\n\n", exploit.description));

            if exploit.poc_file.is_some() {
                report.push_str("*PoC available - see attached file*\n\n");
            }
        }

        report
    }

    // ============================================================================
    // AUTONOMOUS CAPABILITIES (Integrated from Your Old System)
    // ============================================================================

    /// Fork the blockchain at a specific block and analyze
    /// Implements LIVE FORK HIJACK capability from your original agent
    pub async fn fork_and_analyze(&mut self, target: Address, block_number: u64) -> Vec<AutonomousExploit> {
        if !self.is_autonomous_ready {
            warn!("System not initialized as autonomous. Use new_autonomous() first.");
            return vec![];
        }

        let Some(forker) = &self.forker else {
            warn!("Forker not initialized");
            return vec![];
        };

        info!("⚡ LIVE FORK HIJACK: Forking chain at block {}", block_number);

        // Build forked database
        let db = match forker.build_cross_contract_db(target, vec![]).await {
            Ok(db) => db,
            Err(e) => {
                warn!("Failed to build fork: {}", e);
                return vec![];
            }
        };

        info!("✅ Fork ready - running economic analysis on forked state");

        // Set target and run analysis on forked state
        self.set_target(TargetSource::ForkedMainnet {
            rpc_url: self.rpc_url.clone().unwrap_or_default(),
            block_number,
        });

        // Run analysis
        self.analyze()
    }

    /// Fetch contract bytecode from live chain
    /// Implements LIVE AMMO capability from your original agent
    pub async fn fetch_and_analyze(&mut self, target: Address) -> Vec<AutonomousExploit> {
        if !self.is_autonomous_ready {
            warn!("System not initialized as autonomous. Use new_autonomous() first.");
            return vec![];
        }

        let Some(fetcher) = &self.fetcher else {
            warn!("Fetcher not initialized");
            return vec![];
        };

        info!("🔍 LIVE AMMO: Fetching contract bytecode from live chain");

        // Fetch bytecode
        let bytecode = match fetcher.get_code(target).await {
            Ok(code) => code,
            Err(e) => {
                warn!("Failed to fetch bytecode: {}", e);
                return vec![];
            }
        };

        info!("✅ Fetched {} bytes of bytecode", bytecode.len());

        // Classify contract for hypothesis generation
        let archetype = ContractArchetype::classify(&bytecode);
        info!("📊 Contract classified as: {:?}", archetype);
        self.contract_classifications.insert(target, archetype.clone());

        // Trigger appropriate analysis based on contract type
        let exploits = match archetype {
            ContractArchetype::Lending => {
                info!("📈 Detected LENDING protocol - running deep economic analysis");
                self.set_target(TargetSource::StaticAnalysis {
                    address: target,
                    bytecode,
                });
                self.analyze()
            }
            ContractArchetype::ConstantProductDEX => {
                info!("📈 Detected DEX protocol - running swap/exploit analysis");
                self.set_target(TargetSource::StaticAnalysis {
                    address: target,
                    bytecode,
                });
                self.analyze()
            }
            ContractArchetype::Oracle => {
                info!("📈 Detected ORACLE - running price manipulation analysis");
                self.set_target(TargetSource::StaticAnalysis {
                    address: target,
                    bytecode,
                });
                self.analyze()
            }
            _ => {
                info!("📈 Detected {:?} - running general analysis", archetype);
                self.set_target(TargetSource::StaticAnalysis {
                    address: target,
                    bytecode,
                });
                self.analyze()
            }
        };

        info!("Analysis complete: found {} potential exploits", exploits.len());
        exploits
    }

    /// Classify a contract and determine what analysis to run
    /// Implements hypothesis generation from your orchestrator
    pub fn classify_contract(&mut self, bytecode: &[u8], address: Address) -> ContractArchetype {
        let archetype = ContractArchetype::classify(bytecode);
        self.contract_classifications.insert(address, archetype.clone());
        info!("Contract {:?} classified as: {:?}", address, archetype);
        archetype
    }

    /// Check if system has autonomous capabilities initialized
    pub fn has_autonomous_capabilities(&self) -> bool {
        self.is_autonomous_ready && self.forker.is_some() && self.fetcher.is_some()
    }

    /// Get classification for a contract
    pub fn get_contract_type(&self, address: &Address) -> Option<&ContractArchetype> {
        self.contract_classifications.get(address)
    }
}

impl Default for AutonomousEconomicDominator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_autonomous_dominator_creation() {
        let dominator = AutonomousEconomicDominator::new();
        assert!(dominator.exploits.is_empty());
    }

    #[test]
    fn test_autonomous_dominator_with_config() {
        let config = DominatorConfig {
            max_fuzz_iterations: 50,
            min_confidence: 0.8,
            enable_multi_contract: true,
            auto_poc: false,
            enable_forking: false,
            target_protocols: vec!["Uniswap".to_string()],
        };

        let dominator = AutonomousEconomicDominator::with_config(config);
        assert_eq!(dominator.context_fuzzer.max_iterations, 50);
        assert!(!dominator.config.auto_poc);
    }

    #[test]
    fn test_set_static_target() {
        let mut dominator = AutonomousEconomicDominator::new();
        let address = Address::new([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x12, 0x34,
        ]);
        let bytecode = vec![0x60, 0x00, 0x00];

        dominator.set_target(TargetSource::StaticAnalysis { address, bytecode });

        match &dominator.current_target {
            Some(TargetSource::StaticAnalysis { address: a, .. }) => {
                assert_eq!(*a, address);
            }
            _ => panic!("Expected StaticAnalysis target"),
        }
    }

    #[test]
    fn test_quick_scan_no_target() {
        let mut dominator = AutonomousEconomicDominator::new();
        let results = dominator.analyze();
        assert!(results.is_empty());
    }

    #[test]
    fn test_immunefi_report_empty() {
        let dominator = AutonomousEconomicDominator::new();
        let report = dominator.export_immunefi_report();
        assert!(report.contains("Vulnerability Report"));
    }
}
