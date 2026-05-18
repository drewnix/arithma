use num_rational::BigRational;
use num_traits::One;

use crate::mod_poly::factor_over_q;
use crate::polynomial::Polynomial;

/// A single term in a partial fraction decomposition.
///
/// Represents numerator(x) / denominator(x)^power,
/// where deg(numerator) < deg(denominator) and denominator is irreducible.
#[derive(Debug, Clone)]
pub struct PartialFractionTerm {
    pub numerator: Polynomial,
    pub denominator: Polynomial,
    pub power: usize,
}

/// Result of partial fraction decomposition of P(x)/Q(x).
///
/// polynomial_part + Σ terms[i].numerator / terms[i].denominator^terms[i].power
#[derive(Debug, Clone)]
pub struct PartialFractionDecomposition {
    pub polynomial_part: Polynomial,
    pub terms: Vec<PartialFractionTerm>,
}

/// Decompose P(x)/Q(x) into partial fractions.
///
/// Returns the polynomial part (from long division if deg(P) ≥ deg(Q))
/// plus a sum of terms N_i(x) / q_i(x)^k where each q_i is irreducible
/// over Q and deg(N_i) < deg(q_i).
pub fn partial_fraction_decomposition(
    numerator: &Polynomial,
    denominator: &Polynomial,
) -> Result<PartialFractionDecomposition, String> {
    if denominator.is_zero() {
        return Err("Division by zero polynomial".to_string());
    }

    let var = numerator.variable().to_string();

    // Step 1: Long division if deg(P) ≥ deg(Q)
    let (poly_part, remainder) = if numerator.degree().unwrap_or(0) >= denominator.degree().unwrap()
    {
        let (q, r) = numerator.div_rem(denominator)?;
        (q, r)
    } else {
        (Polynomial::zero(&var), numerator.clone())
    };

    if remainder.is_zero() {
        return Ok(PartialFractionDecomposition {
            polynomial_part: poly_part,
            terms: vec![],
        });
    }

    // Step 2: Factor the denominator
    let (content, irr_factors) = factor_over_q(denominator);

    // Group factors with multiplicities
    let factors_with_mult = group_factors(&irr_factors);

    // Adjust remainder by content: P/Q = P/(c·∏q_i^m_i) = (P/c) / ∏q_i^m_i
    let inv_content = BigRational::one() / &content;
    let adjusted_rem = remainder.scalar_mul(&inv_content);

    // Step 3: Decompose
    let terms = decompose_coprime(&adjusted_rem, &factors_with_mult, &var)?;

    Ok(PartialFractionDecomposition {
        polynomial_part: poly_part,
        terms,
    })
}

/// Group a list of factors (possibly repeated) into (factor, multiplicity) pairs.
fn group_factors(factors: &[Polynomial]) -> Vec<(Polynomial, usize)> {
    let mut result: Vec<(Polynomial, usize)> = Vec::new();
    for f in factors {
        let found = result.iter_mut().find(|(existing, _)| {
            // Compare by checking if they're proportional (both monic, so equal)
            existing == f
        });
        if let Some((_, count)) = found {
            *count += 1;
        } else {
            result.push((f.clone(), 1));
        }
    }
    result
}

/// Decompose P(x) / ∏ q_i(x)^{m_i} where the q_i are pairwise coprime.
///
/// Uses the extended GCD to split into separate denominators, then
/// decomposes each power of an irreducible factor by repeated division.
fn decompose_coprime(
    p: &Polynomial,
    factors: &[(Polynomial, usize)],
    var: &str,
) -> Result<Vec<PartialFractionTerm>, String> {
    if factors.is_empty() {
        return Ok(vec![]);
    }

    if factors.len() == 1 {
        let (q, m) = &factors[0];
        return decompose_single_power(p, q, *m, var);
    }

    // Split: A = q_0^{m_0}, B = ∏_{i>0} q_i^{m_i}
    let (q0, m0) = &factors[0];
    let a = poly_power(q0, *m0, var);
    let mut b = Polynomial::one(var);
    for (qi, mi) in &factors[1..] {
        let qi_pow = poly_power(qi, *mi, var);
        b = &b * &qi_pow;
    }

    // Extended GCD: s·A + t·B = 1 (since A, B are coprime)
    let (g, s, t) = Polynomial::extended_gcd(&a, &b);
    if !g.is_constant() {
        return Err("Factors are not coprime".to_string());
    }
    // Normalize: if gcd is a constant c, divide s and t by c
    let g_val = g.coeff(0);
    let g_inv = BigRational::one() / g_val;
    let s = s.scalar_mul(&g_inv);
    let t = t.scalar_mul(&g_inv);

    // P/(A·B) = P·t/A + P·s/B
    let p_t = &(p * &t);
    let p_s = &(p * &s);

    // Reduce: deg(P·t) might be ≥ deg(A)
    let (qt, remainder_a) = p_t.div_rem(&a)?;
    let (qs, remainder_b) = p_s.div_rem(&b)?;

    // The polynomial parts should cancel: qt + qs = 0
    // (since deg(P) < deg(A·B))
    // But due to rounding, let's just use the remainders
    let _ = qt;
    let _ = qs;

    // Recurse
    let mut terms = decompose_coprime(&remainder_a, &[(q0.clone(), *m0)], var)?;
    let mut terms_b = decompose_coprime(&remainder_b, &factors[1..], var)?;
    terms.append(&mut terms_b);

    Ok(terms)
}

/// Decompose P(x) / q(x)^m where q is irreducible and deg(P) < m·deg(q).
///
/// Produces terms N_k / q^k for k = 1, ..., m, where deg(N_k) < deg(q).
/// Uses repeated polynomial division: at each step, divide the current
/// numerator by q to get a quotient (for higher powers) and remainder
/// (the numerator at the current power).
fn decompose_single_power(
    p: &Polynomial,
    q: &Polynomial,
    m: usize,
    _var: &str,
) -> Result<Vec<PartialFractionTerm>, String> {
    if m == 0 {
        return Ok(vec![]);
    }

    let mut terms = Vec::new();
    let mut current = p.clone();

    // Repeated division by q, collecting remainders from power m down to 1
    for k in (1..=m).rev() {
        let (quotient, remainder) = current.div_rem(q)?;
        if !remainder.is_zero() {
            terms.push(PartialFractionTerm {
                numerator: remainder,
                denominator: q.clone(),
                power: k,
            });
        }
        current = quotient;
    }

    Ok(terms)
}

/// Compute q^n as a Polynomial.
fn poly_power(q: &Polynomial, n: usize, var: &str) -> Polynomial {
    if n == 0 {
        return Polynomial::one(var);
    }
    let mut result = q.clone();
    for _ in 1..n {
        result = &result * q;
    }
    result
}

impl PartialFractionDecomposition {
    /// Convert to a Node AST for display and integration.
    pub fn to_node(&self) -> crate::node::Node {
        use crate::exact::ExactNum;
        use crate::node::Node;

        let mut result: Option<Node> = None;

        // Add polynomial part
        if !self.polynomial_part.is_zero() {
            result = Some(self.polynomial_part.to_node());
        }

        // Add each partial fraction term
        for term in &self.terms {
            let num_node = term.numerator.to_node();
            let den_node = if term.power == 1 {
                term.denominator.to_node()
            } else {
                Node::Power(
                    Box::new(term.denominator.to_node()),
                    Box::new(Node::Num(ExactNum::integer(term.power as i64))),
                )
            };
            let frac = Node::Divide(Box::new(num_node), Box::new(den_node));

            result = Some(match result {
                None => frac,
                Some(existing) => Node::Add(Box::new(existing), Box::new(frac)),
            });
        }

        result.unwrap_or(Node::Num(ExactNum::zero()))
    }
}

impl std::fmt::Display for PartialFractionDecomposition {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut first = true;

        if !self.polynomial_part.is_zero() {
            write!(f, "{}", self.polynomial_part)?;
            first = false;
        }

        for term in &self.terms {
            if !first {
                write!(f, " + ")?;
            }
            write!(f, "({})/({})", term.numerator, term.denominator)?;
            if term.power > 1 {
                write!(f, "^{}", term.power)?;
            }
            first = false;
        }

        if first {
            write!(f, "0")?;
        }

        Ok(())
    }
}

/// Decompose a LaTeX rational expression into partial fractions.
pub fn partial_fractions_latex(
    numerator_latex: &str,
    denominator_latex: &str,
    var: &str,
) -> Result<String, String> {
    use crate::parser::build_expression_tree;
    use crate::tokenizer::Tokenizer;

    let num_expr = {
        let mut tok = Tokenizer::new(numerator_latex);
        build_expression_tree(tok.tokenize())?
    };
    let den_expr = {
        let mut tok = Tokenizer::new(denominator_latex);
        build_expression_tree(tok.tokenize())?
    };

    let num_poly = Polynomial::from_node(&num_expr, var)?;
    let den_poly = Polynomial::from_node(&den_expr, var)?;

    let decomp = partial_fraction_decomposition(&num_poly, &den_poly)?;
    let node = decomp.to_node();

    let env = crate::environment::Environment::new();
    let simplified =
        crate::simplify::Simplifiable::simplify(&node, &env).unwrap_or(node);
    Ok(format!("{}", simplified))
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigInt;

    fn int(n: i64) -> BigRational {
        BigRational::from_integer(BigInt::from(n))
    }

    fn poly(coeffs: &[i64], var: &str) -> Polynomial {
        Polynomial::from_coeffs(coeffs.iter().map(|&c| int(c)).collect(), var)
    }

    fn verify_decomposition(num: &Polynomial, den: &Polynomial) {
        let decomp = partial_fraction_decomposition(num, den).unwrap();
        let var = num.variable();

        // Reconstruct: polynomial_part + Σ N_i/q_i^k_i
        // Multiply everything by den and check we get num back
        let mut reconstructed = &decomp.polynomial_part * den;

        for term in &decomp.terms {
            // Cofactor = den / (q^k)
            let q_pow = poly_power(&term.denominator, term.power, var);
            let (cofactor, rem) = den.div_rem(&q_pow).unwrap();
            assert!(
                rem.is_zero(),
                "Denominator not divisible by factor^power"
            );
            let contribution = &term.numerator * &cofactor;
            reconstructed = &reconstructed + &contribution;
        }

        // reconstructed should equal num
        let diff = num - &reconstructed;
        assert!(
            diff.is_zero(),
            "Partial fraction reconstruction failed for ({}) / ({}):\n  decomp = {}\n  diff = {}",
            num,
            den,
            decomp,
            diff
        );

        // Verify each numerator has degree < denominator
        for term in &decomp.terms {
            let num_deg = term.numerator.degree().unwrap_or(0);
            let den_deg = term.denominator.degree().unwrap();
            assert!(
                num_deg < den_deg,
                "Numerator degree {} >= denominator degree {}",
                num_deg,
                den_deg
            );
        }
    }

    #[test]
    fn test_simple_linear_factors() {
        // 1 / ((x-1)(x+1)) = 1/(x²-1)
        // Should decompose to A/(x-1) + B/(x+1)
        let num = poly(&[1], "x");
        let den = poly(&[-1, 0, 1], "x"); // x²-1
        let decomp = partial_fraction_decomposition(&num, &den).unwrap();
        assert!(decomp.polynomial_part.is_zero());
        assert_eq!(decomp.terms.len(), 2);
        verify_decomposition(&num, &den);
    }

    #[test]
    fn test_repeated_linear_factor() {
        // 1 / (x-1)² = 1/(x²-2x+1)
        let num = poly(&[1], "x");
        let den = poly(&[1, -2, 1], "x"); // x²-2x+1 = (x-1)²
        let decomp = partial_fraction_decomposition(&num, &den).unwrap();
        assert!(decomp.polynomial_part.is_zero());
        verify_decomposition(&num, &den);
    }

    #[test]
    fn test_with_polynomial_part() {
        // (x³+1) / (x²-1) = x + (x+1)/(x²-1) = x + 1/(x-1)
        let num = poly(&[1, 0, 0, 1], "x"); // x³+1
        let den = poly(&[-1, 0, 1], "x"); // x²-1
        let decomp = partial_fraction_decomposition(&num, &den).unwrap();
        assert!(!decomp.polynomial_part.is_zero());
        verify_decomposition(&num, &den);
    }

    #[test]
    fn test_irreducible_quadratic() {
        // 1 / ((x-1)(x²+1))
        let num = poly(&[1], "x");
        let den = poly(&[-1, -1, 1, 1], "x"); // (x-1)(x²+1) = x³-x²+x-1
        let decomp = partial_fraction_decomposition(&num, &den).unwrap();
        assert!(decomp.polynomial_part.is_zero());
        verify_decomposition(&num, &den);
    }

    #[test]
    fn test_three_linear_factors() {
        // 1 / ((x-1)(x-2)(x-3)) = 1/(x³-6x²+11x-6)
        let num = poly(&[1], "x");
        let den = poly(&[-6, 11, -6, 1], "x");
        let decomp = partial_fraction_decomposition(&num, &den).unwrap();
        assert!(decomp.polynomial_part.is_zero());
        assert_eq!(decomp.terms.len(), 3);
        verify_decomposition(&num, &den);
    }

    #[test]
    fn test_x_over_x2_minus_1() {
        // x / (x²-1) = A/(x-1) + B/(x+1)
        // x = A(x+1) + B(x-1); x=1: 1=2A, A=1/2; x=-1: -1=-2B, B=1/2
        let num = poly(&[0, 1], "x"); // x
        let den = poly(&[-1, 0, 1], "x"); // x²-1
        let decomp = partial_fraction_decomposition(&num, &den).unwrap();
        verify_decomposition(&num, &den);
    }

    #[test]
    fn test_constant_numerator_cubic() {
        // 1 / (x³-1) = 1/((x-1)(x²+x+1))
        let num = poly(&[1], "x");
        let den = poly(&[-1, 0, 0, 1], "x"); // x³-1
        let decomp = partial_fraction_decomposition(&num, &den).unwrap();
        verify_decomposition(&num, &den);
    }

    #[test]
    fn test_zero_numerator() {
        let num = poly(&[0], "x");
        let den = poly(&[-1, 0, 1], "x");
        let decomp = partial_fraction_decomposition(&num, &den).unwrap();
        assert!(decomp.polynomial_part.is_zero());
        assert!(decomp.terms.is_empty());
    }

    #[test]
    fn test_latex_interface() {
        let result = partial_fractions_latex("1", "x^2 - 1", "x");
        assert!(result.is_ok());
    }
}
