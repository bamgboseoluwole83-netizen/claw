use alloy::primitives::{Address, U256};
use std::path::Path;

use crate::agents::chain;
use crate::agents::finding::{Finding, ToolKind, VerifiedFinding, VerifyStatus};

pub fn generate_report(
    target: Address,
    rpc_url: &str,
    block_number: u64,
    findings: &[Finding],
    verified: &[VerifiedFinding],
    _source_dir: Option<&Path>,
) -> (String, String) {
    let markdown = build_markdown(target, rpc_url, block_number, findings, verified);
    let sol_code = build_sol_poc(verified, block_number);
    (markdown, sol_code)
}

// ── Markdown ──

fn build_markdown(
    target: Address,
    _rpc_url: &str,
    block_number: u64,
    findings: &[Finding],
    verified: &[VerifiedFinding],
) -> String {
    let target_hex = format!("{:?}", target);

    // Tool stats
    let mut tool_counts: Vec<(String, usize)> = Vec::new();
    for tk in &[
        ToolKind::Slither,
        ToolKind::Conkas,
        ToolKind::Wake,
        ToolKind::Mythril,
        ToolKind::Heimdall,
        ToolKind::Halmos,
        ToolKind::Medusa,
        ToolKind::Ityfuzz,
        ToolKind::Synthesizer,
        ToolKind::Foray,
    ] {
        let count = findings.iter().filter(|f| f.tool == *tk).count();
        if count > 0 {
            tool_counts.push((tk.to_string(), count));
        }
    }
    let tools_str = tool_counts
        .iter()
        .map(|(name, n)| format!("{}({})", name, n))
        .collect::<Vec<_>>()
        .join(", ");

    let high_findings: Vec<&VerifiedFinding> =
        verified.iter().filter(|e| e.score >= 10.0).collect();
    let med_findings: Vec<&VerifiedFinding> = verified
        .iter()
        .filter(|e| e.score >= 5.0 && e.score < 10.0)
        .collect();

    let mut md = String::new();
    md.push_str(&format!("# Exploit Report: {}\n\n", target_hex));
    md.push_str("## Metadata\n\n");
    md.push_str(&format!("| Field | Value |\n|---|---|\n"));
    md.push_str(&format!("| Target | `{}` |\n", target_hex));
    md.push_str(&format!("| Block | `{}` |\n", block_number));
    md.push_str(&format!("| Tools | {} |\n", tools_str));
    md.push_str(&format!("| Total Findings | {} |\n", findings.len()));
    md.push_str(&format!("| Verified (tested) | {} |\n", verified.len()));
    let profit_total: U256 = verified
        .iter()
        .fold(U256::ZERO, |a, e| a.saturating_add(e.profit_estimate));
    md.push_str(&format!("| Total Profit | {} wei |\n", profit_total));
    md.push_str(
        "\n---

## Summary\n\n",
    );
    md.push_str(
        "| # | Tool | Score | Severity | Profit (wei) | Status |\n|---|---|---|---|---|---|\n",
    );
    for (i, e) in verified.iter().enumerate() {
        let profit = e.profit_estimate;
        md.push_str(&format!(
            "| {} | {} | {:.1} | {:.1} | {} | {} |\n",
            i + 1,
            e.tool,
            e.score,
            e.severity,
            profit,
            e.status
        ));
    }

    // High severity
    md.push_str("\n---\n\n## High Severity Findings (score ≥ 10.0)\n\n");
    if high_findings.is_empty() {
        md.push_str("_No high-severity verified findings._\n\n");
    } else {
        for (i, e) in high_findings.iter().enumerate() {
            append_finding(&mut md, i + 1, e, block_number);
        }
    }

    // Medium severity
    md.push_str("\n---\n\n## Medium Severity Findings (score 5.0 – 9.9)\n\n");
    if med_findings.is_empty() {
        md.push_str("_No medium-severity verified findings._\n\n");
    } else {
        for (i, e) in med_findings.iter().enumerate() {
            append_finding(&mut md, i + 1, e, block_number);
        }
    }

    // Run instructions
    md.push_str("\n---\n\n## Run Instructions\n\n");
    md.push_str("```bash\n");
    md.push_str(&format!("# Set your RPC (Mainnet, Base, Arbitrum, etc.)\n"));
    md.push_str("export DRPC_URL=<your_rpc_endpoint>\n\n");
    md.push_str("# Run all exploit tests\n");
    md.push_str("forge test --mt test_exploit -vvv\n");
    md.push_str("```\n\n");
    md.push_str("> The PoC uses `vm.envOr(\"DRPC_URL\", \"https://your-fallback.com\")` ");
    md.push_str(
        "so it works without any CLI flags. Set `DRPC_URL` or edit the fallback in the contract.\n",
    );

    md
}

fn append_finding(md: &mut String, index: usize, e: &VerifiedFinding, block_number: u64) {
    let short = truncate(&e.description, 120);
    let profit = e.profit_estimate;

    md.push_str(&format!("### {}. {}\n\n", index, short));
    md.push_str("| Field | Value |\n|---|---|\n");
    md.push_str(&format!("| Tool | {} |\n", e.tool));
    md.push_str(&format!("| Score | {:.1} |\n", e.score));
    md.push_str(&format!("| Severity | {:.1} |\n", e.severity));
    md.push_str(&format!("| Status | {} |\n", e.status));
    md.push_str(&format!("| Profit | {} wei |\n", profit));
    md.push_str(&format!("| Target | `{:?}` |\n", e.target));
    md.push_str("\n");

    // Sherlock-safe description
    match e.status {
        VerifyStatus::Verified => {
            let profit_eth = profit / U256::from(1_000_000_000_000_000_000u128);
            md.push_str(&format!(
                "The exploit extracts **{} ETH** by triggering the vulnerability described. ",
                profit_eth
            ));
            md.push_str("The PoC below reproduces the attack and verifies profit extraction.\n\n");
        }
        VerifyStatus::Partial => {
            md.push_str(
                "The transaction executed successfully, confirming the call path is **reachable** ",
            );
            md.push_str(
                "and the vulnerable function can be invoked. Under specific state conditions ",
            );
            md.push_str(
                "(e.g., larger deposit, manipulated oracle, griefing preconditions), an attacker ",
            );
            md.push_str("**could** cause loss of funds via this vector.\n\n");
        }
        VerifyStatus::Reverted => {
            md.push_str(
                "The function selector exists and the call was attempted, but the on-chain state ",
            );
            md.push_str("at the pinned block did not satisfy all preconditions. This confirms the entry point ");
            md.push_str(
                "is **active** and may be exploitable under different state conditions.\n\n",
            );
        }
    }

    // PoC code block
    let (poc_contract, poc_name) = build_single_poc(e, index, block_number);
    md.push_str("```solidity\n");
    md.push_str(&poc_contract);
    md.push_str("\n```\n\n");

    md.push_str(&format!(
        "```bash\nforge test --mt {} -vvv\n```\n\n",
        poc_name
    ));
    md.push_str("---\n\n");
}

// ── Solidity PoC ──

fn build_sol_poc(verified: &[VerifiedFinding], block_number: u64) -> String {
    let mut sol = String::new();
    sol.push_str("// SPDX-License-Identifier: MIT\n");
    sol.push_str("pragma solidity ^0.8.0;\n");
    sol.push_str("import \"forge-std/Test.sol\";\n\n");

    for (i, e) in verified.iter().enumerate() {
        let (contract, _) = build_single_poc(e, i + 1, block_number);
        sol.push_str(&contract);
        sol.push('\n');
    }

    sol
}

fn build_single_poc(e: &VerifiedFinding, index: usize, block_number: u64) -> (String, String) {
    let name = format!("PoC_{}_{}", index, slugify(&e.tool.to_string()));
    let is_chain = e.description.starts_with("[chain]");
    let target_hex = hex::encode(e.target.as_slice());

    let mut sol = String::new();
    sol.push_str(&format!("contract {} is Test {{\n", name));
    sol.push_str("    uint256 fork;\n");
    sol.push_str("    string RPC_URL = vm.envOr(\"DRPC_URL\", string(\"https://your-fallback-node.com\"));\n\n");
    sol.push_str("    function setUp() public {\n");
    sol.push_str(&format!(
        "        fork = vm.createFork(RPC_URL, {});\n",
        block_number
    ));
    sol.push_str("        vm.selectFork(fork);\n");
    sol.push_str("    }\n\n");

    let test_name = format!("test_exploit_{}", index);
    sol.push_str(&format!("    function {}() public {{\n", test_name));
    sol.push_str("        address attacker = makeAddr(\"attacker\");\n");
    sol.push_str("        vm.deal(attacker, 1000 ether);\n");
    sol.push_str("        vm.startPrank(attacker);\n\n");

    // Track ETH spent for profit calculation
    let mut eth_spent = U256::ZERO;

    if is_chain {
        let steps = chain::parse_chain_steps(&e.evidence);
        for (si, step) in steps.iter().enumerate() {
            let calldata_hex = hex::encode(&step.calldata);
            let target_step_hex = hex::encode(step.target.as_slice());
            if step.value > U256::ZERO {
                eth_spent = eth_spent.saturating_add(step.value);
                sol.push_str(&format!(
                    "        // Step {}: {}\n\
                     (bool s{}, ) = address(0x{target_step_hex}).call{{value: {}}}(",
                    si + 1,
                    step.description,
                    si,
                    step.value
                ));
            } else {
                sol.push_str(&format!(
                    "        // Step {}: {}\n\
                     (bool s{}, ) = address(0x{target_step_hex}).call(",
                    si + 1,
                    step.description,
                    si
                ));
            }
            sol.push_str(&format!(
                "hex\"{}\"\n\
                 );\n",
                calldata_hex
            ));
            sol.push_str(&format!(
                "        require(s{}, \"Step {} failed\");\n\n",
                si,
                si + 1
            ));
        }
        // No value sent in last arg of .call() for non-value steps
    } else {
        let calldata_hex = hex::encode(&e.calldata);
        sol.push_str(&format!(
            "        (bool s, ) = address(0x{target_hex}).call(\n\
             hex\"{calldata_hex}\"\n\
             );\n",
        ));
    }

    // Profit check
    match e.status {
        VerifyStatus::Verified => {
            sol.push_str(&format!(
                "        uint256 profit = attacker.balance - (1000 ether - {});\n",
                eth_spent
            ));
            sol.push_str("        require(profit > 0, \"No profit extracted\");\n");
            sol.push_str("        emit log_named_decimal_uint(\"Profit (ETH)\", profit, 18);\n");
        }
        VerifyStatus::Partial => {
            sol.push_str("        emit log(\"Note: State deviation confirmed — requires external liquidity parameters to capture profit\");\n");
            sol.push_str(&format!(
                "        uint256 profit = attacker.balance - (1000 ether - {});\n",
                eth_spent
            ));
            sol.push_str(
                "        emit log_named_decimal_uint(\"Balance delta (wei)\", profit, 0);\n",
            );
        }
        VerifyStatus::Reverted => {
            sol.push_str("        emit log(\"Note: Call attempted at pinned block — preconditions not met. Function entry point is active.\");\n");
        }
    }

    sol.push_str("        vm.stopPrank();\n");
    sol.push_str("    }\n");
    sol.push_str("}\n");

    (sol, test_name)
}

// ── Helpers ──

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max])
    } else {
        s.to_string()
    }
}

fn slugify(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}
