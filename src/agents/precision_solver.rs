use alloy_primitives::U256;
use crate::agents::symbolic_stack::SymbolicValue;

pub fn compute_optimal_attack_amount(formula: &SymbolicValue) -> Vec<U256> {
    let mut candidates = vec![U256::from(0), U256::from(1), U256::from(2), U256::MAX];
    let divisor = match formula.extract_divisor() {
        Some(d) => d,
        None => { candidates.push(U256::from(1000)); return candidates; }
    };
    let multiplier = formula.extract_multiplier().unwrap_or(U256::from(1));
    if divisor > U256::from(0) {
        candidates.push(divisor - U256::from(1));
        candidates.push(divisor);
        candidates.push(divisor + U256::from(1));
    }
    if let Some(optimal) = solve_modular_optimal(divisor, multiplier) { candidates.insert(0, optimal); }
    candidates
}

fn solve_modular_optimal(divisor: U256, multiplier: U256) -> Option<U256> {
    if divisor <= U256::from(1) || multiplier.is_zero() { return None; }
    let inv = mod_inverse(multiplier, divisor)?;
    let target = divisor - U256::from(1);
    Some((target * inv) % divisor)
}

fn gcd(a: U256, b: U256) -> U256 { if b.is_zero() { a } else { gcd(b, a % b) } }

fn mod_inverse(a: U256, m: U256) -> Option<U256> {
    let (mut old_r, mut r) = (a % m, m);
    let (mut old_s, mut s) = (U256::from(1), U256::from(0));
    while !r.is_zero() {
        let quotient = old_r / r;
        let new_r = old_r - quotient * r;
        let new_s = if old_s >= quotient * s { old_s - quotient * s } else { m - ((quotient * s - old_s) % m) };
        old_r = r; r = new_r;
        old_s = s; s = new_s;
    }
    if old_r != U256::from(1) { None } else { Some(old_s % m) }
}
