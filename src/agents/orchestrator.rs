use std::sync::Arc;
use std::time::Instant;

use alloy_primitives::{Address, Bytes, U256};
use futures::stream::{self, StreamExt};
use revm::interpreter::opcode::STATICCALL;
use tokio::sync::Semaphore;
use tracing::{info, instrument, warn};

use crate::agents::contract_classifier::ContractArchetype;
use crate::agents::fetcher::Fetcher;
use crate::agents::forker::ForkerAgent;
use crate::agents::invariant_generator::generate_invariants;
use crate::agents::oracle_discovery::{discover_oracle, OracleInfo};
use crate::agents::revenue_calc::RevenueReport;
use crate::agents::severity::{classify, Confidence, Impact, Severity};
use crate::agents::source_fetcher::{FetchedSource, SourceFetcher};
use crate::agents::storage_tracer::{ContractTrace, trace_contract};
use crate::agents::symbolic_executor::SymbolicExecutor;
use crate::agents::symbolic_interpreter::SymbolicInterpreter;

use super::{ityfuzz_integration, oracle_laggard, reward_dilution, ror_optimizer};
use crate::agents::autonomous_economic_dominator::{
    AutonomousEconomicDominator, DominatorConfig, TargetSource,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Hypothesis {
    OracleLaggard,
    RorOptimizer,
    GhostReentrancy,
    RewardDilution,
    AccessControl,
    GuidedFuzzing,
    // New autonomous attack strategies
    CPMMInvariantBreak,
    SandwichAttack,
    CollateralInvariantBreak,
    InflationAttack,
    SelfdestructCheck,
}

#[derive(Debug, Clone)]
pub struct ValidatedExploit {
    pub target: Address,
    pub vuln_class: String,
    pub calldata: Bytes,
    pub profit: U256,
    pub amount: U256,
    pub revenue: RevenueReport,
    pub severity: Severity,
    pub confidence: Confidence,
    pub description: String,
}

pub struct Orchestrator {
    fetcher: Arc<Fetcher>,
    forker: ForkerAgent,
    source_fetcher: SourceFetcher,
    concurrency: usize,
    /// Optional autonomous economic dominator for deep financial analysis
    economic_dominator: Option<AutonomousEconomicDominator>,
    /// Enable economic analysis on financial contracts
    enable_economic_analysis: bool,
}

impl Orchestrator {
    pub async fn new(rpc_url: &str, etherscan_key: Option<String>) -> eyre::Result<Self> {
        let provider = Arc::new(
            alloy::providers::ProviderBuilder::new()
                .on_http(rpc_url.parse()?)
        );
        Ok(Self {
            fetcher: Arc::new(Fetcher::new(provider.clone())),
            forker: ForkerAgent::new(provider),
            source_fetcher: SourceFetcher::new(etherscan_key),
            concurrency: 4,
            economic_dominator: None,
            enable_economic_analysis: false,
        })
    }

    /// Create orchestrator with autonomous economic analysis enabled
    pub async fn with_economic_analysis(
        rpc_url: &str,
        etherscan_key: Option<String>,
        config: DominatorConfig,
    ) -> eyre::Result<Self> {
        let provider = Arc::new(
            alloy::providers::ProviderBuilder::new()
                .on_http(rpc_url.parse()?)
        );
        Ok(Self {
            fetcher: Arc::new(Fetcher::new(provider.clone())),
            forker: ForkerAgent::new(provider),
            source_fetcher: SourceFetcher::new(etherscan_key),
            concurrency: 4,
            economic_dominator: Some(AutonomousEconomicDominator::with_config(config)),
            enable_economic_analysis: true,
        })
    }

    /// Create fully autonomous orchestrator with LIVE FORK HIJACK + LIVE AMMO capabilities
    pub async fn with_full_autonomy(rpc_url: &str, etherscan_key: Option<String>) -> eyre::Result<Self> {
        let dominator = AutonomousEconomicDominator::new_autonomous(rpc_url).await?;
        let provider = Arc::new(
            alloy::providers::ProviderBuilder::new()
                .on_http(rpc_url.parse()?)
        );
        Ok(Self {
            fetcher: Arc::new(Fetcher::new(provider.clone())),
            forker: ForkerAgent::new(provider),
            source_fetcher: SourceFetcher::new(etherscan_key),
            concurrency: 4,
            economic_dominator: Some(dominator),
            enable_economic_analysis: true,
        })
    }

    /// PRIMARY ANALYSIS METHOD - Uses enhanced AutonomousEconomicDominator
    /// Combines old system's hunting with new economic analysis
    pub async fn analyze_contract_autonomous(
        &mut self,
        address: Address,
        name: &str,
    ) -> Vec<ValidatedExploit> {
        info!("🚀 Starting AUTONOMOUS analysis of {} ({})", name, address);

        let mut all_exploits = Vec::new();

        // Step 1: Fetch bytecode from live chain
        let bytecode = match self.fetcher.get_code(address).await {
            Ok(code) => {
                info!("📥 Fetched {} bytes of bytecode", code.len());
                code.to_vec()
            }
            Err(e) => {
                warn!("Failed to fetch bytecode: {}", e);
                return vec![];
            }
        };

        // Step 2: If economic analysis enabled, run deep analysis
        if self.enable_economic_analysis {
            if let Some(dominator) = &mut self.economic_dominator {
                // Use the enhanced autonomous system
                let economic_exploits = dominator.fetch_and_analyze(address).await;
                
                // Convert to ValidatedExploit format
                for exploit in economic_exploits {
                    let severity = match exploit.severity {
                        crate::agents::autonomous_economic_dominator::VulnerabilitySeverity::Critical => Severity::Critical,
                        crate::agents::autonomous_economic_dominator::VulnerabilitySeverity::High => Severity::High,
                        crate::agents::autonomous_economic_dominator::VulnerabilitySeverity::Medium => Severity::Medium,
                        crate::agents::autonomous_economic_dominator::VulnerabilitySeverity::Low => Severity::Low,
                    };

                    let confidence = if exploit.confidence > 0.9 {
                        Confidence::Proven
                    } else if exploit.confidence > 0.7 {
                        Confidence::High
                    } else if exploit.confidence > 0.5 {
                        Confidence::Medium
                    } else {
                        Confidence::Low
                    };

                    let attacker = Address::repeat_byte(0xde);
                    let revenue = RevenueReport {
                        attack: exploit.vulnerability_type.clone(),
                        attacker,
                        target: exploit.target,
                        eth_gained: exploit.profit_estimate,
                        tokens_gained: vec![],
                        gas_cost_wei: U256::from(200_000),
                        net_profit_wei: exploit.profit_estimate,
                        viability: exploit.confidence as f64,
                    };

                    all_exploits.push(ValidatedExploit {
                        target: exploit.target,
                        vuln_class: exploit.vulnerability_type,
                        calldata: alloy_primitives::Bytes::new(),
                        profit: exploit.profit_estimate,
                        amount: exploit.profit_estimate,
                        revenue,
                        severity,
                        confidence,
                        description: exploit.description,
                    });
                }

                info!("🔍 Found {} economic exploits", all_exploits.len());
            }
        }

        // Step 3: Also run traditional hypothesis-based analysis
        let traditional_exploits = self.analyze_contract(address, name).await;
        all_exploits.extend(traditional_exploits);

        info!("🎯 Total exploits found: {}", all_exploits.len());
        all_exploits
    }

    #[instrument(skip(self), fields(target = %address))]
    pub async fn analyze_contract(
        &self,
        address: Address,
        name: &str,
    ) -> Vec<ValidatedExploit> {
        info!("🔍 Starting analysis of {} ({})", name, address);
        let start = Instant::now();

        let (bytecode, source) = self.fetch_essentials(address).await;
        let bytecode = match bytecode {
            Some(b) => b,
            None => {
                warn!("No bytecode for {}, skipping", address);
                return vec![];
            }
        };

        let trace = trace_contract(&self.forker, address).await.unwrap_or_default();

        // Classify contract and load targeted attack strategies
        let archetype = ContractArchetype::classify(&bytecode);
        let invariants = generate_invariants(&archetype);
        info!(
            "🏛️  {:?} → {} invariants generated",
            archetype,
            invariants.len()
        );

        let oracle_info = self.discover_oracle_info(&bytecode, &source).await;
        info!("Oracle info: {:?}", oracle_info);

        let mut hypotheses = self.generate_hypotheses(&trace, &oracle_info, &source);

        // Autonomous branching based on contract identity
        match archetype {
            ContractArchetype::ConstantProductDEX => {
                hypotheses.push(Hypothesis::CPMMInvariantBreak);
                hypotheses.push(Hypothesis::SandwichAttack);
            }
            ContractArchetype::Lending => {
                hypotheses.push(Hypothesis::OracleLaggard);
                hypotheses.push(Hypothesis::RorOptimizer);
                hypotheses.push(Hypothesis::CollateralInvariantBreak);
            }
            ContractArchetype::Vault => {
                hypotheses.push(Hypothesis::InflationAttack);
            }
            ContractArchetype::Staking => {
                hypotheses.push(Hypothesis::RewardDilution);
            }
            ContractArchetype::Oracle => {
                hypotheses.push(Hypothesis::OracleLaggard);
            }
            _ => {
                hypotheses.push(Hypothesis::AccessControl);
                hypotheses.push(Hypothesis::SelfdestructCheck);
            }
        }

        hypotheses.push(Hypothesis::GuidedFuzzing);

        // Symbolic proof: attach symbolic paths that provably violate invariants
        let sym_paths = SymbolicExecutor::prove_vulnerability(&bytecode, address);
        for p in &sym_paths {
            info!("🔮 Symbolic exploit candidate with profit {} wei", p.profit_estimate);
        }

        // SymbolicInterpreter: prove invariants via SMT solving
        let proven = SymbolicInterpreter::prove(&invariants, &bytecode, address);
        let mut proven_exploits: Vec<ValidatedExploit> = proven
            .into_iter()
            .map(|p| {
                info!(
                    "🔮 Proven exploit: {} | profit: {} | severity: {:?} | confidence: {:?}",
                    p.invariant_broken, p.profit_estimate, p.severity, p.confidence
                );
                let attacker = Address::repeat_byte(0xde);
                let revenue = RevenueReport {
                    attack: format!("Symbolic: {}", p.invariant_broken),
                    attacker,
                    target: p.target,
                    eth_gained: p.profit_estimate,
                    tokens_gained: vec![],
                    gas_cost_wei: U256::from(150_000),
                    net_profit_wei: p.profit_estimate.saturating_sub(U256::from(150_000)),
                    viability: 0.0,
                };
                ValidatedExploit {
                    target: p.target,
                    vuln_class: p.invariant_broken,
                    calldata: p.calldata,
                    profit: p.profit_estimate,
                    amount: U256::ZERO,
                    revenue,
                    severity: p.severity,
                    confidence: p.confidence,
                    description: p.description,
                }
            })
            .collect();

        let sem = Arc::new(Semaphore::new(self.concurrency));
        let stream = stream::iter(hypotheses.into_iter().map(|hyp| {
            let sem = sem.clone();
            let address = address;
            let trace = trace.clone();
            let source = source.clone();
            let oracle_info = oracle_info.clone();
            let forker = &self.forker;
            async move {
                let _permit = sem.acquire().await.unwrap();
                Self::execute_hypothesis(hyp, address, &trace, &oracle_info, &source, forker)
                    .await
            }
        }))
        .buffer_unordered(self.concurrency);

        let results: Vec<Vec<ValidatedExploit>> = stream.collect().await;
        let exploits: Vec<ValidatedExploit> = results.into_iter().flatten().collect();
        proven_exploits.extend(exploits);
        info!(
            "✅ Analysis of {} finished in {:?}, found {} exploits ({} proven)",
            name,
            start.elapsed(),
            proven_exploits.len(),
            proven_exploits.iter().filter(|e| e.confidence == Confidence::Proven).count(),
        );
        proven_exploits
    }

    async fn fetch_essentials(
        &self,
        address: Address,
    ) -> (Option<Vec<u8>>, Option<FetchedSource>) {
        let bytecode_future = self.fetcher.get_code(address);
        let addr_str = format!("{address:x}");
        let source_future = self.source_fetcher.fetch(1, &addr_str);

        let (bytecode_res, source_res) = tokio::join!(bytecode_future, source_future);
        let bytecode = match bytecode_res {
            Ok(b) if !b.is_empty() => Some(b),
            Ok(_) => {
                warn!("Fetcher returned empty bytecode for {}", address);
                None
            }
            Err(e) => {
                warn!("Failed to fetch bytecode for {}: {:?}", address, e);
                None
            }
        };
        (bytecode, source_res)
    }

    async fn discover_oracle_info(
        &self,
        bytecode: &[u8],
        source: &Option<FetchedSource>,
    ) -> Option<OracleInfo> {
        if let Some(info) = discover_oracle(bytecode, 0, bytecode.len()) {
            return Some(info);
        }
        if let Some(src) = source {
            for file in &src.sources {
                if file.content.contains("latestRoundData")
                    || file.content.contains("AggregatorV3Interface")
                {
                    return Some(OracleInfo {
                        oracle_address: None,
                        price_slot: None,
                        function_selector: Some([0xfe, 0xaf, 0x96, 0x8c]),
                    });
                }
            }
        }
        None
    }

    fn generate_hypotheses(
        &self,
        trace: &ContractTrace,
        oracle_info: &Option<OracleInfo>,
        source: &Option<FetchedSource>,
    ) -> Vec<Hypothesis> {
        let mut hyps = Vec::new();
        if trace.storage_writes.iter().any(|w| !w.caller_checked) {
            hyps.push(Hypothesis::AccessControl);
        }
        if oracle_info.is_some()
            || trace.external_calls.iter().any(|c| c.opcode == STATICCALL)
        {
            hyps.push(Hypothesis::OracleLaggard);
            hyps.push(Hypothesis::RorOptimizer);
        }
        let has_sstore = !trace.storage_writes.is_empty();
        let has_call = !trace.external_calls.is_empty();
        if has_sstore && has_call {
            hyps.push(Hypothesis::GhostReentrancy);
        }
        if let Some(src) = source {
            if src.sources.iter().any(|f| f.content.contains("reward")) {
                hyps.push(Hypothesis::RewardDilution);
            }
        }
        hyps
    }

    async fn execute_hypothesis(
        hyp: Hypothesis,
        target: Address,
        _trace: &ContractTrace,
        oracle_info: &Option<OracleInfo>,
        _source: &Option<FetchedSource>,
        forker: &ForkerAgent,
    ) -> Vec<ValidatedExploit> {
        match hyp {
            Hypothesis::OracleLaggard => {
                Self::run_oracle_laggard(target, oracle_info, forker).await
            }
            Hypothesis::RorOptimizer => {
                Self::run_ror_optimizer(target, oracle_info, forker).await
            }
            Hypothesis::GhostReentrancy => vec![],
            Hypothesis::RewardDilution => Self::run_reward_dilution(target, forker).await,
            Hypothesis::AccessControl => vec![],
            Hypothesis::GuidedFuzzing => {
                let bytecode = forker
                    .build_cross_contract_db(target, vec![])
                    .await
                    .ok()
                    .and_then(|db| {
                        db.accounts
                            .get(&target)
                            .and_then(|a| a.info.code.clone())
                            .map(|c| c.original_bytes().to_vec())
                    });
                if let Some(code) = bytecode {
                    let abi = "[]";
                    let exploits = ityfuzz_integration::run_ityfuzz(
                        &code,
                        abi,
                        &format!("{target:x}"),
                    );
                    exploits
                        .into_iter()
                        .map(|(desc, profit)| ValidatedExploit {
                            target,
                            vuln_class: "Guided Fuzzing (ityfuzz)".into(),
                            calldata: Bytes::from(b""),
                            profit,
                            amount: U256::ZERO,
                            revenue: RevenueReport::default(),
                            severity: Severity::Medium,
                            confidence: Confidence::High,
                            description: desc,
                        })
                        .collect()
                } else {
                    vec![]
                }
            }
            // New hypotheses – stubs for now (will be connected to detectors)
            Hypothesis::CPMMInvariantBreak => vec![],
            Hypothesis::SandwichAttack => vec![],
            Hypothesis::CollateralInvariantBreak => vec![],
            Hypothesis::InflationAttack => vec![],
            Hypothesis::SelfdestructCheck => vec![],
        }
    }

    async fn run_oracle_laggard(
        target: Address,
        oracle_info: &Option<OracleInfo>,
        forker: &ForkerAgent,
    ) -> Vec<ValidatedExploit> {
        let mut db = match forker.build_cross_contract_db(target, vec![]).await {
            Ok(db) => db,
            Err(_) => return vec![],
        };
        let attacker = Address::repeat_byte(0xde);
        let price_slot = oracle_info
            .as_ref()
            .and_then(|o| o.price_slot.map(|s| U256::from(s)))
            .unwrap_or(U256::from(13));
        if let Some((profit, calldata)) = oracle_laggard::detect_oracle_laggard_core(
            &mut db, target, target, attacker, price_slot,
        ) {
            if !profit.is_zero() {
                let revenue = RevenueReport {
                    attack: "Oracle Laggard".into(),
                    attacker,
                    target,
                    eth_gained: profit,
                    tokens_gained: vec![],
                    gas_cost_wei: U256::from(150_000),
                    net_profit_wei: profit.saturating_sub(U256::from(150_000)),
                    viability: 0.0,
                };
                let severity = classify(
                    &Impact::from_drain(
                        profit,
                        18,
                        U256::from(1_000_000_000_000_000_000u128),
                        U256::MAX,
                        true,
                        false,
                        0,
                        false,
                    ),
                    Confidence::Proven,
                )
                .0;
                return vec![ValidatedExploit {
                    target,
                    vuln_class: "Oracle Laggard".into(),
                    calldata: Bytes::from(calldata),
                    profit,
                    amount: U256::ZERO,
                    revenue,
                    severity,
                    confidence: Confidence::Proven,
                    description: format!("Stale price oracle exploited, profit: {}", profit),
                }];
            }
        }
        vec![]
    }

    async fn run_ror_optimizer(
        target: Address,
        oracle_info: &Option<OracleInfo>,
        forker: &ForkerAgent,
    ) -> Vec<ValidatedExploit> {
        let mut db = match forker.build_cross_contract_db(target, vec![]).await {
            Ok(db) => db,
            Err(_) => return vec![],
        };
        let attacker = Address::repeat_byte(0xde);
        let price_slot = oracle_info
            .as_ref()
            .and_then(|o| o.price_slot.map(|s| U256::from(s)))
            .unwrap_or(U256::from(13));
        let (amount, calldata) = ror_optimizer::find_optimal_borrow_core(
            &mut db,
            target,
            target,
            attacker,
            U256::from(100_000_000_000_000_000_000u128),
            price_slot,
        );
        if amount.is_zero() || calldata.is_empty() {
            return vec![];
        }
        let profit = amount;
        let revenue = RevenueReport {
            attack: "ROR Optimizer".into(),
            attacker,
            target,
            eth_gained: profit,
            tokens_gained: vec![],
            gas_cost_wei: U256::from(150_000),
            net_profit_wei: profit.saturating_sub(U256::from(150_000)),
            viability: 0.0,
        };
        vec![ValidatedExploit {
            target,
            vuln_class: "ROR".into(),
            calldata,
            profit,
            amount,
            revenue,
            severity: Severity::High,
            confidence: Confidence::Proven,
            description: format!("Borrow amount {} at manipulated price", amount),
        }]
    }

    async fn run_reward_dilution(
        target: Address,
        forker: &ForkerAgent,
    ) -> Vec<ValidatedExploit> {
        let mut db = match forker.build_cross_contract_db(target, vec![]).await {
            Ok(db) => db,
            Err(_) => return vec![],
        };
        let attacker = Address::repeat_byte(0xde);
        if let Some(profit) =
            reward_dilution::detect_reward_dilution(&mut db, target, attacker)
        {
            let revenue = RevenueReport {
                attack: "Reward Dilution".into(),
                attacker,
                target,
                eth_gained: profit,
                tokens_gained: vec![],
                gas_cost_wei: U256::ZERO,
                net_profit_wei: profit,
                viability: 1.0,
            };
            vec![ValidatedExploit {
                target,
                vuln_class: "Reward Dilution".into(),
                calldata: Bytes::from(b""),
                profit,
                amount: U256::ZERO,
                revenue,
                severity: Severity::Medium,
                confidence: Confidence::Proven,
                description: format!("Reward dilution yields profit: {}", profit),
            }]
        } else {
            vec![]
        }
    }

    /// Run autonomous economic analysis on a target contract
    /// Uses Context-First Architecture (Layers 1-3) for deep fuzzing
    pub fn analyze_economic(&mut self, address: Address, bytecode: &[u8]) -> Vec<ValidatedExploit> {
        if !self.enable_economic_analysis {
            warn!("Economic analysis is not enabled. Use with_economic_analysis()");
            return vec![];
        }

        let Some(dominator) = &mut self.economic_dominator else {
            warn!("Economic dominator not initialized");
            return vec![];
        };

        info!("🔬 Running autonomous economic analysis on {:?}", address);

        // Set target and run analysis
        dominator.set_target(TargetSource::StaticAnalysis {
            address,
            bytecode: bytecode.to_vec(),
        });

        let exploits = dominator.analyze();

        // Convert to ValidatedExploit format
        let mut validated = Vec::new();
        for exploit in exploits {
            let severity = match exploit.severity {
                crate::agents::autonomous_economic_dominator::VulnerabilitySeverity::Critical => Severity::Critical,
                crate::agents::autonomous_economic_dominator::VulnerabilitySeverity::High => Severity::High,
                crate::agents::autonomous_economic_dominator::VulnerabilitySeverity::Medium => Severity::Medium,
                crate::agents::autonomous_economic_dominator::VulnerabilitySeverity::Low => Severity::Low,
            };

            let confidence = if exploit.confidence > 0.9 {
                Confidence::Proven
            } else if exploit.confidence > 0.7 {
                Confidence::High
            } else if exploit.confidence > 0.5 {
                Confidence::Medium
            } else {
                Confidence::Low
            };

            let attacker = Address::repeat_byte(0xde);
            let revenue = RevenueReport {
                attack: exploit.vulnerability_type.clone(),
                attacker,
                target: exploit.target,
                eth_gained: exploit.profit_estimate,
                tokens_gained: vec![],
                gas_cost_wei: U256::from(200_000),
                net_profit_wei: exploit.profit_estimate,
                viability: exploit.confidence as f64,
            };

            validated.push(ValidatedExploit {
                target: exploit.target,
                vuln_class: exploit.vulnerability_type,
                calldata: Bytes::new(),
                profit: exploit.profit_estimate,
                amount: exploit.profit_estimate,
                revenue,
                severity,
                confidence,
                description: exploit.description,
            });
        }

        info!("📊 Economic analysis found {} potential exploits", validated.len());
        validated
    }

    /// Get the Immunefi report from the economic dominator
    pub fn get_immunefi_report(&self) -> Option<String> {
        self.economic_dominator.as_ref().map(|d| d.export_immunefi_report())
    }

    /// Check if economic analysis is enabled
    pub fn is_economic_enabled(&self) -> bool {
        self.enable_economic_analysis
    }
}
