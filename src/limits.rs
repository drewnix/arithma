use crate::derivative::differentiate;
use crate::environment::Environment;
use crate::evaluator::Evaluator;
use crate::exact::ExactNum;
use crate::node::Node;
use crate::parser::build_expression_tree;
use crate::polynomial::Polynomial;
use crate::series::try_rationalize;
use crate::simplify::Simplifiable;
use crate::tokenizer::Tokenizer;

const MAX_LHOPITAL_ITERATIONS: usize = 6;

/// Compute the limit of expr as var → point.
pub fn compute_limit(expr: &Node, var: &str, point: &ExactNum) -> Result<ExactNum, String> {
    let env = Environment::new();
    let simplified = expr.simplify(&env).unwrap_or_else(|_| expr.clone());
    let result = limit_internal(&simplified, var, point, 0)?;
    Ok(try_rationalize(&result))
}

fn limit_internal(
    expr: &Node,
    var: &str,
    point: &ExactNum,
    depth: usize,
) -> Result<ExactNum, String> {
    if depth > MAX_LHOPITAL_ITERATIONS {
        return Err("Limit computation did not converge".to_string());
    }

    // Step 1: try direct substitution
    let mut eval_env = Environment::new();
    eval_env.set_exact(var, point.clone());
    if let Ok(val) = Evaluator::evaluate_exact(expr, &eval_env) {
        if !val.is_nan_or_inf() {
            return Ok(val);
        }
    }

    // Step 2: if it's a quotient, analyze the form
    if let Node::Divide(numer, denom) = expr {
        return limit_quotient(numer, denom, var, point, depth);
    }

    // Step 3: try simplifying harder and retrying
    let env = Environment::new();
    if let Ok(resimplified) = expr.simplify(&env) {
        if &resimplified != expr {
            let mut eval_env = Environment::new();
            eval_env.set_exact(var, point.clone());
            if let Ok(val) = Evaluator::evaluate_exact(&resimplified, &eval_env) {
                if !val.is_nan_or_inf() {
                    return Ok(val);
                }
            }
        }
    }

    // Step 4: try Taylor series at the point
    if let Ok(taylor) = crate::series::taylor_series(expr, var, point, 4) {
        let mut eval_env = Environment::new();
        eval_env.set_exact(var, point.clone());
        if let Ok(val) = Evaluator::evaluate_exact(&taylor, &eval_env) {
            if !val.is_nan_or_inf() {
                return Ok(val);
            }
        }
    }

    Err(format!(
        "Cannot compute limit of {} as {} → {}",
        expr, var, point
    ))
}

fn limit_quotient(
    numer: &Node,
    denom: &Node,
    var: &str,
    point: &ExactNum,
    depth: usize,
) -> Result<ExactNum, String> {
    let mut eval_env = Environment::new();
    eval_env.set_exact(var, point.clone());

    let n_val = Evaluator::evaluate_exact(numer, &eval_env);
    let d_val = Evaluator::evaluate_exact(denom, &eval_env);

    match (&n_val, &d_val) {
        (Ok(n), Ok(d)) if !n.is_nan_or_inf() && !d.is_nan_or_inf() => {
            if d.is_zero() && n.is_zero() {
                // 0/0 indeterminate form
                return limit_zero_over_zero(numer, denom, var, point, depth);
            }
            if d.is_zero() {
                return Err("Limit is infinite (nonzero/zero)".to_string());
            }
            // Normal case — should have been caught by direct substitution
            Ok(n / d)
        }
        _ => {
            // Evaluation failed on one or both — try GCD then L'Hôpital
            limit_zero_over_zero(numer, denom, var, point, depth)
        }
    }
}

fn limit_zero_over_zero(
    numer: &Node,
    denom: &Node,
    var: &str,
    point: &ExactNum,
    depth: usize,
) -> Result<ExactNum, String> {
    // Strategy 1: polynomial GCD cancellation
    if let Some(result) = try_polynomial_cancel(numer, denom, var, point) {
        return result;
    }

    // Strategy 2: L'Hôpital's rule
    let env = Environment::new();
    let n_prime = differentiate(numer, var).and_then(|d| d.simplify(&env).or(Ok(d)))?;
    let d_prime = differentiate(denom, var).and_then(|d| d.simplify(&env).or(Ok(d)))?;

    let new_expr = Node::Divide(Box::new(n_prime), Box::new(d_prime));
    limit_internal(&new_expr, var, point, depth + 1)
}

fn try_polynomial_cancel(
    numer: &Node,
    denom: &Node,
    var: &str,
    point: &ExactNum,
) -> Option<Result<ExactNum, String>> {
    let n_poly = Polynomial::from_node(numer, var).ok()?;
    let d_poly = Polynomial::from_node(denom, var).ok()?;

    if d_poly.is_zero() {
        return None;
    }

    let g = n_poly.gcd(&d_poly);
    if g.degree()? == 0 {
        return None;
    }

    let (n_reduced, n_rem) = n_poly.div_rem(&g).ok()?;
    let (d_reduced, d_rem) = d_poly.div_rem(&g).ok()?;
    if !n_rem.is_zero() || !d_rem.is_zero() {
        return None;
    }

    // Evaluate the reduced quotient at the point
    let n_node = n_reduced.to_node();
    let d_node = d_reduced.to_node();

    let mut eval_env = Environment::new();
    eval_env.set_exact(var, point.clone());

    let n_val = Evaluator::evaluate_exact(&n_node, &eval_env).ok()?;
    let d_val = Evaluator::evaluate_exact(&d_node, &eval_env).ok()?;

    if d_val.is_zero() {
        // Still 0/0 after cancellation — shouldn't happen for polynomials
        // but might if point is a repeated root
        return None;
    }

    Some(Ok(&n_val / &d_val))
}

/// Compute limit from LaTeX expression.
pub fn limit_latex(expr_latex: &str, var: &str, point: f64) -> Result<String, String> {
    let mut tokenizer = Tokenizer::new(expr_latex);
    let tokens = tokenizer.tokenize();
    let expr = build_expression_tree(tokens)?;

    let point_exact = if point == 0.0 {
        ExactNum::zero()
    } else if point == point.floor() && point.abs() < 1e15 {
        ExactNum::integer(point as i64)
    } else {
        ExactNum::from_f64(point)
    };

    let result = compute_limit(&expr, var, &point_exact)?;
    Ok(format!("{}", Node::Num(result)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_limit_continuous() {
        // lim_{x→2} x^2 + 1 = 5
        let x = Node::Variable("x".to_string());
        let expr = Node::Add(
            Box::new(Node::Power(
                Box::new(x),
                Box::new(Node::Num(ExactNum::integer(2))),
            )),
            Box::new(Node::Num(ExactNum::integer(1))),
        );
        let result = compute_limit(&expr, "x", &ExactNum::integer(2)).unwrap();
        assert_eq!(result, ExactNum::integer(5));
    }

    #[test]
    fn test_limit_zero_over_zero_polynomial() {
        // lim_{x→1} (x^2 - 1)/(x - 1) = lim (x+1) = 2
        let x = Node::Variable("x".to_string());
        let numer = Node::Subtract(
            Box::new(Node::Power(
                Box::new(x.clone()),
                Box::new(Node::Num(ExactNum::integer(2))),
            )),
            Box::new(Node::Num(ExactNum::integer(1))),
        );
        let denom = Node::Subtract(Box::new(x), Box::new(Node::Num(ExactNum::integer(1))));
        let expr = Node::Divide(Box::new(numer), Box::new(denom));
        let result = compute_limit(&expr, "x", &ExactNum::integer(1)).unwrap();
        assert_eq!(result.to_f64(), 2.0);
    }

    #[test]
    fn test_limit_sinx_over_x() {
        // lim_{x→0} sin(x)/x = 1 (via L'Hôpital: cos(x)/1 = 1)
        let x = Node::Variable("x".to_string());
        let numer = Node::Function("sin".to_string(), vec![x.clone()]);
        let denom = x;
        let expr = Node::Divide(Box::new(numer), Box::new(denom));
        let result = compute_limit(&expr, "x", &ExactNum::zero()).unwrap();
        assert!((result.to_f64() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_limit_one_minus_cosx_over_x2() {
        // lim_{x→0} (1 - cos(x))/x^2 = 1/2
        // L'Hôpital once: sin(x)/(2x) — still 0/0
        // L'Hôpital twice: cos(x)/2 = 1/2
        let x = Node::Variable("x".to_string());
        let numer = Node::Subtract(
            Box::new(Node::Num(ExactNum::integer(1))),
            Box::new(Node::Function("cos".to_string(), vec![x.clone()])),
        );
        let denom = Node::Power(Box::new(x), Box::new(Node::Num(ExactNum::integer(2))));
        let expr = Node::Divide(Box::new(numer), Box::new(denom));
        let result = compute_limit(&expr, "x", &ExactNum::zero()).unwrap();
        assert!((result.to_f64() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_limit_cubic_over_linear() {
        // lim_{x→0} (x^3 + x^2)/(x) = lim x^2 + x = 0
        let x = Node::Variable("x".to_string());
        let x2 = Node::Power(
            Box::new(x.clone()),
            Box::new(Node::Num(ExactNum::integer(2))),
        );
        let x3 = Node::Power(
            Box::new(x.clone()),
            Box::new(Node::Num(ExactNum::integer(3))),
        );
        let numer = Node::Add(Box::new(x3), Box::new(x2));
        let expr = Node::Divide(Box::new(numer), Box::new(x));
        let result = compute_limit(&expr, "x", &ExactNum::zero()).unwrap();
        assert_eq!(result.to_f64(), 0.0);
    }

    #[test]
    fn test_limit_exp_minus_1_over_x() {
        // lim_{x→0} (e^x - 1)/x = 1 (via L'Hôpital: e^x/1 = 1)
        let x = Node::Variable("x".to_string());
        let numer = Node::Subtract(
            Box::new(Node::Function("exp".to_string(), vec![x.clone()])),
            Box::new(Node::Num(ExactNum::integer(1))),
        );
        let expr = Node::Divide(Box::new(numer), Box::new(x));
        let result = compute_limit(&expr, "x", &ExactNum::zero()).unwrap();
        assert!((result.to_f64() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_limit_at_nonzero_point() {
        // lim_{x→3} (x^2 - 9)/(x - 3) = 6
        let x = Node::Variable("x".to_string());
        let numer = Node::Subtract(
            Box::new(Node::Power(
                Box::new(x.clone()),
                Box::new(Node::Num(ExactNum::integer(2))),
            )),
            Box::new(Node::Num(ExactNum::integer(9))),
        );
        let denom = Node::Subtract(Box::new(x), Box::new(Node::Num(ExactNum::integer(3))));
        let expr = Node::Divide(Box::new(numer), Box::new(denom));
        let result = compute_limit(&expr, "x", &ExactNum::integer(3)).unwrap();
        assert_eq!(result.to_f64(), 6.0);
    }

    #[test]
    fn test_limit_latex() {
        // lim_{x→1} (x^2-1)/(x-1) = 2
        let result = limit_latex("\\frac{x^2 - 1}{x - 1}", "x", 1.0).unwrap();
        assert_eq!(result, "2");
    }
}
