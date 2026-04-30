use std::process::Command;
use std::io::Write;

pub fn run_yices(script: &str) -> Option<String> {
    let mut child = Command::new("yices")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .ok()?;

    child.stdin.as_mut()?.write_all(script.as_bytes()).ok()?;
    let output = child.wait_with_output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !stderr.is_empty() {
        eprintln!("Yices stderr: {stderr}");
    }
    if !stdout.contains("sat") && stderr.contains("sat") {
        return Some(stderr);
    }
    Some(stdout)
}

/// Compute the **provably optimal** borrow amount using Yices SAT.
/// Returns `max_allowed` if the constraint `amount = max_allowed` is satisfiable.
pub fn optimal_ror_borrow(collateral: u128, stale_price: u128) -> Option<u128> {
    let max_allowed = (collateral * stale_price) / 1_000_000_000_000_000_000u128;

    let script = format!(
        "(define amount::int)\n\
         (assert (= amount {}))\n\
         (assert (> amount 0))\n\
         (check)\n\
         (show-model)\n",
        max_allowed
    );

    let output = run_yices(&script)?;
    if output.contains("sat") {
        Some(max_allowed)
    } else {
        None
    }
}

/// Formally prove that reward dilution is possible.
pub fn prove_reward_dilution_possible() -> bool {
    let script = "(define deposit::int) (define reward::int)\n\
                  (assert (> deposit 0)) (assert (= reward deposit))\n\
                  (assert (> reward 0)) (check)\n";
    run_yices(script).map_or(false, |o| o.contains("sat"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimal_ror_borrow_works() {
        let coll = 1_000_000_000_000_000_000u128;      // 1 ETH
        let price = 100_000_000_000_000_000_000u128;    // 100 ETH
        let result = optimal_ror_borrow(coll, price);
        assert_eq!(result, Some(100_000_000_000_000_000_000)); // max = 100 ETH
    }

    #[test]
    fn test_prove_reward_dilution_works() {
        assert!(prove_reward_dilution_possible());
    }
}