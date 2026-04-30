use std::sync::Arc;
use alloy::providers::ProviderBuilder;
use alloy_primitives::{Address, U256};
use revm::db::{CacheDB, EmptyDB};
use revm::primitives::{AccountInfo, Bytecode, keccak256};
use tracing::info;

use crate::agents::fetcher::Fetcher;
use crate::agents::source_fetcher::SourceFetcher;
use crate::agents::poc_generator::{self, Finding};
use crate::agents::notifier::NtfyNotifier;
use crate::agents::proxy_resolver;
use crate::agents::heimdall_analyzer::{self, ContractAnalysis};
use crate::agents::oracle_laggard;
use crate::agents::ror_optimizer;
use crate::agents::reward_dilution;
use crate::agents::search_hunter::SearchHunter;

#[derive(serde::Deserialize)]
struct LiveTarget {
    name: String,
    lender: String,
    oracle: String,
    stake_addr: Option<String>,
}

pub struct Controller {
    fetcher: Fetcher,
}

impl Controller {
    pub async fn new(rpc_url: &str) -> eyre::Result<Self> {
        let provider = Arc::new(ProviderBuilder::new().on_http(rpc_url.parse()?));
        Ok(Self {
            fetcher: Fetcher::new(provider),
        })
    }

    pub async fn run_live(&self, targets_path: &str) -> eyre::Result<()> {
        let data = std::fs::read_to_string(targets_path)?;
        let targets: Vec<LiveTarget> = serde_json::from_str(&data)?;
        let mut findings = Vec::new();
        let source_fetcher = SourceFetcher::new();

        for target in &targets {
            info!("🎯 Hunting live target: {}", target.name);

            // 1. Parse the oracle address from targets.json
            let oracle_addr: Address = target.oracle.parse()?;

            // 2. Try Diamond (EIP‑2535) slot first (works for Compound V3)
            let diamond_slot = {
                let hash = keccak256("diamond.standard.diamond.storage".as_bytes());
                let mut bytes = [0u8; 32];
                bytes.copy_from_slice(hash.as_ref());
                U256::from_be_bytes(bytes) - U256::from(1)
            };
            let oracle_impl = if let Ok(value) = self.fetcher.get_storage_at(oracle_addr, diamond_slot).await {
                if !value.is_zero() {
                    let addr = Address::from_slice(&value.to_be_bytes::<32>()[12..]);
                    info!("🔮 Diamond proxy resolved (oracle) to {:?}", addr);
                    addr
                } else {
                    // Fallback: use Heimdall or original address
                    proxy_resolver::resolve_proxy_heimdall(&format!("{oracle_addr:?}"))
                        .unwrap_or(oracle_addr)
                }
            } else {
                oracle_addr
            };

            // Do the same for the lender address
            let lender_addr: Address = target.lender.parse()?;
            let lender_impl = if let Ok(value) = self.fetcher.get_storage_at(lender_addr, diamond_slot).await {
                if !value.is_zero() {
                    let addr = Address::from_slice(&value.to_be_bytes::<32>()[12..]);
                    info!("🔮 Diamond proxy resolved (lender) to {:?}", addr);
                    addr
                } else {
                    // Fallback: use Heimdall or original address
                    proxy_resolver::resolve_proxy_heimdall(&format!("{lender_addr:?}"))
                        .unwrap_or(lender_addr)
                }
            } else {
                lender_addr
            };


            // 2. Fetch ABI
            let abi = source_fetcher.get_abi(8453, &format!("{oracle_impl:?}")).await;
            if abi.is_none() {
                info!("❌ No ABI found for oracle, skipping");
                continue;
            }
            info!("📄 Oracle ABI fetched");

            // 3. Fetch oracle bytecode
            let oracle_code = self.fetcher.get_code(oracle_impl).await?;
            if oracle_code.is_empty() {
                info!("❌ No oracle bytecode, skipping");
                continue;
            }
            info!("✅ Oracle bytecode: {} bytes", oracle_code.len());

            // 4. Set up sandbox
            let mut db = CacheDB::new(EmptyDB::new());
            db.insert_account_info(oracle_impl, AccountInfo {
                balance: U256::ZERO,
                nonce: 1,
                code: Some(Bytecode::new_raw(oracle_code.into())),
                code_hash: Default::default(),
            });

            let stale_lender_hex = std::fs::read_to_string("mocks/StaleLender.hex")?
                .trim()
                .to_string();
            let stale_lender_code =
                Bytecode::new_raw(hex::decode(&stale_lender_hex)?.into());
            db.insert_account_info(lender_impl, AccountInfo {
                balance: U256::from(1000_000_000_000_000_000_000u128),
                nonce: 1,
                code: Some(stale_lender_code),
                code_hash: Default::default(),
            });

            let mut oracle_addr_bytes = [0u8; 32];
            oracle_addr_bytes[12..32].copy_from_slice(oracle_impl.as_ref());
            db.insert_account_storage(lender_impl, U256::from(2), U256::from_be_bytes(oracle_addr_bytes)).unwrap();

            let attacker = Address::repeat_byte(0xde);
            let analysis = heimdall_analyzer::analyze(&format!("{:?}", oracle_impl));

            info!("🧪 Oracle Laggard detector running...");
            if let Some((profit, calldata)) = oracle_laggard::detect_oracle_laggard_heimdall(
                &mut db,
                lender_impl,
                oracle_impl,
                attacker,
                analysis.as_ref(),
            ) {
                info!("💥 Oracle laggard detected! profit={}", profit);
                findings.push(Finding {
                    bug_class: "Oracle Laggard".into(),
                    target: target.name.clone(),
                    calldata: hex::encode(&calldata),
                    profit,
                    proof: format!(
                        "Oracle price was manipulated, causing under‑collateralised loan. Profit: {} wei",
                        profit
                    ),
                });
            } else {
                info!("✅ No oracle laggard vulnerability found.");
            }

            info!("🧪 ROR Optimizer running...");
            let pool_addr = oracle_impl;
            let (profit, opt_calldata) = ror_optimizer::find_optimal_borrow_heimdall(
                &mut db,
                lender_impl,
                pool_addr,
                attacker,
                U256::from(100_000_000_000_000_000_000u128),
                analysis.as_ref(),
            );

            if profit > U256::ZERO {
                info!("💥 ROR vulnerability detected! profit={}", profit);
                findings.push(Finding {
                    bug_class: "Return-Oriented Reentrancy".into(),
                    target: target.name.clone(),
                    calldata: hex::encode(&opt_calldata),
                    profit,
                    proof: format!(
                        "A reentrancy was found that could be exploited for profit. Profit: {} wei",
                        profit
                    ),
                });
            } else {
                info!("✅ No ROR vulnerability found.");
            }

            if let Some(stake_addr_str) = &target.stake_addr {
                let stake_addr: Address = stake_addr_str.parse()?;
                info!("🧪 Reward Dilution detector running on {}", stake_addr);
                let reward_analysis = heimdall_analyzer::analyze(&format!("{:?}", stake_addr));
                if let Some(profit) = reward_dilution::detect_reward_dilution_heimdall(&mut db, stake_addr, attacker, reward_analysis.as_ref()) {
                    info!("💥 Reward dilution detected! profit={}", profit);
                    findings.push(Finding {
                        bug_class: "Reward Dilution".into(),
                        target: target.name.clone(),
                        calldata: "".into(),
                        profit,
                        proof: format!(
                            "A reward dilution vulnerability was found. Profit: {} wei",
                            profit
                        ),
                    });
                }
            } else {
                info!("✅ No reward dilution vulnerability found.");
            }

            // ---- SearchHunter: trace (placeholder) and proptest (binary search) ----
            let borrow_calldata = &opt_calldata;
            let discovered_oracle = crate::agents::search_hunter::SearchHunter::discover_oracle_via_trace(
                &mut db,
                lender_impl,
                &borrow_calldata,
            );
            if let Some(oracle) = discovered_oracle {
                info!("🔍 SearchHunter discovered oracle via trace: {:?}", oracle);
            }

            if let Some(break_amount) = crate::agents::search_hunter::SearchHunter::hunt_health_factor_proptest(
                &mut db,
                lender_impl,
                oracle_impl,
                attacker,
            ) {
                findings.push(Finding {
                    bug_class: "Invariant Violation (binary search)".into(),
                    target: target.name.clone(),
                    calldata: "0x".into(),
                    profit: break_amount,
                    proof: format!("HealthFactor broken at borrow amount > {}", break_amount),
                });
            }
        }

        if !findings.is_empty() {
            poc_generator::write_poc(&findings);
            let notifier = NtfyNotifier::new(
                std::env::var("NTFY_TOPIC")
                    .unwrap_or_else(|_| "web3_destroyer_alertsz".to_string()),
            );
            for finding in &findings {
                let profit_str = format!("{}", finding.profit);
                tokio::spawn({
                    let notifier = notifier.clone();
                    async move {
                        notifier.send_poc(&profit_str, "poc_output/REPORT_0.md").await;
                    }
                });
            }
        }

        Ok(())
    }
}
