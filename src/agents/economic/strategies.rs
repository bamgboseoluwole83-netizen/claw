use std::collections::HashMap;

use alloy::primitives::{Address, U256};

use crate::agents::economic::amm::AMModel;
use crate::agents::economic::flashloan::{simulate_flash_loan, FlashLoanPool};
use crate::agents::economic::graph::ContractGraph;
use crate::agents::economic::pool_state::PoolState;
use crate::agents::economic::{EconStep, EconomicFinding};

const ONE: U256 = U256::from_limbs([1_000_000_000_000_000_000u64, 0, 0, 0]);
const MIN_PROFIT_ETH: U256 = U256::from_limbs([100_000_000_000_000_000u64, 0, 0, 0]);

pub struct ExploitStrategy {
    pub name: &'static str,
    pub evaluate: fn(
        target: Address,
        proxy: Address,
        pools: &HashMap<Address, PoolState>,
        graph: &ContractGraph,
    ) -> Option<EconomicFinding>,
}

pub fn exploit_strategies() -> Vec<ExploitStrategy> {
    vec![
        ExploitStrategy {
            name: "oracle_manipulation",
            evaluate: oracle_manipulation,
        },
        ExploitStrategy {
            name: "flash_liquidity_drain",
            evaluate: flash_liquidity_drain,
        },
        ExploitStrategy {
            name: "price_arbitrage",
            evaluate: price_arbitrage,
        },
        ExploitStrategy {
            name: "twap_oracle_manipulation",
            evaluate: twap_oracle_manipulation,
        },
        ExploitStrategy {
            name: "erc4626_inflation",
            evaluate: erc4626_inflation,
        },
        ExploitStrategy {
            name: "multi_hop_arbitrage",
            evaluate: multi_hop_arbitrage,
        },
        ExploitStrategy {
            name: "mev_sandwich",
            evaluate: mev_sandwich,
        },
        ExploitStrategy {
            name: "cross_protocol_liquidation",
            evaluate: cross_protocol_liquidation,
        },
    ]
}

/// Oracle manipulation via single-block swap
fn oracle_manipulation(
    target: Address,
    proxy: Address,
    pools: &HashMap<Address, PoolState>,
    _graph: &ContractGraph,
) -> Option<EconomicFinding> {
    for (_addr, pool) in pools {
        if !pool.is_valid() {
            continue;
        }

        let amm = pool.amm_model();
        let tvl = amm.tvl(1.0, 1.0);
        if tvl < 10.0 {
            continue;
        }

        let swap_amount = pool.reserve0 / U256::from(20u64);
        let (new_r0, new_r1) = amm.after_swap(swap_amount);
        let price_change = AMModel::new(new_r0, new_r1).spot_price() / amm.spot_price();

        if price_change >= 1.02 {
            let manipulated = AMModel::new(new_r0, new_r1);
            let collateral = swap_amount;
            let old_power = amm.spot_price() * (u256_to_f64(collateral));
            let new_power = manipulated.spot_price() * (u256_to_f64(collateral));
            let extra_borrow = (new_power - old_power).abs();

            if u256_from_f64(extra_borrow) >= MIN_PROFIT_ETH {
                return Some(EconomicFinding {
                    strategy: "oracle_manipulation".to_string(),
                    target,
                    profit_estimate: u256_from_f64(extra_borrow),
                    steps: vec![
                        EconStep {
                            target: proxy,
                            calldata: vec![0xd0, 0xe3, 0x0d, 0xb0],
                            value: collateral,
                            description: format!("Deposit {} ETH", u256_to_f64(collateral) / 1e18),
                        },
                        EconStep {
                            target: pool.address,
                            calldata: vec![],
                            value: U256::ZERO,
                            description: format!("Swap {} ETH, price change {:.2}x", u256_to_f64(swap_amount) / 1e18, price_change),
                        },
                        EconStep {
                            target: proxy,
                            calldata: vec![0xc5, 0xeb, 0xea, 0xec],
                            value: U256::ZERO,
                            description: format!("Borrow extra {:.4} ETH", extra_borrow / 1e18),
                        },
                    ],
                    confidence: 0.4,
                    description: format!(
                        "Oracle manipulation: swap {:.2} ETH → {:.2}x price → borrow extra {:.4} ETH",
                        u256_to_f64(swap_amount) / 1e18, price_change, extra_borrow / 1e18,
                    ),
                });
            }
        }
    }
    None
}

/// Flash loan liquidity drain
fn flash_liquidity_drain(
    target: Address,
    _proxy: Address,
    pools: &HashMap<Address, PoolState>,
    _graph: &ContractGraph,
) -> Option<EconomicFinding> {
    for (_addr, pool) in pools {
        if !pool.is_valid() {
            continue;
        }

        let tvl = pool.amm_model().tvl(1.0, 1.0);
        if tvl < 100.0 {
            continue;
        }

        let max_reserve = pool.reserve0.max(pool.reserve1);
        let flash_pool =
            FlashLoanPool::new(hex::encode(pool.address), max_reserve / U256::from(2u64));
        let borrow = flash_pool.optimal_borrow(max_reserve);
        let flash_sim = simulate_flash_loan(&flash_pool, borrow, max_reserve / U256::from(4u64));

        if flash_sim.is_feasible {
            let max_profit =
                (max_reserve / U256::from(4u64)).saturating_sub(flash_sim.repay_amount);
            if max_profit >= MIN_PROFIT_ETH {
                return Some(EconomicFinding {
                    strategy: "flash_liquidity_drain".to_string(),
                    target,
                    profit_estimate: max_profit,
                    steps: vec![
                        EconStep {
                            target: pool.address,
                            calldata: vec![],
                            value: U256::ZERO,
                            description: format!(
                                "Flash borrow {:.4} ETH",
                                u256_to_f64(borrow) / 1e18
                            ),
                        },
                        EconStep {
                            target,
                            calldata: vec![],
                            value: U256::ZERO,
                            description: "Execute exploit".to_string(),
                        },
                    ],
                    confidence: 0.3,
                    description: format!(
                        "Flash loan drain pool {}: borrow {:.2} ETH, expected profit {:.4} ETH",
                        hex::encode(pool.address),
                        u256_to_f64(borrow) / 1e18,
                        u256_to_f64(max_profit) / 1e18,
                    ),
                });
            }
        }
    }
    None
}

/// Simple two-pool price arbitrage
fn price_arbitrage(
    target: Address,
    _proxy: Address,
    pools: &HashMap<Address, PoolState>,
    _graph: &ContractGraph,
) -> Option<EconomicFinding> {
    let prices: Vec<(Address, f64)> = pools
        .iter()
        .filter(|(_, p)| p.is_valid())
        .map(|(addr, p)| (*addr, p.amm_model().spot_price()))
        .collect();

    for i in 0..prices.len() {
        for j in i + 1..prices.len() {
            let (addr_a, price_a) = prices[i];
            let (addr_b, price_b) = prices[j];
            if price_a == 0.0 || price_b == 0.0 {
                continue;
            }
            let ratio = if price_a > price_b {
                price_a / price_b
            } else {
                price_b / price_a
            };
            if ratio > 1.01 {
                let profit_wei = (ratio - 1.0) * 1e18;
                let profit = U256::from(profit_wei as u64 * 10u64);
                if profit >= MIN_PROFIT_ETH {
                    return Some(EconomicFinding {
                        strategy: "price_arbitrage".to_string(),
                        target,
                        profit_estimate: profit,
                        steps: vec![
                            EconStep {
                                target: addr_a,
                                calldata: vec![],
                                value: ONE * U256::from(10u64),
                                description: format!(
                                    "Buy pool {:.8} at {:.4}",
                                    hex::encode(addr_a),
                                    price_a
                                ),
                            },
                            EconStep {
                                target: addr_b,
                                calldata: vec![],
                                value: U256::ZERO,
                                description: format!(
                                    "Sell pool {:.8} at {:.4}",
                                    hex::encode(addr_b),
                                    price_b
                                ),
                            },
                        ],
                        confidence: 0.5,
                        description: format!("Arbitrage {:.4}x between pools", ratio),
                    });
                }
            }
        }
    }
    None
}

/// ═══════════════════════════════════════════════════════════
///  Enhanced Protocol-Specific Strategies
/// ═══════════════════════════════════════════════════════════

/// TWAP Oracle Manipulation: manipulate the TWAP oracle over multiple blocks.
/// Real Uniswap V2 TWAP requires ~2 blocks to fully manipulate.
fn twap_oracle_manipulation(
    target: Address,
    _proxy: Address,
    pools: &HashMap<Address, PoolState>,
    graph: &ContractGraph,
) -> Option<EconomicFinding> {
    // For each DEX pool, try to find if it's used as a TWAP oracle for the target
    let pool_addrs: Vec<Address> = pools.keys().copied().collect();

    for &pool_addr in &pool_addrs {
        // Check if pool has a path to target in the graph
        let paths = graph.find_path(pool_addr, target, 3);
        if paths.is_empty() {
            continue;
        }

        let pool = &pools[&pool_addr];
        let amm = pool.amm_model();
        let tvl = amm.tvl(1.0, 1.0);
        if tvl < 100.0 {
            continue;
        }

        // TWAP amplification: manipulate price with 2x the normal impact
        // (Uniswap V2 TWAP = (reserve0_cumulative_last / reserve1_cumulative_last))
        let manipulation_amt = pool.reserve0 / U256::from(10u64); // 10% of pool
        let old_price = amm.spot_price();
        let (new_r0, new_r1) = amm.after_swap(manipulation_amt);
        let new_price = AMModel::new(new_r0, new_r1).spot_price();
        let price_change = new_price / old_price;

        if price_change >= 1.10 {
            // 10% manipulation needed for profitable TWAP exploit
            let cost_to_manipulate = manipulation_amt;
            let profit = pool.reserve1 / U256::from(20u64)
                * U256::from(price_change as u64).saturating_sub(cost_to_manipulate);

            if profit >= MIN_PROFIT_ETH {
                return Some(EconomicFinding {
                    strategy: "twap_oracle_manipulation".to_string(),
                    target,
                    profit_estimate: profit,
                    steps: vec![
                        EconStep {
                            target: pool_addr,
                            calldata: vec![],
                            value: manipulation_amt,
                            description: format!("Block N: Swap {:.2} ETH (first TWAP manipulation)", u256_to_f64(manipulation_amt) / 1e18),
                        },
                        EconStep {
                            target: pool_addr,
                            calldata: vec![],
                            value: manipulation_amt,
                            description: format!("Block N+1: Swap {:.2} ETH (second TWAP manipulation)", u256_to_f64(manipulation_amt) / 1e18),
                        },
                        EconStep {
                            target,
                            calldata: vec![],
                            value: U256::ZERO,
                            description: format!("Execute exploit using manipulated TWAP price ({:.2}x)", price_change / 2.0),
                        },
                    ],
                    confidence: 0.35,
                    description: format!(
                        "TWAP oracle manipulation via pool {}: {} ETH manipulation over 2 blocks → {:.2}x price → expected profit {:.4} ETH",
                        hex::encode(pool_addr),
                        u256_to_f64(manipulation_amt) / 1e18,
                        price_change / 2.0,
                        u256_to_f64(profit) / 1e18,
                    ),
                });
            }
        }
    }
    None
}

/// ERC4626 Inflation Attack: detect donation-based share price inflation.
/// If target has deposit/withdraw and we can donate assets, we can inflate
/// share price and profit from later depositors.
fn erc4626_inflation(
    target: Address,
    _proxy: Address,
    pools: &HashMap<Address, PoolState>,
    graph: &ContractGraph,
) -> Option<EconomicFinding> {
    // Check if target is classified as a vault or token
    let is_vault = match graph.classify_node(&target) {
        crate::agents::economic::graph::ContractClass::Token
        | crate::agents::economic::graph::ContractClass::Wallet => true,
        _ => false,
    };
    if !is_vault {
        return None;
    }

    // Look for a pool that trades the target's share token
    // If we can manipulate the pool price → inflate the share price → profit
    for (_addr, pool) in pools {
        if !pool.is_valid() {
            continue;
        }

        let amm = pool.amm_model();
        let tvl = amm.tvl(1.0, 1.0);
        if tvl < 50.0 {
            continue;
        }

        // Inflation vector: donate assets to the vault, inflate share price,
        // then swap inflated shares in the pool
        let donation = pool.reserve0 / U256::from(100u64); // 1% donation
        let (new_r0, _new_r1) = amm.after_swap(donation);
        let price_change = AMModel::new(new_r0, pool.reserve1).spot_price() / amm.spot_price();

        if price_change >= 1.05 {
            // 5% inflation profitable
            let profit = pool.reserve1 / U256::from(50u64) * U256::from(price_change as u64);
            if profit >= MIN_PROFIT_ETH {
                return Some(EconomicFinding {
                    strategy: "erc4626_inflation".to_string(),
                    target,
                    profit_estimate: profit,
                    steps: vec![
                        EconStep {
                            target,
                            calldata: vec![],
                            value: donation,
                            description: format!("Donate {:.4} ETH to inflate share price", u256_to_f64(donation) / 1e18),
                        },
                        EconStep {
                            target,
                            calldata: vec![0x00, 0x00, 0x00, 0x00],
                            value: U256::ZERO,
                            description: "Call deposit/redeem at inflated price".to_string(),
                        },
                        EconStep {
                            target: pool.address,
                            calldata: vec![],
                            value: U256::ZERO,
                            description: "Swap inflated shares for profit".to_string(),
                        },
                    ],
                    confidence: 0.3,
                    description: format!(
                        "ERC4626 inflation attack via pool {}: donate {:.4} ETH to inflate shares {:.2}x",
                        hex::encode(pool.address),
                        u256_to_f64(donation) / 1e18,
                        price_change / 2.0,
                    ),
                });
            }
        }
    }
    None
}

/// Multi-Hop Arbitrage: find profitable paths through 3+ pools
fn multi_hop_arbitrage(
    _target: Address,
    _proxy: Address,
    pools: &HashMap<Address, PoolState>,
    graph: &ContractGraph,
) -> Option<EconomicFinding> {
    let pool_addrs: Vec<Address> = pools.keys().copied().collect();
    if pool_addrs.len() < 3 {
        return None;
    }

    // For each pool pair, check if there's a path through a third pool
    for &start in &pool_addrs {
        for &mid in &pool_addrs {
            if mid == start {
                continue;
            }
            for &end in &pool_addrs {
                if end == mid || end == start {
                    continue;
                }

                // Check if graph has paths connecting these pools
                let path_fwd = graph.find_path(start, mid, 2);
                let path_bwd = graph.find_path(mid, end, 2);
                if path_fwd.is_empty() || path_bwd.is_empty() {
                    continue;
                }

                let start_pool = &pools[&start];
                let mid_pool = &pools[&mid];
                let end_pool = &pools[&end];

                // Simulate: buy on cheapest, sell on most expensive
                let price_start = start_pool.amm_model().spot_price();
                let price_mid = mid_pool.amm_model().spot_price();
                let price_end = end_pool.amm_model().spot_price();

                if price_start == 0.0 || price_mid == 0.0 || price_end == 0.0 {
                    continue;
                }

                // Check: Path A->B->C returns more than A alone
                // Start: buy token on pool A for ETH
                let volume = ONE * U256::from(10u64); // 10 ETH
                let step1 = start_pool.amm_model().swap_output(volume);
                if step1.is_zero() {
                    continue;
                }
                // Swap on mid pool
                let step2 = mid_pool.amm_model().swap_output_inverse(step1);
                if step2.is_zero() {
                    continue;
                }
                // back to ETH
                let step3 = end_pool.amm_model().swap_output(step2);
                if step3 <= volume {
                    continue;
                }

                let profit = step3.saturating_sub(volume);
                if profit >= MIN_PROFIT_ETH {
                    return Some(EconomicFinding {
                        strategy: "multi_hop_arbitrage".to_string(),
                        target: start,
                        profit_estimate: profit,
                        steps: vec![
                            EconStep {
                                target: start,
                                calldata: vec![],
                                value: volume,
                                description: format!(
                                    "Buy on pool A {} at price {:.4}",
                                    hex::encode(start),
                                    price_start
                                ),
                            },
                            EconStep {
                                target: mid,
                                calldata: vec![],
                                value: U256::ZERO,
                                description: format!(
                                    "Swap on pool B {} at price {:.4}",
                                    hex::encode(mid),
                                    price_mid
                                ),
                            },
                            EconStep {
                                target: end,
                                calldata: vec![],
                                value: U256::ZERO,
                                description: format!(
                                    "Sell on pool C {} at price {:.4}",
                                    hex::encode(end),
                                    price_end
                                ),
                            },
                        ],
                        confidence: 0.4,
                        description: format!(
                            "Multi-hop arbitrage {} → {} → {}: profit {:.6} ETH",
                            hex::encode(start),
                            hex::encode(mid),
                            hex::encode(end),
                            u256_to_f64(profit) / 1e18,
                        ),
                    });
                }
            }
        }
    }
    None
}

/// ═══════════════════════════════════════════════════════════
///  MEV + Liquidation Strategies
/// ═══════════════════════════════════════════════════════════

/// MEV Sandwich: frontrun + user tx + backrun on a DEX swap.
/// Detects if a pool is deep enough that sandwiching a large swap is profitable.
fn mev_sandwich(
    target: Address,
    _proxy: Address,
    pools: &HashMap<Address, PoolState>,
    _graph: &ContractGraph,
) -> Option<EconomicFinding> {
    for (_addr, pool) in pools {
        if !pool.is_valid() {
            continue;
        }

        let amm = pool.amm_model();
        let tvl = amm.tvl(1.0, 1.0);
        if tvl < 50.0 {
            continue;
        }

        // Assume a victim swap of 10% of the pool
        let victim_swap = pool.reserve0 / U256::from(10u64);

        // Frontrun: buy before victim (5% of pool)
        let frontrun_amt = pool.reserve0 / U256::from(20u64);
        let (r0_after_frontrun, r1_after_frontrun) = amm.after_swap(frontrun_amt);

        // Victim swap executes at worse price
        let post_frontrun = AMModel::new(r0_after_frontrun, r1_after_frontrun);
        let (_r0_after_victim, r1_after_victim) = post_frontrun.after_swap(victim_swap);

        // Backrun: sell at the new lower price
        let post_victim = AMModel::new(r0_after_frontrun + victim_swap, r1_after_victim);
        let backrun_output = post_victim.swap_output_inverse(frontrun_amt);

        // Profit = backrun_output - frontrun_amt (what we sold minus what we bought)
        let profit = backrun_output.saturating_sub(frontrun_amt);

        if profit >= MIN_PROFIT_ETH {
            let total_vol = frontrun_amt + victim_swap;
            let gas_cost = U256::from(150_000u64) * U256::from(50_000_000_000u64); // 150k gas * 50 gwei
            let net_profit = profit.saturating_sub(gas_cost);

            if net_profit >= MIN_PROFIT_ETH {
                return Some(EconomicFinding {
                    strategy: "mev_sandwich".to_string(),
                    target,
                    profit_estimate: net_profit,
                    steps: vec![
                        EconStep {
                            target: pool.address,
                            calldata: vec![0x02, 0x4e, 0xcf, 0xc8],
                            value: frontrun_amt,
                            description: format!("Frontrun: buy {:.4} ETH on pool", u256_to_f64(frontrun_amt) / 1e18),
                        },
                        EconStep {
                            target: pool.address,
                            calldata: vec![],
                            value: victim_swap,
                            description: format!("Victim swap: {:.4} ETH (sandwiched)", u256_to_f64(victim_swap) / 1e18),
                        },
                        EconStep {
                            target: pool.address,
                            calldata: vec![0x02, 0x4e, 0xcf, 0xc8],
                            value: U256::ZERO,
                            description: format!("Backrun: sell at manipulated price, profit {:.6} ETH", u256_to_f64(net_profit) / 1e18),
                        },
                    ],
                    confidence: 0.45,
                    description: format!(
                        "MEV sandwich on pool {}: frontrun {:.2} ETH → victim {:.2} ETH → backrun, net profit {:.6} ETH",
                        hex::encode(pool.address),
                        u256_to_f64(frontrun_amt) / 1e18,
                        u256_to_f64(victim_swap) / 1e18,
                        u256_to_f64(net_profit) / 1e18,
                    ),
                });
            }
        }
    }
    None
}

/// Cross-Protocol Liquidation: find a lending protocol with an oracle pool,
/// manipulate the oracle price to make positions undercollateralized, then liquidate.
fn cross_protocol_liquidation(
    target: Address,
    proxy: Address,
    pools: &HashMap<Address, PoolState>,
    graph: &ContractGraph,
) -> Option<EconomicFinding> {
    // Check if target or proxy is a lending protocol (has borrow/deposit)
    let is_lending = matches!(
        graph.classify_node(&target),
        crate::agents::economic::graph::ContractClass::LendingPool
    ) || matches!(
        graph.classify_node(&proxy),
        crate::agents::economic::graph::ContractClass::LendingPool
    );
    if !is_lending {
        return None;
    }

    // Also check if any pool-classified node exists in the graph
    let has_dex = pools.values().any(|p| p.is_valid());
    if !has_dex {
        return None;
    }

    // For each pool that could be an oracle feed for the lending protocol
    for (_addr, pool) in pools {
        if !pool.is_valid() {
            continue;
        }

        let amm = pool.amm_model();
        let tvl = amm.tvl(1.0, 1.0);
        if tvl < 100.0 {
            continue;
        }

        // Check if there's a path from this pool to the target in the graph
        let paths = graph.find_path(pool.address, target, 2);
        if paths.is_empty() && pool.address != target {
            // If no direct path, try proxy
            let proxy_paths = graph.find_path(pool.address, proxy, 2);
            if proxy_paths.is_empty() {
                continue;
            }
        }

        // Simulate: manipulate pool price down to trigger liquidations
        let manipulation = pool.reserve1 / U256::from(15u64); // 6.67% dump
        let (new_r0, new_r1) = amm.after_swap(manipulation);
        let price_drop = 1.0 - (AMModel::new(new_r0, new_r1).spot_price() / amm.spot_price());

        if price_drop > 0.05 {
            // >5% price drop triggers liquidation in most protocols
            let manipulated_pool = AMModel::new(new_r0, new_r1);

            // Typical liquidation profit = liquidated_position * (liquidation_bonus - repay)
            // Assuming a position of 10% of available liquidity gets liquidated
            let liquidated_collateral = pool.reserve0 / U256::from(10u64);
            let liquidation_bonus = U256::from(105u64); // 5% bonus
            let discounted_collateral =
                liquidated_collateral * U256::from(100u64) / liquidation_bonus;

            // Cost: we need to swap to manipulate price, then we get the liquidated collateral
            let cost_to_manipulate = manipulation;
            let gain = discounted_collateral;
            let profit = gain.saturating_sub(cost_to_manipulate);

            if profit >= MIN_PROFIT_ETH {
                return Some(EconomicFinding {
                    strategy: "cross_protocol_liquidation".to_string(),
                    target,
                    profit_estimate: profit,
                    steps: vec![
                        EconStep {
                            target: pool.address,
                            calldata: vec![],
                            value: manipulation,
                            description: format!("Dump {:.4} ETH on oracle pool to crash price {:.1}%", u256_to_f64(manipulation) / 1e18, price_drop * 100.0),
                        },
                        EconStep {
                            target: proxy,
                            calldata: vec![0xec, 0x8e, 0x48, 0x60], // liquidate(address,address)
                            value: U256::ZERO,
                            description: format!("Liquidate position at manipulated price, gain {:.4} ETH", u256_to_f64(gain) / 1e18),
                        },
                        EconStep {
                            target: pool.address,
                            calldata: vec![],
                            value: U256::ZERO,
                            description: "Swap liquidated collateral back, repay manipulation cost".to_string(),
                        },
                    ],
                    confidence: 0.35,
                    description: format!(
                        "Cross-protocol liquidation on {} via pool {}: dump {:.2} ETH → {:.1}% price drop → liquidate → profit {:.6} ETH",
                        hex::encode(target),
                        hex::encode(pool.address),
                        u256_to_f64(manipulation) / 1e18,
                        price_drop * 100.0,
                        u256_to_f64(profit) / 1e18,
                    ),
                });
            }
        }
    }
    None
}

// ── Helpers ──

fn u256_to_f64(u: U256) -> f64 {
    crate::agents::economic::u256_to_f64(u)
}

fn u256_from_f64(f: f64) -> U256 {
    U256::from(f as u128)
}
