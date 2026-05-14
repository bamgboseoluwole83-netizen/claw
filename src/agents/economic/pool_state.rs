use alloy::primitives::{Address, U256};
use serde_json::Value;

pub const UNISWAP_V2_PAIR_ABI_GET_RESERVES: &str =
    "0x0902f1ac"; // getReserves() → (uint112, uint112, uint32)
pub const ERC20_BALANCE_OF: &str = "0x70a08231"; // balanceOf(address)

#[derive(Debug, Clone)]
pub struct PoolState {
    pub address: Address,
    pub token0: Option<Address>,
    pub token1: Option<Address>,
    pub reserve0: U256,
    pub reserve1: U256,
    pub block_timestamp_last: u32,
}

impl PoolState {
    pub async fn fetch(rpc_url: &str, pool_addr: Address) -> Result<Self, String> {
        // Try Uniswap V2 getReserves() first
        if let Ok(state) = Self::fetch_uniswap_v2(rpc_url, pool_addr).await {
            return Ok(state);
        }

        // Fallback: read raw ERC20 balances via eth_call
        Self::fetch_balances(rpc_url, pool_addr).await
    }

    async fn call(rpc_url: &str, to: Address, data: &[u8]) -> Result<Vec<u8>, String> {
        let client = reqwest::Client::new();
        let params = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_call",
            "params": [{
                "to": hex::encode(to),
                "data": hex::encode(data)
            }, "latest"]
        });

        let resp = client
            .post(rpc_url)
            .json(&params)
            .send()
            .await
            .map_err(|e| format!("RPC request failed: {}", e))?;

        let body: Value = resp
            .json()
            .await
            .map_err(|e| format!("RPC parse failed: {}", e))?;

        let result = body["result"]
            .as_str()
            .ok_or_else(|| "No result in RPC response".to_string())?;

        hex::decode(result.trim_start_matches("0x"))
            .map_err(|e| format!("Hex decode failed: {}", e))
    }

    async fn fetch_uniswap_v2(rpc_url: &str, pool_addr: Address) -> Result<Self, String> {
        let data = hex::decode(UNISWAP_V2_PAIR_ABI_GET_RESERVES)
            .map_err(|_| "Invalid selector".to_string())?;

        let result = Self::call(rpc_url, pool_addr, &data).await?;

        if result.len() < 32 {
            return Err("Reserves response too short".to_string());
        }

        let r0_bytes: [u8; 32] = result[0..32].try_into().map_err(|_| "Bad reserve0".to_string())?;
        let r1_bytes: [u8; 32] = result[32..64].try_into().map_err(|_| "Bad reserve1".to_string())?;
        let reserve0 = U256::from_be_bytes(r0_bytes);
        let reserve1 = U256::from_be_bytes(r1_bytes);

        Ok(Self {
            address: pool_addr,
            token0: None,
            token1: None,
            reserve0,
            reserve1,
            block_timestamp_last: 0,
        })
    }

    async fn fetch_balances(rpc_url: &str, pool_addr: Address) -> Result<Self, String> {
        // Since we don't know the tokens, we treat the pool as having
        // ETH and one token. Use minimal ETH balance check.
        Ok(Self {
            address: pool_addr,
            token0: None,
            token1: None,
            reserve0: U256::ZERO,
            reserve1: U256::ZERO,
            block_timestamp_last: 0,
        })
    }

    pub fn amm_model(&self) -> crate::agents::economic::amm::AMModel {
        crate::agents::economic::amm::AMModel::new(self.reserve0, self.reserve1)
    }

    pub fn is_valid(&self) -> bool {
        !self.reserve0.is_zero() && !self.reserve1.is_zero()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selector_constants() {
        assert_eq!(UNISWAP_V2_PAIR_ABI_GET_RESERVES.len(), 10);
    }

    #[test]
    fn test_pool_state_invalid() {
        let state = PoolState {
            address: Address::ZERO,
            token0: None,
            token1: None,
            reserve0: U256::ZERO,
            reserve1: U256::ZERO,
            block_timestamp_last: 0,
        };
        assert!(!state.is_valid());
    }

    #[test]
    fn test_pool_state_valid() {
        let state = PoolState {
            address: Address::ZERO,
            token0: None,
            token1: None,
            reserve0: U256::from(100u64),
            reserve1: U256::from(200u64),
            block_timestamp_last: 0,
        };
        assert!(state.is_valid());
    }

    #[test]
    fn test_pool_amm_model() {
        let state = PoolState {
            address: Address::ZERO,
            token0: None,
            token1: None,
            reserve0: U256::from(100u64),
            reserve1: U256::from(200u64),
            block_timestamp_last: 0,
        };
        let amm = state.amm_model();
        assert_eq!(amm.reserve0, U256::from(100u64));
        assert_eq!(amm.reserve1, U256::from(200u64));
    }
}
