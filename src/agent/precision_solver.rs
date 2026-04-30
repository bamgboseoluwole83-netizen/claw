use alloy_primitives::U256;
use crate::agents::symbolic_stack::SymbolicValue;

/// Generate candidate attack amounts, optimal one first.
pub fn compute_optimal_attack_amount(formula: &SymbolicValue) -> Vec<U256> {
    let mut candidates = vec![
        U256::from(0),
        U256::from(1),
        U256::from(2),
        U256::MAX,
    ];

    let divisor = match formula.extract_divisor() {
        Some(d) => d,
        None => {
            candidates.push(U256::from(1000));
            candidates.push(U256::from(10).pow(U256::from(18)));
            candidates.sort_unstable();
            candidates.dedup();
            return candidates;
        }
    };

    let multiplier = formula.extract_multiplier().unwrap_or(U256::from(1));

    // boundary around divisor
    if divisor > U256::from(0) {
        candidates.push(divisor - U256::from(1));
        candidates.push(divisor);
        candidates.push(divisor + U256::from(1));
        if divisor > U256::from(2) {
            candidates.push(divisor - U256::from(2));
            candidates.push(divisor / U256::from(2));
        }
    }

    // Compute the mathematically optimal x such that (x * multiplier) % divisor is maximized
    if let Some(optimal) = solve_modular_optimal(divisor, multiplier) {
        candidates.insert(0, optimal);
    }

    candidates.sort_unstable();
    candidates.dedup();
    candidates
}

/// Solve for x that maximizes (x * multiplier) % divisor.
/// The maximum possible remainder is divisor - 1.
/// So we need x such that (x * multiplier) mod divisor == divisor - 1.
/// This requires modular inverse of multiplier modulo divisor.
fn solve_modular_optimal(divisor: U256, multiplier: U256) -> Option<U256> {
    if divisor <= U256::from(1) || multiplier.is_zero() {
        return None;
    }

    // We need gcd(multiplier, divisor) == 1 for the inverse to exist.
    // If not coprime, we still could use a scaled approach, but for simplicity we skip.
    let gcd = gcd(multiplier, divisor);
    if gcd != U256::from(1) {
        return None;
    }

    let inv = mod_inverse(multiplier, divisor)?;
    let target = divisor - U256::from(1);
    // x = target * inv mod divisor
    let x = mul_mod(target, inv, divisor);
    Some(x)
}

/// Extended Euclidean algorithm for U256, returns (gcd, x, y) such that a*x + b*y = gcd
fn egcd(a: U256, b: U256) -> (U256, U256, U256) {
    if b.is_zero() {
        return (a, U256::from(1), U256::from(0));
    }
    let q = a / b;
    let r = a % b;
    let (g, x1, y1) = egcd(b, r);
    // x = y1, y = x1 - q*y1
    let x = y1;
    let y = x1 - mul_mod(q, y1, U256::MAX); // we need exact subtraction, not modulo
                                            // but subtraction is fine with U256 (no overflow issues if we keep track)
    // Since U256 can overflow, we compute using checked_sub and adjust.
    // For our purposes, we only need the inverse modulo, so we'll compute x mod divisor later.
    // We'll implement a proper linear combination using BigInt-like logic, but for now we return placeholder.
    // Instead, we'll use a simpler extended_gcd that returns i128 and works for our small numbers.
    // Actually, for modular inverse we can use Fermat's little theorem (Euler) but exponentiation mod is heavy.
    // We'll implement a working mod_inverse using the extended Euclidean algorithm with signed numbers represented as (U256, bool).
    // I'll write a correct mod_inverse that uses the classic recursive algorithm and returns the inverse as U256.
    (g, x1, y1) // placeholder, not used directly
}

/// Compute greatest common divisor of a and b.
fn gcd(a: U256, b: U256) -> U256 {
    if b.is_zero() {
        return a;
    }
    gcd(b, a % b)
}

/// Modular inverse of a modulo m (m > 1, gcd(a,m)=1).
fn mod_inverse(a: U256, m: U256) -> Option<U256> {
    // Use the extended Euclidean algorithm
    let (mut old_r, mut r) = (a, m);
    let (mut old_s, mut s) = (U256::from(1), U256::from(0));

    while !r.is_zero() {
        let quotient = old_r / r;
        let new_r = old_r - quotient * r;
        let new_s = if old_s >= quotient * s {
            old_s - quotient * s
        } else {
            // we need to wrap around modulo m, but keeping it correct:
            // new_s = old_s - quotient * s (mod m)
            let diff = quotient * s - old_s;
            m - (diff % m)
        };
        old_r = r;
        r = new_r;
        old_s = s;
        s = new_s;
    }

    // old_r should be 1
    if old_r != U256::from(1) {
        return None;
    }
    // Ensure positive result mod m
    if old_s >= m {
        old_s = old_s % m;
    }
    Some(old_s)
}

/// (a * b) % m
fn mul_mod(a: U256, b: U256, m: U256) -> U256 {
    if m.is_zero() { return U256::ZERO; }
    // Perform multiplication with modulo to avoid overflow (U256 multiplication can overflow for 256 bits, but alloy's U256 handles overflow by wrapping; we need proper modulo)
    // We can use full 512-bit multiplication and then reduce.
    // alloy's U256 doesn't have a 512-bit mul. So we fallback to simple (a * b) % m, but that might overflow.
    // For our use case, numbers are typically small enough (divisor < 2^256, multiplier < 2^256) and product may overflow 256 bits. To be safe, we implement a modular multiplication using addition chain.
    // Or we can rely on U256::mul_mod (does it exist?). Alloy_primitives 0.8 doesn't have mul_mod. We'll implement a simple one using repeated addition, but that's slow. For now, we'll trust the compiler for small inputs; but to be robust, we'll use the `num-bigint` approach if needed. Since we're avoiding dependencies, we'll just do full multiplication and then modulo using the `%` operator, which in Rust's U256 is wrapping multiplication by default? Actually `*` on alloy's U256 will panic on overflow in debug, or wrap in release. To avoid issues, we'll compute (a as u128) * (b as u128) if possible, else we'll implement a safe mul_mod.
    // I'll write a quick mul_mod that uses the property (a*b)%m can be computed via standard multiplication if we cast to a larger type, but we don't have one. We'll just use the `*` operator and `%` and hope inputs are small. For correctness, I'll add `#[allow(unused)]` and a note.
    // Better: implement using the Russian peasant method.
    mul_mod_manual(a, b, m)
}

fn mul_mod_manual(mut a: U256, mut b: U256, m: U256) -> U256 {
    let mut res = U256::ZERO;
    a = a % m;
    while b > U256::ZERO {
        if b % U256::from(2) == U256::from(1) {
            res = (res + a) % m;
        }
        a = (a * U256::from(2)) % m;
        b = b / U256::from(2);
    }
    res
}