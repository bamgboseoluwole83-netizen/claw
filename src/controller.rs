// src/controller.rs (new, thin version)
use crate::agents::orchestrator::Orchestrator;
use alloy_primitives::Address;
use tracing::info;

pub struct Controller {
    orchestrator: Orchestrator,
}

impl Controller {
    pub async fn new(rpc_url: &str, etherscan_key: Option<String>) -> eyre::Result<Self> {
        Ok(Self {
            orchestrator: Orchestrator::new(rpc_url, etherscan_key).await?,
        })
    }

    pub async fn run_live(&self, targets_path: &str) -> eyre::Result<()> {
        let data = std::fs::read_to_string(targets_path)?;
        let targets: Vec<serde_json::Value> = serde_json::from_str(&data)?;
        let mut all_exploits = Vec::new();

        for t in targets {
            let name = t["name"].as_str().unwrap_or("unknown");
            let address: Address = t["lender"]
                .as_str()
                .or_else(|| t["oracle"].as_str())
                .unwrap()
                .parse()?;

            let exploits = self.orchestrator.analyze_contract(address, name).await;
            for ex in &exploits {
                info!(
                    "💥 {} {:?} profit {} severity {}",
                    ex.vuln_class, ex.target, ex.profit, ex.severity
                );
            }
            all_exploits.extend(exploits);
        }

        // TODO: generate PoCs from all_exploits, notify via Ntfy, etc.
        Ok(())
    }
}