use reqwest::Client;
use serde_json::Value;
use solang_parser::pt::{SourceUnitPart, ContractPart};
use solang_parser::parse;
use tracing::{info, warn};

pub struct SourceFetcher {
    client: Client,
}

#[derive(Debug)]
pub struct OracleInfo {
    pub oracle_address_slot: Option<usize>,
    pub price_function: Option<String>,
}

impl SourceFetcher {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    // ---------- Source code ----------

    /// Fetch Solidity source: Sourcify first, then Blockscout.
    /// `address` must already be the implementation (proxy resolved by caller).
    pub async fn get_source(&self, chain_id: u32, address: &str) -> Option<String> {
        if let Ok(source) = self.fetch_sourcify(chain_id, address).await {
            info!("✅ Source retrieved via Sourcify for {}", address);
            return Some(source);
        }
        if let Ok(source) = self.fetch_blockscout(address).await {
            info!("✅ Source retrieved via Blockscout for {}", address);
            return Some(source);
        }
        warn!("❌ Could not find source for {}", address);
        None
    }

    async fn fetch_sourcify(&self, chain_id: u32, address: &str) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!(
            "https://repo.sourcify.dev/contracts/full_match/{}/{}/metadata.json",
            chain_id, address
        );
        let resp: Value = self.client.get(&url).send().await?.json().await?;
        let sources = resp["sources"].as_object().ok_or("no sources")?;
        let (_, file) = sources.iter().next().ok_or("empty")?;
        let content = file["content"].as_str().ok_or("no content")?;
        Ok(content.to_string())
    }

    async fn fetch_blockscout(&self, address: &str) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!(
            "https://base.blockscout.com/api?module=contract&action=getsourcecode&address={}",
            address
        );
        let resp: Value = self.client.get(&url).send().await?.json().await?;
        let result = &resp["result"];
        let source = if let Some(arr) = result.as_array() {
            arr[0]["SourceCode"].as_str().ok_or("no source")?
        } else {
            result["SourceCode"].as_str().ok_or("no source")?
        };
        Ok(source.to_string())
    }

    // ---------- ABI ----------

    /// Fetch the ABI from Sourcify (full_match → partial_match) then Blockscout.
    /// `address` must already be the implementation (proxy resolved by caller).
    pub async fn get_abi(&self, chain_id: u32, address: &str) -> Option<String> {
        // Sourcify full match
        if let Ok(abi) = self.fetch_sourcify_abi(chain_id, address, "full_match").await {
            return Some(abi);
        }
        // Sourcify partial match
        if let Ok(abi) = self.fetch_sourcify_abi(chain_id, address, "partial_match").await {
            return Some(abi);
        }
        // Blockscout
        if let Ok(abi) = self.fetch_blockscout_abi(address).await {
            return Some(abi);
        }
        None
    }

    async fn fetch_sourcify_abi(
        &self,
        chain_id: u32,
        address: &str,
        match_type: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!(
            "https://repo.sourcify.dev/contracts/{}/{}/{}/metadata.json",
            match_type, chain_id, address
        );
        let resp: Value = self.client.get(&url).send().await?.json().await?;
        let abi = &resp["output"]["abi"];
        Ok(serde_json::to_string(abi)?)
    }

    async fn fetch_blockscout_abi(&self, address: &str) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!(
            "https://base.blockscout.com/api?module=contract&action=getabi&address={}",
            address
        );
        let resp: Value = self.client.get(&url).send().await?.json().await?;
        let abi_str = resp["result"].as_str().ok_or("no abi")?;
        // Blockscout returns the ABI as a JSON string
        let abi: Value = serde_json::from_str(abi_str).map_err(|_| "invalid abi json")?;
        Ok(serde_json::to_string(&abi)?)
    }

    // ---------- AST parsing ----------

    /// Parse the Solidity source and extract oracle information.
    pub fn parse_oracle_from_source(source: &str) -> Option<OracleInfo> {
        let (ast, _) = parse(source, 0).ok()?;
        let mut oracle_slot = None;
        let mut price_func = None;

        for part in &ast.0 {
            if let SourceUnitPart::ContractDefinition(contract) = part {
                for contract_part in &contract.parts {
                    match contract_part {
                        ContractPart::VariableDefinition(var_def) => {
                            let ty_str = format!("{}", var_def.ty);
                            if ty_str.contains("Aggregator") || ty_str.contains("IStaleOracle") || ty_str.contains("IPriceOracle") {
                                oracle_slot = Some(2);
                                if let Some(ref id) = var_def.name {
                                    info!("🔍 Oracle variable found: {:?} (assumed slot 2)", id.name);
                                }
                            }
                        }
                        ContractPart::FunctionDefinition(func_def) => {
                            if let Some(ref body) = func_def.body {
                                let body_str = format!("{}", body);
                                if body_str.contains(".price()") || body_str.contains(".latestAnswer()") {
                                    price_func = Some("latestAnswer".into());
                                    info!("🔍 Oracle price call logic detected in code");
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if oracle_slot.is_some() || price_func.is_some() {
            Some(OracleInfo {
                oracle_address_slot: oracle_slot,
                price_function: price_func,
            })
        } else {
            None
        }
    }
}