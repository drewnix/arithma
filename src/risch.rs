use crate::exact::ExactNum;
use crate::ext_poly::ExtPoly;
use crate::node::Node;
use crate::polynomial::Polynomial;
use crate::rational_function::RationalFunction;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, ToPrimitive, Zero};

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

/// Solve the generalised Risch differential equation for polynomial solutions.
///
/// Given polynomials s, F, G ∈ Q\[x\], find p ∈ Q\[x\] such that
/// **s·p' + F·p = G**, or return `None` if no polynomial solution exists.
///
/// # Algorithm
///
/// 1. **F = 0 and s = 1:** p' = G, so p = ∫G dx.
/// 2. **Degree bound:** Determined by the leading-order balance of s·p' vs F·p.
/// 3. **Top-down coefficient matching:** At each degree r the equation
///    `Σ_j s_j·(r+1-j)·b_{r+1-j} + Σ_j F_j·b_{r-j} = G_r`
///    is solved for the single unknown coefficient b[target].
/// 4. **Verification:** s·p' + F·p must equal G exactly.
pub fn solve_risch_de(
    s: &Polynomial,
    f_poly: &Polynomial,
    g_poly: &Polynomial,
    var: &str,
) -> Option<Polynomial> {
    // Special case: F = 0 and s = constant → s·p' = G → p = ∫(G/s) dx
    if f_poly.is_zero() && s.is_constant() {
        let lc_s = s.coeff(0);
        if lc_s.is_zero() {
            // s = 0 and F = 0: equation is 0 = G
            return if g_poly.is_zero() {
                Some(Polynomial::zero(var))
            } else {
                None
            };
        }
        // s·p' = G → p' = G/lc(s) → p = ∫(G/lc(s))
        let scaled = g_poly.scalar_mul(&(&BigRational::one() / &lc_s));
        return Some(scaled.integral());
    }

    // g = 0 → p = 0 is always a solution
    if g_poly.is_zero() {
        return Some(Polynomial::zero(var));
    }

    let deg_s = s.degree().unwrap_or(0);
    let deg_f = f_poly.degree().unwrap_or(0);
    let n_g = g_poly.degree().unwrap(); // g is nonzero

    // Degree bound for p.
    // Leading balance: if deg(F) >= deg(s), the F·p term dominates and
    //   deg(G) = deg(F) + deg(p), so deg(p) = n_g - deg(F).
    // If deg(F) < deg(s), the s·p' term dominates and
    //   deg(G) = deg(s) + deg(p) - 1, so deg(p) = n_g - deg(s) + 1.
    // When F = 0 (with non-constant s), use the s-branch.
    let k: usize = if f_poly.is_zero() || deg_f < deg_s {
        // s·p' dominates
        if n_g + 1 < deg_s {
            return None;
        }
        n_g + 1 - deg_s
    } else if deg_f > deg_s {
        // F·p dominates
        if n_g < deg_f {
            return None;
        }
        n_g - deg_f
    } else {
        // deg_f == deg_s: both contribute at the same order.
        // The effective leading coefficient at degree (k + deg_f) is
        //   lc(s)·k + lc(F).  If that vanishes for the naive bound k = n_g - deg_f,
        //   the actual degree could be lower.  Try k = n_g - deg_f first.
        if n_g < deg_f {
            return None;
        }
        n_g - deg_f
    };

    // The highest degree that appears in the equation s·p' + F·p = G.
    // It is max(deg_s + k - 1, deg_f + k, n_g).  We iterate from max_r down to 0.
    let max_r = if k == 0 {
        n_g
    } else {
        let from_s = deg_s + k - 1;
        let from_f = if !f_poly.is_zero() { deg_f + k } else { 0 };
        from_s.max(from_f).max(n_g)
    };

    let mut b = vec![BigRational::zero(); k + 1];

    for r in (0..=max_r).rev() {
        let g_r = g_poly.coeff(r);

        // Determine the target b-index at degree r.
        // The terms that produce degree r from s·p' are s_i * (j+1) * b_{j+1}
        // where i + j = r, so j = r - i and the b-index is j+1 = r+1-i.
        // The terms from F·p at degree r are F_i * b_j where i + j = r.
        //
        // "Target" = the b-index we solve for (not yet determined from higher degrees).
        // All b[j] with j > target are already known.
        let target_j: Option<usize> = if f_poly.is_zero() || deg_f < deg_s {
            // s dominates: target is b_{r+1-deg_s}
            let idx = (r + 1).checked_sub(deg_s)?;
            if idx <= k {
                Some(idx)
            } else {
                None
            }
        } else if deg_f > deg_s {
            // F dominates: target is b_{r-deg_f}
            if r >= deg_f && r - deg_f <= k {
                Some(r - deg_f)
            } else {
                None
            }
        } else {
            // deg_f == deg_s: both terms can contribute to the same target.
            // Target from F: b_{r - deg_f} = b_{r - deg_s}
            // Target from s: b_{r + 1 - deg_s}
            // Since deg_f == deg_s, the F-target (r - deg_f) is one less than
            // the s-target (r + 1 - deg_s).  So the lowest unknown is from F.
            if r >= deg_f && r - deg_f <= k {
                Some(r - deg_f)
            } else {
                None
            }
        };

        // Accumulate known contributions from s·p' (derivative terms).
        let mut known = BigRational::zero();
        for i in 0..=deg_s.min(r) {
            let s_i = s.coeff(i);
            if s_i.is_zero() {
                continue;
            }
            // b-index from derivative: j+1 where j = r - i, so b_{r+1-i}
            let b_idx = r + 1 - i;
            if b_idx > k || b_idx == 0 {
                continue;
            }
            // But b_idx refers to b[b_idx], and the derivative produces (b_idx)*b[b_idx]
            // from the term (j+1)*b_{j+1} where j = r-i, j+1 = b_idx
            // Skip if this is the target
            if Some(b_idx) == target_j {
                continue;
            }
            known += &s_i * BigRational::from_integer(BigInt::from(b_idx as i64)) * &b[b_idx];
        }

        // Accumulate known contributions from F·p (convolution terms).
        if !f_poly.is_zero() {
            for i in 0..=deg_f.min(r) {
                let j = r - i;
                if j > k {
                    continue;
                }
                if Some(j) == target_j {
                    continue;
                }
                let f_i = f_poly.coeff(i);
                if !f_i.is_zero() {
                    known += &f_i * &b[j];
                }
            }
        }

        let residual = &g_r - &known;

        match target_j {
            Some(j) => {
                // Compute the divisor: coefficient of b[j] in the equation at degree r.
                let mut divisor = BigRational::zero();

                // Contribution from s·p': s_i * b_idx where b_idx = j, so i = r+1-j
                // and the factor is j (from (j)*b[j] in the derivative).
                let deriv_i = r + 1 - j;
                if deriv_i <= deg_s && j > 0 {
                    divisor +=
                        &s.coeff(deriv_i) * BigRational::from_integer(BigInt::from(j as i64));
                }

                // Contribution from F·p: F_i * b_j where i = r - j
                if !f_poly.is_zero() && r >= j && r - j <= deg_f {
                    divisor += f_poly.coeff(r - j);
                }

                if divisor.is_zero() {
                    if !residual.is_zero() {
                        return None;
                    }
                    // b[j] is free; leave it as 0
                } else {
                    b[j] = &residual / &divisor;
                }
            }
            None => {
                if !residual.is_zero() {
                    return None;
                }
            }
        }
    }

    // Build p and verify: s·p' + F·p must equal G
    let p = Polynomial::from_coeffs(b, var);
    let check = &(s * &p.derivative()) + &(f_poly * &p);
    if check == *g_poly {
        Some(p)
    } else {
        None
    }
}

/// Solve the Risch differential equation **q' + f·q = g** for polynomial q.
///
/// This is a thin wrapper around [`solve_risch_de`] with s = 1.
pub fn solve_risch_de_poly(f: &Polynomial, g: &Polynomial, var: &str) -> Option<Polynomial> {
    solve_risch_de(&Polynomial::one(var), f, g, var)
}

/// Solve the Risch differential equation **q' + f·q = g** where g is a rational function.
///
/// Returns `Some(q)` with `q` a [`RationalFunction`] satisfying the ODE, or `None`
/// if no rational solution exists.
///
/// # Algorithm
///
/// 1. If g is polynomial (denominator is 1), delegate to [`solve_risch_de`] with s = 1.
/// 2. Compute the squarefree decomposition of `den(g)`.  If any factor has
///    multiplicity 1, the ODE has no rational solution (the simple pole creates
///    an uncancellable singularity).
/// 3. Build the denominator bound `s = ∏ factor^{mult-1}` for each factor with
///    multiplicity ≥ 2.
/// 4. Substitute `q = p / s` to obtain a polynomial ODE `s·p' + F·p = G` with
///    `F = f·s − s'`, `G = num(g) · (s² / den(g))`, and solve with
///    [`solve_risch_de`].
/// 5. Verify the result by polynomial cross-multiplication.
pub fn solve_risch_de_rational(
    f: &Polynomial,
    g: &RationalFunction,
    var: &str,
) -> Option<RationalFunction> {
    let one_poly = Polynomial::one(var);

    // Step 1: trivial case — g is polynomial
    if g.denominator() == &one_poly {
        let p = solve_risch_de(&one_poly, f, g.numerator(), var)?;
        return Some(RationalFunction::from_poly(p));
    }

    // Step 2: squarefree rejection and denominator bound
    let sfd = g.denominator().square_free_decomposition();

    // Reject if any factor has multiplicity 1 and is not constant.
    for (factor, mult) in &sfd {
        if *mult == 1 && !factor.is_constant() {
            return None;
        }
    }

    // Step 3: denominator bound s = ∏ factor^{mult-1}
    let mut s = Polynomial::one(var);
    for (factor, mult) in &sfd {
        if *mult >= 2 {
            for _ in 0..(*mult - 1) {
                s = &s * factor;
            }
        }
    }

    // Step 4: transform to polynomial ODE
    let s_prime = s.derivative();
    // F = f·s - s'
    let big_f = &(f * &s) - &s_prime;

    // G = num(g) · (s² / den(g))
    let s_sq = &s * &s;
    let (ratio, rem) = s_sq.div_rem(g.denominator()).unwrap();
    debug_assert!(
        rem.is_zero(),
        "s² / den(g) must divide evenly, but got remainder: {:?}",
        rem
    );
    let big_g = &(g.numerator().clone()) * &ratio;

    // Solve s·p' + F·p = G
    let p = solve_risch_de(&s, &big_f, &big_g, var)?;

    // Step 5: verify by polynomial cross-multiplication
    // q = p/s, q' + f·q should equal g = num(g)/den(g)
    // q' = (p'·s - p·s')/s²
    // q' + f·q = (p'·s - p·s' + f·p·s) / s²
    // Check: (p'·s - p·s' + f·p·s) · den(g) == num(g) · s²
    let p_prime = p.derivative();
    let check_num = &(&(&p_prime * &s) - &(&p * &s_prime)) + &(&(f * &p) * &s);
    let check_den_factor = &s * &s; // s²
    if &check_num * g.denominator() == g.numerator() * &check_den_factor {
        Some(RationalFunction::new(p, s))
    } else {
        None
    }
}

/// Result of integrating a rational function over the base field (Q(x)).
///
/// The integral has the form: rational_part + ln_x_coeff * ln(x).
#[derive(Debug)]
pub struct BaseFieldIntegral {
    pub rational_part: RationalFunction,
    pub ln_x_coeff: BigRational,
}

/// Integrate a rational function of `var` over the base field Q(x).
///
/// Returns the rational part and the coefficient of ln(x). Any term that
/// would produce ln(x + a) with a ≠ 0 (or log of an irreducible quadratic)
/// is rejected as non-elementary in this restricted setting.
pub fn integrate_rational_base(
    rf: &RationalFunction,
    var: &str,
) -> Result<BaseFieldIntegral, String> {
    if rf.is_zero() {
        return Ok(BaseFieldIntegral {
            rational_part: RationalFunction::from_constant(BigRational::zero(), var),
            ln_x_coeff: BigRational::zero(),
        });
    }

    let decomp =
        crate::partial_fractions::partial_fraction_decomposition(rf.numerator(), rf.denominator())?;

    // Integrate the polynomial part
    let poly_integral = decomp.polynomial_part.integral();
    let mut rational_part = RationalFunction::from_poly(poly_integral);
    let mut ln_x_coeff = BigRational::zero();

    for term in &decomp.terms {
        let deg = term.denominator.degree().unwrap_or(0);
        if deg >= 2 {
            return Err(format!(
                "Irreducible factor of degree {} produces non-elementary integral",
                deg
            ));
        }

        let c = term.numerator.coeff(0);
        let a = term.denominator.coeff(0);

        if term.power == 1 {
            if a.is_zero() {
                // Term is c/x → integral is c·ln(x)
                ln_x_coeff += c;
            } else {
                // Term is c/(x+a) → integral is c·ln(x+a), non-elementary in base field
                return Err(format!(
                    "Term produces ln(x + {}) which is non-elementary in the base field",
                    a
                ));
            }
        } else {
            // power > 1: c/(x+a)^k → c/((1-k)·(x+a)^{k-1})
            let k = term.power;
            let one_minus_k = BigRational::from_integer(BigInt::from(1i64 - k as i64));
            let scale = &c / &one_minus_k;

            // Build (x+a)^{k-1}
            let mut den_power = Polynomial::constant(BigRational::one(), var);
            for _ in 0..(k - 1) {
                den_power = &den_power * &term.denominator;
            }

            let term_rf = RationalFunction::new(Polynomial::constant(scale, var), den_power);
            rational_part = &rational_part + &term_rf;
        }
    }

    Ok(BaseFieldIntegral {
        rational_part,
        ln_x_coeff,
    })
}

/// Result of a Risch integration attempt.
#[derive(Debug)]
pub enum RischResult {
    /// Found an elementary antiderivative.
    Elementary(Node),
    /// Proved that no elementary antiderivative exists.
    NonElementary(String),
}

/// Determinant of a square matrix of ExtPolys via cofactor expansion.
/// For small matrices (size ≤ 5, covering Risch degrees in practice).
#[allow(dead_code)] // Used by Rothstein-Trager resultant (upcoming)
pub(crate) fn extpoly_matrix_det(m: &[Vec<ExtPoly>], var: &str) -> ExtPoly {
    let n = m.len();
    if n == 0 {
        return ExtPoly::one(var);
    }
    if n == 1 {
        return m[0][0].clone();
    }
    if n == 2 {
        return &(&m[0][0] * &m[1][1]) - &(&m[0][1] * &m[1][0]);
    }
    let mut result = ExtPoly::zero(var);
    for j in 0..n {
        if m[0][j].is_zero() {
            continue;
        }
        let minor: Vec<Vec<ExtPoly>> = (1..n)
            .map(|row| {
                (0..n)
                    .filter(|&col| col != j)
                    .map(|col| m[row][col].clone())
                    .collect()
            })
            .collect();
        let cofactor = extpoly_matrix_det(&minor, var);
        let term = &m[0][j] * &cofactor;
        if j % 2 == 0 {
            result = &result + &term;
        } else {
            result = &result - &term;
        }
    }
    result
}

/// Build the Sylvester matrix and compute R(z) = res_θ(d, a − z·D(d)).
///
/// d and a are ExtPolys in θ (tower variable) with RF(x) coefficients.
/// dd = D(d) is the full derivative of d in the extension.
/// Returns R(z) as an ExtPoly where the "variable" represents z, with RF(x) coefficients.
#[allow(dead_code)] // Used by Rothstein-Trager integration (upcoming)
fn rothstein_trager_resultant(d: &ExtPoly, a: &ExtPoly, dd: &ExtPoly, var: &str) -> ExtPoly {
    let m = d.degree().unwrap_or(0); // degree of d in θ
                                     // degree of g = a - z·dd in θ
    let n = {
        let da = a.degree().unwrap_or(0);
        let ddd = dd.degree().unwrap_or(0);
        da.max(ddd)
    };

    if m == 0 && n == 0 {
        // Both constant in θ: resultant is a₀ - z·dd₀
        let c0 = a.coeff(0);
        let c1 = -&dd.coeff(0);
        return ExtPoly::from_coeffs(vec![c0, c1], var);
    }

    let size = m + n;
    if size == 0 {
        return ExtPoly::one(var);
    }

    let zero_z = ExtPoly::zero(var);
    let mut matrix: Vec<Vec<ExtPoly>> = Vec::with_capacity(size);

    // First n rows from d (no z-dependence — constant ExtPolys in z)
    // Sylvester convention: row i has d's coefficients shifted by i positions
    // Coefficients listed highest-to-lowest: d_m, d_{m-1}, ..., d_0
    for i in 0..n {
        let mut row = vec![zero_z.clone(); size];
        for k in 0..=m {
            let col = i + k;
            if col < size {
                row[col] = ExtPoly::from_rf(d.coeff(m - k));
            }
        }
        matrix.push(row);
    }

    // Last m rows from g = a - z·dd (linear in z)
    // g has degree n in θ; coefficients g_k = a_k - z·dd_k
    for i in 0..m {
        let mut row = vec![zero_z.clone(); size];
        for k in 0..=n {
            let col = i + k;
            if col < size {
                let a_coeff = a.coeff(n - k);
                let dd_coeff = dd.coeff(n - k);
                if dd_coeff.is_zero() {
                    row[col] = ExtPoly::from_rf(a_coeff);
                } else {
                    row[col] = ExtPoly::from_coeffs(vec![a_coeff, -&dd_coeff], var);
                }
            }
        }
        matrix.push(row);
    }

    extpoly_matrix_det(&matrix, var)
}

/// Find all c ∈ Q such that R(c) = 0, where R(z) is a polynomial in z
/// with RationalFunction(x) coefficients.
///
/// Strategy: specialize x to a concrete value x₀, find rational roots of
/// the resulting Q[z] polynomial, then verify each candidate against the
/// full R(z).
#[allow(dead_code)] // Used by Rothstein-Trager integration (upcoming)
fn find_constant_roots(rz: &ExtPoly, var: &str) -> Vec<BigRational> {
    let deg = match rz.degree() {
        Some(d) => d,
        None => return vec![],
    };

    if deg == 0 {
        return vec![];
    }

    let candidates_x = [2i64, 3, 5, 7, 11];
    let mut candidate_roots: Option<Vec<BigRational>> = None;

    for &x_val in &candidates_x {
        let x_br = BigRational::from_integer(BigInt::from(x_val));
        let mut specialized_coeffs = Vec::with_capacity(deg + 1);
        let mut valid = true;
        for i in 0..=deg {
            match rz.coeff(i).evaluate(&x_br) {
                Some(val) => specialized_coeffs.push(val),
                None => {
                    valid = false;
                    break;
                }
            }
        }
        if !valid {
            continue;
        }

        let spec_poly = Polynomial::from_coeffs(specialized_coeffs, "z");
        if spec_poly.is_zero() {
            continue;
        }

        candidate_roots = Some(spec_poly.rational_roots());
        break;
    }

    let candidates = match candidate_roots {
        Some(c) => c,
        None => return vec![],
    };

    // Verify each candidate: compute R(c) as an RF and check if zero
    let mut verified = Vec::new();
    for c in candidates {
        let mut sum = RationalFunction::zero(var);
        let mut c_power = BigRational::one();
        for i in 0..=deg {
            let term = &rz.coeff(i) * &RationalFunction::from_constant(c_power.clone(), var);
            sum = &sum + &term;
            c_power = &c_power * &c;
        }
        if sum.is_zero() && !verified.contains(&c) {
            verified.push(c);
        }
    }

    verified
}

/// Convert a RationalFunction p(x)/q(x) to a Node expression.
#[allow(dead_code)]
fn rf_to_node(rf: &RationalFunction, var: &str) -> Node {
    let num_node = rf.numerator().to_node();
    if *rf.denominator() == Polynomial::one(var) {
        num_node
    } else {
        Node::Divide(Box::new(num_node), Box::new(rf.denominator().to_node()))
    }
}

/// Convert an ExtPoly Σ aᵢ(x)·θⁱ to a Node expression,
/// replacing θ with `theta_node` (e.g., ln(x)).
#[allow(dead_code)]
fn extpoly_to_node(ep: &ExtPoly, theta_node: &Node, var: &str) -> Node {
    let deg = match ep.degree() {
        Some(d) => d,
        None => return Node::Num(ExactNum::zero()),
    };

    let mut terms: Vec<Node> = Vec::new();
    for i in 0..=deg {
        let coeff = ep.coeff(i);
        if coeff.is_zero() {
            continue;
        }
        let coeff_node = rf_to_node(&coeff, var);
        let term = if i == 0 {
            coeff_node
        } else {
            let theta_power = if i == 1 {
                theta_node.clone()
            } else {
                Node::Power(
                    Box::new(theta_node.clone()),
                    Box::new(Node::Num(ExactNum::integer(i as i64))),
                )
            };
            if coeff == RationalFunction::one(var) {
                theta_power
            } else {
                Node::Multiply(Box::new(coeff_node), Box::new(theta_power))
            }
        };
        terms.push(term);
    }

    if terms.is_empty() {
        return Node::Num(ExactNum::zero());
    }

    let mut result = terms.remove(0);
    for term in terms {
        result = Node::Add(Box::new(result), Box::new(term));
    }
    result
}

/// Convert a BigRational to a Node expression.
fn bigrat_to_node(r: &BigRational) -> Node {
    if r.denom() == &BigInt::one() {
        Node::Num(ExactNum::integer(r.numer().to_i64().unwrap_or(0)))
    } else {
        Node::Divide(
            Box::new(Node::Num(ExactNum::integer(
                r.numer().to_i64().unwrap_or(0),
            ))),
            Box::new(Node::Num(ExactNum::integer(
                r.denom().to_i64().unwrap_or(1),
            ))),
        )
    }
}

// ---------------------------------------------------------------------------
// Tower builder: scanning and generalized ExtPoly conversion
// ---------------------------------------------------------------------------

/// Classification of a transcendental extension for the tower builder.
#[derive(Debug, Clone)]
#[allow(dead_code)] // wired in by subsequent tower-builder tasks
enum ExtensionKind {
    Logarithmic,
    Exponential(Polynomial),
}

/// Returns `true` if `expr` contains `ln(var)` anywhere in the tree.
fn contains_ln(expr: &Node, var: &str) -> bool {
    match expr {
        Node::Function(name, args) if name == "ln" && args.len() == 1 => {
            matches!(&args[0], Node::Variable(v) if v == var)
        }
        Node::Add(l, r) | Node::Subtract(l, r) | Node::Multiply(l, r) | Node::Divide(l, r) => {
            contains_ln(l, var) || contains_ln(r, var)
        }
        Node::Negate(inner) | Node::Sqrt(inner) | Node::Abs(inner) => contains_ln(inner, var),
        Node::Power(base, exp) => contains_ln(base, var) || contains_ln(exp, var),
        Node::Function(_, args) => args.iter().any(|a| contains_ln(a, var)),
        _ => false,
    }
}

/// If the expression contains `exp(g(x))`, return the polynomial `g` when it
/// is consistent across the whole tree.  Returns `None` when no `exp` is found
/// or when two incompatible exponent polynomials appear.
fn find_exp_argument(expr: &Node, var: &str) -> Option<Polynomial> {
    match expr {
        Node::Function(name, args) if name == "exp" && args.len() == 1 => {
            Polynomial::from_node(&args[0], var).ok()
        }
        Node::Add(l, r) | Node::Subtract(l, r) | Node::Multiply(l, r) | Node::Divide(l, r) => {
            match (find_exp_argument(l, var), find_exp_argument(r, var)) {
                (Some(a), Some(b)) if a == b => Some(a),
                (Some(a), None) | (None, Some(a)) => Some(a),
                _ => None,
            }
        }
        Node::Negate(inner) => find_exp_argument(inner, var),
        Node::Power(base, _) => find_exp_argument(base, var),
        _ => None,
    }
}

/// Find ln(f(exp(g(x)))) in the expression tree.
/// Returns (g, h) where g is the exp argument polynomial and h is the ln argument
/// parsed as an ExtPoly in θ₁ = exp(g(x)).
/// Returns None if no ln-of-exp pattern is found.
fn find_ln_of_exp_argument(expr: &Node, var: &str) -> Option<(Polynomial, ExtPoly)> {
    match expr {
        Node::Function(name, args) if name == "ln" && args.len() == 1 => {
            let exp_arg = find_exp_argument(&args[0], var)?;
            let kind = ExtensionKind::Exponential(exp_arg.clone());
            let h = node_to_extpoly_general(&args[0], var, &kind)?;
            if h.degree().unwrap_or(0) == 0 {
                return None;
            }
            Some((exp_arg, h))
        }
        Node::Add(l, r) | Node::Subtract(l, r) | Node::Multiply(l, r) | Node::Divide(l, r) => {
            find_ln_of_exp_argument(l, var).or_else(|| find_ln_of_exp_argument(r, var))
        }
        Node::Negate(inner) => find_ln_of_exp_argument(inner, var),
        Node::Power(base, _) => find_ln_of_exp_argument(base, var),
        _ => None,
    }
}

/// Generalized conversion from AST node to `ExtPoly` that handles both
/// logarithmic (θ = ln(x)) and exponential (θ = exp(g(x))) extensions.
#[allow(dead_code)] // wired in by subsequent tower-builder tasks
fn node_to_extpoly_general(expr: &Node, var: &str, kind: &ExtensionKind) -> Option<ExtPoly> {
    match expr {
        Node::Num(n) => {
            if let ExactNum::Rational(val) = n {
                Some(ExtPoly::from_rf(RationalFunction::from_constant(
                    val.clone(),
                    var,
                )))
            } else {
                None
            }
        }
        Node::Variable(v) if v == var => Some(ExtPoly::from_rf(RationalFunction::from_poly(
            Polynomial::x(var),
        ))),
        Node::Variable(_) => None,
        Node::Function(name, args) if name == "ln" && args.len() == 1 => {
            if matches!(kind, ExtensionKind::Logarithmic) {
                if let Node::Variable(v) = &args[0] {
                    if v == var {
                        return Some(ExtPoly::theta(var));
                    }
                }
            }
            None
        }
        Node::Function(name, args) if name == "exp" && args.len() == 1 => {
            if let ExtensionKind::Exponential(ref g) = kind {
                if let Ok(arg_poly) = Polynomial::from_node(&args[0], var) {
                    if arg_poly == *g {
                        return Some(ExtPoly::theta(var));
                    }
                }
            }
            None
        }
        Node::Power(base, exp) => {
            if matches!(kind, ExtensionKind::Logarithmic) {
                if let Node::Function(name, args) = base.as_ref() {
                    if name == "ln" && args.len() == 1 {
                        if let Node::Variable(v) = &args[0] {
                            if v == var {
                                if let Node::Num(n) = exp.as_ref() {
                                    if let Some(e) = n.to_i64() {
                                        if e >= 1 {
                                            let mut r = ExtPoly::theta(var);
                                            for _ in 1..e {
                                                r = &r * &ExtPoly::theta(var);
                                            }
                                            return Some(r);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if let Node::Variable(v) = base.as_ref() {
                if v == var {
                    if let Node::Num(n) = exp.as_ref() {
                        if let Some(e) = n.to_i64() {
                            if e >= 1 {
                                let p = Polynomial::monomial(BigRational::one(), e as usize, var);
                                return Some(ExtPoly::from_rf(RationalFunction::from_poly(p)));
                            }
                        }
                    }
                }
            }
            None
        }
        Node::Add(l, r) => {
            Some(&node_to_extpoly_general(l, var, kind)? + &node_to_extpoly_general(r, var, kind)?)
        }
        Node::Subtract(l, r) => {
            Some(&node_to_extpoly_general(l, var, kind)? - &node_to_extpoly_general(r, var, kind)?)
        }
        Node::Negate(inner) => Some(-&node_to_extpoly_general(inner, var, kind)?),
        Node::Multiply(l, r) => {
            Some(&node_to_extpoly_general(l, var, kind)? * &node_to_extpoly_general(r, var, kind)?)
        }
        Node::Divide(num, den) => {
            let n = node_to_extpoly_general(num, var, kind)?;
            let den_poly = Polynomial::from_node(den, var).ok()?;
            if den_poly.is_zero() {
                return None;
            }
            let inv = RationalFunction::new(Polynomial::one(var), den_poly);
            Some(n.scalar_mul(&inv))
        }
        _ => None,
    }
}

/// Build a single-level transcendental tower from a Node expression.
/// Returns (numerator, denominator, extension) or None.
pub fn build_tower(expr: &Node, var: &str) -> Option<(ExtPoly, ExtPoly, DifferentialExtension)> {
    if let Some(r) = build_tower_inner(expr, var) {
        return Some(r);
    }
    let env = crate::environment::Environment::new();
    let simplified =
        crate::simplify::Simplifiable::simplify(expr, &env).unwrap_or_else(|_| expr.clone());
    build_tower_inner(&simplified, var)
}

fn build_tower_inner(expr: &Node, var: &str) -> Option<(ExtPoly, ExtPoly, DifferentialExtension)> {
    let has_ln = contains_ln(expr, var);
    let exp_arg = find_exp_argument(expr, var);

    let (kind, ext) = match (has_ln, exp_arg) {
        (true, None) => (
            ExtensionKind::Logarithmic,
            DifferentialExtension::logarithmic(
                RationalFunction::from_poly(Polynomial::x(var)),
                var,
            ),
        ),
        (false, Some(g)) => (
            ExtensionKind::Exponential(g.clone()),
            DifferentialExtension::exponential(RationalFunction::from_poly(g), var),
        ),
        _ => return None,
    };

    match expr {
        Node::Divide(num_node, den_node) => {
            let num = node_to_extpoly_general(num_node, var, &kind)?;
            let den = node_to_extpoly_general(den_node, var, &kind)?;
            if den.is_zero() {
                return None;
            }
            Some((num, den, ext))
        }
        Node::Multiply(left, right) => {
            // a * (b/c) where c involves θ
            if let Node::Divide(n, d) = right.as_ref() {
                if let Some(d_ep) = node_to_extpoly_general(d, var, &kind) {
                    if !d_ep.is_constant() {
                        let n_ep = node_to_extpoly_general(n, var, &kind)?;
                        let l_ep = node_to_extpoly_general(left, var, &kind)?;
                        return Some((&l_ep * &n_ep, d_ep, ext));
                    }
                }
            }
            // (a/b) * c where b involves θ
            if let Node::Divide(n, d) = left.as_ref() {
                if let Some(d_ep) = node_to_extpoly_general(d, var, &kind) {
                    if !d_ep.is_constant() {
                        let n_ep = node_to_extpoly_general(n, var, &kind)?;
                        let r_ep = node_to_extpoly_general(right, var, &kind)?;
                        return Some((&r_ep * &n_ep, d_ep, ext));
                    }
                }
            }
            // Polynomial in θ (den = 1)
            let num = node_to_extpoly_general(expr, var, &kind)?;
            Some((num, ExtPoly::one(var), ext))
        }
        _ => {
            let num = node_to_extpoly_general(expr, var, &kind)?;
            Some((num, ExtPoly::one(var), ext))
        }
    }
}

/// Integrate a polynomial in θ = exp(g(x)): Σ aᵢ(x)·θⁱ.
/// Each degree decouples: qᵢ' + i·g'·qᵢ = aᵢ.
#[allow(dead_code)] // Will be wired into the main integration engine in a subsequent task.
fn integrate_poly_exp(
    num: &ExtPoly,
    ext: &DifferentialExtension,
    var: &str,
) -> Option<RischResult> {
    let deg = num.degree().unwrap_or(0);
    let g_prime_rf = ext.argument().derivative();
    if *g_prime_rf.denominator() != Polynomial::one(var) {
        return None;
    }
    let g_prime = g_prime_rf.numerator().clone();

    let mut q: Vec<RationalFunction> = vec![RationalFunction::zero(var); deg + 1];

    for i in 0..=deg {
        let a_i_rf = num.coeff(i);
        if a_i_rf.is_zero() {
            continue;
        }

        if i == 0 {
            // Degree 0: q_0 = ∫a_0 dx
            if *a_i_rf.denominator() != Polynomial::one(var) {
                return None; // Rational function integration of x not yet supported
            }
            q[0] = RationalFunction::from_poly(a_i_rf.numerator().clone().integral());
        } else {
            let f = g_prime.scalar_mul(&BigRational::from_integer(BigInt::from(i as i64)));
            match solve_risch_de_rational(&f, &a_i_rf, var) {
                Some(qi) => q[i] = qi,
                None => {
                    return Some(RischResult::NonElementary(format!(
                        "No elementary antiderivative exists. \
                         The differential equation q' + ({})·q = {} has no rational solution.",
                        f, a_i_rf
                    )));
                }
            }
        }
    }

    let g_node = ext.argument().numerator().to_node();
    let mut terms: Vec<Node> = Vec::new();
    for (i, qi) in q.iter().enumerate() {
        if qi.is_zero() {
            continue;
        }
        let q_node = rf_to_node(qi, var);
        let term = if i == 0 {
            q_node
        } else {
            let exp_g = Node::Function("exp".to_string(), vec![g_node.clone()]);
            let exp_part = if i == 1 {
                exp_g
            } else {
                Node::Power(
                    Box::new(exp_g),
                    Box::new(Node::Num(ExactNum::integer(i as i64))),
                )
            };
            if *qi == RationalFunction::one(var) {
                exp_part
            } else {
                Node::Multiply(Box::new(q_node), Box::new(exp_part))
            }
        };
        terms.push(term);
    }

    if terms.is_empty() {
        return Some(RischResult::Elementary(Node::Num(ExactNum::zero())));
    }
    let mut result = terms.remove(0);
    for t in terms {
        result = Node::Add(Box::new(result), Box::new(t));
    }
    Some(RischResult::Elementary(result))
}

/// Integrate an ExtPoly in the exponential extension, returning the result as an ExtPoly.
///
/// For p = Σ aᵢ·θ₁ⁱ, each degree decouples:
///   i=0: b₀ = ∫a₀(x) dx (a₀ must be polynomial for the result to be RationalFunction)
///   i≥1: solve b_i' + i·g'·b_i = a_i (rational Risch DE)
///
/// Returns None if any degree has no rational solution (non-elementary).
fn integrate_in_exp_ext_structured(
    p: &ExtPoly,
    ext: &DifferentialExtension,
    var: &str,
) -> Option<ExtPoly> {
    let deg = p.degree().unwrap_or(0);
    let g_prime_rf = ext.argument().derivative();
    if *g_prime_rf.denominator() != Polynomial::one(var) {
        return None;
    }
    let g_prime = g_prime_rf.numerator().clone();

    let mut result_coeffs: Vec<RationalFunction> = vec![RationalFunction::zero(var); deg + 1];

    for i in 0..=deg {
        let a_i = p.coeff(i);
        if a_i.is_zero() {
            continue;
        }

        if i == 0 {
            // Degree 0: ∫a₀(x) dx — a₀ must be polynomial for polynomial antiderivative
            if *a_i.denominator() != Polynomial::one(var) {
                return None;
            }
            result_coeffs[0] = RationalFunction::from_poly(a_i.numerator().clone().integral());
        } else {
            // Degree i≥1: solve q' + i·g'·q = aᵢ
            let f = g_prime.scalar_mul(&BigRational::from_integer(BigInt::from(i as i64)));
            match solve_risch_de_rational(&f, &a_i, var) {
                Some(qi) => result_coeffs[i] = qi,
                None => return None,
            }
        }
    }

    Some(ExtPoly::from_coeffs(result_coeffs, var))
}

/// Integrate a polynomial in θ = ln(x): Σ aᵢ(x)·θⁱ.
/// Top-down: qₙ' = aₙ, then qₖ' = aₖ - (k+1)·q_{k+1}/x.
///
/// Each coefficient aₖ(x) may be a rational function of x. Integration of
/// each right-hand side uses `integrate_rational_base`, which decomposes the
/// result into a rational part plus a coefficient of ln(x). Since ln(x) = θ,
/// any ln(x) produced at degree k > 0 is "absorbed" into the θ¹ coefficient
/// via a Δ accumulator. A ln(x) term at degree 0 means the integral is
/// non-elementary (it would require θ² in the result, contradicting degree).
#[allow(dead_code)] // Will be wired into the main integration engine in a subsequent task.
fn integrate_poly_log(
    num: &ExtPoly,
    _ext: &DifferentialExtension,
    var: &str,
) -> Option<RischResult> {
    let deg = num.degree().unwrap_or(0);
    let mut q: Vec<RationalFunction> = vec![RationalFunction::zero(var); deg + 1];
    let mut ln_x_accum = BigRational::zero();
    let x_rf = RationalFunction::from_poly(Polynomial::x(var));

    for k in (0..=deg).rev() {
        let a_k_rf = num.coeff(k);

        let rhs = if k == deg {
            // Top degree: RHS = a_n directly
            a_k_rf
        } else if k == 0 {
            // Degree 0: use (q[1] + Δ) in the correction instead of q[1]
            let delta_rf = RationalFunction::from_constant(ln_x_accum.clone(), var);
            let q1_adjusted = &q[1] + &delta_rf;
            let q1_div_x = q1_adjusted.checked_div(&x_rf).ok()?;
            let scalar_rf =
                RationalFunction::from_constant(BigRational::from_integer(BigInt::from(1i64)), var);
            let correction = &q1_div_x * &scalar_rf;
            &a_k_rf - &correction
        } else {
            // Intermediate degrees: RHS = a_k - (k+1)·q_{k+1}/x
            let q_kp1_div_x = q[k + 1].checked_div(&x_rf).ok()?;
            let scalar = BigRational::from_integer(BigInt::from(k as i64 + 1));
            let scalar_rf = RationalFunction::from_constant(scalar, var);
            let correction = &q_kp1_div_x * &scalar_rf;
            &a_k_rf - &correction
        };

        match integrate_rational_base(&rhs, var) {
            Ok(result) => {
                q[k] = result.rational_part;
                // ln(x) = θ, so any ln(x) produced by integration is absorbed
                // into the θ¹ coefficient via the Δ accumulator.
                ln_x_accum += result.ln_x_coeff;
            }
            Err(msg) => {
                // integrate_rational_base returns Err for ln(x+a) with a≠0
                // or irreducible quadratic factors — genuinely non-elementary.
                return Some(RischResult::NonElementary(msg));
            }
        }
    }

    // Adjust q[1] by adding Δ
    if deg >= 1 && !ln_x_accum.is_zero() {
        let delta_rf = RationalFunction::from_constant(ln_x_accum, var);
        q[1] = &q[1] + &delta_rf;
    }

    let ln_x = Node::Function("ln".to_string(), vec![Node::Variable(var.to_string())]);
    let mut terms: Vec<Node> = Vec::new();
    for (k, qk) in q.iter().enumerate() {
        if qk.is_zero() {
            continue;
        }
        let q_node = rf_to_node(qk, var);
        let term = if k == 0 {
            q_node
        } else if k == 1 {
            Node::Multiply(Box::new(q_node), Box::new(ln_x.clone()))
        } else {
            Node::Multiply(
                Box::new(q_node),
                Box::new(Node::Power(
                    Box::new(ln_x.clone()),
                    Box::new(Node::Num(ExactNum::integer(k as i64))),
                )),
            )
        };
        terms.push(term);
    }

    if terms.is_empty() {
        return Some(RischResult::Elementary(Node::Num(ExactNum::zero())));
    }
    let mut result = terms.remove(0);
    for t in terms {
        result = Node::Add(Box::new(result), Box::new(t));
    }
    Some(RischResult::Elementary(result))
}

/// Integrate a rational function in a single transcendental extension.
/// Uses Hermite reduction + Rothstein-Trager.
/// For exponential extensions, computes the residual after RT and integrates it.
#[allow(dead_code)]
fn integrate_rational_ext(
    num: &ExtPoly,
    den: &ExtPoly,
    ext: &DifferentialExtension,
    var: &str,
) -> Option<RischResult> {
    let hr = hermite_reduce(num, den, var).ok()?;

    let theta_node = match ext.ext_type() {
        ExtensionType::Logarithmic => {
            Node::Function("ln".to_string(), vec![Node::Variable(var.to_string())])
        }
        ExtensionType::Exponential => Node::Function(
            "exp".to_string(),
            vec![ext.argument().numerator().to_node()],
        ),
    };

    let mut result_terms: Vec<Node> = Vec::new();

    // Rational part from Hermite reduction
    if !hr.g_num.is_zero() {
        result_terms.push(Node::Divide(
            Box::new(extpoly_to_node(&hr.g_num, &theta_node, var)),
            Box::new(extpoly_to_node(&hr.g_den, &theta_node, var)),
        ));
    }

    if !hr.h_num.is_zero() {
        if hr.h_den.is_constant() {
            // Polynomial remainder — integrate as polynomial
            let poly_result = match ext.ext_type() {
                ExtensionType::Logarithmic => integrate_poly_log(&hr.h_num, ext, var),
                ExtensionType::Exponential => integrate_poly_exp(&hr.h_num, ext, var),
            };
            match poly_result {
                Some(RischResult::Elementary(n)) => result_terms.push(n),
                Some(RischResult::NonElementary(r)) => return Some(RischResult::NonElementary(r)),
                None => return None,
            }
        } else {
            // Rothstein-Trager on squarefree remainder
            let dd = ext.differentiate(&hr.h_den);
            let rz = rothstein_trager_resultant(&hr.h_den, &hr.h_num, &dd, var);
            let roots = find_constant_roots(&rz, var);

            if roots.is_empty() {
                return Some(RischResult::NonElementary(
                    "No elementary antiderivative exists. \
                     The Rothstein-Trager resultant has no rational roots."
                        .into(),
                ));
            }

            let h_den_deg = hr.h_den.degree().unwrap_or(0);
            let mut gcd_deg_sum = 0;
            let mut log_terms: Vec<(BigRational, ExtPoly)> = Vec::new();

            for c in &roots {
                let c_rf = RationalFunction::from_constant(c.clone(), var);
                let g_c = &hr.h_num - &dd.scalar_mul(&c_rf);
                let v = hr.h_den.gcd(&g_c);
                let v_deg = v.degree().unwrap_or(0);
                gcd_deg_sum += v_deg;
                if v_deg > 0 {
                    log_terms.push((c.clone(), v));
                }
            }

            if gcd_deg_sum != h_den_deg {
                return Some(RischResult::NonElementary(format!(
                    "No elementary antiderivative exists. \
                     Rational residues cover degree {} but denominator has degree {}.",
                    gcd_deg_sum, h_den_deg
                )));
            }

            // Build log terms: Σ cᵢ·ln(vᵢ)
            for (c, v) in &log_terms {
                let v_node = extpoly_to_node(v, &theta_node, var);
                let ln_v = Node::Function("ln".to_string(), vec![v_node]);
                let term = if *c == BigRational::one() {
                    ln_v
                } else {
                    Node::Multiply(Box::new(bigrat_to_node(c)), Box::new(ln_v))
                };
                result_terms.push(term);
            }

            // For exponential extensions: compute residual
            // residual = h_num/h_den - Σ cᵢ·D(vᵢ)/vᵢ
            // If nonzero, integrate the polynomial part
            if matches!(ext.ext_type(), ExtensionType::Exponential) {
                // Compute Σ cᵢ · (h_den/vᵢ) · D(vᵢ) (numerator over common den h_den)
                let mut log_deriv_num = ExtPoly::zero(var);
                for (c, v) in &log_terms {
                    let (w, rem) = hr.h_den.div_rem(v).unwrap();
                    debug_assert!(rem.is_zero(), "v should divide h_den");
                    let dv = ext.differentiate(v);
                    let c_rf = RationalFunction::from_constant(c.clone(), var);
                    log_deriv_num = &log_deriv_num + &(&w * &dv).scalar_mul(&c_rf);
                }
                // residual_num = h_num - log_deriv_num
                let residual_num = &hr.h_num - &log_deriv_num;

                if !residual_num.is_zero() {
                    // Should divide evenly by h_den to give a polynomial
                    let (quotient, remainder) = residual_num.div_rem(&hr.h_den).unwrap();
                    if !remainder.is_zero() {
                        return Some(RischResult::NonElementary(
                            "No elementary antiderivative. \
                             Residual after Rothstein-Trager is not polynomial."
                                .into(),
                        ));
                    }
                    // Integrate the polynomial residual
                    match integrate_poly_exp(&quotient, ext, var) {
                        Some(RischResult::Elementary(n)) => result_terms.push(n),
                        Some(RischResult::NonElementary(r)) => {
                            return Some(RischResult::NonElementary(r))
                        }
                        None => return None,
                    }
                }
            }
        }
    }

    if result_terms.is_empty() {
        return Some(RischResult::Elementary(Node::Num(ExactNum::zero())));
    }
    let mut result = result_terms.remove(0);
    for t in result_terms {
        result = Node::Add(Box::new(result), Box::new(t));
    }
    Some(RischResult::Elementary(result))
}

/// Unified Risch integration via tower builder.
///
/// Replaces try_risch_exponential, try_risch_logarithmic, and try_risch_log_rational
/// with a single entry point that handles both extension types.
pub fn try_risch_tower(expr: &Node, var: &str) -> Option<RischResult> {
    // Try single-extension tower first
    if let Some((num, den, ext)) = build_tower(expr, var) {
        if den == ExtPoly::one(var) {
            return match ext.ext_type() {
                ExtensionType::Logarithmic => integrate_poly_log(&num, &ext, var),
                ExtensionType::Exponential => integrate_poly_exp(&num, &ext, var),
            };
        } else if den.is_constant() {
            // Denominator is a polynomial in x only (constant in θ).
            // Fold it into the numerator's coefficients: num/den → num · (1/den).
            let den_rf = den.coeff(0);
            let inv =
                RationalFunction::new(den_rf.denominator().clone(), den_rf.numerator().clone());
            let adjusted = num.scalar_mul(&inv);
            return match ext.ext_type() {
                ExtensionType::Logarithmic => integrate_poly_log(&adjusted, &ext, var),
                ExtensionType::Exponential => integrate_poly_exp(&adjusted, &ext, var),
            };
        } else {
            return integrate_rational_ext(&num, &den, &ext, var);
        }
    }

    // Try two-level tower (exp over ln)
    try_risch_two_level(expr, var)
}

/// Convert a Node containing both exp(g(x)) and ln(x) into a two-level
/// polynomial: Vec<ExtPoly> indexed by θ₂-degree, where each ExtPoly
/// is a polynomial in θ₁ = ln(x) with Q(x) coefficients.
fn node_to_two_level(expr: &Node, var: &str, exp_arg: &Polynomial) -> Option<Vec<ExtPoly>> {
    match expr {
        Node::Num(n) => {
            if let ExactNum::Rational(val) = n {
                let rf = RationalFunction::from_constant(val.clone(), var);
                Some(vec![ExtPoly::from_rf(rf)])
            } else {
                None
            }
        }
        Node::Variable(v) if v == var => {
            let rf = RationalFunction::from_poly(Polynomial::x(var));
            Some(vec![ExtPoly::from_rf(rf)])
        }
        Node::Variable(_) => None,
        Node::Function(name, args) if name == "exp" && args.len() == 1 => {
            if let Ok(arg_poly) = Polynomial::from_node(&args[0], var) {
                if arg_poly == *exp_arg {
                    return Some(vec![ExtPoly::zero(var), ExtPoly::one(var)]);
                }
            }
            None
        }
        Node::Function(name, args) if name == "ln" && args.len() == 1 => {
            if let Node::Variable(v) = &args[0] {
                if v == var {
                    return Some(vec![ExtPoly::theta(var)]);
                }
            }
            None
        }
        Node::Power(base, exp) => {
            // Handle ln(x)^n
            if let Node::Function(name, args) = base.as_ref() {
                if name == "ln" && args.len() == 1 {
                    if let Node::Variable(v) = &args[0] {
                        if v == var {
                            if let Node::Num(n) = exp.as_ref() {
                                if let Some(e) = n.to_i64() {
                                    if e >= 1 {
                                        let mut r = ExtPoly::theta(var);
                                        for _ in 1..e {
                                            r = &r * &ExtPoly::theta(var);
                                        }
                                        return Some(vec![r]);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // Handle x^n
            if let Node::Variable(v) = base.as_ref() {
                if v == var {
                    if let Node::Num(n) = exp.as_ref() {
                        if let Some(e) = n.to_i64() {
                            if e >= 1 {
                                let p = Polynomial::monomial(BigRational::one(), e as usize, var);
                                return Some(vec![ExtPoly::from_rf(RationalFunction::from_poly(
                                    p,
                                ))]);
                            }
                        }
                    }
                }
            }
            None
        }
        Node::Negate(inner) => {
            let v = node_to_two_level(inner, var, exp_arg)?;
            Some(negate_two_level(&v))
        }
        Node::Add(l, r) => {
            let left = node_to_two_level(l, var, exp_arg)?;
            let right = node_to_two_level(r, var, exp_arg)?;
            Some(add_two_level(&left, &right, var))
        }
        Node::Subtract(l, r) => {
            let left = node_to_two_level(l, var, exp_arg)?;
            let right = node_to_two_level(r, var, exp_arg)?;
            let neg_right = negate_two_level(&right);
            Some(add_two_level(&left, &neg_right, var))
        }
        Node::Multiply(l, r) => {
            let left = node_to_two_level(l, var, exp_arg)?;
            let right = node_to_two_level(r, var, exp_arg)?;
            Some(mul_two_level(&left, &right, var))
        }
        Node::Divide(num, den) => {
            let den_poly = Polynomial::from_node(den, var).ok()?;
            if den_poly.is_zero() {
                return None;
            }
            let num_v = node_to_two_level(num, var, exp_arg)?;
            let inv = RationalFunction::new(Polynomial::one(var), den_poly);
            let inv_ep = ExtPoly::from_rf(inv);
            Some(scalar_mul_two_level(&num_v, &inv_ep))
        }
        _ => None,
    }
}

/// Parse an expression as polynomial in θ₂ = ln(h(x, θ₁)) with ExtPoly (θ₁=exp(g)) coefficients.
/// Returns Vec<ExtPoly> where index i = coefficient of θ₂ⁱ.
fn node_to_two_level_log_over_exp(
    expr: &Node,
    var: &str,
    exp_arg: &Polynomial,
    h: &ExtPoly,
) -> Option<Vec<ExtPoly>> {
    let kind = ExtensionKind::Exponential(exp_arg.clone());
    match expr {
        Node::Num(n) => {
            if let ExactNum::Rational(val) = n {
                let rf = RationalFunction::from_constant(val.clone(), var);
                Some(vec![ExtPoly::from_rf(rf)])
            } else {
                None
            }
        }
        Node::Variable(v) if v == var => {
            let rf = RationalFunction::from_poly(Polynomial::x(var));
            Some(vec![ExtPoly::from_rf(rf)])
        }
        Node::Variable(_) => None,
        Node::Function(name, args) if name == "exp" && args.len() == 1 => {
            // exp(g(x)) → θ₁ as coefficient (degree 0 in θ₂)
            if let Ok(arg_poly) = Polynomial::from_node(&args[0], var) {
                if arg_poly == *exp_arg {
                    let theta1 = ExtPoly::from_coeffs(
                        vec![RationalFunction::zero(var), RationalFunction::one(var)],
                        var,
                    );
                    return Some(vec![theta1]);
                }
            }
            None
        }
        Node::Function(name, args) if name == "ln" && args.len() == 1 => {
            // Check if this is ln(h) — our θ₂
            let arg_ep = node_to_extpoly_general(&args[0], var, &kind)?;
            if arg_ep == *h {
                return Some(vec![ExtPoly::zero(var), ExtPoly::one(var)]);
            }
            None
        }
        Node::Power(base, exp_node) => {
            // Handle ln(h)^n = θ₂^n
            if let Node::Function(name, args) = base.as_ref() {
                if name == "ln" && args.len() == 1 {
                    let arg_ep = node_to_extpoly_general(&args[0], var, &kind)?;
                    if arg_ep == *h {
                        if let Node::Num(n) = exp_node.as_ref() {
                            if let Some(e) = n.to_i64() {
                                if e >= 1 {
                                    let mut result = vec![ExtPoly::zero(var); e as usize + 1];
                                    result[e as usize] = ExtPoly::one(var);
                                    return Some(result);
                                }
                            }
                        }
                    }
                }
            }
            // Handle exp(g)^n = θ₁^n
            if let Node::Function(name, args) = base.as_ref() {
                if name == "exp" && args.len() == 1 {
                    if let Ok(arg_poly) = Polynomial::from_node(&args[0], var) {
                        if arg_poly == *exp_arg {
                            if let Node::Num(n) = exp_node.as_ref() {
                                if let Some(e) = n.to_i64() {
                                    if e >= 1 {
                                        let mut coeffs =
                                            vec![RationalFunction::zero(var); e as usize + 1];
                                        coeffs[e as usize] = RationalFunction::one(var);
                                        return Some(vec![ExtPoly::from_coeffs(coeffs, var)]);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // Handle x^n
            if let Node::Variable(v) = base.as_ref() {
                if v == var {
                    if let Node::Num(n) = exp_node.as_ref() {
                        if let Some(e) = n.to_i64() {
                            if e >= 1 {
                                let p = Polynomial::monomial(BigRational::one(), e as usize, var);
                                return Some(vec![ExtPoly::from_rf(RationalFunction::from_poly(
                                    p,
                                ))]);
                            }
                        }
                    }
                }
            }
            None
        }
        Node::Negate(inner) => {
            let v = node_to_two_level_log_over_exp(inner, var, exp_arg, h)?;
            Some(negate_two_level(&v))
        }
        Node::Add(l, r) => {
            let left = node_to_two_level_log_over_exp(l, var, exp_arg, h)?;
            let right = node_to_two_level_log_over_exp(r, var, exp_arg, h)?;
            Some(add_two_level(&left, &right, var))
        }
        Node::Subtract(l, r) => {
            let left = node_to_two_level_log_over_exp(l, var, exp_arg, h)?;
            let right = node_to_two_level_log_over_exp(r, var, exp_arg, h)?;
            Some(sub_two_level(&left, &right, var))
        }
        Node::Multiply(l, r) => {
            let left = node_to_two_level_log_over_exp(l, var, exp_arg, h)?;
            let right = node_to_two_level_log_over_exp(r, var, exp_arg, h)?;
            Some(mul_two_level(&left, &right, var))
        }
        Node::Divide(num, den) => {
            // Only handle x-polynomial denominators
            let den_poly = Polynomial::from_node(den, var).ok()?;
            if den_poly.is_zero() {
                return None;
            }
            let num_v = node_to_two_level_log_over_exp(num, var, exp_arg, h)?;
            let inv = RationalFunction::new(Polynomial::one(var), den_poly);
            let inv_ep = ExtPoly::from_rf(inv);
            Some(scalar_mul_two_level(&num_v, &inv_ep))
        }
        _ => None,
    }
}

fn negate_two_level(v: &[ExtPoly]) -> Vec<ExtPoly> {
    v.iter().map(|c| -c).collect()
}

fn add_two_level(a: &[ExtPoly], b: &[ExtPoly], var: &str) -> Vec<ExtPoly> {
    let len = a.len().max(b.len());
    let mut result = Vec::with_capacity(len);
    for i in 0..len {
        let ai = a.get(i).cloned().unwrap_or_else(|| ExtPoly::zero(var));
        let bi = b.get(i).cloned().unwrap_or_else(|| ExtPoly::zero(var));
        result.push(&ai + &bi);
    }
    while result.last().is_some_and(|c| c.is_zero()) {
        result.pop();
    }
    if result.is_empty() {
        result.push(ExtPoly::zero(var));
    }
    result
}

fn mul_two_level(a: &[ExtPoly], b: &[ExtPoly], var: &str) -> Vec<ExtPoly> {
    if a.is_empty() || b.is_empty() {
        return vec![ExtPoly::zero(var)];
    }
    let result_len = a.len() + b.len() - 1;
    let mut result = vec![ExtPoly::zero(var); result_len];
    for (i, ai) in a.iter().enumerate() {
        if ai.is_zero() {
            continue;
        }
        for (j, bj) in b.iter().enumerate() {
            if bj.is_zero() {
                continue;
            }
            let product = ai * bj;
            result[i + j] = &result[i + j] + &product;
        }
    }
    while result.last().is_some_and(|c| c.is_zero()) {
        result.pop();
    }
    if result.is_empty() {
        result.push(ExtPoly::zero(var));
    }
    result
}

fn scalar_mul_two_level(v: &[ExtPoly], s: &ExtPoly) -> Vec<ExtPoly> {
    let var = s.variable().to_string();
    let mut r: Vec<ExtPoly> = v.iter().map(|c| c * s).collect();
    while r.last().is_some_and(|c| c.is_zero()) {
        r.pop();
    }
    if r.is_empty() {
        r.push(ExtPoly::zero(&var));
    }
    r
}

/// Solve the Risch DE q' + f·q = g in the logarithmic extension field Q(x)[θ₁],
/// where θ₁ = ln(x) and f ∈ Q[x].
///
/// Returns q ∈ Q(x)[θ₁] as an ExtPoly, or None if no solution exists.
///
/// The derivative q' uses the tower rule: d/dx[θ₁] = 1/x, so
/// d/dx[Σ bₖ·θ₁ᵏ] = Σ (bₖ' + (k+1)·b_{k+1}/x)·θ₁ᵏ.
///
/// At each θ₁-degree k (top-down from n):
///   bₖ' + f·bₖ = gₖ − (k+1)·b_{k+1}/x
///
/// Each is a standard Risch DE over Q(x), solved by `solve_risch_de_rational`.
fn solve_risch_de_in_log_ext(f: &Polynomial, g: &ExtPoly, var: &str) -> Option<ExtPoly> {
    if g.is_zero() {
        return Some(ExtPoly::zero(var));
    }

    let n = g.degree().unwrap_or(0);
    let mut b: Vec<RationalFunction> = vec![RationalFunction::zero(var); n + 1];
    let x_rf = RationalFunction::from_poly(Polynomial::x(var));

    for k in (0..=n).rev() {
        let g_k = g.coeff(k);

        // Correction from higher degree: (k+1)·b_{k+1}/x
        let correction = if k < n {
            let scale = BigRational::from_integer(BigInt::from(k as i64 + 1));
            let scaled_b = &b[k + 1] * &RationalFunction::from_constant(scale, var);
            match scaled_b.checked_div(&x_rf) {
                Ok(result) => result,
                Err(_) => return None,
            }
        } else {
            RationalFunction::zero(var)
        };

        let rhs = &g_k - &correction;

        if rhs.is_zero() && f.is_zero() {
            continue;
        }

        match solve_risch_de_rational(f, &rhs, var) {
            Some(bk) => b[k] = bk,
            None => return None,
        }
    }

    Some(ExtPoly::from_coeffs(b, var))
}

/// Integrate a polynomial in θ₂ = exp(g(x)) whose coefficients are
/// ExtPolys in θ₁ = ln(x) with Q(x) coefficients.
///
/// For each θ₂-degree i:
/// - i = 0: integrate in Q(x, θ₁) via `integrate_poly_log`
/// - i ≥ 1: solve Risch DE qᵢ' + i·g'·qᵢ = aᵢ via `solve_risch_de_in_log_ext`
///
/// Returns the antiderivative as a Node, or proves non-elementarity.
fn integrate_two_level_exp_log(
    outer_coeffs: &[ExtPoly],
    inner_ext: &DifferentialExtension,
    outer_ext: &DifferentialExtension,
    var: &str,
) -> Option<RischResult> {
    if outer_coeffs.is_empty() {
        return Some(RischResult::Elementary(Node::Num(ExactNum::zero())));
    }

    // All coefficients zero?
    if outer_coeffs.iter().all(|c| c.is_zero()) {
        return Some(RischResult::Elementary(Node::Num(ExactNum::zero())));
    }

    let g_prime_rf = outer_ext.argument().derivative();
    if *g_prime_rf.denominator() != Polynomial::one(var) {
        return None;
    }
    let g_prime = g_prime_rf.numerator().clone();
    let g_node = outer_ext.argument().numerator().to_node();

    let ln_x = Node::Function("ln".to_string(), vec![Node::Variable(var.to_string())]);
    let mut result_terms: Vec<Node> = Vec::new();

    for (i, a_i) in outer_coeffs.iter().enumerate() {
        if a_i.is_zero() {
            continue;
        }

        if i == 0 {
            match integrate_poly_log(a_i, inner_ext, var) {
                Some(RischResult::Elementary(node)) => {
                    result_terms.push(node);
                }
                Some(RischResult::NonElementary(reason)) => {
                    return Some(RischResult::NonElementary(reason));
                }
                None => return None,
            }
        } else {
            let f_scaled = g_prime.scalar_mul(&BigRational::from_integer(BigInt::from(i as i64)));
            match solve_risch_de_in_log_ext(&f_scaled, a_i, var) {
                Some(qi) => {
                    let qi_node = extpoly_to_node(&qi, &ln_x, var);
                    let exp_g = Node::Function("exp".to_string(), vec![g_node.clone()]);
                    let exp_part = if i == 1 {
                        exp_g
                    } else {
                        Node::Power(
                            Box::new(exp_g),
                            Box::new(Node::Num(ExactNum::integer(i as i64))),
                        )
                    };
                    let term = Node::Multiply(Box::new(qi_node), Box::new(exp_part));
                    result_terms.push(term);
                }
                None => {
                    return Some(RischResult::NonElementary(format!(
                        "No elementary antiderivative exists. \
                         The Risch DE q' + ({})·q = {} has no solution in Q(x, ln(x)), \
                         so the integral cannot be expressed in terms of elementary functions.",
                        f_scaled, a_i
                    )));
                }
            }
        }
    }

    if result_terms.is_empty() {
        return Some(RischResult::Elementary(Node::Num(ExactNum::zero())));
    }
    let mut result = result_terms.remove(0);
    for t in result_terms {
        result = Node::Add(Box::new(result), Box::new(t));
    }
    Some(RischResult::Elementary(result))
}

/// Try to integrate via a two-level tower (exp on top of ln).
///
/// Called when `build_tower` returns None because both exp and ln are present.
fn try_risch_two_level(expr: &Node, var: &str) -> Option<RischResult> {
    // Try log-over-exp tower: ln(h(x, exp(g(x))))
    if let Some((exp_poly_loe, h_loe)) = find_ln_of_exp_argument(expr, var) {
        if let Some(outer_coeffs) = node_to_two_level_log_over_exp(expr, var, &exp_poly_loe, &h_loe)
        {
            let inner_ext = DifferentialExtension::exponential(
                RationalFunction::from_poly(exp_poly_loe.clone()),
                var,
            );
            if let Some(result) =
                integrate_two_level_log_over_exp(&outer_coeffs, &inner_ext, &h_loe, var)
            {
                return Some(result);
            }
        }
        // Try after simplification
        let env = crate::environment::Environment::new();
        let simplified =
            crate::simplify::Simplifiable::simplify(expr, &env).unwrap_or_else(|_| expr.clone());
        if let Some((exp_poly2, h2)) = find_ln_of_exp_argument(&simplified, var) {
            if let Some(outer_coeffs) =
                node_to_two_level_log_over_exp(&simplified, var, &exp_poly2, &h2)
            {
                let inner_ext =
                    DifferentialExtension::exponential(RationalFunction::from_poly(exp_poly2), var);
                if let Some(result) =
                    integrate_two_level_log_over_exp(&outer_coeffs, &inner_ext, &h2, var)
                {
                    return Some(result);
                }
            }
        }
    }

    let has_ln = contains_ln(expr, var);
    let exp_arg = find_exp_argument(expr, var);

    let exp_poly = match (has_ln, exp_arg) {
        (true, Some(g)) => g,
        _ => return None,
    };

    // Try rational case first: expression has Divide with θ₂ in denominator
    if let Some((num_tl, den_tl)) = extract_two_level_rational(expr, var, &exp_poly) {
        if let Some(den_ep) = two_level_to_extpoly(&den_tl, var) {
            let inner_ext = DifferentialExtension::logarithmic(
                RationalFunction::from_poly(Polynomial::x(var)),
                var,
            );
            let outer_ext = DifferentialExtension::exponential(
                RationalFunction::from_poly(exp_poly.clone()),
                var,
            );
            return integrate_rational_two_level(&num_tl, &den_ep, &inner_ext, &outer_ext, var);
        }
    }

    // Try after simplification for rational case
    {
        let env = crate::environment::Environment::new();
        let simplified =
            crate::simplify::Simplifiable::simplify(expr, &env).unwrap_or_else(|_| expr.clone());
        if let Some((num_tl, den_tl)) = extract_two_level_rational(&simplified, var, &exp_poly) {
            if let Some(den_ep) = two_level_to_extpoly(&den_tl, var) {
                let inner_ext = DifferentialExtension::logarithmic(
                    RationalFunction::from_poly(Polynomial::x(var)),
                    var,
                );
                let outer_ext = DifferentialExtension::exponential(
                    RationalFunction::from_poly(exp_poly.clone()),
                    var,
                );
                return integrate_rational_two_level(&num_tl, &den_ep, &inner_ext, &outer_ext, var);
            }
        }
    }

    // Polynomial case (existing code)
    let build_result = node_to_two_level(expr, var, &exp_poly);

    let outer_coeffs = match build_result {
        Some(c) => c,
        None => {
            let env = crate::environment::Environment::new();
            let simplified = crate::simplify::Simplifiable::simplify(expr, &env)
                .unwrap_or_else(|_| expr.clone());
            node_to_two_level(&simplified, var, &exp_poly)?
        }
    };

    let inner_ext =
        DifferentialExtension::logarithmic(RationalFunction::from_poly(Polynomial::x(var)), var);
    let outer_ext = DifferentialExtension::exponential(RationalFunction::from_poly(exp_poly), var);

    integrate_two_level_exp_log(&outer_coeffs, &inner_ext, &outer_ext, var)
}

/// Extract numerator and denominator as two-level polynomials from a
/// Node that is a rational function in θ₂ = exp(g(x)) with θ₁ = ln(x) coefficients.
///
/// Returns Some((num, den)) where both are Vec<ExtPoly>, or None if the
/// expression is not a recognizable rational function in θ₂ with a
/// non-trivial denominator.
fn extract_two_level_rational(
    expr: &Node,
    var: &str,
    exp_arg: &Polynomial,
) -> Option<(Vec<ExtPoly>, Vec<ExtPoly>)> {
    match expr {
        Node::Divide(num_node, den_node) => {
            let num = node_to_two_level(num_node, var, exp_arg)?;
            let den = node_to_two_level(den_node, var, exp_arg)?;
            if den.len() <= 1 {
                return None;
            }
            Some((num, den))
        }
        Node::Multiply(left, right) => {
            // a * (b/c) where c has θ₂
            if let Node::Divide(n, d) = right.as_ref() {
                let d_tl = node_to_two_level(d, var, exp_arg)?;
                if d_tl.len() > 1 {
                    let l_tl = node_to_two_level(left, var, exp_arg)?;
                    let n_tl = node_to_two_level(n, var, exp_arg)?;
                    return Some((mul_two_level(&l_tl, &n_tl, var), d_tl));
                }
            }
            // (a/b) * c where b has θ₂
            if let Node::Divide(n, d) = left.as_ref() {
                let d_tl = node_to_two_level(d, var, exp_arg)?;
                if d_tl.len() > 1 {
                    let r_tl = node_to_two_level(right, var, exp_arg)?;
                    let n_tl = node_to_two_level(n, var, exp_arg)?;
                    return Some((mul_two_level(&r_tl, &n_tl, var), d_tl));
                }
            }
            None
        }
        _ => None,
    }
}

fn sub_two_level(a: &[ExtPoly], b: &[ExtPoly], var: &str) -> Vec<ExtPoly> {
    let neg_b = negate_two_level(b);
    add_two_level(a, &neg_b, var)
}

/// Compute gcd(d, g_c) where d ∈ Q(x)[θ₂] and g_c ∈ Q(x)[θ₁][θ₂].
///
/// Since θ₁ is transcendental over Q(x), the GCD equals gcd(d, r₀, r₁, ...)
/// where rⱼ is the coefficient of θ₁ʲ in g_c, extracted as a standard ExtPoly.
fn gcd_extpoly_with_two_level(d: &ExtPoly, g_c: &[ExtPoly], var: &str) -> ExtPoly {
    let max_theta1_deg = g_c.iter().filter_map(|ep| ep.degree()).max().unwrap_or(0);

    let mut result = d.clone();
    for j in 0..=max_theta1_deg {
        let rj_coeffs: Vec<RationalFunction> = g_c.iter().map(|ep| ep.coeff(j)).collect();
        let rj = ExtPoly::from_coeffs(rj_coeffs, var);
        if !rj.is_zero() {
            result = result.gcd(&rj);
            if result.is_constant() {
                break;
            }
        }
    }
    result
}

/// Compute the θ₁-content of a two-level polynomial: gcd of all θ₂-coefficients
/// as ExtPolys in θ₁.
///
/// Returns the content (an ExtPoly in θ₁). Returns a constant (degree 0) ExtPoly
/// if the coefficients have no common θ₁ factor.
#[allow(dead_code)] // Used by two-level denominator factoring (upcoming)
fn compute_theta1_content(tl: &[ExtPoly], var: &str) -> ExtPoly {
    let mut result: Option<ExtPoly> = None;
    for ep in tl {
        if ep.is_zero() {
            continue;
        }
        result = Some(match result {
            None => ep.clone(),
            Some(acc) => acc.gcd(ep),
        });
    }
    result.unwrap_or_else(|| ExtPoly::one(var))
}

/// Result of two-level Hermite reduction.
struct HermiteResultTwoLevel {
    /// Numerator of rational part (θ₁-structured, indexed by θ₂ degree)
    g_num: Vec<ExtPoly>,
    /// Denominator of rational part (standard ExtPoly in θ₂)
    g_den: ExtPoly,
    /// Numerator of integrand remainder (θ₁-structured)
    h_num: Vec<ExtPoly>,
    /// Squarefree denominator (standard ExtPoly in θ₂)
    h_den: ExtPoly,
}

/// Hermite reduction for two-level integrand A/D where A has θ₁ coefficients
/// and D ∈ Q(x)[θ₂].
///
/// Exploits linearity: runs existing `hermite_reduce` on each θ₁-degree
/// independently, then combines. The h_den is identical for all θ₁-degrees.
fn hermite_reduce_two_level(
    num: &[ExtPoly],
    den: &ExtPoly,
    var: &str,
) -> Result<HermiteResultTwoLevel, String> {
    // Find max θ₁-degree across all θ₂-coefficients
    let max_theta1_deg = num.iter().filter_map(|ep| ep.degree()).max().unwrap_or(0);

    // For each θ₁-degree j, extract Aⱼ (standard ExtPoly in θ₂) and run Hermite reduction
    let mut all_results: Vec<HermiteResult> = Vec::new();

    for j in 0..=max_theta1_deg {
        // Aⱼ[i] = num[i].coeff(j) — Q(x) coefficient of θ₁ʲ at θ₂ⁱ
        let a_j_coeffs: Vec<RationalFunction> = (0..num.len()).map(|i| num[i].coeff(j)).collect();
        let a_j = ExtPoly::from_coeffs(a_j_coeffs, var);
        let hr = hermite_reduce(&a_j, den, var)?;
        all_results.push(hr);
    }

    // h_den is the same for all j
    let h_den = all_results[0].h_den.clone();

    // Combine g_den using LCM
    let mut g_den = ExtPoly::one(var);
    for hr in &all_results {
        if !hr.g_num.is_zero() {
            let g = g_den.gcd(&hr.g_den);
            if !g.is_zero() {
                let (factor, _) = hr.g_den.div_rem(&g)?;
                g_den = &g_den * &factor;
            }
        }
    }

    // Build output: distribute θ₁ʲ coefficients
    let max_g_theta2 = all_results
        .iter()
        .filter_map(|hr| hr.g_num.degree())
        .max()
        .unwrap_or(0);
    let max_h_theta2 = all_results
        .iter()
        .filter_map(|hr| hr.h_num.degree())
        .max()
        .unwrap_or(0);

    let mut g_num_out = vec![ExtPoly::zero(var); max_g_theta2 + 1];
    let mut h_num_out = vec![ExtPoly::zero(var); max_h_theta2 + 1];

    for (j, hr) in all_results.iter().enumerate() {
        // Scale g_num by common denominator factor
        let scale = if hr.g_num.is_zero() {
            ExtPoly::one(var)
        } else {
            let g = g_den.gcd(&hr.g_den);
            if g.is_zero() {
                ExtPoly::one(var)
            } else {
                let (s, _) = g_den.div_rem(&hr.g_den)?;
                s
            }
        };
        let scaled_g = &hr.g_num * &scale;

        for (i, g_out) in g_num_out.iter_mut().enumerate().take(max_g_theta2 + 1) {
            let coeff = scaled_g.coeff(i);
            if !coeff.is_zero() {
                let mut new_coeffs = vec![RationalFunction::zero(var); j + 1];
                new_coeffs[j] = coeff;
                let term = ExtPoly::from_coeffs(new_coeffs, var);
                *g_out = &*g_out + &term;
            }
        }

        for (i, h_out) in h_num_out.iter_mut().enumerate().take(max_h_theta2 + 1) {
            let coeff = hr.h_num.coeff(i);
            if !coeff.is_zero() {
                let mut new_coeffs = vec![RationalFunction::zero(var); j + 1];
                new_coeffs[j] = coeff;
                let term = ExtPoly::from_coeffs(new_coeffs, var);
                *h_out = &*h_out + &term;
            }
        }
    }

    // Strip trailing zeros
    while g_num_out.last().is_some_and(|c| c.is_zero()) {
        g_num_out.pop();
    }
    while h_num_out.last().is_some_and(|c| c.is_zero()) {
        h_num_out.pop();
    }
    if g_num_out.is_empty() {
        g_num_out.push(ExtPoly::zero(var));
    }
    if h_num_out.is_empty() {
        h_num_out.push(ExtPoly::zero(var));
    }

    Ok(HermiteResultTwoLevel {
        g_num: g_num_out,
        g_den,
        h_num: h_num_out,
        h_den,
    })
}

/// Two-level Rothstein-Trager resultant: R(z) = res_θ₂(d, a − z·D(d)).
///
/// d ∈ Q(x)[θ₂], a has θ₁ coefficients (Vec<ExtPoly>), D(d) ∈ Q(x)[θ₂].
/// Returns R(z) as Vec<ExtPoly> — polynomial in z with ExtPoly-in-θ₁ coefficients.
fn rothstein_trager_two_level(
    d: &ExtPoly,
    a: &[ExtPoly],
    dd: &ExtPoly,
    content: Option<&ExtPoly>,
    var: &str,
) -> Vec<ExtPoly> {
    let m = d.degree().unwrap_or(0);
    let n = {
        let da = if a.is_empty() { 0 } else { a.len() - 1 };
        let ddd = dd.degree().unwrap_or(0);
        da.max(ddd)
    };

    if m == 0 && n == 0 {
        let c0 = a.first().cloned().unwrap_or_else(|| ExtPoly::zero(var));
        let dd_c0 = dd.coeff(0);
        let c1 = match content {
            Some(c) => {
                let scaled = c.scalar_mul(&dd_c0);
                -&scaled
            }
            None => ExtPoly::from_rf(-&dd_c0),
        };
        return vec![c0, c1];
    }

    let size = m + n;
    if size == 0 {
        return vec![ExtPoly::one(var)];
    }

    let zero_z: Vec<ExtPoly> = vec![ExtPoly::zero(var)];
    let mut matrix: Vec<Vec<Vec<ExtPoly>>> = Vec::with_capacity(size);

    // First n rows from d (constant in z, no θ₁)
    for i in 0..n {
        let mut row = vec![zero_z.clone(); size];
        for k in 0..=m {
            let col = i + k;
            if col < size {
                row[col] = vec![ExtPoly::from_rf(d.coeff(m - k))];
            }
        }
        matrix.push(row);
    }

    // Last m rows from g = a − z·dd (linear in z, θ₁ in constant term)
    for i in 0..m {
        let mut row = vec![zero_z.clone(); size];
        for k in 0..=n {
            let col = i + k;
            if col < size {
                let a_coeff = a.get(n - k).cloned().unwrap_or_else(|| ExtPoly::zero(var));
                let dd_coeff = dd.coeff(n - k);
                if dd_coeff.is_zero() {
                    row[col] = vec![a_coeff];
                } else {
                    let z_coeff = match content {
                        Some(c) => {
                            let scaled = c.scalar_mul(&dd_coeff);
                            -&scaled
                        }
                        None => ExtPoly::from_rf(-&dd_coeff),
                    };
                    row[col] = vec![a_coeff, z_coeff];
                }
            }
        }
        matrix.push(row);
    }

    two_level_det(&matrix, var)
}

/// Determinant of a square matrix whose entries are Vec<ExtPoly>
/// (polynomials in z with ExtPoly coefficients).
fn two_level_det(m: &[Vec<Vec<ExtPoly>>], var: &str) -> Vec<ExtPoly> {
    let n = m.len();
    if n == 0 {
        return vec![ExtPoly::one(var)];
    }
    if n == 1 {
        return m[0][0].clone();
    }
    if n == 2 {
        let a = mul_two_level(&m[0][0], &m[1][1], var);
        let b = mul_two_level(&m[0][1], &m[1][0], var);
        return sub_two_level(&a, &b, var);
    }
    let mut result = vec![ExtPoly::zero(var)];
    for j in 0..n {
        if m[0][j].iter().all(|ep| ep.is_zero()) {
            continue;
        }
        let minor: Vec<Vec<Vec<ExtPoly>>> = (1..n)
            .map(|row| {
                (0..n)
                    .filter(|&col| col != j)
                    .map(|col| m[row][col].clone())
                    .collect()
            })
            .collect();
        let cofactor = two_level_det(&minor, var);
        let term = mul_two_level(&m[0][j], &cofactor, var);
        if j % 2 == 0 {
            result = add_two_level(&result, &term, var);
        } else {
            result = sub_two_level(&result, &term, var);
        }
    }
    result
}

/// Find constant roots c ∈ Q of R(z) where R has ExtPoly (θ₁) coefficients.
///
/// Strategy: specialize x, evaluate θ₁-degree-0 coefficients to get Q[z],
/// find rational roots, verify as ExtPoly identities.
fn find_constant_roots_two_level(rz: &[ExtPoly], var: &str) -> Vec<BigRational> {
    let deg = match rz.len().checked_sub(1) {
        Some(d) if d > 0 => d,
        _ => return vec![],
    };

    if rz.iter().all(|ep| ep.is_zero()) {
        return vec![];
    }

    let candidates_x = [2i64, 3, 5, 7, 11];
    let mut candidate_roots: Option<Vec<BigRational>> = None;

    for &x_val in &candidates_x {
        let x_br = BigRational::from_integer(BigInt::from(x_val));
        let mut spec_coeffs = Vec::with_capacity(deg + 1);
        let mut valid = true;
        for rz_k in rz.iter().take(deg + 1) {
            // Evaluate θ₁-degree-0 coefficient at x = x₀
            let rf_at_0 = rz_k.coeff(0);
            match rf_at_0.evaluate(&x_br) {
                Some(val) => spec_coeffs.push(val),
                None => {
                    valid = false;
                    break;
                }
            }
        }
        if !valid {
            continue;
        }

        let spec_poly = Polynomial::from_coeffs(spec_coeffs, "z");
        if spec_poly.is_zero() {
            continue;
        }

        candidate_roots = Some(spec_poly.rational_roots());
        break;
    }

    let candidates = match candidate_roots {
        Some(c) => c,
        None => return vec![],
    };

    // Verify: R(c) must be zero as ExtPoly (all θ₁-coefficients vanish)
    let mut verified = Vec::new();
    for c in candidates {
        let mut sum = ExtPoly::zero(var);
        let mut c_power = BigRational::one();
        for rz_k in rz.iter().take(deg + 1) {
            let scaled = rz_k.scalar_mul(&RationalFunction::from_constant(c_power.clone(), var));
            sum = &sum + &scaled;
            c_power = &c_power * &c;
        }
        if sum.is_zero() && !verified.contains(&c) {
            verified.push(c);
        }
    }

    verified
}

/// Polynomial long division: two-level numerator / standard ExtPoly denominator.
/// Requires den's leading coefficient to be invertible in Q(x).
fn div_rem_two_level_by_extpoly(
    num: &[ExtPoly],
    den: &ExtPoly,
    var: &str,
) -> Option<(Vec<ExtPoly>, Vec<ExtPoly>)> {
    if den.is_zero() {
        return None;
    }
    let den_deg = den.degree().unwrap();

    // If numerator degree < denominator degree, quotient is 0
    let num_effective_len = {
        let mut l = num.len();
        while l > 0 && num[l - 1].is_zero() {
            l -= 1;
        }
        l
    };
    if num_effective_len == 0 || num_effective_len <= den_deg {
        return Some((vec![ExtPoly::zero(var)], num.to_vec()));
    }

    let den_lc = den.leading_coeff().unwrap();
    let den_lc_inv = RationalFunction::one(var).checked_div(den_lc).ok()?;

    let mut remainder = num.to_vec();
    let num_deg = num_effective_len - 1;
    let mut quotient = vec![ExtPoly::zero(var); num_deg - den_deg + 1];

    loop {
        // Find effective degree of remainder
        let rem_deg = {
            let mut d = remainder.len();
            while d > 0 && remainder[d - 1].is_zero() {
                d -= 1;
            }
            if d == 0 || d - 1 < den_deg {
                break;
            }
            d - 1
        };

        let rem_lc = remainder[rem_deg].clone();
        let q_coeff = rem_lc.scalar_mul(&den_lc_inv);
        let deg_diff = rem_deg - den_deg;
        quotient[deg_diff] = q_coeff.clone();

        // Subtract q_coeff * den (shifted by deg_diff) from remainder
        for k in 0..=den_deg {
            let den_k = den.coeff(k);
            if den_k.is_zero() {
                continue;
            }
            let sub = q_coeff.scalar_mul(&den_k);
            let idx = deg_diff + k;
            if idx < remainder.len() {
                remainder[idx] = &remainder[idx] - &sub;
            }
        }
    }

    // Strip trailing zeros from remainder
    while remainder.last().is_some_and(|c| c.is_zero()) {
        remainder.pop();
    }
    if remainder.is_empty() {
        remainder.push(ExtPoly::zero(var));
    }

    Some((quotient, remainder))
}

/// Convert a two-level polynomial to a standard ExtPoly, if all
/// coefficients are in Q(x) (no θ₁ terms).
fn two_level_to_extpoly(tl: &[ExtPoly], var: &str) -> Option<ExtPoly> {
    let coeffs: Option<Vec<RationalFunction>> = tl
        .iter()
        .map(|ep| {
            if ep.degree().unwrap_or(0) > 0 {
                None
            } else {
                Some(ep.coeff(0))
            }
        })
        .collect();
    Some(ExtPoly::from_coeffs(coeffs?, var))
}

/// Convert a two-level polynomial to a Node expression.
/// Produces: Σ cᵢ(θ₁) · θ₂ⁱ where cᵢ is rendered via extpoly_to_node.
fn two_level_to_node(
    coeffs: &[ExtPoly],
    theta1_node: &Node,
    theta2_node: &Node,
    var: &str,
) -> Node {
    let mut terms: Vec<Node> = Vec::new();
    for (i, ci) in coeffs.iter().enumerate() {
        if ci.is_zero() {
            continue;
        }
        let ci_node = extpoly_to_node(ci, theta1_node, var);
        let term = if i == 0 {
            ci_node
        } else {
            let theta2_power = if i == 1 {
                theta2_node.clone()
            } else {
                Node::Power(
                    Box::new(theta2_node.clone()),
                    Box::new(Node::Num(ExactNum::integer(i as i64))),
                )
            };
            Node::Multiply(Box::new(ci_node), Box::new(theta2_power))
        };
        terms.push(term);
    }
    if terms.is_empty() {
        return Node::Num(ExactNum::zero());
    }
    let mut result = terms.remove(0);
    for t in terms {
        result = Node::Add(Box::new(result), Box::new(t));
    }
    result
}

/// Integrate a rational function in θ₂ = exp(g(x)) with θ₁ = ln(x) coefficients.
///
/// num is Vec<ExtPoly> (polynomial in θ₂ with θ₁ coefficients).
/// den is ExtPoly ∈ Q(x)[θ₂] (no θ₁ in denominator).
///
/// Pipeline: polynomial division → Hermite reduce → RT → result assembly.
fn integrate_rational_two_level(
    num: &[ExtPoly],
    den: &ExtPoly,
    inner_ext: &DifferentialExtension,
    outer_ext: &DifferentialExtension,
    var: &str,
) -> Option<RischResult> {
    if den.is_zero() {
        return None;
    }

    // Polynomial long division
    let (quotient, remainder) = div_rem_two_level_by_extpoly(num, den, var)?;

    let ln_x = Node::Function("ln".to_string(), vec![Node::Variable(var.to_string())]);
    let g_node = outer_ext.argument().numerator().to_node();
    let exp_g = Node::Function("exp".to_string(), vec![g_node]);
    let mut result_terms: Vec<Node> = Vec::new();

    // Integrate the polynomial quotient
    if !quotient.iter().all(|ep| ep.is_zero()) {
        match integrate_two_level_exp_log(&quotient, inner_ext, outer_ext, var) {
            Some(RischResult::Elementary(n)) => result_terms.push(n),
            Some(RischResult::NonElementary(r)) => return Some(RischResult::NonElementary(r)),
            None => return None,
        }
    }

    // Handle proper rational remainder
    if !remainder.iter().all(|ep| ep.is_zero()) {
        // Hermite reduce
        let hr = hermite_reduce_two_level(&remainder, den, var).ok()?;

        // Rational part from Hermite reduction
        if !hr.g_num.iter().all(|ep| ep.is_zero()) {
            let g_num_node = two_level_to_node(&hr.g_num, &ln_x, &exp_g, var);
            let g_den_node = extpoly_to_node(&hr.g_den, &exp_g, var);
            result_terms.push(Node::Divide(Box::new(g_num_node), Box::new(g_den_node)));
        }

        // Squarefree remainder
        if !hr.h_num.iter().all(|ep| ep.is_zero()) {
            if hr.h_den.is_constant() {
                // Polynomial remainder
                match integrate_two_level_exp_log(&hr.h_num, inner_ext, outer_ext, var) {
                    Some(RischResult::Elementary(n)) => result_terms.push(n),
                    Some(RischResult::NonElementary(r)) => {
                        return Some(RischResult::NonElementary(r))
                    }
                    None => return None,
                }
            } else {
                // Rothstein-Trager on squarefree remainder
                let dd = outer_ext.differentiate(&hr.h_den);
                let rz = rothstein_trager_two_level(&hr.h_den, &hr.h_num, &dd, None, var);
                let roots = find_constant_roots_two_level(&rz, var);

                if roots.is_empty() {
                    return Some(RischResult::NonElementary(
                        "No elementary antiderivative exists. \
                         The two-level Rothstein-Trager resultant has no constant roots, \
                         so the integral cannot be expressed in terms of elementary functions."
                            .into(),
                    ));
                }

                // Compute GCD and build log terms for each root
                let h_den_deg = hr.h_den.degree().unwrap_or(0);
                let mut gcd_deg_sum = 0;
                let mut log_terms: Vec<(BigRational, ExtPoly)> = Vec::new();

                for c in &roots {
                    let c_rf = RationalFunction::from_constant(c.clone(), var);
                    // g_c = h_num − c·D(d) as two-level
                    let mut g_c = hr.h_num.clone();
                    let dd_len = dd.degree().map_or(0, |d| d + 1);
                    while g_c.len() < dd_len {
                        g_c.push(ExtPoly::zero(var));
                    }
                    for (i, g_c_i) in g_c.iter_mut().enumerate() {
                        let dd_coeff = dd.coeff(i);
                        if !dd_coeff.is_zero() {
                            let sub = ExtPoly::from_rf(&dd_coeff * &c_rf);
                            *g_c_i = &*g_c_i - &sub;
                        }
                    }

                    let v = gcd_extpoly_with_two_level(&hr.h_den, &g_c, var);
                    let v_deg = v.degree().unwrap_or(0);
                    gcd_deg_sum += v_deg;
                    if v_deg > 0 {
                        log_terms.push((c.clone(), v));
                    }
                }

                if gcd_deg_sum != h_den_deg {
                    return Some(RischResult::NonElementary(format!(
                        "No elementary antiderivative exists. \
                         Rational residues cover degree {} but denominator has degree {}.",
                        gcd_deg_sum, h_den_deg
                    )));
                }

                // Build log terms: Σ cᵢ·ln(vᵢ)
                for (c, v) in &log_terms {
                    let v_node = extpoly_to_node(v, &exp_g, var);
                    let ln_v = Node::Function("ln".to_string(), vec![v_node]);
                    let term = if *c == BigRational::one() {
                        ln_v
                    } else {
                        Node::Multiply(Box::new(bigrat_to_node(c)), Box::new(ln_v))
                    };
                    result_terms.push(term);
                }

                // For exp extensions: compute and integrate residual
                // residual_num = h_num - Σ cᵢ · (h_den/vᵢ) · D(vᵢ)  (over common den h_den)
                let max_len = hr.h_num.len().max(dd.degree().map_or(1, |d| d + 1));
                let mut log_deriv_num = vec![ExtPoly::zero(var); max_len];

                for (c, v) in &log_terms {
                    let (w, rem) = hr.h_den.div_rem(v).unwrap();
                    debug_assert!(rem.is_zero(), "v should divide h_den");
                    let dv = outer_ext.differentiate(v);
                    let w_dv = &w * &dv;
                    let c_rf = RationalFunction::from_constant(c.clone(), var);
                    let scaled = w_dv.scalar_mul(&c_rf);
                    for i in 0..=scaled.degree().unwrap_or(0) {
                        let coeff = scaled.coeff(i);
                        if !coeff.is_zero() {
                            if i >= log_deriv_num.len() {
                                log_deriv_num.resize(i + 1, ExtPoly::zero(var));
                            }
                            let term = ExtPoly::from_rf(coeff);
                            log_deriv_num[i] = &log_deriv_num[i] + &term;
                        }
                    }
                }

                let residual = sub_two_level(&hr.h_num, &log_deriv_num, var);

                if !residual.iter().all(|ep| ep.is_zero()) {
                    let (poly_residual, rem) =
                        div_rem_two_level_by_extpoly(&residual, &hr.h_den, var)?;
                    if !rem.iter().all(|ep| ep.is_zero()) {
                        return Some(RischResult::NonElementary(
                            "No elementary antiderivative. \
                             Residual after two-level Rothstein-Trager is not polynomial."
                                .into(),
                        ));
                    }
                    match integrate_two_level_exp_log(&poly_residual, inner_ext, outer_ext, var) {
                        Some(RischResult::Elementary(n)) => result_terms.push(n),
                        Some(RischResult::NonElementary(r)) => {
                            return Some(RischResult::NonElementary(r))
                        }
                        None => return None,
                    }
                }
            }
        }
    }

    if result_terms.is_empty() {
        return Some(RischResult::Elementary(Node::Num(ExactNum::zero())));
    }
    let mut result = result_terms.remove(0);
    for t in result_terms {
        result = Node::Add(Box::new(result), Box::new(t));
    }
    Some(RischResult::Elementary(result))
}

fn make_ln_power(ln_node: &Node, k: usize) -> Node {
    if k == 0 {
        Node::Num(ExactNum::integer(1))
    } else if k == 1 {
        ln_node.clone()
    } else {
        Node::Power(
            Box::new(ln_node.clone()),
            Box::new(Node::Num(ExactNum::integer(k as i64))),
        )
    }
}

/// Integrate a polynomial in θ₂ = ln(h(x, θ₁)) with ExtPoly (θ₁=exp(g)) coefficients.
///
/// Uses top-down logarithmic polynomial integration:
///   Degree n: D(bₙ) = aₙ (integrate in inner exp extension)
///   Degree k < n: D(bₖ) = aₖ − (k+1)·bₖ₊₁·h'/h
fn integrate_two_level_log_over_exp(
    outer_coeffs: &[ExtPoly],
    inner_ext: &DifferentialExtension,
    h: &ExtPoly,
    var: &str,
) -> Option<RischResult> {
    if outer_coeffs.is_empty() || outer_coeffs.iter().all(|c| c.is_zero()) {
        return Some(RischResult::Elementary(Node::Num(ExactNum::zero())));
    }

    // Effective degree
    let n = {
        let mut d = outer_coeffs.len() - 1;
        while d > 0 && outer_coeffs[d].is_zero() {
            d -= 1;
        }
        d
    };

    if n == 0 {
        // No θ₂ — just integrate in the inner exp extension
        return integrate_poly_exp(&outer_coeffs[0], inner_ext, var);
    }

    // Compute h' = D(h) in the exp extension
    let h_prime = inner_ext.differentiate(h);

    let g_node = inner_ext.argument().numerator().to_node();
    let exp_g = Node::Function("exp".to_string(), vec![g_node]);
    let h_node = extpoly_to_node(h, &exp_g, var);
    let ln_h = Node::Function("ln".to_string(), vec![h_node]);

    let mut result_terms: Vec<Node> = Vec::new();
    let mut b_prev: Option<ExtPoly> = None;

    for k in (0..=n).rev() {
        let a_k = &outer_coeffs[k];

        if k == n {
            // Top degree: D(bₙ) = aₙ
            match integrate_in_exp_ext_structured(a_k, inner_ext, var) {
                Some(b_k) => {
                    if !b_k.is_zero() {
                        let b_node = extpoly_to_node(&b_k, &exp_g, var);
                        let theta2_pow = make_ln_power(&ln_h, k);
                        result_terms.push(Node::Multiply(Box::new(b_node), Box::new(theta2_pow)));
                    }
                    b_prev = Some(b_k);
                }
                None => {
                    return Some(RischResult::NonElementary(format!(
                        "No elementary antiderivative exists. \
                         Cannot integrate the degree-{} coefficient in the inner exp extension.",
                        k
                    )));
                }
            }
        } else {
            // Lower degree: D(bₖ) = aₖ − (k+1)·bₖ₊₁·h'/h
            // = (aₖ·h − (k+1)·bₖ₊₁·h') / h
            let b_prev_ref = b_prev.as_ref().unwrap();
            let scale = BigRational::from_integer(BigInt::from((k + 1) as i64));
            let scale_rf = RationalFunction::from_constant(scale, var);
            let correction = b_prev_ref.scalar_mul(&scale_rf);
            let correction_times_h_prime = &correction * &h_prime;

            let a_k_times_h = a_k * h;
            let rhs_num = &a_k_times_h - &correction_times_h_prime;

            // Try to simplify rhs_num / h by GCD
            let g = rhs_num.gcd(h);
            let (rhs_num_reduced, _) = rhs_num.div_rem(&g).unwrap();
            let (rhs_den_reduced, _) = h.div_rem(&g).unwrap();

            if rhs_den_reduced.is_constant() {
                // Polynomial RHS — integrate structurally
                let den_scalar = rhs_den_reduced.coeff(0);
                let rhs_poly = if den_scalar == RationalFunction::one(var) {
                    rhs_num_reduced
                } else {
                    let inv = RationalFunction::one(var).checked_div(&den_scalar).ok()?;
                    rhs_num_reduced.scalar_mul(&inv)
                };

                match integrate_in_exp_ext_structured(&rhs_poly, inner_ext, var) {
                    Some(b_k_ep) => {
                        if !b_k_ep.is_zero() {
                            let b_node = extpoly_to_node(&b_k_ep, &exp_g, var);
                            if k == 0 {
                                result_terms.push(b_node);
                            } else {
                                let theta2_pow = make_ln_power(&ln_h, k);
                                result_terms
                                    .push(Node::Multiply(Box::new(b_node), Box::new(theta2_pow)));
                            }
                        }
                        b_prev = Some(b_k_ep);
                    }
                    None => {
                        return Some(RischResult::NonElementary(format!(
                            "No elementary antiderivative exists. \
                             Cannot integrate the degree-{} correction in the inner exp extension.",
                            k
                        )));
                    }
                }
            } else {
                // Rational RHS — use integrate_rational_ext
                match integrate_rational_ext(&rhs_num_reduced, &rhs_den_reduced, inner_ext, var) {
                    Some(RischResult::NonElementary(reason)) => {
                        return Some(RischResult::NonElementary(reason));
                    }
                    Some(RischResult::Elementary(node)) => {
                        if k > 0 {
                            // Need structured b_k for next step — can't extract from Node
                            return None;
                        }
                        result_terms.push(node);
                    }
                    None => return None,
                }
            }
        }
    }

    if result_terms.is_empty() {
        return Some(RischResult::Elementary(Node::Num(ExactNum::zero())));
    }
    let mut result = result_terms.remove(0);
    for t in result_terms {
        result = Node::Add(Box::new(result), Box::new(t));
    }
    Some(RischResult::Elementary(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exact::ExactNum;
    use crate::polynomial::Polynomial;
    use num_bigint::BigInt;
    use num_rational::BigRational;
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

    #[test]
    fn test_solve_risch_de_s1_identity() {
        // s=1: p' + x·p = x → p = 1
        let s = Polynomial::one("x");
        let f = poly(&[0, 1], "x");
        let g = poly(&[0, 1], "x");
        let result = solve_risch_de(&s, &f, &g, "x");
        assert_eq!(result, Some(poly(&[1], "x")));
    }

    #[test]
    fn test_solve_risch_de_s_x() {
        // x·p' + (x-1)·p = 1-x → p = -1
        let s = poly(&[0, 1], "x");
        let f = poly(&[-1, 1], "x");
        let g = poly(&[1, -1], "x");
        let result = solve_risch_de(&s, &f, &g, "x");
        assert_eq!(result, Some(poly(&[-1], "x")));
    }

    #[test]
    fn test_solve_risch_de_s_x_no_solution() {
        // x·p' + (x-1)·p = 1 → no solution
        let s = poly(&[0, 1], "x");
        let f = poly(&[-1, 1], "x");
        let g = poly(&[1], "x");
        let result = solve_risch_de(&s, &f, &g, "x");
        assert_eq!(result, None);
    }

    #[test]
    fn test_solve_risch_de_g_zero() {
        let s = poly(&[0, 1], "x");
        let f = poly(&[1, 1], "x");
        let g = Polynomial::zero("x");
        let result = solve_risch_de(&s, &f, &g, "x");
        assert_eq!(result, Some(Polynomial::zero("x")));
    }

    #[test]
    fn test_solve_risch_de_rational_poly_rhs() {
        // When g is polynomial: q' + x·q = x → q = 1
        let f = poly(&[0, 1], "x");
        let g = RationalFunction::from_poly(poly(&[0, 1], "x"));
        let result = solve_risch_de_rational(&f, &g, "x");
        assert_eq!(result, Some(RationalFunction::from_poly(poly(&[1], "x"))));
    }

    #[test]
    fn test_solve_risch_de_rational_simple_pole_rejection() {
        // q' + q = 1/x → no rational solution (simple pole)
        let f = poly(&[1], "x");
        let g = RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x"));
        let result = solve_risch_de_rational(&f, &g, "x");
        assert_eq!(result, None);
    }

    #[test]
    fn test_solve_risch_de_rational_double_pole_success() {
        // q' + q = (1-x)/x² → q = -1/x
        let f = poly(&[1], "x");
        let g = RationalFunction::new(poly(&[1, -1], "x"), poly(&[0, 0, 1], "x"));
        let result = solve_risch_de_rational(&f, &g, "x");
        let expected = RationalFunction::new(poly(&[-1], "x"), poly(&[0, 1], "x"));
        assert_eq!(result, Some(expected));
    }

    #[test]
    fn test_solve_risch_de_rational_double_pole_no_solution() {
        // q' + q = 1/x² → no polynomial p satisfies x·p' + (x-1)·p = 1
        let f = poly(&[1], "x");
        let g = RationalFunction::new(poly(&[1], "x"), poly(&[0, 0, 1], "x"));
        let result = solve_risch_de_rational(&f, &g, "x");
        assert_eq!(result, None);
    }

    #[test]
    fn test_solve_risch_de_rational_simple_pole_in_product() {
        // q' + q = 2/x → no solution (simple pole in denominator x)
        let f = poly(&[1], "x");
        let g = RationalFunction::new(poly(&[2], "x"), poly(&[0, 1], "x"));
        let result = solve_risch_de_rational(&f, &g, "x");
        assert_eq!(result, None);
    }

    #[test]
    fn test_extpoly_det_1x1() {
        let m = vec![vec![ExtPoly::from_rf(rf_const(3))]];
        let result = extpoly_matrix_det(&m, "x");
        assert_eq!(result, ExtPoly::from_rf(rf_const(3)));
    }

    #[test]
    fn test_extpoly_det_2x2() {
        // det([[1, 2], [3, 4]]) = 1*4 - 2*3 = -2
        let m = vec![
            vec![ExtPoly::from_rf(rf_const(1)), ExtPoly::from_rf(rf_const(2))],
            vec![ExtPoly::from_rf(rf_const(3)), ExtPoly::from_rf(rf_const(4))],
        ];
        let result = extpoly_matrix_det(&m, "x");
        assert_eq!(result, ExtPoly::from_rf(rf_const(-2)));
    }

    #[test]
    fn test_extpoly_det_2x2_with_theta() {
        // det([[θ, 1], [1, θ]]) = θ² - 1
        let theta = ExtPoly::theta("x");
        let one = ExtPoly::from_rf(rf_const(1));
        let m = vec![
            vec![theta.clone(), one.clone()],
            vec![one.clone(), theta.clone()],
        ];
        let result = extpoly_matrix_det(&m, "x");
        let expected = ExtPoly::from_coeffs(vec![rf_const(-1), rf_const(0), rf_const(1)], "x");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_extpoly_det_3x3_identity() {
        let one = ExtPoly::from_rf(rf_const(1));
        let zero = ExtPoly::zero("x");
        let m = vec![
            vec![one.clone(), zero.clone(), zero.clone()],
            vec![zero.clone(), one.clone(), zero.clone()],
            vec![zero.clone(), zero.clone(), one.clone()],
        ];
        assert_eq!(extpoly_matrix_det(&m, "x"), one);
    }

    #[test]
    fn test_resultant_z_simple() {
        // ∫1/(x·ln(x))dx: d=θ, a=1/x, D(d)=1/x
        // R(z) = (1-z)/x: coeff(0) = 1/x, coeff(1) = -1/x
        let one_over_x = RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x"));
        let d = ExtPoly::theta("x");
        let a = ExtPoly::from_rf(one_over_x.clone());
        let ext = DifferentialExtension::logarithmic(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let dd = ext.differentiate(&d);
        let rz = rothstein_trager_resultant(&d, &a, &dd, "x");
        assert_eq!(rz.degree(), Some(1));
        assert_eq!(rz.coeff(0), one_over_x);
        assert_eq!(rz.coeff(1), -&one_over_x);
    }

    #[test]
    fn test_resultant_z_non_elementary() {
        // ∫1/ln(x)dx: d=θ, a=1, D(d)=1/x
        // R(z) = 1 - z/x: coeff(0) = 1, coeff(1) = -1/x
        let d = ExtPoly::theta("x");
        let a = ExtPoly::from_rf(rf_const(1));
        let ext = DifferentialExtension::logarithmic(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let dd = ext.differentiate(&d);
        let rz = rothstein_trager_resultant(&d, &a, &dd, "x");
        assert_eq!(rz.degree(), Some(1));
        assert_eq!(rz.coeff(0), rf_const(1));
        let neg_one_over_x = RationalFunction::new(poly(&[-1], "x"), poly(&[0, 1], "x"));
        assert_eq!(rz.coeff(1), neg_one_over_x);
    }

    #[test]
    fn test_resultant_z_degree2() {
        // d = θ²+θ, a = (2θ+1)/x, D(d) = (2θ+1)/x
        // R(z) = -(1-z)²/x²: verify R(1) = 0
        let one_over_x = RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x"));
        let two_over_x = RationalFunction::new(poly(&[2], "x"), poly(&[0, 1], "x"));
        let d = ExtPoly::from_coeffs(vec![rf_const(0), rf_const(1), rf_const(1)], "x");
        let a = ExtPoly::from_coeffs(vec![one_over_x, two_over_x], "x");
        let ext = DifferentialExtension::logarithmic(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let dd = ext.differentiate(&d);
        let rz = rothstein_trager_resultant(&d, &a, &dd, "x");
        assert_eq!(rz.degree(), Some(2));
        // Verify R(1) = 0: sum of all coefficients should be zero
        let r1 = &(&rz.coeff(0) + &rz.coeff(1)) + &rz.coeff(2);
        assert!(r1.is_zero(), "R(1) should be 0, got {}", r1);
    }

    // ======== find_constant_roots tests ========

    #[test]
    fn test_find_constant_roots_simple() {
        // R(z) = (1-z)/x → root at z=1
        let one_over_x = RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x"));
        let rz = ExtPoly::from_coeffs(vec![one_over_x.clone(), -&one_over_x], "x");
        let roots = find_constant_roots(&rz, "x");
        assert_eq!(roots, vec![int(1)]);
    }

    #[test]
    fn test_find_constant_roots_none() {
        // R(z) = 1 - z/x → z=x not constant, no roots
        let neg_one_over_x = RationalFunction::new(poly(&[-1], "x"), poly(&[0, 1], "x"));
        let rz = ExtPoly::from_coeffs(vec![rf_const(1), neg_one_over_x], "x");
        let roots = find_constant_roots(&rz, "x");
        assert!(roots.is_empty());
    }

    #[test]
    fn test_find_constant_roots_repeated() {
        // R(z) = -(1-z)²/x² = (-1/x² + 2z/x² - z²/x²)
        let one_over_x2 = RationalFunction::new(poly(&[1], "x"), poly(&[0, 0, 1], "x"));
        let two_over_x2 = RationalFunction::new(poly(&[2], "x"), poly(&[0, 0, 1], "x"));
        let rz = ExtPoly::from_coeffs(vec![-&one_over_x2, two_over_x2, -&one_over_x2], "x");
        let roots = find_constant_roots(&rz, "x");
        assert_eq!(roots, vec![int(1)]);
    }

    #[test]
    fn test_rf_to_node_constant() {
        let rf = rf_const(5);
        let result = rf_to_node(&rf, "x");
        assert_eq!(format!("{}", result), "5");
    }

    #[test]
    fn test_rf_to_node_polynomial() {
        let rf = rf_poly(&[1, 1]); // x + 1
        let result = rf_to_node(&rf, "x");
        let s = format!("{}", result);
        assert!(s.contains("x"), "Expected x in {}", s);
    }

    #[test]
    fn test_rf_to_node_fraction() {
        // 1/x
        let rf = RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x"));
        let result = rf_to_node(&rf, "x");
        let s = format!("{}", result);
        assert!(s.contains("x"), "Expected x in {}", s);
    }

    #[test]
    fn test_extpoly_to_node_constant() {
        let ep = ExtPoly::from_rf(rf_const(3));
        let ln_x = Node::Function("ln".to_string(), vec![Node::Variable("x".to_string())]);
        let result = extpoly_to_node(&ep, &ln_x, "x");
        assert_eq!(format!("{}", result), "3");
    }

    #[test]
    fn test_extpoly_to_node_theta() {
        let ep = ExtPoly::theta("x");
        let ln_x = Node::Function("ln".to_string(), vec![Node::Variable("x".to_string())]);
        let result = extpoly_to_node(&ep, &ln_x, "x");
        assert_eq!(format!("{}", result), "\\ln(x)");
    }

    #[test]
    fn test_extpoly_to_node_theta_plus_one() {
        let ep = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
        let ln_x = Node::Function("ln".to_string(), vec![Node::Variable("x".to_string())]);
        let result = extpoly_to_node(&ep, &ln_x, "x");
        let s = format!("{}", result);
        assert!(s.contains("\\ln(x)"), "Expected ln(x) in {}", s);
    }

    // === Scanning tests ===

    #[test]
    fn test_contains_ln_yes() {
        let expr = Node::Function("ln".to_string(), vec![Node::Variable("x".to_string())]);
        assert!(contains_ln(&expr, "x"));
    }

    #[test]
    fn test_contains_ln_nested() {
        let expr = Node::Add(
            Box::new(Node::Num(ExactNum::integer(1))),
            Box::new(Node::Function(
                "ln".to_string(),
                vec![Node::Variable("x".to_string())],
            )),
        );
        assert!(contains_ln(&expr, "x"));
    }

    #[test]
    fn test_contains_ln_no() {
        let expr = Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())]);
        assert!(!contains_ln(&expr, "x"));
    }

    #[test]
    fn test_find_exp_arg_simple() {
        let expr = Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())]);
        let arg = find_exp_argument(&expr, "x").unwrap();
        assert_eq!(arg, poly(&[0, 1], "x"));
    }

    #[test]
    fn test_find_exp_arg_x_squared() {
        let expr = Node::Function(
            "exp".to_string(),
            vec![Node::Power(
                Box::new(Node::Variable("x".to_string())),
                Box::new(Node::Num(ExactNum::integer(2))),
            )],
        );
        let arg = find_exp_argument(&expr, "x").unwrap();
        assert_eq!(arg, poly(&[0, 0, 1], "x"));
    }

    #[test]
    fn test_find_exp_arg_none() {
        let expr = Node::Variable("x".to_string());
        assert!(find_exp_argument(&expr, "x").is_none());
    }

    #[test]
    fn test_find_exp_arg_in_product() {
        let expr = Node::Multiply(
            Box::new(Node::Variable("x".to_string())),
            Box::new(Node::Function(
                "exp".to_string(),
                vec![Node::Variable("x".to_string())],
            )),
        );
        let arg = find_exp_argument(&expr, "x").unwrap();
        assert_eq!(arg, poly(&[0, 1], "x"));
    }

    // === Generalized node_to_extpoly tests ===

    #[test]
    fn test_general_extpoly_exp_x() {
        let expr = Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())]);
        let kind = ExtensionKind::Exponential(poly(&[0, 1], "x"));
        let result = node_to_extpoly_general(&expr, "x", &kind).unwrap();
        assert_eq!(result, ExtPoly::theta("x"));
    }

    #[test]
    fn test_general_extpoly_x_times_exp() {
        let expr = Node::Multiply(
            Box::new(Node::Variable("x".to_string())),
            Box::new(Node::Function(
                "exp".to_string(),
                vec![Node::Variable("x".to_string())],
            )),
        );
        let kind = ExtensionKind::Exponential(poly(&[0, 1], "x"));
        let result = node_to_extpoly_general(&expr, "x", &kind).unwrap();
        assert_eq!(result.degree(), Some(1));
        assert_eq!(result.coeff(1), rf_poly(&[0, 1]));
        assert!(result.coeff(0).is_zero());
    }

    #[test]
    fn test_general_extpoly_one_plus_exp() {
        let expr = Node::Add(
            Box::new(Node::Num(ExactNum::integer(1))),
            Box::new(Node::Function(
                "exp".to_string(),
                vec![Node::Variable("x".to_string())],
            )),
        );
        let kind = ExtensionKind::Exponential(poly(&[0, 1], "x"));
        let result = node_to_extpoly_general(&expr, "x", &kind).unwrap();
        let expected = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_general_extpoly_log_still_works() {
        let expr = Node::Add(
            Box::new(Node::Function(
                "ln".to_string(),
                vec![Node::Variable("x".to_string())],
            )),
            Box::new(Node::Num(ExactNum::integer(1))),
        );
        let kind = ExtensionKind::Logarithmic;
        let result = node_to_extpoly_general(&expr, "x", &kind).unwrap();
        let expected = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
        assert_eq!(result, expected);
    }

    // === build_tower tests ===

    #[test]
    fn test_build_tower_log() {
        // 1/(x·ln(x)) → Logarithmic, num constant, den degree 1
        let expr = Node::Divide(
            Box::new(Node::Num(ExactNum::integer(1))),
            Box::new(Node::Multiply(
                Box::new(Node::Variable("x".to_string())),
                Box::new(Node::Function(
                    "ln".to_string(),
                    vec![Node::Variable("x".to_string())],
                )),
            )),
        );
        let (num, den, ext) = build_tower(&expr, "x").unwrap();
        assert!(matches!(ext.ext_type(), ExtensionType::Logarithmic));
        assert!(num.is_constant());
        assert_eq!(den.degree(), Some(1));
    }

    #[test]
    fn test_build_tower_exp_polynomial() {
        // 2x·exp(x²) → Exponential, num degree 1, den = 1
        let two_x = Node::Multiply(
            Box::new(Node::Num(ExactNum::integer(2))),
            Box::new(Node::Variable("x".to_string())),
        );
        let exp_x2 = Node::Function(
            "exp".to_string(),
            vec![Node::Power(
                Box::new(Node::Variable("x".to_string())),
                Box::new(Node::Num(ExactNum::integer(2))),
            )],
        );
        let expr = Node::Multiply(Box::new(two_x), Box::new(exp_x2));
        let (num, den, ext) = build_tower(&expr, "x").unwrap();
        assert!(matches!(ext.ext_type(), ExtensionType::Exponential));
        assert_eq!(num.degree(), Some(1));
        assert!(den.is_constant() || den == ExtPoly::one("x"));
    }

    #[test]
    fn test_build_tower_exp_rational() {
        // exp(x)/(1+exp(x)) → Exponential, num=[0,1], den=[1,1]
        let expr = Node::Divide(
            Box::new(Node::Function(
                "exp".to_string(),
                vec![Node::Variable("x".to_string())],
            )),
            Box::new(Node::Add(
                Box::new(Node::Num(ExactNum::integer(1))),
                Box::new(Node::Function(
                    "exp".to_string(),
                    vec![Node::Variable("x".to_string())],
                )),
            )),
        );
        let (num, den, ext) = build_tower(&expr, "x").unwrap();
        assert!(matches!(ext.ext_type(), ExtensionType::Exponential));
        assert_eq!(num.degree(), Some(1));
        assert_eq!(den.degree(), Some(1));
    }

    #[test]
    fn test_build_tower_mixed_returns_none() {
        // ln(x) * exp(x) → mixed, None
        let expr = Node::Multiply(
            Box::new(Node::Function(
                "ln".to_string(),
                vec![Node::Variable("x".to_string())],
            )),
            Box::new(Node::Function(
                "exp".to_string(),
                vec![Node::Variable("x".to_string())],
            )),
        );
        assert!(build_tower(&expr, "x").is_none());
    }

    #[test]
    fn test_build_tower_no_transcendental() {
        let expr = Node::Variable("x".to_string());
        assert!(build_tower(&expr, "x").is_none());
    }

    #[test]
    fn test_integrate_poly_exp_simple() {
        // ∫exp(x)dx: num=[0,1], θ=exp(x), g'=1
        // q₁' + 1·q₁ = 1 → q₁ = 1, result = exp(x)
        let ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let num = ExtPoly::from_coeffs(vec![rf_const(0), rf_const(1)], "x");
        match integrate_poly_exp(&num, &ext, "x").unwrap() {
            RischResult::Elementary(node) => {
                let s = format!("{}", node);
                assert!(s.contains("exp"), "Expected exp in {}", s);
            }
            r => panic!("Expected elementary, got {:?}", r),
        }
    }

    #[test]
    fn test_integrate_poly_exp_2x_exp_x2() {
        // ∫2x·exp(x²)dx: num=[0,2x], θ=exp(x²), g'=2x
        // q₁' + 2x·q₁ = 2x → q₁ = 1
        let ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(poly(&[0, 0, 1], "x")),
            "x",
        );
        let num = ExtPoly::from_coeffs(vec![rf_const(0), rf_poly(&[0, 2])], "x");
        match integrate_poly_exp(&num, &ext, "x").unwrap() {
            RischResult::Elementary(_) => {}
            r => panic!("Expected elementary, got {:?}", r),
        }
    }

    #[test]
    fn test_integrate_poly_exp_non_elementary() {
        // ∫exp(-x²)dx: num=[0,1], θ=exp(-x²), g'=-2x
        // q₁' + (-2x)·q₁ = 1 → no polynomial solution
        let ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(poly(&[0, 0, -1], "x")),
            "x",
        );
        let num = ExtPoly::from_coeffs(vec![rf_const(0), rf_const(1)], "x");
        match integrate_poly_exp(&num, &ext, "x").unwrap() {
            RischResult::NonElementary(_) => {}
            r => panic!("Expected non-elementary, got {:?}", r),
        }
    }

    #[test]
    fn test_integrate_poly_exp_with_constant_term() {
        // ∫(1 + exp(x))dx: num=[1,1], θ=exp(x)
        // q₀ = x, q₁ = 1, result = x + exp(x)
        let ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let num = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
        match integrate_poly_exp(&num, &ext, "x").unwrap() {
            RischResult::Elementary(node) => {
                let s = format!("{}", node);
                assert!(s.contains("x"), "Expected x in {}", s);
                assert!(s.contains("exp"), "Expected exp in {}", s);
            }
            r => panic!("Expected elementary, got {:?}", r),
        }
    }

    #[test]
    fn test_integrate_poly_log_ln_x() {
        // ∫ln(x)dx: num=[0,1] → result = -x + x·ln(x)
        let ext = DifferentialExtension::logarithmic(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let num = ExtPoly::from_coeffs(vec![rf_const(0), rf_const(1)], "x");
        match integrate_poly_log(&num, &ext, "x").unwrap() {
            RischResult::Elementary(node) => {
                let s = format!("{}", node);
                assert!(s.contains("\\ln"), "Expected ln in {}", s);
            }
            r => panic!("Expected elementary, got {:?}", r),
        }
    }

    #[test]
    fn test_integrate_poly_log_x_ln_x() {
        // ∫x·ln(x)dx: num=[0,x]
        let ext = DifferentialExtension::logarithmic(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let num = ExtPoly::from_coeffs(vec![rf_const(0), rf_poly(&[0, 1])], "x");
        match integrate_poly_log(&num, &ext, "x").unwrap() {
            RischResult::Elementary(_) => {}
            r => panic!("Expected elementary, got {:?}", r),
        }
    }

    #[test]
    fn test_integrate_poly_log_rational_coeff() {
        // ∫(1/x²)·ln(x) dx = -(ln(x)+1)/x
        let num = ExtPoly::from_coeffs(
            vec![
                RationalFunction::zero("x"),
                RationalFunction::new(poly(&[1], "x"), poly(&[0, 0, 1], "x")),
            ],
            "x",
        );
        let ext = DifferentialExtension::logarithmic(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        match integrate_poly_log(&num, &ext, "x") {
            Some(RischResult::Elementary(_)) => {}
            other => panic!("Expected Elementary, got: {:?}", other),
        }
    }

    #[test]
    fn test_integrate_poly_log_ln_x_absorption() {
        // ∫(1/x + ln(x)) dx = (x+1)·ln(x) - x
        // coeffs: a_0 = 1/x, a_1 = 1
        // degree 1: q_1 = ∫1 dx = x
        // degree 0: RHS = 1/x - q_1/x = 1/x - x/x = 1/x - 1
        //   ∫(1/x - 1) dx = ln(x) - x → ln_x_coeff=1 absorbed into Δ
        //   rational part = -x
        // Final: q[0] + (q[1] + Δ)·θ = -x + (x+1)·ln(x)
        let num = ExtPoly::from_coeffs(
            vec![
                RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x")),
                RationalFunction::from_poly(poly(&[1], "x")),
            ],
            "x",
        );
        let ext = DifferentialExtension::logarithmic(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        match integrate_poly_log(&num, &ext, "x") {
            Some(RischResult::Elementary(_)) => {}
            other => panic!("Expected Elementary, got: {:?}", other),
        }
    }

    #[test]
    fn test_integrate_poly_log_rational_non_elementary() {
        // ∫(1/(x+1))·ln(x) dx is non-elementary in single tower
        let num = ExtPoly::from_coeffs(
            vec![
                RationalFunction::zero("x"),
                RationalFunction::new(poly(&[1], "x"), poly(&[1, 1], "x")),
            ],
            "x",
        );
        let ext = DifferentialExtension::logarithmic(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        match integrate_poly_log(&num, &ext, "x") {
            Some(RischResult::NonElementary(_)) => {}
            other => panic!("Expected NonElementary, got: {:?}", other),
        }
    }

    #[test]
    fn test_integrate_rational_log() {
        // ∫(1/x)/θ where θ=ln(x) → ln(ln(x))
        let ext = DifferentialExtension::logarithmic(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let one_over_x = RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x"));
        let num = ExtPoly::from_rf(one_over_x);
        let den = ExtPoly::theta("x");
        match integrate_rational_ext(&num, &den, &ext, "x").unwrap() {
            RischResult::Elementary(node) => {
                let s = format!("{}", node);
                assert!(s.contains("\\ln"), "Expected ln in {}", s);
            }
            _ => panic!("Expected elementary"),
        }
    }

    #[test]
    fn test_integrate_rational_exp_no_residual() {
        // ∫θ/(1+θ) where θ=exp(x) → ln(1+exp(x))
        // RT: c=1, v=1+θ, D(v)=θ. Residual: θ/(1+θ) - 1·θ/(1+θ) = 0.
        let ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let num = ExtPoly::from_coeffs(vec![rf_const(0), rf_const(1)], "x");
        let den = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
        match integrate_rational_ext(&num, &den, &ext, "x").unwrap() {
            RischResult::Elementary(node) => {
                let s = format!("{}", node);
                assert!(s.contains("\\ln"), "Expected ln in {}", s);
            }
            _ => panic!("Expected elementary"),
        }
    }

    #[test]
    fn test_integrate_rational_exp_with_residual() {
        // ∫1/(1+θ) where θ=exp(x) → x - ln(1+exp(x))
        // RT: c=-1, v=1+θ. Residual: 1/(1+θ) - (-θ/(1+θ)) = (1+θ)/(1+θ) = 1.
        // ∫1 = x.
        let ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let num = ExtPoly::from_rf(rf_const(1));
        let den = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
        match integrate_rational_ext(&num, &den, &ext, "x").unwrap() {
            RischResult::Elementary(node) => {
                let s = format!("{}", node);
                assert!(s.contains("\\ln"), "Expected ln: {}", s);
                assert!(s.contains("x"), "Expected x from residual: {}", s);
            }
            _ => panic!("Expected elementary with residual"),
        }
    }

    #[test]
    fn test_integrate_poly_exp_rational_coeff_elementary() {
        // ∫((1-x)/x²)·exp(x)dx = -exp(x)/x
        let num = ExtPoly::from_coeffs(
            vec![
                RationalFunction::zero("x"),
                RationalFunction::new(
                    poly(&[1, -1], "x"),   // 1 - x
                    poly(&[0, 0, 1], "x"), // x²
                ),
            ],
            "x",
        );
        let ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        match integrate_poly_exp(&num, &ext, "x") {
            Some(RischResult::Elementary(_node)) => {
                // Success — the result should be equivalent to -exp(x)/x
            }
            other => panic!("Expected Elementary, got: {:?}", other),
        }
    }

    #[test]
    fn test_integrate_poly_exp_rational_coeff_non_elementary_simple_pole() {
        // ∫(1/x)·exp(x)dx is non-elementary (Ei function)
        let num = ExtPoly::from_coeffs(
            vec![
                RationalFunction::zero("x"),
                RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x")),
            ],
            "x",
        );
        let ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        match integrate_poly_exp(&num, &ext, "x") {
            Some(RischResult::NonElementary(_)) => {}
            other => panic!("Expected NonElementary, got: {:?}", other),
        }
    }

    #[test]
    fn test_integrate_poly_exp_rational_coeff_non_elementary_double_pole() {
        // ∫(1/x²)·exp(x)dx is non-elementary
        let num = ExtPoly::from_coeffs(
            vec![
                RationalFunction::zero("x"),
                RationalFunction::new(poly(&[1], "x"), poly(&[0, 0, 1], "x")),
            ],
            "x",
        );
        let ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        match integrate_poly_exp(&num, &ext, "x") {
            Some(RischResult::NonElementary(_)) => {}
            other => panic!("Expected NonElementary, got: {:?}", other),
        }
    }

    #[test]
    fn test_integrate_rational_base_polynomial() {
        // ∫x dx = x²/2 (polynomial, no log)
        let rf = RationalFunction::from_poly(poly(&[0, 1], "x")); // x
        let result = integrate_rational_base(&rf, "x").unwrap();
        assert!(result.ln_x_coeff.is_zero());
        assert!(!result.rational_part.is_zero());
    }

    #[test]
    fn test_integrate_rational_base_inv_x_sq() {
        // ∫1/x² dx = -1/x
        let rf = RationalFunction::new(poly(&[1], "x"), poly(&[0, 0, 1], "x"));
        let result = integrate_rational_base(&rf, "x").unwrap();
        let expected = RationalFunction::new(poly(&[-1], "x"), poly(&[0, 1], "x"));
        assert_eq!(result.rational_part, expected);
        assert!(result.ln_x_coeff.is_zero());
    }

    #[test]
    fn test_integrate_rational_base_inv_x() {
        // ∫1/x dx = ln(x)
        let rf = RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x"));
        let result = integrate_rational_base(&rf, "x").unwrap();
        assert!(result.rational_part.is_zero());
        assert_eq!(result.ln_x_coeff, int(1));
    }

    #[test]
    fn test_integrate_rational_base_inv_x_plus_1() {
        // ∫1/(x+1) dx = ln(x+1) → non-elementary
        let rf = RationalFunction::new(poly(&[1], "x"), poly(&[1, 1], "x"));
        let result = integrate_rational_base(&rf, "x");
        assert!(result.is_err());
    }

    #[test]
    fn test_integrate_rational_base_mixed() {
        // ∫(x+1)/x² dx = ln(x) - 1/x
        let rf = RationalFunction::new(poly(&[1, 1], "x"), poly(&[0, 0, 1], "x"));
        let result = integrate_rational_base(&rf, "x").unwrap();
        let expected_rat = RationalFunction::new(poly(&[-1], "x"), poly(&[0, 1], "x"));
        assert_eq!(result.rational_part, expected_rat);
        assert_eq!(result.ln_x_coeff, int(1));
    }

    // === Two-level tower tests ===

    #[test]
    fn test_two_level_exp_times_ln() {
        let expr = Node::Multiply(
            Box::new(Node::Function(
                "exp".to_string(),
                vec![Node::Variable("x".to_string())],
            )),
            Box::new(Node::Function(
                "ln".to_string(),
                vec![Node::Variable("x".to_string())],
            )),
        );
        let result = node_to_two_level(&expr, "x", &poly(&[0, 1], "x")).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result[0].is_zero());
        assert_eq!(result[1], ExtPoly::theta("x"));
    }

    #[test]
    fn test_two_level_exp_times_ln_plus_exp_over_x() {
        let exp_x = Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())]);
        let ln_x = Node::Function("ln".to_string(), vec![Node::Variable("x".to_string())]);
        let exp_ln = Node::Multiply(Box::new(exp_x.clone()), Box::new(ln_x));
        let exp_over_x = Node::Divide(Box::new(exp_x), Box::new(Node::Variable("x".to_string())));
        let expr = Node::Add(Box::new(exp_ln), Box::new(exp_over_x));
        let result = node_to_two_level(&expr, "x", &poly(&[0, 1], "x")).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result[0].is_zero());
        let expected_coeff = ExtPoly::from_coeffs(
            vec![
                RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x")),
                RationalFunction::one("x"),
            ],
            "x",
        );
        assert_eq!(result[1], expected_coeff);
    }

    #[test]
    fn test_two_level_exp_times_ln_squared() {
        let exp_x = Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())]);
        let ln_x = Node::Function("ln".to_string(), vec![Node::Variable("x".to_string())]);
        let ln_x_sq = Node::Power(Box::new(ln_x), Box::new(Node::Num(ExactNum::integer(2))));
        let expr = Node::Multiply(Box::new(exp_x), Box::new(ln_x_sq));
        let result = node_to_two_level(&expr, "x", &poly(&[0, 1], "x")).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result[0].is_zero());
        let theta = ExtPoly::theta("x");
        let theta_sq = &theta * &theta;
        assert_eq!(result[1], theta_sq);
    }

    #[test]
    fn test_two_level_just_exp() {
        let expr = Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())]);
        let result = node_to_two_level(&expr, "x", &poly(&[0, 1], "x")).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result[0].is_zero());
        assert_eq!(result[1], ExtPoly::from_rf(RationalFunction::one("x")));
    }

    #[test]
    fn test_two_level_constant() {
        let expr = Node::Num(ExactNum::integer(3));
        let result = node_to_two_level(&expr, "x", &poly(&[0, 1], "x")).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], ExtPoly::from_rf(rf_const(3)));
    }

    // === Inner Risch DE in log extension tests ===

    #[test]
    fn test_inner_de_constant_rhs() {
        // q' + q = 1 → q = 1
        let f = poly(&[1], "x");
        let g = ExtPoly::from_rf(rf_const(1));
        let result = solve_risch_de_in_log_ext(&f, &g, "x").unwrap();
        assert_eq!(result, ExtPoly::from_rf(rf_const(1)));
    }

    #[test]
    fn test_inner_de_theta1_rhs_elementary() {
        // q' + q = θ₁ + 1/x → q = θ₁ (b₁=1, b₀=0)
        // Check: d/dx[ln(x)] = 1/x, so q'+q = 1/x + ln(x) ✓
        let f = poly(&[1], "x");
        let one_over_x = RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x"));
        let g = ExtPoly::from_coeffs(vec![one_over_x, RationalFunction::one("x")], "x");
        let result = solve_risch_de_in_log_ext(&f, &g, "x").unwrap();
        assert_eq!(result, ExtPoly::theta("x"));
    }

    #[test]
    fn test_inner_de_theta1_rhs_non_elementary() {
        // q' + q = θ₁ (just ln(x))
        // b₁' + b₁ = 1 → b₁ = 1
        // b₀' + b₀ = 0 - 1·1/x = -1/x → no rational solution (simple pole)
        let f = poly(&[1], "x");
        let g = ExtPoly::from_coeffs(
            vec![RationalFunction::zero("x"), RationalFunction::one("x")],
            "x",
        );
        let result = solve_risch_de_in_log_ext(&f, &g, "x");
        assert!(result.is_none());
    }

    #[test]
    fn test_inner_de_theta1_squared_rhs() {
        // q' + q = θ₁² + 2θ₁/x → q = θ₁²
        // b₂' + b₂ = 1 → b₂ = 1
        // b₁' + b₁ = 2/x - 2·1/x = 0 → b₁ = 0
        // b₀' + b₀ = 0 - 1·0/x = 0 → b₀ = 0
        let f = poly(&[1], "x");
        let two_over_x = RationalFunction::new(poly(&[2], "x"), poly(&[0, 1], "x"));
        let g = ExtPoly::from_coeffs(
            vec![
                RationalFunction::zero("x"),
                two_over_x,
                RationalFunction::one("x"),
            ],
            "x",
        );
        let result = solve_risch_de_in_log_ext(&f, &g, "x").unwrap();
        let theta = ExtPoly::theta("x");
        assert_eq!(result, &theta * &theta);
    }

    #[test]
    fn test_inner_de_zero_rhs() {
        // q' + q = 0 → q = 0
        let f = poly(&[1], "x");
        let g = ExtPoly::zero("x");
        let result = solve_risch_de_in_log_ext(&f, &g, "x").unwrap();
        assert!(result.is_zero());
    }

    #[test]
    fn test_inner_de_2x_coefficient() {
        // q' + 2x·q = 2x → q = 1 (b₀ = 1)
        // Check: 0 + 2x·1 = 2x ✓
        let f = poly(&[0, 2], "x");
        let g = ExtPoly::from_rf(rf_poly(&[0, 2]));
        let result = solve_risch_de_in_log_ext(&f, &g, "x").unwrap();
        assert_eq!(result, ExtPoly::from_rf(rf_const(1)));
    }

    #[test]
    fn test_inner_de_rational_coeff_non_elementary() {
        // q' + q = 1/x + θ₁/x → b₁' + b₁ = 1/x → no rational solution (simple pole)
        let f = poly(&[1], "x");
        let one_over_x = RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x"));
        let g = ExtPoly::from_coeffs(vec![one_over_x.clone(), one_over_x], "x");
        let result = solve_risch_de_in_log_ext(&f, &g, "x");
        assert!(result.is_none());
    }

    #[test]
    fn test_inner_de_higher_f_coefficient() {
        // q' + 2x·q = 2x·θ₁ + 2/x
        // b₁' + 2x·b₁ = 2x → b₁ = 1
        // b₀' + 2x·b₀ = 2/x - 1·1/x = 1/x → no rational solution (simple pole)
        let f = poly(&[0, 2], "x");
        let two_x_rf = rf_poly(&[0, 2]);
        let two_over_x = RationalFunction::new(poly(&[2], "x"), poly(&[0, 1], "x"));
        let g = ExtPoly::from_coeffs(vec![two_over_x, two_x_rf], "x");
        let result = solve_risch_de_in_log_ext(&f, &g, "x");
        assert!(result.is_none());
    }

    // === Two-level integration tests ===

    #[test]
    fn test_integrate_two_level_exp_ln_non_elementary() {
        // ∫exp(x)·ln(x) dx → non-elementary
        let coeffs = vec![ExtPoly::zero("x"), ExtPoly::theta("x")];
        let outer_ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let inner_ext = DifferentialExtension::logarithmic(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        match integrate_two_level_exp_log(&coeffs, &inner_ext, &outer_ext, "x").unwrap() {
            RischResult::NonElementary(_) => {}
            r => panic!("Expected non-elementary, got {:?}", r),
        }
    }

    #[test]
    fn test_integrate_two_level_exp_ln_plus_exp_over_x() {
        // ∫(exp(x)·ln(x) + exp(x)/x) dx = exp(x)·ln(x)
        let one_over_x = RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x"));
        let coeff1 = ExtPoly::from_coeffs(vec![one_over_x, RationalFunction::one("x")], "x");
        let coeffs = vec![ExtPoly::zero("x"), coeff1];
        let outer_ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let inner_ext = DifferentialExtension::logarithmic(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        match integrate_two_level_exp_log(&coeffs, &inner_ext, &outer_ext, "x").unwrap() {
            RischResult::Elementary(node) => {
                let s = format!("{}", node);
                assert!(s.contains("\\ln"), "Expected ln in {}", s);
                assert!(s.contains("exp"), "Expected exp in {}", s);
            }
            r => panic!("Expected elementary, got {:?}", r),
        }
    }

    #[test]
    fn test_integrate_two_level_exp_ln_sq_plus_correction() {
        // ∫(exp(x)·ln(x)² + 2·exp(x)·ln(x)/x) dx = exp(x)·ln(x)²
        let two_over_x = RationalFunction::new(poly(&[2], "x"), poly(&[0, 1], "x"));
        let coeff1 = ExtPoly::from_coeffs(
            vec![
                RationalFunction::zero("x"),
                two_over_x,
                RationalFunction::one("x"),
            ],
            "x",
        );
        let coeffs = vec![ExtPoly::zero("x"), coeff1];
        let outer_ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let inner_ext = DifferentialExtension::logarithmic(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        match integrate_two_level_exp_log(&coeffs, &inner_ext, &outer_ext, "x").unwrap() {
            RischResult::Elementary(node) => {
                let s = format!("{}", node);
                assert!(s.contains("\\ln"), "Expected ln in {}", s);
                assert!(s.contains("exp"), "Expected exp in {}", s);
            }
            r => panic!("Expected elementary, got {:?}", r),
        }
    }

    #[test]
    fn test_integrate_two_level_pure_exp() {
        // ∫exp(x) dx = exp(x)
        let coeffs = vec![ExtPoly::zero("x"), ExtPoly::from_rf(rf_const(1))];
        let outer_ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let inner_ext = DifferentialExtension::logarithmic(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        match integrate_two_level_exp_log(&coeffs, &inner_ext, &outer_ext, "x").unwrap() {
            RischResult::Elementary(node) => {
                let s = format!("{}", node);
                assert!(s.contains("exp"), "Expected exp in {}", s);
            }
            r => panic!("Expected elementary, got {:?}", r),
        }
    }

    // === Two-level rational parsing tests ===

    #[test]
    fn test_two_level_rational_ln_over_1_plus_exp() {
        // ln(x)/(1+exp(x)) → num=[θ₁], den=[1,1]
        let ln_x = Node::Function("ln".to_string(), vec![Node::Variable("x".to_string())]);
        let one = Node::Num(ExactNum::integer(1));
        let exp_x = Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())]);
        let den = Node::Add(Box::new(one), Box::new(exp_x));
        let expr = Node::Divide(Box::new(ln_x), Box::new(den));
        let exp_arg = poly(&[0, 1], "x");
        let (num, denom) = extract_two_level_rational(&expr, "x", &exp_arg).unwrap();
        assert_eq!(num.len(), 1);
        assert_eq!(num[0], ExtPoly::theta("x"));
        assert_eq!(denom.len(), 2);
        assert_eq!(denom[0], ExtPoly::from_rf(rf_const(1)));
        assert_eq!(denom[1], ExtPoly::from_rf(rf_const(1)));
    }

    #[test]
    fn test_two_level_rational_exp_ln_over_1_plus_exp() {
        // exp(x)*ln(x)/(1+exp(x)) → num=[0, θ₁], den=[1,1]
        let exp_x = Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())]);
        let ln_x = Node::Function("ln".to_string(), vec![Node::Variable("x".to_string())]);
        let num_node = Node::Multiply(Box::new(exp_x.clone()), Box::new(ln_x));
        let one = Node::Num(ExactNum::integer(1));
        let den_node = Node::Add(Box::new(one), Box::new(exp_x));
        let expr = Node::Divide(Box::new(num_node), Box::new(den_node));
        let exp_arg = poly(&[0, 1], "x");
        let (num, denom) = extract_two_level_rational(&expr, "x", &exp_arg).unwrap();
        assert_eq!(num.len(), 2);
        assert!(num[0].is_zero());
        assert_eq!(num[1], ExtPoly::theta("x"));
        assert_eq!(denom.len(), 2);
    }

    #[test]
    fn test_two_level_rational_polynomial_returns_none() {
        // exp(x)*ln(x) has no denominator with θ₂ → None
        let exp_x = Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())]);
        let ln_x = Node::Function("ln".to_string(), vec![Node::Variable("x".to_string())]);
        let expr = Node::Multiply(Box::new(exp_x), Box::new(ln_x));
        let exp_arg = poly(&[0, 1], "x");
        assert!(extract_two_level_rational(&expr, "x", &exp_arg).is_none());
    }

    // === Two-level Hermite reduction tests ===

    #[test]
    fn test_hermite_reduce_two_level_squarefree() {
        // Squarefree denominator: no reduction needed
        let num = vec![ExtPoly::theta("x")]; // [θ₁]
        let den = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x"); // 1+θ₂
        let result = hermite_reduce_two_level(&num, &den, "x").unwrap();
        assert!(result.g_num.iter().all(|ep| ep.is_zero()));
        assert_eq!(result.h_num.len(), 1);
        assert_eq!(result.h_num[0], ExtPoly::theta("x"));
        assert_eq!(result.h_den, den);
    }

    #[test]
    fn test_hermite_reduce_two_level_non_squarefree() {
        // den = (1+θ₂)², num = [θ₁]
        let num = vec![ExtPoly::theta("x")];
        let t_plus_1 = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
        let den = &t_plus_1 * &t_plus_1;
        let result = hermite_reduce_two_level(&num, &den, "x").unwrap();
        assert!(!result.g_num.iter().all(|ep| ep.is_zero()));
        let sfd = result.h_den.square_free_decomposition();
        assert!(sfd.iter().all(|(_, m)| *m <= 1));
    }

    // === Two-level Rothstein-Trager tests ===

    #[test]
    fn test_rt_two_level_ln_over_1_plus_exp() {
        // d=1+θ₂, a=[θ₁], D(d)=θ₂ → R(z) = θ₁ + z → no constant roots
        let d = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
        let a = vec![ExtPoly::theta("x")];
        let ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let dd = ext.differentiate(&d);
        let rz = rothstein_trager_two_level(&d, &a, &dd, None, "x");
        let roots = find_constant_roots_two_level(&rz, "x");
        assert!(
            roots.is_empty(),
            "Should have no constant roots, got {:?}",
            roots
        );
    }

    #[test]
    fn test_rt_two_level_constant_coeff_has_root() {
        // d=1+θ₂, a=[1] (no θ₁), D(d)=θ₂ → R(z) = 1+z → root at -1
        let d = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
        let a = vec![ExtPoly::from_rf(rf_const(1))];
        let ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let dd = ext.differentiate(&d);
        let rz = rothstein_trager_two_level(&d, &a, &dd, None, "x");
        let roots = find_constant_roots_two_level(&rz, "x");
        assert_eq!(roots, vec![int(-1)]);
    }

    #[test]
    fn test_rt_two_level_with_content_no_roots() {
        // ∫1/(θ₁·(1+θ₂)): content=θ₁, d=1+θ₂, a=[1], D(d)=θ₂
        // R_scaled(z) = res(1+θ₂, 1 − z·θ₁·θ₂)
        // det = 1·1 − 1·(−z·θ₁) = 1 + z·θ₁ → no constant roots
        let d = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
        let a = vec![ExtPoly::from_rf(rf_const(1))];
        let ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let dd = ext.differentiate(&d);
        let content = ExtPoly::theta("x"); // θ₁
        let rz = rothstein_trager_two_level(&d, &a, &dd, Some(&content), "x");
        let roots = find_constant_roots_two_level(&rz, "x");
        assert!(
            roots.is_empty(),
            "Should have no constant roots, got {:?}",
            roots
        );
    }

    #[test]
    fn test_rt_two_level_with_content_none_regression() {
        // Regression: content=None gives same result as before
        // ∫1/(1+exp(x)): d=1+θ₂, a=[1], D(d)=θ₂. Root at z=-1.
        let d = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
        let a = vec![ExtPoly::from_rf(rf_const(1))];
        let ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let dd = ext.differentiate(&d);
        let rz = rothstein_trager_two_level(&d, &a, &dd, None, "x");
        let roots = find_constant_roots_two_level(&rz, "x");
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0], BigRational::from_integer(BigInt::from(-1)));
    }

    // === Two-level rational integration pipeline tests ===

    #[test]
    fn test_integrate_rational_two_level_ln_over_1_plus_exp() {
        // ∫ln(x)/(1+exp(x)) dx → non-elementary
        let num = vec![ExtPoly::theta("x")];
        let den = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
        let inner_ext = DifferentialExtension::logarithmic(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let outer_ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        match integrate_rational_two_level(&num, &den, &inner_ext, &outer_ext, "x").unwrap() {
            RischResult::NonElementary(_) => {}
            r => panic!("Expected non-elementary, got {:?}", r),
        }
    }

    #[test]
    fn test_integrate_rational_two_level_exp_ln_over_1_plus_exp() {
        // ∫exp(x)·ln(x)/(1+exp(x)) dx → non-elementary
        // After poly division: quotient = θ₁ (polynomial, non-elementary by itself)
        // OR: remainder has θ₁ terms → RT non-elementary
        let num = vec![ExtPoly::zero("x"), ExtPoly::theta("x")]; // θ₁·θ₂
        let den = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x"); // 1+θ₂
        let inner_ext = DifferentialExtension::logarithmic(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let outer_ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        match integrate_rational_two_level(&num, &den, &inner_ext, &outer_ext, "x").unwrap() {
            RischResult::NonElementary(_) => {}
            r => panic!("Expected non-elementary, got {:?}", r),
        }
    }

    // === Two-level GCD tests ===

    #[test]
    fn test_gcd_two_level_full_divisor() {
        // d = θ₂² - 1, g_c = (θ₁-1)(1-θ₂²) = -(θ₁-1)(θ₂²-1)
        // g_c as Vec<ExtPoly>: [(1-θ₁), 0, (θ₁-1)]
        // gcd should be θ₂²-1
        let d = ExtPoly::from_coeffs(vec![rf_const(-1), rf_const(0), rf_const(1)], "x");
        let one_minus_theta1 = {
            let ep = ExtPoly::theta("x");
            &ExtPoly::from_rf(rf_const(1)) - &ep
        };
        let theta1_minus_one = -&one_minus_theta1;
        let g_c = vec![one_minus_theta1, ExtPoly::zero("x"), theta1_minus_one];
        let v = gcd_extpoly_with_two_level(&d, &g_c, "x");
        assert_eq!(
            v.make_monic(),
            d.make_monic(),
            "GCD should be θ₂²-1 (monic)"
        );
    }

    #[test]
    fn test_gcd_two_level_partial_factor() {
        // d = θ₂²-1 = (θ₂-1)(θ₂+1)
        // g_c = [0, θ₁, θ₁] = θ₁·θ₂·(1+θ₂)
        // θ₁-component j=0: [0, 0, 0] → 0
        // θ₁-component j=1: [0, 1, 1] → θ₂ + θ₂²
        // gcd(θ₂²-1, θ₂+θ₂²) = gcd((θ₂-1)(θ₂+1), θ₂(θ₂+1)) = θ₂+1
        let d = ExtPoly::from_coeffs(vec![rf_const(-1), rf_const(0), rf_const(1)], "x");
        let g_c = vec![ExtPoly::zero("x"), ExtPoly::theta("x"), ExtPoly::theta("x")];
        let v = gcd_extpoly_with_two_level(&d, &g_c, "x");
        let expected = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
        assert_eq!(v.make_monic(), expected.make_monic());
    }

    #[test]
    fn test_gcd_two_level_coprime() {
        // d = θ₂²+1, g_c = [θ₁, 1] (= θ₁ + θ₂)
        // θ₁-component j=0: [0, 1] → θ₂
        // θ₁-component j=1: [1, 0] → 1
        // gcd(θ₂²+1, θ₂, 1) = 1
        let d = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(0), rf_const(1)], "x");
        let g_c = vec![ExtPoly::theta("x"), ExtPoly::from_rf(rf_const(1))];
        let v = gcd_extpoly_with_two_level(&d, &g_c, "x");
        assert!(
            v.is_constant(),
            "GCD should be 1 (constant), got degree {:?}",
            v.degree()
        );
    }

    #[test]
    fn test_gcd_two_level_no_theta1() {
        // Pure Q(x) coefficients — should match single-level GCD.
        // d = θ₂²-1, g_c = [1, 0, -1] = 1-θ₂² = -(θ₂²-1)
        // gcd = θ₂²-1
        let d = ExtPoly::from_coeffs(vec![rf_const(-1), rf_const(0), rf_const(1)], "x");
        let g_c = vec![
            ExtPoly::from_rf(rf_const(1)),
            ExtPoly::zero("x"),
            ExtPoly::from_rf(rf_const(-1)),
        ];
        let v = gcd_extpoly_with_two_level(&d, &g_c, "x");
        assert_eq!(v.make_monic(), d.make_monic());
    }

    #[test]
    fn test_integrate_rational_two_level_degree2_non_elementary() {
        // ∫ln(x)/(1+exp(2x)) dx → non-elementary
        // d = 1 + θ₂² (degree 2), a = [θ₁]
        // Previously returned None (unsupported). Now should return NonElementary.
        let num = vec![ExtPoly::theta("x")];
        let den = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(0), rf_const(1)], "x");
        let inner_ext = DifferentialExtension::logarithmic(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let outer_ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        match integrate_rational_two_level(&num, &den, &inner_ext, &outer_ext, "x") {
            Some(RischResult::NonElementary(_)) => {}
            other => panic!("Expected non-elementary, got {:?}", other),
        }
    }

    #[test]
    fn test_integrate_rational_two_level_degree2_no_theta1_elementary() {
        // Route 1/(θ₂²-1) through two-level pipeline (no θ₁).
        // ∫1/(exp(2x)-1)dx. d=θ₂²-1, D(d)=2θ₂².
        // R(z) = (1-2z)² → root z=1/2.
        // g_c = 1-(1/2)·2θ₂² = 1-θ₂²
        // gcd(θ₂²-1, 1-θ₂²) = θ₂²-1 → v = d.
        // Result should be elementary.
        let num = vec![ExtPoly::from_rf(rf_const(1))];
        let den = ExtPoly::from_coeffs(vec![rf_const(-1), rf_const(0), rf_const(1)], "x");
        let inner_ext = DifferentialExtension::logarithmic(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let outer_ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        match integrate_rational_two_level(&num, &den, &inner_ext, &outer_ext, "x") {
            Some(RischResult::Elementary(_)) => {}
            Some(RischResult::NonElementary(msg)) => {
                panic!("Expected elementary, got non-elementary: {}", msg)
            }
            None => panic!("Expected elementary, got None"),
        }
    }

    #[test]
    fn test_gcd_two_level_with_x_coefficients() {
        // d = θ₂² - x²  (RationalFunction coefficients)
        // g_c = [x, 0, -x] = x(1 - θ₂²)
        // d = (θ₂-x)(θ₂+x), g_c = -x(θ₂-1)(θ₂+1)
        // gcd = 1 (coprime — different roots)
        let x_rf = RationalFunction::from_poly(poly(&[0, 1], "x"));
        let neg_x_sq = -&(&x_rf * &x_rf);
        let d = ExtPoly::from_coeffs(vec![neg_x_sq, rf_const(0), rf_const(1)], "x");
        let g_c = vec![
            ExtPoly::from_rf(x_rf.clone()),
            ExtPoly::zero("x"),
            ExtPoly::from_rf(-&x_rf),
        ];
        let v = gcd_extpoly_with_two_level(&d, &g_c, "x");
        assert!(
            v.is_constant(),
            "GCD should be 1, got degree {:?}",
            v.degree()
        );
    }

    // === Log-over-exp detection tests ===

    #[test]
    fn test_find_ln_of_exp_basic() {
        // ln(1+exp(x)) → Some(g=[0,1], h=[1,1])
        let expr = Node::Function(
            "ln".to_string(),
            vec![Node::Add(
                Box::new(Node::Num(ExactNum::integer(1))),
                Box::new(Node::Function(
                    "exp".to_string(),
                    vec![Node::Variable("x".to_string())],
                )),
            )],
        );
        let (g, h) = find_ln_of_exp_argument(&expr, "x").unwrap();
        assert_eq!(g, poly(&[0, 1], "x"));
        assert_eq!(h, ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x"));
    }

    #[test]
    fn test_find_ln_of_exp_nested() {
        // exp(x) * ln(1+exp(x)) → finds the ln pattern in the subexpression
        let ln_part = Node::Function(
            "ln".to_string(),
            vec![Node::Add(
                Box::new(Node::Num(ExactNum::integer(1))),
                Box::new(Node::Function(
                    "exp".to_string(),
                    vec![Node::Variable("x".to_string())],
                )),
            )],
        );
        let exp_part = Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())]);
        let expr = Node::Multiply(Box::new(exp_part), Box::new(ln_part));
        let (g, h) = find_ln_of_exp_argument(&expr, "x").unwrap();
        assert_eq!(g, poly(&[0, 1], "x"));
        assert_eq!(h, ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x"));
    }

    #[test]
    fn test_find_ln_of_exp_none_for_ln_x() {
        // ln(x) → None (the arg has no exp)
        let expr = Node::Function("ln".to_string(), vec![Node::Variable("x".to_string())]);
        assert!(find_ln_of_exp_argument(&expr, "x").is_none());
    }

    // === Log-over-exp parser tests ===

    #[test]
    fn test_log_over_exp_parse_bare_ln() {
        // ln(1+exp(x)) → [0, 1] (= θ₂)
        let expr = Node::Function(
            "ln".to_string(),
            vec![Node::Add(
                Box::new(Node::Num(ExactNum::integer(1))),
                Box::new(Node::Function(
                    "exp".to_string(),
                    vec![Node::Variable("x".to_string())],
                )),
            )],
        );
        let exp_arg = poly(&[0, 1], "x");
        let h = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
        let result = node_to_two_level_log_over_exp(&expr, "x", &exp_arg, &h).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result[0].is_zero());
        assert_eq!(result[1], ExtPoly::one("x"));
    }

    #[test]
    fn test_log_over_exp_parse_exp_times_ln() {
        // exp(x) * ln(1+exp(x)) → [0, θ₁] (= θ₁·θ₂)
        let ln_part = Node::Function(
            "ln".to_string(),
            vec![Node::Add(
                Box::new(Node::Num(ExactNum::integer(1))),
                Box::new(Node::Function(
                    "exp".to_string(),
                    vec![Node::Variable("x".to_string())],
                )),
            )],
        );
        let exp_part = Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())]);
        let expr = Node::Multiply(Box::new(exp_part), Box::new(ln_part));
        let exp_arg = poly(&[0, 1], "x");
        let h = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
        let result = node_to_two_level_log_over_exp(&expr, "x", &exp_arg, &h).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result[0].is_zero());
        let theta1 = ExtPoly::from_coeffs(vec![rf_const(0), rf_const(1)], "x");
        assert_eq!(result[1], theta1);
    }

    #[test]
    fn test_log_over_exp_parse_constant() {
        // 3 → [3] (constant)
        let expr = Node::Num(ExactNum::integer(3));
        let exp_arg = poly(&[0, 1], "x");
        let h = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
        let result = node_to_two_level_log_over_exp(&expr, "x", &exp_arg, &h).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], ExtPoly::from_rf(rf_const(3)));
    }

    // === Structured exp integration tests ===

    #[test]
    fn test_integrate_in_exp_structured_constant() {
        // ∫1 dx in exp(x) extension → x
        let p = ExtPoly::from_rf(rf_const(1));
        let ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let result = integrate_in_exp_ext_structured(&p, &ext, "x").unwrap();
        // x as RationalFunction at θ₁-degree 0
        assert_eq!(result.coeff(0), rf_poly(&[0, 1]));
        assert_eq!(result.degree(), Some(0));
    }

    #[test]
    fn test_integrate_in_exp_structured_theta1() {
        // ∫exp(x) dx = exp(x). θ₁ = exp(x).
        // D(b₁·θ₁) = (b₁' + b₁)·θ₁ = θ₁ → b₁' + b₁ = 1 → b₁ = 1 (constant)
        let p = ExtPoly::from_coeffs(vec![rf_const(0), rf_const(1)], "x");
        let ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let result = integrate_in_exp_ext_structured(&p, &ext, "x").unwrap();
        // Result: 1·θ₁ = ExtPoly [0, 1]
        assert_eq!(result.coeff(0), rf_const(0));
        assert_eq!(result.coeff(1), rf_const(1));
    }

    #[test]
    fn test_integrate_in_exp_structured_non_elementary() {
        // ∫exp(x²) dx — non-elementary.
        // g=x², g'=2x. DE at degree 1: q' + 2x·q = 1 → no polynomial solution.
        let p = ExtPoly::from_coeffs(vec![rf_const(0), rf_const(1)], "x");
        let ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(poly(&[0, 0, 1], "x")),
            "x",
        );
        assert!(
            integrate_in_exp_ext_structured(&p, &ext, "x").is_none(),
            "Should return None (non-elementary)"
        );
    }

    // === Log-over-exp integration tests ===

    #[test]
    fn test_integrate_two_level_log_over_exp_non_elementary() {
        // ∫ln(1+exp(x)) dx → non-elementary
        let outer_coeffs = vec![ExtPoly::zero("x"), ExtPoly::one("x")]; // θ₂
        let inner_ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let h = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x"); // 1+θ₁
        match integrate_two_level_log_over_exp(&outer_coeffs, &inner_ext, &h, "x") {
            Some(RischResult::NonElementary(_)) => {}
            other => panic!("Expected non-elementary, got {:?}", other),
        }
    }

    #[test]
    fn test_integrate_two_level_log_over_exp_exp_times_ln_elementary() {
        // ∫exp(x)·ln(1+exp(x)) dx → elementary
        // D[exp(x)·ln(1+exp(x)) + ln(1+exp(x)) − exp(x)] = exp(x)·ln(1+exp(x))
        let theta1 = ExtPoly::from_coeffs(vec![rf_const(0), rf_const(1)], "x");
        let outer_coeffs = vec![ExtPoly::zero("x"), theta1]; // θ₁·θ₂
        let inner_ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(poly(&[0, 1], "x")),
            "x",
        );
        let h = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
        match integrate_two_level_log_over_exp(&outer_coeffs, &inner_ext, &h, "x") {
            Some(RischResult::Elementary(_)) => {}
            other => panic!("Expected elementary, got {:?}", other),
        }
    }

    #[test]
    fn test_theta1_content_uniform() {
        // [θ₁, θ₁] → content = θ₁
        let tl = vec![ExtPoly::theta("x"), ExtPoly::theta("x")];
        let content = compute_theta1_content(&tl, "x");
        assert_eq!(content.make_monic(), ExtPoly::theta("x").make_monic());
    }

    #[test]
    fn test_theta1_content_no_common_factor() {
        // [1, θ₁] → content = 1 (coprime)
        let tl = vec![ExtPoly::one("x"), ExtPoly::theta("x")];
        let content = compute_theta1_content(&tl, "x");
        assert!(content.is_constant());
    }

    #[test]
    fn test_theta1_content_power() {
        // [θ₁², θ₁²] → content = θ₁²
        let theta1_sq = {
            let t = ExtPoly::theta("x");
            &t * &t
        };
        let tl = vec![theta1_sq.clone(), theta1_sq.clone()];
        let content = compute_theta1_content(&tl, "x");
        assert_eq!(content.degree(), Some(2));
    }

    #[test]
    fn test_theta1_content_with_zero() {
        // [0, θ₁] → content = θ₁ (skip zeros)
        let tl = vec![ExtPoly::zero("x"), ExtPoly::theta("x")];
        let content = compute_theta1_content(&tl, "x");
        assert_eq!(content.make_monic(), ExtPoly::theta("x").make_monic());
    }

    #[test]
    fn test_theta1_content_all_constant() {
        // [rf(1), rf(2)] → content = 1 (Q(x) only, no θ₁)
        let tl = vec![ExtPoly::from_rf(rf_const(1)), ExtPoly::from_rf(rf_const(2))];
        let content = compute_theta1_content(&tl, "x");
        assert!(content.is_constant());
    }
}
