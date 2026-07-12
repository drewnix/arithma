//! Integer number-theory helpers (GCD/LCM, factorial, prime factorization, square-factor extraction).
//!
//! All public APIs take and return [`ExactNum`]. Internally, integer algorithms run on
//! [`BigInt`] end-to-end — there is no `usize`/`i64` cap on inputs for [`factorial`],
//! [`binom`], [`gcd`], [`lcm`], or [`prime_factorize`].

use crate::exact::ExactNum;
use crate::node::Node;
use num_bigint::BigInt;
use num_integer::Integer;
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};

fn bigint_to_exact(n: BigInt) -> ExactNum {
    ExactNum::Rational(BigRational::from_integer(n))
}

/// Convert a whole non-negative `f64` to `BigInt` when exactly representable.
///
/// `f64` only guarantees exact integers up to 2^53; larger whole floats are rejected
/// rather than cast through `i64` (which can silently truncate).
fn float_to_non_negative_bigint(f: f64) -> Option<BigInt> {
    if !f.is_finite() || f < 0.0 || f.fract() != 0.0 {
        return None;
    }
    const MAX_EXACT_F64_INT: f64 = (1u64 << 53) as f64;
    if f > MAX_EXACT_F64_INT {
        return None;
    }
    Some(BigInt::from(f as i64))
}

/// Non-negative integer as `BigInt`, when `n` is an exact whole number ≥ 0.
pub fn as_non_negative_integer(n: &ExactNum) -> Option<BigInt> {
    match n {
        ExactNum::Rational(r) if r.is_integer() && !r.is_negative() => Some(r.numer().clone()),
        ExactNum::Float(f) => float_to_non_negative_bigint(*f),
        _ => None,
    }
}

/// Parse a non-negative integer string into `ExactNum`.
pub fn parse_non_negative_integer(s: &str) -> Option<ExactNum> {
    let n = s.trim().parse::<BigInt>().ok()?;
    if n.is_negative() {
        return None;
    }
    Some(bigint_to_exact(n))
}

// GCD / LCM

/// Greatest common divisor of two non-negative integer `ExactNum`s.
pub fn gcd(a: &ExactNum, b: &ExactNum) -> Option<ExactNum> {
    let a = as_non_negative_integer(a)?;
    let b = as_non_negative_integer(b)?;
    Some(bigint_to_exact(a.gcd(&b)))
}

/// Least common multiple of two non-negative integer `ExactNum`s.
pub fn lcm(a: &ExactNum, b: &ExactNum) -> Option<ExactNum> {
    let a = as_non_negative_integer(a)?;
    let b = as_non_negative_integer(b)?;
    if a.is_zero() || b.is_zero() {
        return Some(ExactNum::integer(0));
    }
    let g = a.gcd(&b);
    Some(bigint_to_exact(&a / &g * &b))
}

// Factorial

fn factorial_bigint(n: &BigInt) -> BigInt {
    if n.is_zero() {
        return BigInt::one();
    }
    let mut result = BigInt::one();
    let mut i = BigInt::from(2u32);
    while i <= *n {
        result *= &i;
        i += 1;
    }
    result
}

/// Exact factorial `n!` for non-negative integer `n` in an [`ExactNum`].
pub fn factorial(n: &ExactNum) -> Option<ExactNum> {
    let n = as_non_negative_integer(n)?;
    Some(bigint_to_exact(factorial_bigint(&n)))
}

// Binomial coefficient

fn binomial_bigint(n: &BigInt, k: &BigInt) -> BigInt {
    if k > n {
        return BigInt::zero();
    }
    if k.is_zero() {
        return BigInt::one();
    }
    let k = k.clone().min(n - k);
    let mut result = BigInt::one();
    let mut i = BigInt::zero();
    while i < k {
        result = &result * (n - &i);
        result /= &i + 1;
        i += 1;
    }
    result
}

/// Exact binomial coefficient C(n, k); returns `0` when `k > n`.
pub fn binom(n: &ExactNum, k: &ExactNum) -> Option<ExactNum> {
    let n = as_non_negative_integer(n)?;
    let k = as_non_negative_integer(k)?;
    Some(bigint_to_exact(binomial_bigint(&n, &k)))
}

// Prime factorization

/// Maximum trial divisor tried by [`prime_factorize`]. Inputs whose smallest
/// prime factor exceeds this bound (or whose unfactored cofactor remains after
/// the budget is spent) return `None` rather than a partial factorization.
const PRIME_FACTORIZE_TRIAL_BUDGET: u64 = 1_000_000;

fn prime_factorize_bigint(mut n: BigInt) -> Option<Vec<(BigInt, u32)>> {
    if n <= BigInt::one() {
        return Some(Vec::new());
    }

    let mut factors = Vec::new();
    let mut d = BigInt::from(2u32);
    let zero = BigInt::zero();
    let one = BigInt::one();
    let budget = BigInt::from(PRIME_FACTORIZE_TRIAL_BUDGET);

    loop {
        if &d * &d > n {
            break;
        }
        if d > budget {
            return None;
        }
        let (_, r) = n.div_rem(&d);
        if r == zero {
            let mut exp = 0u32;
            loop {
                let (q, rem) = n.div_rem(&d);
                if rem != zero {
                    break;
                }
                n = q;
                exp += 1;
            }
            factors.push((d.clone(), exp));
        }
        d += &one;
    }
    if n > one {
        factors.push((n, 1));
    }
    Some(factors)
}

/// Prime-factorize `n` into `(prime, exponent)` pairs with `n = ∏ p^e`.
///
/// Returns `None` when `n` is not a non-negative integer, or when trial division
/// exceeds [`PRIME_FACTORIZE_TRIAL_BUDGET`]. Returns an empty vector for `n <= 1`.
pub fn prime_factorize(n: &ExactNum) -> Option<Vec<(ExactNum, u32)>> {
    let n = as_non_negative_integer(n)?;
    prime_factorize_bigint(n).map(|factors| {
        factors
            .into_iter()
            .map(|(p, e)| (bigint_to_exact(p), e))
            .collect()
    })
}

/// Extract square factors so that `√n = outside · √inside` with `inside` square-free.
pub fn extract_square_factors(n: &ExactNum) -> Option<(ExactNum, ExactNum)> {
    let n = as_non_negative_integer(n)?;
    if n.is_zero() {
        return Some((ExactNum::integer(0), ExactNum::integer(0)));
    }

    let mut outside = BigInt::one();
    let mut inside = BigInt::one();
    for (p, e) in prime_factorize_bigint(n)? {
        outside *= p.pow(e / 2);
        if e % 2 == 1 {
            inside *= p;
        }
    }
    Some((bigint_to_exact(outside), bigint_to_exact(inside)))
}

fn prime_factor_term(prime: &ExactNum, exponent: u32) -> Node {
    let base = Node::Num(prime.clone());
    if exponent == 1 {
        base
    } else {
        Node::Power(
            Box::new(base),
            Box::new(Node::Num(ExactNum::from_usize(exponent as usize))),
        )
    }
}

/// Prime-factorize `n` and format as LaTeX (e.g. `2^{4} \cdot 3^{2} \cdot 5`).
pub fn prime_factorize_latex(n: &ExactNum) -> Result<String, String> {
    let factors = prime_factorize(n).ok_or_else(|| {
        "unable to compute prime factorization (trial budget exceeded)".to_string()
    })?;
    if factors.is_empty() {
        return Ok(format!("{n}"));
    }
    Ok(factors
        .iter()
        .map(|(prime, exponent)| format!("{}", prime_factor_term(prime, *exponent)))
        .collect::<Vec<_>>()
        .join(" \\cdot "))
}

#[cfg(test)]
mod tests {
    use super::{
        as_non_negative_integer, bigint_to_exact, binom, extract_square_factors, factorial, gcd,
        lcm, parse_non_negative_integer, prime_factorize, prime_factorize_latex,
    };
    use crate::ExactNum;

    #[test]
    fn test_as_non_negative_integer() {
        assert_eq!(
            as_non_negative_integer(&ExactNum::integer(12)),
            Some(num_bigint::BigInt::from(12))
        );
        assert_eq!(as_non_negative_integer(&ExactNum::integer(-1)), None);
        assert_eq!(as_non_negative_integer(&ExactNum::rational(1, 2)), None);
        assert_eq!(as_non_negative_integer(&ExactNum::Float(1.5)), None);
        assert_eq!(
            as_non_negative_integer(&ExactNum::Float(9007199254740992.0)),
            Some(num_bigint::BigInt::from(9007199254740992u64))
        );
        // Whole float above 2^53 is not a trusted exact integer source.
        assert_eq!(
            as_non_negative_integer(&ExactNum::Float(10_000_000_000_000_000.0)),
            None
        );
    }

    #[test]
    fn test_gcd_bigint_beyond_i64() {
        let a = parse_non_negative_integer("10000000000000000000").unwrap();
        let b = parse_non_negative_integer("30000000000000000000").unwrap();
        assert_eq!(
            gcd(&a, &b),
            Some(parse_non_negative_integer("10000000000000000000").unwrap())
        );
    }

    #[test]
    fn test_lcm_bigint_beyond_i64() {
        let a = parse_non_negative_integer("10000000000000000000").unwrap();
        let b = parse_non_negative_integer("30000000000000000000").unwrap();
        assert_eq!(
            lcm(&a, &b),
            Some(parse_non_negative_integer("30000000000000000000").unwrap())
        );
    }

    #[test]
    fn test_gcd() {
        assert_eq!(
            gcd(&ExactNum::integer(24), &ExactNum::integer(36)),
            Some(ExactNum::integer(12))
        );
        assert_eq!(
            gcd(&ExactNum::integer(0), &ExactNum::integer(5)),
            Some(ExactNum::integer(5))
        );
        assert_eq!(
            gcd(&ExactNum::integer(17), &ExactNum::integer(13)),
            Some(ExactNum::integer(1))
        );
    }

    #[test]
    fn test_lcm() {
        assert_eq!(
            lcm(&ExactNum::integer(4), &ExactNum::integer(6)),
            Some(ExactNum::integer(12))
        );
        assert_eq!(
            lcm(&ExactNum::integer(12), &ExactNum::integer(18)),
            Some(ExactNum::integer(36))
        );
        assert_eq!(
            lcm(&ExactNum::integer(0), &ExactNum::integer(5)),
            Some(ExactNum::integer(0))
        );
    }

    #[test]
    fn test_factorial() {
        assert_eq!(factorial(&ExactNum::integer(0)), Some(ExactNum::integer(1)));
        assert_eq!(factorial(&ExactNum::integer(1)), Some(ExactNum::integer(1)));
        assert_eq!(
            factorial(&ExactNum::integer(5)),
            Some(ExactNum::integer(120))
        );
        assert_eq!(factorial(&ExactNum::integer(-1)), None);
        assert_eq!(factorial(&ExactNum::rational(1, 2)), None);

        let large = factorial(&ExactNum::integer(23)).unwrap();
        assert_eq!(
            large.to_rational().unwrap().numer().to_string(),
            "25852016738884976640000"
        );
    }

    #[test]
    fn test_binom() {
        assert_eq!(
            binom(&ExactNum::integer(0), &ExactNum::integer(0)),
            Some(ExactNum::integer(1))
        );
        assert_eq!(
            binom(&ExactNum::integer(5), &ExactNum::integer(2)),
            Some(ExactNum::integer(10))
        );
        assert_eq!(
            binom(&ExactNum::integer(5), &ExactNum::integer(0)),
            Some(ExactNum::integer(1))
        );
        assert_eq!(
            binom(&ExactNum::integer(3), &ExactNum::integer(5)),
            Some(ExactNum::integer(0))
        );
        assert_eq!(
            binom(&ExactNum::integer(10), &ExactNum::integer(5)),
            Some(ExactNum::integer(252))
        );

        let large = binom(&ExactNum::integer(68), &ExactNum::integer(34)).unwrap();
        assert!(large.is_integer());
        assert_eq!(
            large.to_rational().unwrap().numer().to_string(),
            "28453041475240576740"
        );
    }

    #[test]
    fn test_factorial_bigint_beyond_i64_index() {
        let n = parse_non_negative_integer("30").unwrap();
        let result = factorial(&n).unwrap();
        assert_eq!(
            result.to_rational().unwrap().numer().to_string(),
            "265252859812191058636308480000000"
        );
    }

    #[test]
    fn test_binom_bigint_beyond_i64_index() {
        let n = parse_non_negative_integer("1000").unwrap();
        let k = parse_non_negative_integer("500").unwrap();
        let result = binom(&n, &k).unwrap();
        assert!(result.is_integer());
        assert_eq!(
            result.to_rational().unwrap().numer().to_string(),
            "270288240945436569515614693625975275496152008446548287007392875106625428705522193898612483924502370165362606085021546104802209750050679917549894219699518475423665484263751733356162464079737887344364574161119497604571044985756287880514600994219426752366915856603136862602484428109296905863799821216320"
        );
    }

    #[test]
    fn test_binom_bigint_large_n_small_k() {
        let n = parse_non_negative_integer("1000000").unwrap();
        let k = parse_non_negative_integer("3").unwrap();
        let result = binom(&n, &k).unwrap();
        assert_eq!(
            result.to_rational().unwrap().numer().to_string(),
            "166666166667000000"
        );
    }

    #[test]
    fn test_prime_factorize_small() {
        assert_eq!(prime_factorize(&ExactNum::integer(0)), Some(vec![]));
        assert_eq!(prime_factorize(&ExactNum::integer(1)), Some(vec![]));
        assert_eq!(
            prime_factorize(&ExactNum::integer(12)),
            Some(vec![(ExactNum::integer(2), 2), (ExactNum::integer(3), 1)])
        );
        assert_eq!(
            prime_factorize(&ExactNum::integer(720)),
            Some(vec![
                (ExactNum::integer(2), 4),
                (ExactNum::integer(3), 2),
                (ExactNum::integer(5), 1),
            ])
        );
    }

    #[test]
    fn test_prime_factorize_prime() {
        assert_eq!(
            prime_factorize(&ExactNum::integer(65537)),
            Some(vec![(ExactNum::integer(65537), 1)])
        );
    }

    #[test]
    fn test_prime_factorize_budget_exceeded() {
        // Product of two large ~10^16 factors: the remaining cofactor needs a
        // divisor beyond the 10^6 trial budget, so factorization gives up.
        let p = as_non_negative_integer(&parse_non_negative_integer("10000000000000003").unwrap())
            .unwrap();
        let q = as_non_negative_integer(&parse_non_negative_integer("10000000000000033").unwrap())
            .unwrap();
        let n = bigint_to_exact(p * q);
        assert_eq!(prime_factorize(&n), None);
    }

    #[test]
    fn test_prime_factorize_latex() {
        assert_eq!(
            prime_factorize_latex(&ExactNum::integer(720)).unwrap(),
            "2^{4} \\cdot 3^{2} \\cdot 5"
        );
        assert_eq!(
            prime_factorize_latex(&ExactNum::integer(12)).unwrap(),
            "2^{2} \\cdot 3"
        );
        assert_eq!(prime_factorize_latex(&ExactNum::integer(7)).unwrap(), "7");
        assert_eq!(prime_factorize_latex(&ExactNum::integer(1)).unwrap(), "1");
    }

    #[test]
    fn test_extract_square_factors() {
        assert_eq!(
            extract_square_factors(&ExactNum::integer(12)),
            Some((ExactNum::integer(2), ExactNum::integer(3)))
        );
        assert_eq!(
            extract_square_factors(&ExactNum::integer(8)),
            Some((ExactNum::integer(2), ExactNum::integer(2)))
        );
        assert_eq!(
            extract_square_factors(&ExactNum::integer(7)),
            Some((ExactNum::integer(1), ExactNum::integer(7)))
        );
        assert_eq!(
            extract_square_factors(&ExactNum::integer(0)),
            Some((ExactNum::integer(0), ExactNum::integer(0)))
        );
        assert_eq!(
            extract_square_factors(&ExactNum::integer(100)),
            Some((ExactNum::integer(10), ExactNum::integer(1)))
        );
    }
}
