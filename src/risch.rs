use crate::ext_poly::ExtPoly;
use crate::node::Node;
use crate::polynomial::Polynomial;
use crate::rational_function::RationalFunction;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};

/// Type of transcendental extension.
#[derive(Debug, Clone)]
pub enum ExtensionType {
    /// θ = log(f), θ' = f'/f
    Logarithmic,
    /// θ = exp(f), θ' = f' · θ
    Exponential,
}

/// A single-level differential extension of Q(x).
/// Represents Q(x, θ) where θ = exp(f) or θ = log(f) for f ∈ Q(x).
#[derive(Debug, Clone)]
pub struct DifferentialExtension {
    ext_type: ExtensionType,
    argument: RationalFunction, // f(x) — the argument to exp or log
    var: String,                // base variable (e.g., "x")
}

impl DifferentialExtension {
    /// Create a logarithmic extension: θ = log(f).
    pub fn logarithmic(argument: RationalFunction, var: &str) -> Self {
        DifferentialExtension {
            ext_type: ExtensionType::Logarithmic,
            argument,
            var: var.to_string(),
        }
    }

    /// Create an exponential extension: θ = exp(f).
    pub fn exponential(argument: RationalFunction, var: &str) -> Self {
        DifferentialExtension {
            ext_type: ExtensionType::Exponential,
            argument,
            var: var.to_string(),
        }
    }

    /// The type of this extension (logarithmic or exponential).
    pub fn ext_type(&self) -> &ExtensionType {
        &self.ext_type
    }

    /// The argument f(x) to exp or log.
    pub fn argument(&self) -> &RationalFunction {
        &self.argument
    }

    /// The base variable name.
    pub fn variable(&self) -> &str {
        &self.var
    }

    /// Compute d/dx of an ExtPoly element in this extension.
    ///
    /// For an element P(θ) = Σ aᵢ(x) θⁱ:
    ///
    /// **Logarithmic** (θ = log(f), θ' = f'/f):
    ///   d/dx[Σ aᵢ θⁱ] = Σ [aᵢ' θⁱ + i · aᵢ · (f'/f) · θⁱ⁻¹]
    ///
    /// **Exponential** (θ = exp(f), θ' = f'·θ):
    ///   d/dx[Σ aᵢ θⁱ] = Σ [(aᵢ' + i · f' · aᵢ) θⁱ]
    pub fn differentiate(&self, p: &ExtPoly) -> ExtPoly {
        if p.is_zero() {
            return ExtPoly::zero(&self.var);
        }

        let deg = match p.degree() {
            Some(d) => d,
            None => return ExtPoly::zero(&self.var),
        };

        match self.ext_type {
            ExtensionType::Exponential => {
                // θ' = f'·θ, so d/dx[aᵢ θⁱ] = (aᵢ' + i·f'·aᵢ) θⁱ
                let f_prime = self.argument.derivative();
                let mut coeffs = Vec::with_capacity(deg + 1);
                for i in 0..=deg {
                    let a_i = p.coeff(i);
                    let a_i_prime = a_i.derivative();
                    if i == 0 {
                        coeffs.push(a_i_prime);
                    } else {
                        let scalar = RationalFunction::from_constant(
                            BigRational::from_integer(BigInt::from(i)),
                            &self.var,
                        );
                        let term = &(&scalar * &f_prime) * &a_i;
                        coeffs.push(&a_i_prime + &term);
                    }
                }
                ExtPoly::from_coeffs(coeffs, &self.var)
            }
            ExtensionType::Logarithmic => {
                // θ' = f'/f, so d/dx[aᵢ θⁱ] = aᵢ' θⁱ + i·aᵢ·(f'/f)·θⁱ⁻¹
                // For degree k in result: coeff_k = aₖ' + (k+1)·a_{k+1}·(f'/f)
                let theta_prime_coeff = self
                    .argument
                    .derivative()
                    .checked_div(&self.argument)
                    .expect("logarithmic argument must be nonzero");

                // The result can have degree at most deg (from the aᵢ' terms at degree i=deg),
                // but the chain rule terms shift down by 1, so max degree is deg.
                let mut coeffs = Vec::with_capacity(deg + 1);
                for k in 0..=deg {
                    // Term 1: aₖ' (derivative of coefficient at degree k)
                    let a_k_prime = p.coeff(k).derivative();

                    // Term 2: (k+1)·a_{k+1}·(f'/f) from the chain rule on θⁱ⁻¹
                    let term2 = if k < deg {
                        let a_k_plus_1 = p.coeff(k + 1);
                        let scalar = RationalFunction::from_constant(
                            BigRational::from_integer(BigInt::from(k as i64 + 1)),
                            &self.var,
                        );
                        &(&scalar * &a_k_plus_1) * &theta_prime_coeff
                    } else {
                        RationalFunction::zero(&self.var)
                    };

                    coeffs.push(&a_k_prime + &term2);
                }
                ExtPoly::from_coeffs(coeffs, &self.var)
            }
        }
    }
}

/// Result of Hermite reduction.
///
/// Given ∫ A/D, Hermite reduction produces:
///   ∫ A/D = g_num/g_den + ∫ h_num/h_den
///
/// where h_den is squarefree (no repeated factors in θ).
#[derive(Debug)]
pub struct HermiteResult {
    /// Numerator of the rational (non-integral) part.
    pub g_num: ExtPoly,
    /// Denominator of the rational (non-integral) part.
    pub g_den: ExtPoly,
    /// Numerator of the remaining integrand (squarefree denominator).
    pub h_num: ExtPoly,
    /// Denominator of the remaining integrand (squarefree).
    pub h_den: ExtPoly,
}

/// Hermite reduction: decompose ∫ A/D into a rational part plus an integral
/// with squarefree denominator.
///
/// Given A, D ∈ k[θ] with D ≠ 0, computes g and h such that:
///   ∫ A/D = g_num/g_den + ∫ h_num/h_den
///
/// where h_den is squarefree. If deg(A) >= deg(D), polynomial division is
/// performed first so that the remainder has deg < deg(D).
///
/// Uses the iterative method from Bronstein's "Symbolic Integration I",
/// Algorithm 2.2 (Hermite reduction, quadratic version): for each squarefree
/// factor V with multiplicity n >= 2, reduce ∫ A_j/V^j using the extended
/// GCD of V and V' (formal derivative w.r.t. θ).
pub fn hermite_reduce(a: &ExtPoly, d: &ExtPoly, var: &str) -> Result<HermiteResult, String> {
    if d.is_zero() {
        return Err("Hermite reduction: denominator is zero".to_string());
    }

    // Polynomial division: separate any polynomial part so deg(A) < deg(D).
    // The polynomial part must be integrated separately; we prepend it to h.
    let (poly_part, a_rem) = a.div_rem(d)?;

    // Square-free decomposition of D.
    let sfd = d.square_free_decomposition();

    // If D is already squarefree (all multiplicities <= 1), nothing to reduce.
    if sfd.iter().all(|(_, m)| *m <= 1) {
        // h = poly_part + a_rem/d, but we combine poly_part into h_num:
        // poly_part + a_rem/d = (poly_part * d + a_rem) / d
        let h_num = &(&poly_part * d) + &a_rem;
        return Ok(HermiteResult {
            g_num: ExtPoly::zero(var),
            g_den: ExtPoly::one(var),
            h_num,
            h_den: d.clone(),
        });
    }

    // Single factor case: D = V^n (up to constant).
    // This is the common case and avoids partial fraction splitting.
    if sfd.len() == 1 {
        let (v, n) = &sfd[0];
        let result = hermite_reduce_power(&a_rem, v, *n, var)?;
        // Fold poly_part into the integrand: h = poly_part + result.h_num / result.h_den
        let h_num = &(&poly_part * &result.h_den) + &result.h_num;
        return Ok(HermiteResult {
            g_num: result.g_num,
            g_den: result.g_den,
            h_num,
            h_den: result.h_den,
        });
    }

    // Multiple factors: split A/(V1^n1 * V2^n2 * ...) via iterative partial fractions.
    // Use extended GCD to peel off one factor at a time.
    hermite_reduce_multi_factor(&a_rem, &poly_part, &sfd, d, var)
}

/// Hermite reduction for D = V^n where V is squarefree and n >= 1.
///
/// For each power j from n down to 2, applies:
///   extended_gcd(V, V') = 1  (since V is squarefree)
///   Find B, C with B*V + C*V' = A_j
///   Rational contribution: -C / ((j-1) * V^(j-1))
///   New numerator: B + C'_formal / (j-1)
fn hermite_reduce_power(
    a: &ExtPoly,
    v: &ExtPoly,
    n: usize,
    var: &str,
) -> Result<HermiteResult, String> {
    if n <= 1 {
        return Ok(HermiteResult {
            g_num: ExtPoly::zero(var),
            g_den: ExtPoly::one(var),
            h_num: a.clone(),
            h_den: v.clone(),
        });
    }

    let v_deriv = v.formal_derivative();

    let mut g_num = ExtPoly::zero(var);
    let mut g_den = ExtPoly::one(var);
    let mut a_curr = a.clone();

    for j in (2..=n).rev() {
        // Since V is squarefree, gcd(V, V') should be constant (degree 0).
        let (gcd_vvp, s_raw, t_raw) = ExtPoly::extended_gcd(v, &v_deriv);
        // s_raw * V + t_raw * V' = gcd_vvp (a nonzero constant)

        // Scale to get B*V + C*V' = a_curr:
        //   B = s_raw * (a_curr / gcd_vvp), C = t_raw * (a_curr / gcd_vvp)
        let (a_scaled, rem) = a_curr.div_rem(&gcd_vvp)?;
        if !rem.is_zero() {
            return Err(
                "Hermite reduction: GCD does not divide numerator (invalid input)".to_string(),
            );
        }

        let c_full = &t_raw * &a_scaled;

        // Reduce C modulo V to keep degrees bounded.
        let (c_extra, c) = c_full.div_rem(v)?;

        // Recompute B from the identity: B*V + C*V' = a_curr
        // => B = (a_curr - C*V') / V
        // But we also have c_extra such that c_full = c_extra*V + c,
        // so B_full = s_raw * a_scaled, and
        // B_adjusted = B_full + c_extra*V' (from redistributing c_extra*V from C to B side).
        // Actually: B_full*V + c_full*V' = a_curr
        //   = B_full*V + (c_extra*V + c)*V'
        //   = (B_full + c_extra*V')*V + c*V'
        // So the effective B with the reduced C is: B_full + c_extra*V'
        let b_full = &s_raw * &a_scaled;
        let b = &b_full + &(&c_extra * &v_deriv);

        // Rational part contribution: -C / ((j-1) * V^(j-1))
        let j_minus_1 = BigRational::from_integer(BigInt::from(j as i64 - 1));
        let j_scalar = ExtPoly::from_rf(RationalFunction::from_constant(j_minus_1.clone(), var));

        let mut v_pow = ExtPoly::one(var);
        for _ in 0..(j - 1) {
            v_pow = &v_pow * v;
        }
        let contrib_den = &v_pow * &j_scalar;
        let neg_c = -&c;

        // Accumulate: g += neg_c / contrib_den
        // g_num/g_den + neg_c/contrib_den
        //   = (g_num * contrib_den + neg_c * g_den) / (g_den * contrib_den)
        g_num = &(&g_num * &contrib_den) + &(&neg_c * &g_den);
        g_den = &g_den * &contrib_den;

        // Simplify g_num/g_den by dividing out GCD.
        let g_gcd = g_num.gcd(&g_den);
        if !g_gcd.is_constant() || !g_gcd.is_zero() {
            let (gn, _) = g_num.div_rem(&g_gcd).unwrap();
            let (gd, _) = g_den.div_rem(&g_gcd).unwrap();
            g_num = gn;
            g_den = gd;
        }

        // New numerator for next iteration:
        // a_next = B + C'_formal / (j-1)
        let c_prime = c.formal_derivative();
        let inv_j = BigRational::new(BigInt::one(), BigInt::from(j as i64 - 1));
        let inv_j_rf = RationalFunction::from_constant(inv_j, var);
        a_curr = &b + &c_prime.scalar_mul(&inv_j_rf);
    }

    // Remaining: ∫ a_curr / V (squarefree denominator).
    Ok(HermiteResult {
        g_num,
        g_den,
        h_num: a_curr,
        h_den: v.clone(),
    })
}

/// Hermite reduction for multiple distinct squarefree factors.
///
/// Given D = V1^n1 * V2^n2 * ..., splits A/D into partial fractions
/// using extended GCD, then reduces each piece via `hermite_reduce_power`.
fn hermite_reduce_multi_factor(
    a: &ExtPoly,
    poly_part: &ExtPoly,
    sfd: &[(ExtPoly, usize)],
    _d: &ExtPoly,
    var: &str,
) -> Result<HermiteResult, String> {
    // Split A/D into sum of A_i / V_i^n_i using iterative partial fractions.
    //
    // Strategy: for factors (V1^n1, V2^n2, ..., Vk^nk),
    // let P1 = V1^n1 and P2 = V2^n2 * ... * Vk^nk.
    // Since V_i are pairwise coprime and squarefree, P1 and P2 are coprime.
    // Extended GCD gives s*P1 + t*P2 = 1, so:
    //   A/D = A / (P1*P2) = (A*t)/P1 + (A*s)/P2
    // Recurse on the P2 side for the remaining factors.

    // Build powers: V_i^n_i for each factor.
    let mut powers: Vec<ExtPoly> = Vec::with_capacity(sfd.len());
    for (v, n) in sfd {
        let mut vn = ExtPoly::one(var);
        for _ in 0..*n {
            vn = &vn * v;
        }
        powers.push(vn);
    }

    // Iteratively split: peel off one factor at a time.
    // pieces[i] = (numerator, factor V_i, multiplicity n_i)
    let mut pieces: Vec<(ExtPoly, ExtPoly, usize)> = Vec::with_capacity(sfd.len());
    let mut remaining_num = a.clone();

    for i in 0..sfd.len() {
        if i == sfd.len() - 1 {
            // Last factor gets the remaining numerator.
            pieces.push((remaining_num.clone(), sfd[i].0.clone(), sfd[i].1));
            break;
        }

        let p1 = &powers[i];
        // P2 = product of powers[i+1..].
        let mut p2 = ExtPoly::one(var);
        for pj in &powers[i + 1..] {
            p2 = &p2 * pj;
        }

        // Extended GCD: s*P1 + t*P2 = gcd (should be 1 since coprime).
        let (gcd_pp, s_coeff, t_coeff) = ExtPoly::extended_gcd(p1, &p2);

        // Scale: we need s2*P1 + t2*P2 = remaining_num
        let (scale, rem) = remaining_num.div_rem(&gcd_pp)?;
        if !rem.is_zero() {
            return Err(
                "Hermite reduction: partial fraction split failed (gcd doesn't divide numerator)"
                    .to_string(),
            );
        }

        // A_i for factor i: (remaining_num * t) / P1 -> numerator is t*scale, reduce mod P1
        let num_for_p1 = &t_coeff * &scale;
        let (_, a_i) = num_for_p1.div_rem(p1)?;
        pieces.push((a_i, sfd[i].0.clone(), sfd[i].1));

        // Remaining for factors i+1..: (remaining_num * s) / P2 -> s*scale, reduce mod P2
        let num_for_p2 = &s_coeff * &scale;
        let (_, new_remaining) = num_for_p2.div_rem(&p2)?;
        remaining_num = new_remaining;
    }

    // Reduce each piece via hermite_reduce_power.
    let mut total_g_num = ExtPoly::zero(var);
    let mut total_g_den = ExtPoly::one(var);
    let mut total_h_num = ExtPoly::zero(var);
    let mut total_h_den = ExtPoly::one(var);

    for (a_i, v_i, n_i) in &pieces {
        let result = hermite_reduce_power(a_i, v_i, *n_i, var)?;

        // Accumulate rational part: total_g += result.g
        // total_g_num/total_g_den + result.g_num/result.g_den
        total_g_num = &(&total_g_num * &result.g_den) + &(&result.g_num * &total_g_den);
        total_g_den = &total_g_den * &result.g_den;

        // Simplify g by GCD.
        let g_gcd = total_g_num.gcd(&total_g_den);
        if !g_gcd.is_zero() && !g_gcd.is_constant() {
            let (gn, _) = total_g_num.div_rem(&g_gcd).unwrap();
            let (gd, _) = total_g_den.div_rem(&g_gcd).unwrap();
            total_g_num = gn;
            total_g_den = gd;
        }

        // Accumulate integrand: total_h += result.h_num / result.h_den
        total_h_num = &(&total_h_num * &result.h_den) + &(&result.h_num * &total_h_den);
        total_h_den = &total_h_den * &result.h_den;

        // Simplify h by GCD.
        let h_gcd = total_h_num.gcd(&total_h_den);
        if !h_gcd.is_zero() && !h_gcd.is_constant() {
            let (hn, _) = total_h_num.div_rem(&h_gcd).unwrap();
            let (hd, _) = total_h_den.div_rem(&h_gcd).unwrap();
            total_h_num = hn;
            total_h_den = hd;
        }
    }

    // Fold poly_part into the integrand.
    let h_num = &(poly_part * &total_h_den) + &total_h_num;

    Ok(HermiteResult {
        g_num: total_g_num,
        g_den: total_g_den,
        h_num,
        h_den: total_h_den,
    })
}

/// Solve the Risch differential equation for polynomial solutions.
///
/// Given polynomials f, g ∈ Q\[x\], find q ∈ Q\[x\] such that **q' + f·q = g**,
/// or return `None` if no polynomial solution exists (indicating the
/// corresponding integral is non-elementary).
///
/// # Algorithm
///
/// 1. **f = 0:** q' = g, so q = ∫g dx (always succeeds).
/// 2. **Degree bound:** For f of degree m and g of degree n:
///    - m ≥ 1: deg(q) = n − m (if n < m and g ≠ 0, no solution).
///    - m = 0: deg(q) = n.
/// 3. **Top-down coefficient matching:** For m ≥ 1, the leading coefficient f_m
///    is the divisor at each step; for m = 0, f_0 is the divisor.
pub fn solve_risch_de_poly(f: &Polynomial, g: &Polynomial, var: &str) -> Option<Polynomial> {
    // Special case: f = 0 → q' = g → q = ∫g
    if f.is_zero() {
        return Some(g.integral());
    }

    // g = 0 → q = 0 is always a solution
    if g.is_zero() {
        return Some(Polynomial::zero(var));
    }

    let m = f.degree().unwrap(); // f is nonzero
    let n = g.degree().unwrap(); // g is nonzero

    // Degree bound
    let k: usize = if m >= 1 {
        if n < m {
            return None; // No polynomial solution exists
        }
        n - m
    } else {
        // m = 0, f is a nonzero constant
        n
    };

    let mut b = vec![BigRational::zero(); k + 1];

    // Process degrees from n down to 0.
    //
    // At degree r, the equation q' + f·q = g gives:
    //   (r+1)·b_{r+1} + Σ_{i+j=r, 0≤i≤m, 0≤j≤k} f_i·b_j = g_r
    //
    // For m ≥ 1: the "new" unknown at degree r is b_{r-m} (coefficient f_m),
    //   all b_j for j > r-m are already determined from higher degrees.
    // For m = 0: the "new" unknown at degree r is b_r (coefficient f_0).
    for r in (0..=n).rev() {
        let g_r = g.coeff(r);

        // Derivative contribution: (r+1)·b_{r+1} if r+1 ≤ k
        let mut known = if r < k {
            BigRational::from_integer(BigInt::from(r as i64 + 1)) * &b[r + 1]
        } else {
            BigRational::zero()
        };

        // Determine which b index we're solving for at this degree
        let target_j: Option<usize> = if m >= 1 {
            if r >= m && r - m <= k {
                Some(r - m)
            } else {
                None
            }
        } else {
            // m = 0
            if r <= k {
                Some(r)
            } else {
                None
            }
        };

        // Convolution contribution: Σ_{i=0}^{min(m,r)} f_i · b_{r-i}, skipping target
        for i in 0..=m.min(r) {
            let j = r - i;
            if j > k {
                continue;
            }
            if Some(j) == target_j {
                continue;
            }
            let f_i = f.coeff(i);
            if !f_i.is_zero() {
                known += &f_i * &b[j];
            }
        }

        let residual = &g_r - &known;

        match target_j {
            Some(j) => {
                // The coefficient of b_j in the equation
                let divisor = if m >= 1 { f.coeff(m) } else { f.coeff(0) };
                if divisor.is_zero() {
                    // b_j doesn't appear; check consistency
                    if !residual.is_zero() {
                        return None;
                    }
                    // b_j is free; set to 0
                } else {
                    b[j] = &residual / &divisor;
                }
            }
            None => {
                // No unknown at this degree — check consistency
                if !residual.is_zero() {
                    return None;
                }
            }
        }
    }

    // Build q and verify: q' + f·q must equal g
    let q = Polynomial::from_coeffs(b, var);
    let check = &q.derivative() + &(f * &q);
    if check == *g {
        Some(q)
    } else {
        None
    }
}

/// Try to decompose a Node into r(x) · exp(g(x)) where r and g are polynomials in var.
/// Returns (r, g) if the pattern matches, None otherwise.
pub fn extract_exp_pattern(expr: &Node, var: &str) -> Option<(Polynomial, Polynomial)> {
    // Simplify to canonical form first.
    let env = crate::environment::Environment::new();
    let expr = crate::simplify::Simplifiable::simplify(expr, &env).unwrap_or_else(|_| expr.clone());

    extract_exp_pattern_inner(&expr, var)
}

/// Inner helper that pattern-matches on an already-simplified expression.
fn extract_exp_pattern_inner(expr: &Node, var: &str) -> Option<(Polynomial, Polynomial)> {
    match expr {
        // Pattern 1: exp(arg) → r=1, g=from_node(arg)
        Node::Function(name, args) if name == "exp" && args.len() == 1 => {
            let g = Polynomial::from_node(&args[0], var).ok()?;
            Some((Polynomial::one(var), g))
        }

        // Pattern 3 & 4: Negate(inner)
        Node::Negate(inner) => {
            let (r, g) = extract_exp_pattern_inner(inner, var)?;
            Some((-&r, g))
        }

        // Pattern 2 & 5: Multiply(left, right) — try exp on either side
        Node::Multiply(left, right) => {
            // Try exp on the right
            if let Node::Function(name, args) = right.as_ref() {
                if name == "exp" && args.len() == 1 {
                    let g = Polynomial::from_node(&args[0], var).ok()?;
                    let r = Polynomial::from_node(left, var).ok()?;
                    return Some((r, g));
                }
            }
            // Try exp on the left
            if let Node::Function(name, args) = left.as_ref() {
                if name == "exp" && args.len() == 1 {
                    let g = Polynomial::from_node(&args[0], var).ok()?;
                    let r = Polynomial::from_node(right, var).ok()?;
                    return Some((r, g));
                }
            }
            None
        }

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exact::ExactNum;
    use crate::polynomial::Polynomial;
    use num_bigint::BigInt;
    use num_rational::BigRational;
    use num_traits::Signed;

    fn int(n: i64) -> BigRational {
        BigRational::from_integer(BigInt::from(n))
    }

    fn poly(coeffs: &[i64], var: &str) -> Polynomial {
        let cs: Vec<BigRational> = coeffs.iter().map(|&c| int(c)).collect();
        Polynomial::from_coeffs(cs, var)
    }

    fn rf_const(n: i64) -> RationalFunction {
        RationalFunction::from_constant(int(n), "x")
    }

    fn rf_poly(coeffs: &[i64]) -> RationalFunction {
        RationalFunction::from_poly(poly(coeffs, "x"))
    }

    #[test]
    fn test_diff_ext_exp_x_theta() {
        // θ = exp(x), θ' = θ
        // d/dx[θ] = 1·x'·θ = 1·1·θ = θ
        let ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let theta = ExtPoly::theta("x");
        let d = ext.differentiate(&theta);
        assert_eq!(d.degree(), Some(1));
        assert_eq!(d.coeff(1), rf_const(1));
        assert!(d.coeff(0).is_zero());
    }

    #[test]
    fn test_diff_ext_exp_x_squared_theta() {
        // θ = exp(x^2), θ' = 2x·θ
        // d/dx[θ] = 2x·θ
        let ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(poly(&[0, 0, 1], "x")),
            "x",
        );
        let theta = ExtPoly::theta("x");
        let d = ext.differentiate(&theta);
        assert_eq!(d.degree(), Some(1));
        assert_eq!(d.coeff(1), rf_poly(&[0, 2])); // 2x
    }

    #[test]
    fn test_diff_ext_exp_theta_squared() {
        // θ = exp(x), d/dx[θ^2] = 2·1·θ^2 = 2θ^2
        let ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let theta_sq = ExtPoly::from_coeffs(
            vec![
                RationalFunction::zero("x"),
                RationalFunction::zero("x"),
                rf_const(1),
            ],
            "x",
        );
        let d = ext.differentiate(&theta_sq);
        assert_eq!(d.degree(), Some(2));
        assert_eq!(d.coeff(2), rf_const(2));
    }

    #[test]
    fn test_diff_ext_exp_x_times_theta() {
        // θ = exp(x), d/dx[x·θ] = θ + x·θ = (x+1)·θ
        // a₁ = x, a₁' = 1, f' = 1
        // result coeff at degree 1 = a₁' + 1·f'·a₁ = 1 + x = x + 1
        let ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let p = ExtPoly::from_coeffs(vec![RationalFunction::zero("x"), rf_poly(&[0, 1])], "x");
        let d = ext.differentiate(&p);
        assert_eq!(d.degree(), Some(1));
        assert_eq!(d.coeff(1), rf_poly(&[1, 1])); // x + 1
    }

    #[test]
    fn test_diff_ext_log_theta() {
        // θ = log(x), θ' = 1/x
        // d/dx[θ] = 1/x (a constant ExtPoly, degree 0)
        let ext = DifferentialExtension::logarithmic(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let theta = ExtPoly::theta("x");
        let d = ext.differentiate(&theta);
        assert_eq!(d.degree(), Some(0));
        // coefficient at degree 0 should be 1/x
        let expected_1_over_x = RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x"));
        assert_eq!(d.coeff(0), expected_1_over_x);
    }

    #[test]
    fn test_diff_ext_log_theta_squared() {
        // θ = log(x), d/dx[θ^2] = 2θ·(1/x) = (2/x)θ
        let ext = DifferentialExtension::logarithmic(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let theta_sq = ExtPoly::from_coeffs(
            vec![
                RationalFunction::zero("x"),
                RationalFunction::zero("x"),
                rf_const(1),
            ],
            "x",
        );
        let d = ext.differentiate(&theta_sq);
        assert_eq!(d.degree(), Some(1));
        // coefficient at degree 1 should be 2/x
        let expected_2_over_x = RationalFunction::new(poly(&[2], "x"), poly(&[0, 1], "x"));
        assert_eq!(d.coeff(1), expected_2_over_x);
    }

    #[test]
    fn test_diff_ext_log_constant() {
        // d/dx[5] = 0 (constant has no θ dependence, and 5' = 0 w.r.t. x)
        let ext = DifferentialExtension::logarithmic(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let c = ExtPoly::from_rf(rf_const(5));
        let d = ext.differentiate(&c);
        assert!(d.is_zero());
    }

    #[test]
    fn test_diff_ext_exp_constant() {
        // d/dx[5] = 0
        let ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let c = ExtPoly::from_rf(rf_const(5));
        let d = ext.differentiate(&c);
        assert!(d.is_zero());
    }

    // ======== Hermite reduction tests ========

    /// Helper: verify the Hermite reduction identity by formal differentiation.
    ///
    /// If ∫ A/D = g_num/g_den + ∫ h_num/h_den, then differentiating both sides
    /// w.r.t. θ gives:
    ///   A/D = d/dθ[g_num/g_den] + h_num/h_den
    ///
    /// Using the quotient rule: d/dθ[g_num/g_den] = (g_num'·g_den - g_num·g_den') / g_den²
    ///
    /// Cross-multiplying: A · g_den² · h_den = (g_num'·g_den - g_num·g_den') · D · h_den
    ///                                        + h_num · D · g_den²
    fn verify_hermite_identity(a: &ExtPoly, d: &ExtPoly, result: &HermiteResult) {
        // LHS: A * g_den^2 * h_den
        let g_den_sq = &result.g_den * &result.g_den;
        let lhs = &(&(a * &g_den_sq) * &result.h_den);

        // d/dθ[g_num/g_den] numerator = g_num' * g_den - g_num * g_den'
        let gn_prime = result.g_num.formal_derivative();
        let gd_prime = result.g_den.formal_derivative();
        let deriv_num = &(&gn_prime * &result.g_den) - &(&result.g_num * &gd_prime);

        // RHS term 1: deriv_num * D * h_den  (but over g_den^2, which we've cross-multiplied)
        // Actually after cross-multiplying by g_den^2:
        // deriv_num * D * h_den  (this already has g_den^2 cleared from denominator)
        // Wait, let me redo this carefully.
        //
        // A/D = (g_num'*g_den - g_num*g_den')/g_den^2 + h_num/h_den
        //
        // Multiply through by D * g_den^2 * h_den:
        // A * g_den^2 * h_den = (g_num'*g_den - g_num*g_den') * D * h_den + h_num * D * g_den^2
        let rhs_term1 = &(&deriv_num * d) * &result.h_den;
        let rhs_term2 = &(&result.h_num * d) * &g_den_sq;
        let rhs = &rhs_term1 + &rhs_term2;

        assert_eq!(
            *lhs, rhs,
            "Hermite identity check failed:\n  LHS = {lhs}\n  RHS = {rhs}"
        );
    }

    #[test]
    fn test_hermite_squarefree_noop() {
        // D = θ + 1 (already squarefree) -> no reduction needed.
        let a = ExtPoly::from_rf(rf_const(1));
        let d = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x"); // θ + 1
        let result = hermite_reduce(&a, &d, "x").unwrap();

        // No rational part.
        assert!(result.g_num.is_zero(), "Expected zero rational part");

        // Integrand should be unchanged.
        assert_eq!(result.h_num, a);
        assert_eq!(result.h_den, d);
    }

    #[test]
    fn test_hermite_square_1_over_t_plus_1_sq() {
        // ∫ 1/(θ+1)^2
        // D = (θ+1)^2 = θ^2 + 2θ + 1
        // Expected: g = -1/(θ+1), h = 0
        //
        // Working: V = θ+1, V' = 1, n = 2
        // ext_gcd(V, V') = ext_gcd(θ+1, 1): gcd=1, s=0, t=1
        // So s*(θ+1) + t*1 = 0*(θ+1) + 1*1 = 1
        // Scale by A=1: B=0, C=1
        // Reduce C mod V: C = 1 (already < deg V)
        // B_adjusted: B_full + c_extra*V' = 0 + 0 = 0
        // Rational part: -C / ((2-1) * V^(2-1)) = -1 / (1 * (θ+1)) = -1/(θ+1)
        // New numerator: B + C'_formal / (j-1) = 0 + 0/1 = 0
        let a = ExtPoly::from_rf(rf_const(1));
        let t_plus_1 = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
        let d = &t_plus_1 * &t_plus_1; // (θ+1)^2
        let result = hermite_reduce(&a, &d, "x").unwrap();

        // Remaining integrand should be zero.
        assert!(
            result.h_num.is_zero(),
            "Expected zero remaining integrand for 1/(θ+1)^2, got h = {}/{}",
            result.h_num,
            result.h_den
        );

        // Rational part should be nonzero.
        assert!(
            !result.g_num.is_zero(),
            "Expected nonzero rational part for 1/(θ+1)^2"
        );

        // Verify the identity.
        verify_hermite_identity(&a, &d, &result);
    }

    #[test]
    fn test_hermite_cube_1_over_t_plus_1_cubed() {
        // ∫ 1/(θ+1)^3
        // After reduction, denominator should be squarefree.
        let a = ExtPoly::from_rf(rf_const(1));
        let t_plus_1 = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
        let d = &(&t_plus_1 * &t_plus_1) * &t_plus_1; // (θ+1)^3
        let result = hermite_reduce(&a, &d, "x").unwrap();

        // Rational part should be nonzero.
        assert!(
            !result.g_num.is_zero(),
            "Expected nonzero rational part for 1/(θ+1)^3"
        );

        // Remaining denominator should be squarefree.
        let h_den_sfd = result.h_den.square_free_decomposition();
        assert!(
            h_den_sfd.iter().all(|(_, m)| *m <= 1),
            "Remaining denominator should be squarefree, got SFD: {:?}",
            h_den_sfd
        );

        // Verify the identity.
        verify_hermite_identity(&a, &d, &result);
    }

    #[test]
    fn test_hermite_higher_power() {
        // ∫ 1/(θ+1)^4
        let a = ExtPoly::from_rf(rf_const(1));
        let t_plus_1 = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
        let t_plus_1_sq = &t_plus_1 * &t_plus_1;
        let d = &t_plus_1_sq * &t_plus_1_sq; // (θ+1)^4
        let result = hermite_reduce(&a, &d, "x").unwrap();

        // Rational part should be nonzero.
        assert!(!result.g_num.is_zero());

        // Remaining denominator should be squarefree.
        let h_den_sfd = result.h_den.square_free_decomposition();
        assert!(h_den_sfd.iter().all(|(_, m)| *m <= 1));

        // Verify the identity.
        verify_hermite_identity(&a, &d, &result);
    }

    #[test]
    fn test_hermite_with_nontrivial_numerator() {
        // ∫ (2θ + 3) / (θ+1)^2
        let a = ExtPoly::from_coeffs(vec![rf_const(3), rf_const(2)], "x"); // 2θ + 3
        let t_plus_1 = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
        let d = &t_plus_1 * &t_plus_1; // (θ+1)^2
        let result = hermite_reduce(&a, &d, "x").unwrap();

        // Remaining denominator should be squarefree.
        let h_den_sfd = result.h_den.square_free_decomposition();
        assert!(h_den_sfd.iter().all(|(_, m)| *m <= 1));

        // Verify the identity.
        verify_hermite_identity(&a, &d, &result);
    }

    #[test]
    fn test_hermite_multi_factor() {
        // ∫ 1 / ((θ+1)^2 * (θ-1))
        // D = (θ+1)^2 * (θ-1) has SFD [(θ-1, 1), (θ+1, 2)]
        let t_plus_1 = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
        let t_minus_1 = ExtPoly::from_coeffs(vec![rf_const(-1), rf_const(1)], "x");
        let d = &(&t_plus_1 * &t_plus_1) * &t_minus_1;
        let a = ExtPoly::from_rf(rf_const(1));
        let result = hermite_reduce(&a, &d, "x").unwrap();

        // Remaining denominator should be squarefree.
        let h_den_sfd = result.h_den.square_free_decomposition();
        assert!(
            h_den_sfd.iter().all(|(_, m)| *m <= 1),
            "Multi-factor: remaining denominator should be squarefree, got SFD: {:?}",
            h_den_sfd
        );

        // Verify the identity.
        verify_hermite_identity(&a, &d, &result);
    }

    #[test]
    fn test_hermite_constant_denominator() {
        // D = 1 (constant, squarefree). This is a degenerate case.
        let a = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(2)], "x"); // 2θ + 1
        let d = ExtPoly::one("x");
        let result = hermite_reduce(&a, &d, "x").unwrap();

        // No rational part.
        assert!(result.g_num.is_zero());

        // The integrand should be a/1.
        assert_eq!(result.h_num, a);
    }

    #[test]
    fn test_hermite_deg_a_ge_deg_d() {
        // deg(A) >= deg(D): polynomial division should happen first.
        // A = θ^3, D = (θ+1)^2 = θ^2 + 2θ + 1
        // θ^3 / (θ+1)^2 = (θ - 2) + (3θ + 2)/(θ+1)^2 (by long division)
        let a = ExtPoly::from_coeffs(
            vec![rf_const(0), rf_const(0), rf_const(0), rf_const(1)],
            "x",
        ); // θ^3
        let t_plus_1 = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
        let d = &t_plus_1 * &t_plus_1;
        let result = hermite_reduce(&a, &d, "x").unwrap();

        // Remaining denominator should be squarefree.
        let h_den_sfd = result.h_den.square_free_decomposition();
        assert!(h_den_sfd.iter().all(|(_, m)| *m <= 1));

        // Verify the identity (the polynomial part is folded into h).
        verify_hermite_identity(&a, &d, &result);
    }

    #[test]
    fn test_hermite_with_rf_coefficients() {
        // ∫ x / (θ + x)^2 — coefficients are rational functions of x.
        let x_rf = rf_poly(&[0, 1]); // x as RF
        let a = ExtPoly::from_rf(x_rf.clone()); // numerator = x
        let t_plus_x = ExtPoly::from_coeffs(vec![x_rf, rf_const(1)], "x"); // θ + x
        let d = &t_plus_x * &t_plus_x; // (θ + x)^2
        let result = hermite_reduce(&a, &d, "x").unwrap();

        // Remaining denominator should be squarefree.
        let h_den_sfd = result.h_den.square_free_decomposition();
        assert!(h_den_sfd.iter().all(|(_, m)| *m <= 1));

        // Verify the identity.
        verify_hermite_identity(&a, &d, &result);
    }

    // ======== Risch DE solver tests ========

    fn rat(n: i64, d: i64) -> BigRational {
        BigRational::new(BigInt::from(n), BigInt::from(d))
    }

    #[test]
    fn test_risch_de_trivial() {
        // q' + 0·q = 2x → q = x²
        let f = Polynomial::zero("x");
        let g = poly(&[0, 2], "x");
        let result = solve_risch_de_poly(&f, &g, "x");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), poly(&[0, 0, 1], "x"));
    }

    #[test]
    fn test_risch_de_exp_x() {
        // q' + q = 1 → q = 1 (since 0 + 1·1 = 1)
        let f = poly(&[1], "x");
        let g = poly(&[1], "x");
        let q = solve_risch_de_poly(&f, &g, "x").unwrap();
        assert_eq!(q, poly(&[1], "x"));
    }

    #[test]
    fn test_risch_de_x_exp_neg_x_sq() {
        // q' + (-2x)·q = x → q = -1/2
        let f = poly(&[0, -2], "x");
        let g = poly(&[0, 1], "x");
        let q = solve_risch_de_poly(&f, &g, "x").unwrap();
        // Verify: q' + f*q = 0 + (-2x)(-1/2) = x ✓
        let check = &q.derivative() + &(&f * &q);
        assert_eq!(check, g);
        // Also check the value: q should be -1/2
        assert_eq!(q.coeff(0), rat(-1, 2));
        assert!(q.degree() == Some(0));
    }

    #[test]
    fn test_risch_de_exp_neg_x_sq_non_elementary() {
        // q' + (-2x)·q = 1 → no solution (deg bound = 0 - 1 = -1)
        let f = poly(&[0, -2], "x");
        let g = poly(&[1], "x");
        assert!(solve_risch_de_poly(&f, &g, "x").is_none());
    }

    #[test]
    fn test_risch_de_exp_x_cubed_non_elementary() {
        // q' + 3x²·q = 1 → no solution (deg bound = 0 - 2 = -2)
        let f = poly(&[0, 0, 3], "x");
        let g = poly(&[1], "x");
        assert!(solve_risch_de_poly(&f, &g, "x").is_none());
    }

    #[test]
    fn test_risch_de_x_sq_exp_neg_x_sq() {
        // q' + (-2x)·q = x² → deg bound = 2-1 = 1, but contradiction at deg 0
        let f = poly(&[0, -2], "x");
        let g = poly(&[0, 0, 1], "x");
        assert!(solve_risch_de_poly(&f, &g, "x").is_none());
    }

    #[test]
    fn test_risch_de_2x_exp_x_sq() {
        // q' + 2x·q = 2x → q = 1 (since 0 + 2x·1 = 2x)
        let f = poly(&[0, 2], "x");
        let g = poly(&[0, 2], "x");
        let q = solve_risch_de_poly(&f, &g, "x").unwrap();
        assert_eq!(q, poly(&[1], "x"));
    }

    #[test]
    fn test_risch_de_constant_f() {
        // q' + 3q = 6x + 3
        // q = 2x + 1/3. Check: q' + 3q = 2 + 6x + 1 = 6x + 3 ✓
        let f = poly(&[3], "x");
        let g = poly(&[3, 6], "x");
        let q = solve_risch_de_poly(&f, &g, "x").unwrap();
        let check = &q.derivative() + &(&f * &q);
        assert_eq!(check, g);
    }

    #[test]
    fn test_risch_de_zero_g() {
        // q' + 2x·q = 0 → q = 0
        let f = poly(&[0, 2], "x");
        let g = Polynomial::zero("x");
        let q = solve_risch_de_poly(&f, &g, "x").unwrap();
        assert!(q.is_zero());
    }

    #[test]
    fn test_risch_de_higher_degree() {
        // q' + x·q = x³ + x²
        // m=1, n=3, k=2, q = b₂x² + b₁x + b₀
        // Solving top-down reveals a contradiction at deg 0
        let f = poly(&[0, 1], "x"); // x
        let g = poly(&[0, 0, 1, 1], "x"); // x³ + x²
        assert!(solve_risch_de_poly(&f, &g, "x").is_none());
    }

    #[test]
    fn test_risch_de_both_zero() {
        // q' + 0·q = 0 → q = 0 (or any constant, but integral of 0 is 0)
        let f = Polynomial::zero("x");
        let g = Polynomial::zero("x");
        let q = solve_risch_de_poly(&f, &g, "x").unwrap();
        assert!(q.is_zero());
    }

    // ======== extract_exp_pattern tests ========

    #[test]
    fn test_extract_exp_simple() {
        // exp(x) → r=1, g=x
        let expr = Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())]);
        let (r, g) = extract_exp_pattern(&expr, "x").unwrap();
        assert_eq!(r, Polynomial::one("x"));
        assert_eq!(g, Polynomial::x("x"));
    }

    #[test]
    fn test_extract_exp_neg_x_sq() {
        // exp(-x^2) → r=1, g=-x²
        let expr = Node::Function(
            "exp".to_string(),
            vec![Node::Negate(Box::new(Node::Power(
                Box::new(Node::Variable("x".to_string())),
                Box::new(Node::Num(ExactNum::integer(2))),
            )))],
        );
        let (r, g) = extract_exp_pattern(&expr, "x").unwrap();
        assert_eq!(r, Polynomial::one("x"));
        // g should be -x²
        assert_eq!(g.degree(), Some(2));
        assert!(g.leading_coeff().unwrap().is_negative());
    }

    #[test]
    fn test_extract_exp_with_coeff() {
        // x * exp(x) → r=x, g=x
        let expr = Node::Multiply(
            Box::new(Node::Variable("x".to_string())),
            Box::new(Node::Function(
                "exp".to_string(),
                vec![Node::Variable("x".to_string())],
            )),
        );
        let (r, g) = extract_exp_pattern(&expr, "x").unwrap();
        assert_eq!(r, Polynomial::x("x"));
        assert_eq!(g, Polynomial::x("x"));
    }

    #[test]
    fn test_extract_exp_coeff_left() {
        // exp(-x^2) * x → r=x, g=-x² (exp on left)
        let exp_node = Node::Function(
            "exp".to_string(),
            vec![Node::Negate(Box::new(Node::Power(
                Box::new(Node::Variable("x".to_string())),
                Box::new(Node::Num(ExactNum::integer(2))),
            )))],
        );
        let expr = Node::Multiply(
            Box::new(exp_node),
            Box::new(Node::Variable("x".to_string())),
        );
        let (r, _g) = extract_exp_pattern(&expr, "x").unwrap();
        assert_eq!(r, Polynomial::x("x"));
    }

    #[test]
    fn test_extract_exp_numeric_coeff() {
        // 3 * exp(x) → r=3, g=x
        let expr = Node::Multiply(
            Box::new(Node::Num(ExactNum::integer(3))),
            Box::new(Node::Function(
                "exp".to_string(),
                vec![Node::Variable("x".to_string())],
            )),
        );
        let (r, _g) = extract_exp_pattern(&expr, "x").unwrap();
        assert_eq!(
            r,
            Polynomial::constant(
                num_rational::BigRational::from_integer(num_bigint::BigInt::from(3)),
                "x"
            )
        );
    }

    #[test]
    fn test_extract_exp_no_match_sin() {
        // sin(x) — not an exponential pattern
        let expr = Node::Function("sin".to_string(), vec![Node::Variable("x".to_string())]);
        assert!(extract_exp_pattern(&expr, "x").is_none());
    }

    #[test]
    fn test_extract_exp_no_match_non_poly_arg() {
        // exp(sin(x)) — arg is not a polynomial
        let expr = Node::Function(
            "exp".to_string(),
            vec![Node::Function(
                "sin".to_string(),
                vec![Node::Variable("x".to_string())],
            )],
        );
        assert!(extract_exp_pattern(&expr, "x").is_none());
    }

    #[test]
    fn test_extract_exp_negated() {
        // -exp(x) → r=-1, g=x
        let expr = Node::Negate(Box::new(Node::Function(
            "exp".to_string(),
            vec![Node::Variable("x".to_string())],
        )));
        let (r, g) = extract_exp_pattern(&expr, "x").unwrap();
        assert!(r.leading_coeff().unwrap().is_negative());
        assert_eq!(g, Polynomial::x("x"));
    }
}
