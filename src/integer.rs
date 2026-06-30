//! Integer number-theory helpers (prime factorization, square-factor extraction).

use crate::exact::ExactNum;
use crate::node::Node;

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
    use super::{extract_square_factors, prime_factorize, prime_factorize_latex};

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
