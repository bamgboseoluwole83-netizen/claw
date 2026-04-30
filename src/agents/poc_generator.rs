use alloy_primitives::U256;
use std::fs::File;
use std::io::Write;
use tracing::info;

#[derive(Debug)]
pub struct Finding {
    pub bug_class: String,
    pub target: String,
    pub calldata: String,
    pub profit: U256,
    pub proof: String,
}

pub fn write_poc(findings: &[Finding]) {
    if findings.is_empty() {
        info!("✅ No findings found.");
        return;
    }

    let mut file = File::create("poc.md").expect("Unable to create file");
    for (i, finding) in findings.iter().enumerate() {
        let report = format!(
            "# PoC Report {}: {}\n\n- **Target:** `{}`\n- **Profit:** {} (wei)\n- **Calldata:** `0x{}`\n\n**Proof:**\n```\n{}
```\n\n---\n\n",
            i + 1,
            finding.bug_class,
            finding.target,
            finding.profit,
            finding.calldata,
            finding.proof,
        );
        file.write_all(report.as_bytes()).expect("Unable to write data");
    }
    info!("📝 PoC report generated at poc.md");
}
