use crate::ext_poly::ExtPoly;
use crate::rational_function::RationalFunction;
use num_bigint::BigInt;
use num_rational::BigRational;

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

#[cfg(test)]
mod tests {
    use super::*;
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
}
