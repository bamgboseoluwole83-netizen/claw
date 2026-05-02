use std::sync::Arc;
use std::time::Instant;

use alloy_primitives::{Address, Bytes, U256};
use futures::stream::{self, StreamExt};
use revm::interpreter::opcode::STATICCALL;
use tokio::sync::Semaphore;
use tracing::{info, instrument, warn};

use crate::agents::fetcher::Fetcher;
use crate::agents::forker::ForkerAgent;
use crate::agents::oracle_discovery::{discover_oracle, OracleInfo};
use crate::agents::revenue_calc::RevenueReport;
use crate::agents::severity::{classify, Confidence, Impact, Severity};
use crate::agents::source_fetcher::{FetchedSource, SourceFetcher};
use crate::agents::storage_tracer::{ContractTrace, trace_contract};

use super::{ityfuzz_integration, oracle_laggard, reward_dilution, ror_optimizer};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Hypothesis {
    OracleLaggard,
    RorOptimizer,
    GhostReentrancy,
    RewardDilution,
    AccessControl,
    GuidedFuzzing,
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
        })
    }

    #[instrument(skip(self), fields(target = %address))]
    pub async fn analyze_contract(&self, address: Address, name: &str) -> Vec<ValidatedExploit> {
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

        let oracle_info = self.discover_oracle_info(&bytecode, &source).await;
        info!("Oracle info: {:?}", oracle_info);

        let mut hypotheses = self.generate_hypotheses(&trace, &oracle_info, &source);
        hypotheses.push(Hypothesis::GuidedFuzzing);

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
                Self::execute_hypothesis(hyp, address, &trace, &oracle_info, &source, forker).await
            }
        }))
        .buffer_unordered(self.concurrency);

        let results: Vec<Vec<ValidatedExploit>> = stream.collect().await;
        let exploits: Vec<ValidatedExploit> = results.into_iter().flatten().collect();

        info!("✅ Analysis of {} finished in {:?}, found {} exploits", name, start.elapsed(), exploits.len());
        exploits
    }

    async fn fetch_essentials(&self, address: Address) -> (Option<Vec<u8>>, Option<FetchedSource>) {
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

    async fn discover_oracle_info(&self, bytecode: &[u8], source: &Option<FetchedSource>) -> Option<OracleInfo> {
        if let Some(info) = discover_oracle(bytecode, 0, bytecode.len()) {
            return Some(info);
        }
        if let Some(src) = source {
            for file in &src.sources {
                if file.content.contains("latestRoundData") || file.content.contains("AggregatorV3Interface") {
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

    fn generate_hypotheses(&self, trace: &ContractTrace, oracle_info: &Option<OracleInfo>, source: &Option<FetchedSource>) -> Vec<Hypothesis> {
        let mut hyps = Vec::new();
        if trace.storage_writes.iter().any(|w| !w.caller_checked) {
            hyps.push(Hypothesis::AccessControl);
        }
        if oracle_info.is_some() || trace.external_calls.iter().any(|c| c.opcode == STATICCALL) {
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
            Hypothesis::OracleLaggard => Self::run_oracle_laggard(target, oracle_info, forker).await,
            Hypothesis::RorOptimizer => Self::run_ror_optimizer(target, oracle_info, forker).await,
            Hypothesis::GhostReentrancy => vec![],
            Hypothesis::RewardDilution => Self::run_reward_dilution(target, forker).await,
            Hypothesis::AccessControl => vec![],
            Hypothesis::GuidedFuzzing => {
                let bytecode = forker.build_cross_contract_db(target, vec![]).await.ok()
                    .and_then(|db| db.accounts.get(&target).and_then(|a| a.info.code.clone()).map(|c| c.original_bytes().to_vec()));
                if let Some(code) = bytecode {
                    let abi = "[]";
                    let exploits = ityfuzz_integration::run_ityfuzz(&code, abi, &format!("{target:x}"));
                    exploits.into_iter().map(|(desc, profit)| ValidatedExploit {
                        target,
                        vuln_class: "Guided Fuzzing (ityfuzz)".into(),
                        calldata: Bytes::from(b""),
                        profit,
                        amount: U256::ZERO,
                        revenue: RevenueReport::default(),
                        severity: Severity::Medium,
                        confidence: Confidence::High,
                        description: desc,
                    }).collect()
                } else { vec![] }
            }
        }
    }

    async fn run_oracle_laggard(target: Address, oracle_info: &Option<OracleInfo>, forker: &ForkerAgent) -> Vec<ValidatedExploit> {
        let mut db = match forker.build_cross_contract_db(target, vec![]).await { Ok(db) => db, Err(_) => return vec![] };
        let attacker = Address::repeat_byte(0xde);
        let price_slot = oracle_info.as_ref().and_then(|o| o.price_slot.map(|s| U256::from(s))).unwrap_or(U256::from(13));
        if let Some((profit, calldata)) = oracle_laggard::detect_oracle_laggard_core(&mut db, target, target, attacker, price_slot) {
            if !profit.is_zero() {
                let revenue = RevenueReport {
                    attack: "Oracle Laggard".into(), attacker, target, eth_gained: profit,
                    tokens_gained: vec![], gas_cost_wei: U256::from(150_000),
                    net_profit_wei: profit.saturating_sub(U256::from(150_000)), viability: 0.0,
                };
                let severity = classify(&Impact::from_drain(profit, 18, U256::from(1_000_000_000_000_000_000u128), U256::MAX, true, false, 0, false), Confidence::Proven).0;
                return vec![ValidatedExploit { target, vuln_class: "Oracle Laggard".into(), calldata: Bytes::from(calldata), profit, amount: U256::ZERO, revenue, severity, confidence: Confidence::Proven, description: format!("Stale price oracle exploited, profit: {}", profit) }];
            }
        }
        vec![]
    }

    async fn run_ror_optimizer(target: Address, oracle_info: &Option<OracleInfo>, forker: &ForkerAgent) -> Vec<ValidatedExploit> {
        let mut db = match forker.build_cross_contract_db(target, vec![]).await { Ok(db) => db, Err(_) => return vec![] };
        let attacker = Address::repeat_byte(0xde);
        let price_slot = oracle_info.as_ref().and_then(|o| o.price_slot.map(|s| U256::from(s))).unwrap_or(U256::from(13));
        let (amount, calldata) = ror_optimizer::find_optimal_borrow_core(&mut db, target, target, attacker, U256::from(100_000_000_000_000_000_000u128), price_slot);
        if amount.is_zero() || calldata.is_empty() { return vec![]; }
        let profit = amount;
        let revenue = RevenueReport { attack: "ROR Optimizer".into(), attacker, target, eth_gained: profit, tokens_gained: vec![], gas_cost_wei: U256::from(150_000), net_profit_wei: profit.saturating_sub(U256::from(150_000)), viability: 0.0 };
        vec![ValidatedExploit { target, vuln_class: "ROR".into(), calldata, profit, amount, revenue, severity: Severity::High, confidence: Confidence::Proven, description: format!("Borrow amount {} at manipulated price", amount) }]
    }

    async fn run_reward_dilution(target: Address, forker: &ForkerAgent) -> Vec<ValidatedExploit> {
        let mut db = match forker.build_cross_contract_db(target, vec![]).await { Ok(db) => db, Err(_) => return vec![] };
        let attacker = Address::repeat_byte(0xde);
        if let Some(profit) = reward_dilution::detect_reward_dilution(&mut db, target, attacker) {
            let revenue = RevenueReport { attack: "Reward Dilution".into(), attacker, target, eth_gained: profit, tokens_gained: vec![], gas_cost_wei: U256::ZERO, net_profit_wei: profit, viability: 1.0 };
            vec![ValidatedExploit { target, vuln_class: "Reward Dilution".into(), calldata: Bytes::from(b""), profit, amount: U256::ZERO, revenue, severity: Severity::Medium, confidence: Confidence::Proven, description: format!("Reward dilution yields profit: {}", profit) }]
        } else { vec![] }
    }
}
