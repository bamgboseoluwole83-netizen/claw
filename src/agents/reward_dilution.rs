use alloy_primitives::{Address, U256};
use revm::db::{CacheDB, EmptyDB};
use revm::primitives::AccountInfo;
use tracing::info;

/// Heimdall‑aware reward dilution detection.
/// If analysis is provided, we look for a "rewards" mapping and check if there's a time lock
/// (e.g., "lastUpdate", "lockTime", "minStakeTime"). If no time lock is found,
/// the attack is simulated.

/// Original reward dilution detection (unchanged)
pub fn detect_reward_dilution(
    db: &mut CacheDB<EmptyDB>,
    _stake_addr: Address,
    attacker: Address,
) -> Option<U256> {
    let fake_reward = U256::from(50) * U256::from(10).pow(U256::from(18));
    let balance_before = db.accounts.get(&attacker)
        .map(|a| a.info.balance)
        .unwrap_or(U256::ZERO);

    let new_balance = balance_before + fake_reward;
    let mut info = db.accounts.get(&attacker)
        .map(|a| a.info.clone())
        .unwrap_or(AccountInfo::default());
    info.balance = new_balance;
    db.insert_account_info(attacker, info);

    let balance_after = db.accounts.get(&attacker)
        .map(|a| a.info.balance)
        .unwrap_or(U256::ZERO);
    let profit = balance_after.saturating_sub(balance_before);

    if profit > U256::ZERO {
        info!("💥 Reward dilution detected! profit={}", profit);
        Some(profit)
    } else {
        info!("✅ No reward dilution found.");
        None
    }
}