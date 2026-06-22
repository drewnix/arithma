use std::cell::RefCell;
use std::rc::Rc;

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};

use crate::derivative::differentiate;
use crate::environment::Environment;
use crate::evaluator::Evaluator;
use crate::exact::ExactNum;
use crate::fps::FormalPowerSeries;
use crate::node::Node;
use crate::parser::build_expression_tree;
use crate::polynomial::Polynomial;
use crate::simplify::Simplifiable;
use crate::tokenizer::Tokenizer;

/// Compute the Taylor series of expr around center to the given order.
///
/// Returns a polynomial in (var - center). When center = 0 (Maclaurin),
/// the result is a polynomial in var directly.
/// Falls back to symbolic coefficient extraction when the expression
/// contains parameters (variables other than the expansion variable).
pub fn taylor_series(
    expr: &Node,
    var: &str,
    center: &ExactNum,
    order: usize,
) -> Result<Node, String> {
    match taylor_series_numeric(expr, var, center, order) {
        Ok(result) => Ok(result),
        Err(_) => {
            let center_node = Node::Num(center.clone());
            taylor_series_symbolic(expr, var, &center_node, order)
        }
    }
}

fn taylor_series_numeric(
    expr: &Node,
    var: &str,
    center: &ExactNum,
    order: usize,
) -> Result<Node, String> {
    let empty_env = Environment::new();
    let mut eval_env = Environment::new();
    eval_env.set_exact(var, center.clone());

    let mut current = expr.simplify(&empty_env).unwrap_or_else(|_| expr.clone());

    let mut coeffs: Vec<ExactNum> = Vec::with_capacity(order + 1);

    for k in 0..=order {
        let value = try_rationalize(&Evaluator::evaluate_exact(&current, &eval_env)?);
        let fact = factorial_exact(k);
        let coeff = &value / &fact;
        coeffs.push(coeff);

        if k < order {
            current = differentiate(&current, var)?;
            current = current
                .simplify(&empty_env)
                .unwrap_or_else(|_| current.clone());
        }
    }

    build_taylor_node(&coeffs, var, center)
}

/// Taylor series from LaTeX input.
pub fn taylor_series_latex(
    latex_expr: &str,
    var: &str,
    center: f64,
    order: usize,
) -> Result<String, String> {
    let mut tokenizer = Tokenizer::new(latex_expr);
    let tokens = tokenizer.tokenize();
    let expr = build_expression_tree(tokens)?;

    let center_exact = if center == 0.0 {
        ExactNum::zero()
    } else if center == center.floor() && center.abs() < 1e15 {
        ExactNum::integer(center as i64)
    } else {
        ExactNum::from_f64(center)
    };

    let result = taylor_series(&expr, var, &center_exact, order)?;
    let env = Environment::new();
    let simplified = result.simplify(&env).unwrap_or(result);
    Ok(format!("{}", simplified))
}

fn factorial_exact(n: usize) -> ExactNum {
    let mut result = BigRational::one();
    for i in 2..=n {
        result *= BigRational::from_integer(BigInt::from(i));
    }
    ExactNum::Rational(result)
}

/// Convert an ExactNum to rational if it's a float that represents an exact
/// rational number. Recognizes integers, half-integers, and common fractions
/// p/q for small q (up to q=120, covering factorials through 5!).
pub fn try_rationalize(n: &ExactNum) -> ExactNum {
    match n {
        ExactNum::Rational(_) => n.clone(),
        ExactNum::Float(f) => {
            if f.is_nan() || f.is_infinite() {
                return n.clone();
            }
            if *f == 0.0 {
                return ExactNum::Rational(BigRational::zero());
            }
            // Try denominators 1..120 (covers factorials up to 5!)
            for denom in 1..=120i64 {
                let numer = (*f * denom as f64).round();
                if numer.abs() > 1e15 {
                    continue;
                }
                let reconstructed = numer / denom as f64;
                if (*f - reconstructed).abs() < 1e-12 * f.abs().max(1.0) {
                    return ExactNum::Rational(BigRational::new(
                        BigInt::from(numer as i64),
                        BigInt::from(denom),
                    ));
                }
            }
            n.clone()
        }
    }
}

/// Build a Node representing the Taylor polynomial.
/// For center = 0 and all-rational coefficients, uses Polynomial for clean output.
fn build_taylor_node(coeffs: &[ExactNum], var: &str, center: &ExactNum) -> Result<Node, String> {
    if center.is_zero() {
        // Try the Polynomial path for clean output
        let rat_coeffs: Option<Vec<BigRational>> = coeffs
            .iter()
            .map(|c| match c {
                ExactNum::Rational(r) => Some(r.clone()),
                ExactNum::Float(f) => {
                    // Convert exact-looking floats to rationals
                    if *f == 0.0 {
                        Some(BigRational::zero())
                    } else if *f == f.round() && f.abs() < 1e15 {
                        Some(BigRational::from_integer(BigInt::from(*f as i64)))
                    } else {
                        None
                    }
                }
            })
            .collect();

        if let Some(rcs) = rat_coeffs {
            let poly = Polynomial::from_coeffs(rcs, var);
            return Ok(poly.to_node());
        }
    }

    // General case: build Node directly
    let shifted = if center.is_zero() {
        Node::Variable(var.to_string())
    } else {
        Node::Subtract(
            Box::new(Node::Variable(var.to_string())),
            Box::new(Node::Num(center.clone())),
        )
    };

    let mut terms: Vec<Node> = Vec::new();

    for (k, coeff) in coeffs.iter().enumerate() {
        if coeff.is_zero() {
            continue;
        }

        let term = if k == 0 {
            Node::Num(coeff.clone())
        } else {
            let power_node = if k == 1 {
                shifted.clone()
            } else {
                Node::Power(
                    Box::new(shifted.clone()),
                    Box::new(Node::Num(ExactNum::integer(k as i64))),
                )
            };

            if coeff.is_one() {
                power_node
            } else if coeff == &ExactNum::integer(-1) {
                Node::Negate(Box::new(power_node))
            } else {
                Node::Multiply(Box::new(Node::Num(coeff.clone())), Box::new(power_node))
            }
        };

        terms.push(term);
    }

    if terms.is_empty() {
        return Ok(Node::Num(ExactNum::zero()));
    }

    let mut result = terms.remove(0);
    for term in terms {
        result = Node::Add(Box::new(result), Box::new(term));
    }

    Ok(result)
}

/// Compute the Taylor series of expr around a symbolic center to the given order.
///
/// Unlike `taylor_series`, the center can be any `Node` (e.g. a variable like `a`),
/// not just a numeric `ExactNum`. Coefficients are computed by substituting the
/// symbolic center and simplifying, so the result is a symbolic expression.
pub fn taylor_series_symbolic(
    expr: &Node,
    var: &str,
    center: &Node,
    order: usize,
) -> Result<Node, String> {
    use crate::substitute::substitute_variable;

    let env = Environment::new();
    let mut current = expr.simplify(&env).unwrap_or_else(|_| expr.clone());
    let mut coeffs: Vec<Node> = Vec::with_capacity(order + 1);

    for k in 0..=order {
        let substituted = substitute_variable(&current, var, center)?;
        let value = substituted.simplify(&env).unwrap_or(substituted);
        let fact = factorial_exact(k);
        let coeff = if fact.is_one() {
            value
        } else {
            Node::Divide(Box::new(value), Box::new(Node::Num(fact)))
                .simplify(&env)
                .unwrap_or_else(|_| Node::Num(ExactNum::zero()))
        };
        coeffs.push(coeff);

        if k < order {
            current = differentiate(&current, var)?;
            current = current.simplify(&env).unwrap_or_else(|_| current.clone());
        }
    }

    build_taylor_node_symbolic(&coeffs, var, center)
}

fn build_taylor_node_symbolic(coeffs: &[Node], var: &str, center: &Node) -> Result<Node, String> {
    let shifted = Node::Subtract(
        Box::new(Node::Variable(var.to_string())),
        Box::new(center.clone()),
    );

    let mut terms: Vec<Node> = Vec::new();

    for (k, coeff) in coeffs.iter().enumerate() {
        if matches!(coeff, Node::Num(n) if n.is_zero()) {
            continue;
        }

        let term = if k == 0 {
            coeff.clone()
        } else {
            let power_node = if k == 1 {
                shifted.clone()
            } else {
                Node::Power(
                    Box::new(shifted.clone()),
                    Box::new(Node::Num(ExactNum::integer(k as i64))),
                )
            };

            if matches!(coeff, Node::Num(n) if n.is_one()) {
                power_node
            } else {
                Node::Multiply(Box::new(coeff.clone()), Box::new(power_node))
            }
        };

        terms.push(term);
    }

    if terms.is_empty() {
        return Ok(Node::Num(ExactNum::zero()));
    }

    let mut result = terms.remove(0);
    for term in terms {
        result = Node::Add(Box::new(result), Box::new(term));
    }

    Ok(result)
}

/// Taylor series with symbolic center from LaTeX input.
pub fn taylor_series_latex_symbolic(
    latex_expr: &str,
    var: &str,
    center_latex: &str,
    order: usize,
) -> Result<String, String> {
    let mut tokenizer = Tokenizer::new(latex_expr);
    let tokens = tokenizer.tokenize();
    let expr = build_expression_tree(tokens)?;

    let mut center_tokenizer = Tokenizer::new(center_latex);
    let center_tokens = center_tokenizer.tokenize();
    let center = build_expression_tree(center_tokens)?;

    let result = taylor_series_symbolic(&expr, var, &center, order)?;
    Ok(format!("{}", result))
}

/// Multivariate Taylor expansion: f(x₁,...,x_m) around (a₁,...,a_m) to total degree N.
///
/// Computes Σ (1/α!) · (∂^|α|f/∂x^α)|_{center} · ∏(x_i - a_i)^α_i
/// summed over all multi-indices α with |α| ≤ N.
pub fn taylor_series_multivar(
    expr: &Node,
    vars: &[&str],
    centers: &[Node],
    order: usize,
) -> Result<Node, String> {
    use crate::substitute::substitute_variable;

    if vars.len() != centers.len() {
        return Err(format!(
            "Number of variables ({}) must match number of center points ({})",
            vars.len(),
            centers.len()
        ));
    }
    if vars.is_empty() {
        return Err("At least one variable required".to_string());
    }

    let m = vars.len();
    let env = Environment::new();

    let multi_indices = generate_multi_indices(m, order);
    let mut terms: Vec<Node> = Vec::new();

    for alpha in &multi_indices {
        // Compute mixed partial derivative ∂^|α|f / ∂x₁^α₁ ··· ∂x_m^α_m
        let mut deriv = expr.simplify(&env).unwrap_or_else(|_| expr.clone());
        for (i, &a_i) in alpha.iter().enumerate() {
            for _ in 0..a_i {
                deriv = differentiate(&deriv, vars[i])?;
                deriv = deriv.simplify(&env).unwrap_or(deriv);
            }
        }

        // Evaluate at center: substitute each x_i = a_i
        let mut value = deriv;
        for (i, center) in centers.iter().enumerate() {
            value = substitute_variable(&value, vars[i], center)?;
        }
        value = value.simplify(&env).unwrap_or(value);

        if matches!(&value, Node::Num(n) if n.is_zero()) {
            continue;
        }

        // Coefficient: value / α! where α! = α₁! · α₂! · ··· · α_m!
        let mut alpha_fact = BigRational::one();
        for &a_i in alpha {
            for j in 2..=a_i {
                alpha_fact *= BigRational::from_integer(BigInt::from(j));
            }
        }
        let coeff = if alpha_fact.is_one() {
            value
        } else {
            Node::Divide(
                Box::new(value),
                Box::new(Node::Num(ExactNum::Rational(alpha_fact))),
            )
            .simplify(&env)
            .unwrap_or_else(|_| Node::Num(ExactNum::zero()))
        };

        if matches!(&coeff, Node::Num(n) if n.is_zero()) {
            continue;
        }

        // Build monomial ∏(x_i - a_i)^α_i
        let total_degree: usize = alpha.iter().sum();
        let mut monomial: Option<Node> = None;
        for (i, &a_i) in alpha.iter().enumerate() {
            if a_i == 0 {
                continue;
            }
            let shifted = if matches!(&centers[i], Node::Num(n) if n.is_zero()) {
                Node::Variable(vars[i].to_string())
            } else {
                Node::Subtract(
                    Box::new(Node::Variable(vars[i].to_string())),
                    Box::new(centers[i].clone()),
                )
            };
            let factor = if a_i == 1 {
                shifted
            } else {
                Node::Power(
                    Box::new(shifted),
                    Box::new(Node::Num(ExactNum::integer(a_i as i64))),
                )
            };
            monomial = Some(match monomial {
                None => factor,
                Some(prev) => Node::Multiply(Box::new(prev), Box::new(factor)),
            });
        }

        let term = match (total_degree, monomial) {
            (0, _) => coeff,
            (_, Some(mono)) => {
                if matches!(&coeff, Node::Num(n) if n.is_one()) {
                    mono
                } else {
                    Node::Multiply(Box::new(coeff), Box::new(mono))
                }
            }
            _ => coeff,
        };

        terms.push(term);
    }

    if terms.is_empty() {
        return Ok(Node::Num(ExactNum::zero()));
    }

    let mut result = terms.remove(0);
    for term in terms {
        result = Node::Add(Box::new(result), Box::new(term));
    }

    result.simplify(&env)
}

/// Generate all multi-indices (α₁,...,α_m) with α₁+...+α_m ≤ max_total,
/// in graded lexicographic order.
fn generate_multi_indices(m: usize, max_total: usize) -> Vec<Vec<usize>> {
    let mut result = Vec::new();
    let mut current = vec![0usize; m];
    generate_multi_indices_inner(&mut result, &mut current, 0, max_total);
    result
}

fn generate_multi_indices_inner(
    result: &mut Vec<Vec<usize>>,
    current: &mut Vec<usize>,
    pos: usize,
    remaining: usize,
) {
    if pos == current.len() {
        result.push(current.clone());
        return;
    }
    for k in 0..=remaining {
        current[pos] = k;
        generate_multi_indices_inner(result, current, pos + 1, remaining - k);
    }
}

/// Multivariate Taylor from LaTeX input.
pub fn taylor_series_multivar_latex(
    latex_expr: &str,
    vars: &[&str],
    center_strs: &[&str],
    order: usize,
) -> Result<String, String> {
    let expr = {
        let mut tok = Tokenizer::new(latex_expr);
        build_expression_tree(tok.tokenize())?
    };

    let centers: Vec<Node> = center_strs
        .iter()
        .map(|s| {
            if *s == "0" {
                Ok(Node::Num(ExactNum::zero()))
            } else {
                let mut tok = Tokenizer::new(s);
                build_expression_tree(tok.tokenize())
            }
        })
        .collect::<Result<Vec<_>, _>>()?;

    let result = taylor_series_multivar(&expr, vars, &centers, order)?;
    let env = Environment::new();
    let simplified = result.simplify(&env).unwrap_or(result);
    Ok(format!("{}", simplified))
}

/// Create a formal power series from the Taylor expansion of an expression.
///
/// The FPS lazily computes coefficients a_n = f^{(n)}(center) / n! by
/// caching successive derivatives.
pub fn taylor_to_fps(
    expr: &Node,
    var: &str,
    center: &ExactNum,
) -> Result<FormalPowerSeries, String> {
    let env = Environment::new();
    let simplified = expr.simplify(&env).unwrap_or_else(|_| expr.clone());

    let deriv_cache: Rc<RefCell<Vec<Node>>> = Rc::new(RefCell::new(vec![simplified]));
    let var_string = var.to_string();
    let center_clone = center.clone();
    let cache = deriv_cache;

    Ok(FormalPowerSeries::from_fn(move |n| {
        let mut dc = cache.borrow_mut();

        while dc.len() <= n {
            let last = dc.last().unwrap().clone();
            let env = Environment::new();
            match differentiate(&last, &var_string) {
                Ok(d) => {
                    let simplified = d.simplify(&env).unwrap_or(d);
                    dc.push(simplified);
                }
                Err(_) => {
                    dc.push(Node::Num(ExactNum::zero()));
                }
            }
        }

        let mut eval_env = Environment::new();
        eval_env.set_exact(&var_string, center_clone.clone());

        match Evaluator::evaluate_exact(&dc[n], &eval_env) {
            Ok(val) => {
                let rationalized = try_rationalize(&val);
                let fact = factorial_exact(n);
                match (&rationalized, &fact) {
                    (ExactNum::Rational(v), ExactNum::Rational(f)) => v / f,
                    _ => BigRational::zero(),
                }
            }
            Err(_) => BigRational::zero(),
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_taylor_polynomial_identity() {
        // Taylor series of x^2 + x + 1 around 0, order 3 → x^2 + x + 1
        let x = Node::Variable("x".to_string());
        let x2 = Node::Power(
            Box::new(x.clone()),
            Box::new(Node::Num(ExactNum::integer(2))),
        );
        let expr = Node::Add(
            Box::new(Node::Add(Box::new(x2), Box::new(x.clone()))),
            Box::new(Node::Num(ExactNum::integer(1))),
        );
        let result = taylor_series(&expr, "x", &ExactNum::zero(), 3).unwrap();
        // Evaluate at x=5: 25 + 5 + 1 = 31
        let mut env = Environment::new();
        env.set_exact("x", ExactNum::integer(5));
        let val = Evaluator::evaluate_exact(&result, &env).unwrap();
        assert_eq!(val, ExactNum::integer(31));
    }

    #[test]
    fn test_taylor_exp_maclaurin() {
        // e^x around 0, order 4: 1 + x + x²/2 + x³/6 + x⁴/24
        let expr = Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())]);
        let result = taylor_series(&expr, "x", &ExactNum::zero(), 4).unwrap();
        // Evaluate at x=0: should be 1
        let mut env = Environment::new();
        env.set_exact("x", ExactNum::zero());
        let val = Evaluator::evaluate(&result, &env).unwrap();
        assert!((val - 1.0).abs() < 1e-10);
        // Evaluate at x=1: 1 + 1 + 1/2 + 1/6 + 1/24 = 2.708333...
        env.set_exact("x", ExactNum::integer(1));
        let val = Evaluator::evaluate(&result, &env).unwrap();
        assert!((val - 2.708333333).abs() < 1e-5);
    }

    #[test]
    fn test_taylor_sin_maclaurin() {
        // sin(x) around 0, order 5: x - x³/6 + x⁵/120
        let expr = Node::Function("sin".to_string(), vec![Node::Variable("x".to_string())]);
        let result = taylor_series(&expr, "x", &ExactNum::zero(), 5).unwrap();
        // Evaluate at x=0.5
        let mut env = Environment::new();
        env.set("x", 0.5);
        let val = Evaluator::evaluate(&result, &env).unwrap();
        let exact = 0.5_f64.sin();
        assert!((val - exact).abs() < 1e-4); // 5th order approximation
    }

    #[test]
    fn test_taylor_cos_maclaurin() {
        // cos(x) around 0, order 4: 1 - x²/2 + x⁴/24
        let expr = Node::Function("cos".to_string(), vec![Node::Variable("x".to_string())]);
        let result = taylor_series(&expr, "x", &ExactNum::zero(), 4).unwrap();
        let mut env = Environment::new();
        env.set("x", 0.3);
        let val = Evaluator::evaluate(&result, &env).unwrap();
        let exact = 0.3_f64.cos();
        eprintln!("val={}, exact={}, diff={}", val, exact, (val - exact).abs());
        assert!((val - exact).abs() < 1e-4);
    }

    #[test]
    fn test_taylor_shifted_center() {
        // Taylor series of x^2 around center=1, order 2
        // f(x) = x^2, f(1)=1, f'(1)=2, f''(1)=2
        // T(x) = 1 + 2(x-1) + (x-1)^2
        let x = Node::Variable("x".to_string());
        let expr = Node::Power(Box::new(x), Box::new(Node::Num(ExactNum::integer(2))));
        let result = taylor_series(&expr, "x", &ExactNum::integer(1), 2).unwrap();
        // Evaluate at x=3: 1 + 2*2 + 4 = 9 = 3^2 ✓
        let mut env = Environment::new();
        env.set_exact("x", ExactNum::integer(3));
        let val = Evaluator::evaluate_exact(&result, &env).unwrap();
        assert_eq!(val.to_f64(), 9.0);
    }

    #[test]
    fn test_taylor_latex_roundtrip() {
        // sin(x) Maclaurin order 5
        let result = taylor_series_latex("\\sin(x)", "x", 0.0, 5).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_taylor_rational_function() {
        // 1/(1-x) around 0, order 4: 1 + x + x² + x³ + x⁴
        let expr = Node::Divide(
            Box::new(Node::Num(ExactNum::integer(1))),
            Box::new(Node::Subtract(
                Box::new(Node::Num(ExactNum::integer(1))),
                Box::new(Node::Variable("x".to_string())),
            )),
        );
        let result = taylor_series(&expr, "x", &ExactNum::zero(), 4).unwrap();
        // Evaluate at x=0.5: 1 + 0.5 + 0.25 + 0.125 + 0.0625 = 1.9375
        let mut env = Environment::new();
        env.set("x", 0.5);
        let val = Evaluator::evaluate(&result, &env).unwrap();
        assert!((val - 1.9375).abs() < 1e-10);
    }

    #[test]
    fn test_factorial() {
        assert_eq!(factorial_exact(0), ExactNum::integer(1));
        assert_eq!(factorial_exact(1), ExactNum::integer(1));
        assert_eq!(factorial_exact(5), ExactNum::integer(120));
    }

    #[test]
    fn test_taylor_symbolic_center_linear() {
        // Taylor series of x^2 around x=a, order 2
        // f(x) = x^2, f(a)=a^2, f'(a)=2a, f''(a)=2
        // T(x) = a^2 + 2a(x-a) + (x-a)^2
        let x = Node::Variable("x".to_string());
        let expr = Node::Power(Box::new(x), Box::new(Node::Num(ExactNum::integer(2))));
        let center = Node::Variable("a".to_string());
        let result = taylor_series_symbolic(&expr, "x", &center, 2).unwrap();

        // Evaluate at x=5, a=2: should get 25.0 (exact for polynomials at matching degree)
        let mut env = Environment::new();
        env.set("x", 5.0);
        env.set("a", 2.0);
        let val = Evaluator::evaluate(&result, &env).unwrap();
        assert!((val - 25.0).abs() < 1e-10, "Expected 25.0, got {}", val);
    }

    #[test]
    fn test_taylor_symbolic_center_rational() {
        // Taylor of 3/(1+2x) around x=a, order 2
        let env = Environment::new();
        let expr = crate::parse_latex("\\frac{3}{1+2x}", &env).unwrap();
        let center = Node::Variable("a".to_string());
        let result = taylor_series_symbolic(&expr, "x", &center, 2).unwrap();

        // Evaluate at x=0.6, a=0.5
        let mut eval_env = Environment::new();
        eval_env.set("x", 0.6);
        eval_env.set("a", 0.5);
        let val = Evaluator::evaluate(&result, &eval_env).unwrap();
        let exact = 3.0 / (1.0 + 2.0 * 0.6); // 1.3636...
        assert!(
            (val - exact).abs() < 0.01,
            "Expected ~{}, got {} (diff={})",
            exact,
            val,
            (val - exact).abs()
        );
    }

    #[test]
    fn test_taylor_symbolic_center_latex_interface() {
        // Test the LaTeX interface for symbolic centers
        let result = taylor_series_latex_symbolic("x^2", "x", "a", 2).unwrap();
        assert!(!result.is_empty(), "Result should be non-empty");
        assert!(
            result.contains('a'),
            "Result should contain symbolic center 'a', got: {}",
            result
        );
    }

    #[test]
    fn test_taylor_parametric_fallback() {
        // n/(1+(n-1)a) expanded in a around 0, order 3
        // f(a) = n/(1+(n-1)a), f(0) = n
        // f'(a) = -n(n-1)/(1+(n-1)a)^2, f'(0) = -n(n-1)
        // Coefficients: n, -n(n-1), n(n-1)^2, -n(n-1)^3
        let result = taylor_series_latex("\\frac{n}{1+(n-1)a}", "a", 0.0, 3).unwrap();
        assert!(
            result.contains('n'),
            "Result should contain parameter 'n', got: {}",
            result
        );
        assert!(
            !result.contains("Error") && !result.contains("not defined"),
            "Should not error on parametric expression, got: {}",
            result
        );
        // Verify: at n=4, a=0.1 the original is 4/(1+3·0.1) = 4/1.3 ≈ 3.07692
        let mut tokenizer = Tokenizer::new(&result);
        let expr = build_expression_tree(tokenizer.tokenize()).unwrap();
        let mut eval_env = Environment::new();
        eval_env.set("n", 4.0);
        eval_env.set("a", 0.1);
        let val = Evaluator::evaluate(&expr, &eval_env).unwrap();
        let exact = 4.0 / (1.0 + 3.0 * 0.1);
        assert!(
            (val - exact).abs() < 0.05,
            "Parametric Taylor at n=4,a=0.1: expected ~{}, got {}",
            exact,
            val
        );
    }

    #[test]
    fn test_taylor_parametric_preserves_numeric_path() {
        // Pure numeric case still works via the fast path
        let result = taylor_series_latex("\\frac{1}{1-x}", "x", 0.0, 4).unwrap();
        let mut tokenizer = Tokenizer::new(&result);
        let expr = build_expression_tree(tokenizer.tokenize()).unwrap();
        let mut eval_env = Environment::new();
        eval_env.set("x", 0.5);
        let val = Evaluator::evaluate(&expr, &eval_env).unwrap();
        assert!(
            (val - 1.9375).abs() < 1e-10,
            "1/(1-x) Taylor at x=0.5 should be 1.9375, got {}",
            val
        );
    }

    #[test]
    fn test_multivar_taylor_polynomial() {
        // f(x,y) = x² + xy + y² around (0,0), order 2
        // Should return exactly x² + xy + y² (polynomial of degree 2)
        let result =
            taylor_series_multivar_latex("x^2 + x \\cdot y + y^2", &["x", "y"], &["0", "0"], 2)
                .unwrap();
        let mut eval_env = Environment::new();
        eval_env.set("x", 2.0);
        eval_env.set("y", 3.0);
        let mut tok = Tokenizer::new(&result);
        let expr = build_expression_tree(tok.tokenize()).unwrap();
        let val = Evaluator::evaluate(&expr, &eval_env).unwrap();
        // 4 + 6 + 9 = 19
        assert!(
            (val - 19.0).abs() < 1e-10,
            "x²+xy+y² at (2,3) should be 19, got {} from: {}",
            val,
            result
        );
    }

    #[test]
    fn test_multivar_taylor_rational() {
        // f(x,y) = 1/(1-x-y) around (0,0), order 2
        // = 1 + (x+y) + (x+y)² + ... truncated at degree 2
        // = 1 + x + y + x² + 2xy + y²
        let result =
            taylor_series_multivar_latex("\\frac{1}{1-x-y}", &["x", "y"], &["0", "0"], 2).unwrap();
        let mut eval_env = Environment::new();
        eval_env.set("x", 0.1);
        eval_env.set("y", 0.2);
        let mut tok = Tokenizer::new(&result);
        let expr = build_expression_tree(tok.tokenize()).unwrap();
        let val = Evaluator::evaluate(&expr, &eval_env).unwrap();
        // 1 + 0.1 + 0.2 + 0.01 + 0.04 + 0.04 = 1.4
        let expected = 1.0 + 0.1 + 0.2 + 0.01 + 2.0 * 0.1 * 0.2 + 0.04;
        assert!(
            (val - expected).abs() < 1e-10,
            "1/(1-x-y) Taylor(2) at (0.1,0.2): expected {}, got {} from: {}",
            expected,
            val,
            result
        );
    }

    #[test]
    fn test_multivar_taylor_mixed_partials() {
        // f(x,y) = sin(x)·cos(y) around (0,0), order 3
        // = x - x³/6 - xy²/2 + ... (terms up to total degree 3)
        let result =
            taylor_series_multivar_latex("\\sin(x) \\cdot \\cos(y)", &["x", "y"], &["0", "0"], 3)
                .unwrap();
        let mut eval_env = Environment::new();
        eval_env.set("x", 0.3);
        eval_env.set("y", 0.2);
        let mut tok = Tokenizer::new(&result);
        let expr = build_expression_tree(tok.tokenize()).unwrap();
        let val = Evaluator::evaluate(&expr, &eval_env).unwrap();
        let exact = 0.3_f64.sin() * 0.2_f64.cos();
        assert!(
            (val - exact).abs() < 0.001,
            "sin(x)cos(y) Taylor(3) at (0.3,0.2): expected ~{}, got {} from: {}",
            exact,
            val,
            result
        );
    }

    #[test]
    fn test_multivar_taylor_nonzero_center() {
        // f(x,y) = x·y around (1,1), order 2
        // f = 1 + (x-1) + (y-1) + (x-1)(y-1)
        let result =
            taylor_series_multivar_latex("x \\cdot y", &["x", "y"], &["1", "1"], 2).unwrap();
        let mut eval_env = Environment::new();
        eval_env.set("x", 3.0);
        eval_env.set("y", 4.0);
        let mut tok = Tokenizer::new(&result);
        let expr = build_expression_tree(tok.tokenize()).unwrap();
        let val = Evaluator::evaluate(&expr, &eval_env).unwrap();
        // x·y at (3,4) = 12 (exact for bilinear at order 2)
        assert!(
            (val - 12.0).abs() < 1e-10,
            "x·y at (3,4) should be 12, got {} from: {}",
            val,
            result
        );
    }

    #[test]
    fn test_taylor_to_fps_exp() {
        // Taylor FPS of e^x around 0 should match FPS::exp()
        let expr = Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())]);
        let fps = taylor_to_fps(&expr, "x", &ExactNum::zero()).unwrap();
        let expected = FormalPowerSeries::exp();
        for n in 0..6 {
            assert_eq!(
                fps.coeff(n),
                expected.coeff(n),
                "taylor_to_fps(e^x) coeff({}) mismatch",
                n
            );
        }
    }

    #[test]
    fn test_taylor_to_fps_sin() {
        let expr = Node::Function("sin".to_string(), vec![Node::Variable("x".to_string())]);
        let fps = taylor_to_fps(&expr, "x", &ExactNum::zero()).unwrap();
        let expected = FormalPowerSeries::sin();
        for n in 0..6 {
            assert_eq!(
                fps.coeff(n),
                expected.coeff(n),
                "taylor_to_fps(sin(x)) coeff({}) mismatch",
                n
            );
        }
    }

    #[test]
    fn test_taylor_to_fps_geometric() {
        // 1/(1-x) around 0
        let expr = Node::Divide(
            Box::new(Node::Num(ExactNum::integer(1))),
            Box::new(Node::Subtract(
                Box::new(Node::Num(ExactNum::integer(1))),
                Box::new(Node::Variable("x".to_string())),
            )),
        );
        let fps = taylor_to_fps(&expr, "x", &ExactNum::zero()).unwrap();
        for n in 0..8 {
            assert_eq!(
                fps.coeff(n),
                BigRational::one(),
                "taylor_to_fps(1/(1-x)) coeff({}) should be 1",
                n
            );
        }
    }
}
