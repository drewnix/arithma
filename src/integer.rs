//! Integer number-theory helpers (GCD/LCM, factorial, prime factorization, square-factor extraction).

use crate::exact::ExactNum;
use crate::node::Node;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::One;

// Integer arithmetic

/// Greatest common divisor of two non-negative integers (Euclidean algorithm).
pub fn gcd_u64(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }
    a
}

/// Least common multiple of two non-negative integers.
pub fn lcm_u64(a: u64, b: u64) -> u64 {
    if a == 0 || b == 0 {
        return 0;
    }
    a / gcd_u64(a, b) * b
}

/// Largest `n` with `n!` fitting in `u64` (20! = 2_432_902_008_176_640_000).
pub const MAX_FACTORIAL_U64: u64 = 20;

/// Factorial of a non-negative integer, returning an error if `n > MAX_FACTORIAL_U64`.
pub fn factorial_u64(n: u64) -> Result<u64, String> {
    if n > MAX_FACTORIAL_U64 {
        return Err(format!(
            "factorial overflow: {}! exceeds u64 range (max {}!)",
            n, MAX_FACTORIAL_U64
        ));
    }
    let mut result = 1u64;
    for i in 2..=n {
        result *= i;
    }
    Ok(result)
}

/// Exact factorial as `ExactNum` (uses rationals; supports Taylor series orders beyond 20).
pub fn factorial_exact(n: usize) -> ExactNum {
    let mut result = BigRational::one();
    for i in 2..=n {
        result *= BigRational::from_integer(BigInt::from(i));
    }
    ExactNum::Rational(result)
}

/// Prime-factorize `n` into `(prime, exponent)` pairs with `n = ∏ p^e`.
///
/// Returns an empty vector for `n <= 1`. Factors are sorted by increasing prime.
///
/// # Examples
///
/// ```
/// use arithma::prime_factorize;
///
/// assert_eq!(prime_factorize(1), vec![]);
/// assert_eq!(prime_factorize(12), vec![(2, 2), (3, 1)]);
/// assert_eq!(prime_factorize(720), vec![(2, 4), (3, 2), (5, 1)]);
/// ```
pub fn prime_factorize(n: u64) -> Vec<(u64, u32)> {
    if n <= 1 {
        return Vec::new();
    }
    let mut n = n;
    let mut factors = Vec::new();
    let mut d = 2u64;
    while d * d <= n {
        if n.is_multiple_of(d) {
            let mut exp = 0u32;
            while n.is_multiple_of(d) {
                n /= d;
                exp += 1;
            }
            factors.push((d, exp));
        }
        d += 1;
    }
    if n > 1 {
        factors.push((n, 1));
    }
    factors
}

/// Extract square factors from `n` so that `√n = outside · √inside` with `inside` square-free.
pub fn extract_square_factors(n: u64) -> (u64, u64) {
    if n == 0 {
        return (0, 0);
    }
    let mut outside = 1u64;
    let mut inside = 1u64;
    for (p, e) in prime_factorize(n) {
        outside *= p.pow(e / 2);
        if e % 2 == 1 {
            inside *= p;
        }
    }
    (outside, inside)
}

fn prime_factor_term(prime: u64, exponent: u32) -> Node {
    let base = Node::Num(ExactNum::integer(prime as i64));
    if exponent == 1 {
        base
    } else {
        Node::Power(
            Box::new(base),
            Box::new(Node::Num(ExactNum::integer(exponent as i64))),
        )
    }
}

/// Prime-factorize `n` and format the result as LaTeX (e.g. `2^{4} \cdot 3^{2} \cdot 5`).
pub fn prime_factorize_latex(n: u64) -> String {
    let factors = prime_factorize(n);
    if factors.is_empty() {
        return n.to_string();
    }
    factors
        .iter()
        .map(|&(prime, exponent)| format!("{}", prime_factor_term(prime, exponent)))
        .collect::<Vec<_>>()
        .join(" \\cdot ")
}

#[cfg(test)]
mod tests {
    use super::{
        extract_square_factors, factorial_exact, factorial_u64, gcd_u64, lcm_u64, prime_factorize,
        prime_factorize_latex, MAX_FACTORIAL_U64,
    };
    use crate::ExactNum;

    // Integer arithmetic

    #[test]
    fn test_gcd_u64() {
        assert_eq!(gcd_u64(24, 36), 12);
        assert_eq!(gcd_u64(0, 5), 5);
        assert_eq!(gcd_u64(17, 13), 1);
    }

    #[test]
    fn test_lcm_u64() {
        assert_eq!(lcm_u64(4, 6), 12);
        assert_eq!(lcm_u64(12, 18), 36);
        assert_eq!(lcm_u64(0, 5), 0);
    }

    #[test]
    fn test_factorial_u64() {
        assert_eq!(factorial_u64(0).unwrap(), 1);
        assert_eq!(factorial_u64(1).unwrap(), 1);
        assert_eq!(factorial_u64(5).unwrap(), 120);
        assert_eq!(
            factorial_u64(MAX_FACTORIAL_U64).unwrap(),
            2432902008176640000
        );
        assert!(factorial_u64(21).unwrap_err().contains("overflow"));
    }

    #[test]
    fn test_factorial_exact() {
        assert_eq!(factorial_exact(0), ExactNum::integer(1));
        assert_eq!(factorial_exact(1), ExactNum::integer(1));
        assert_eq!(factorial_exact(5), ExactNum::integer(120));
    }

    #[test]
    fn test_prime_factorize_small() {
        assert_eq!(prime_factorize(0), vec![]);
        assert_eq!(prime_factorize(1), vec![]);
        assert_eq!(prime_factorize(2), vec![(2, 1)]);
        assert_eq!(prime_factorize(9), vec![(3, 2)]);
        assert_eq!(prime_factorize(12), vec![(2, 2), (3, 1)]);
        assert_eq!(prime_factorize(64), vec![(2, 6)]);
        assert_eq!(prime_factorize(720), vec![(2, 4), (3, 2), (5, 1)]);
    }

    #[test]
    fn test_prime_factorize_prime() {
        assert_eq!(prime_factorize(65537), vec![(65537, 1)]);
    }

    #[test]
    fn test_prime_factorize_latex() {
        assert_eq!(prime_factorize_latex(720), "2^{4} \\cdot 3^{2} \\cdot 5");
        assert_eq!(prime_factorize_latex(12), "2^{2} \\cdot 3");
        assert_eq!(prime_factorize_latex(7), "7");
        assert_eq!(prime_factorize_latex(1), "1");
    }

    #[test]
    fn test_extract_square_factors() {
        assert_eq!(extract_square_factors(12), (2, 3));
        assert_eq!(extract_square_factors(8), (2, 2));
        assert_eq!(extract_square_factors(18), (3, 2));
        assert_eq!(extract_square_factors(7), (1, 7));
        assert_eq!(extract_square_factors(4), (2, 1));
        assert_eq!(extract_square_factors(1), (1, 1));
        assert_eq!(extract_square_factors(72), (6, 2));
        assert_eq!(extract_square_factors(100), (10, 1));
        assert_eq!(extract_square_factors(50), (5, 2));
        assert_eq!(extract_square_factors(0), (0, 0));
    }
}
