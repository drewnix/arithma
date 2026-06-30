use std::fmt;

use num_traits::{Signed, ToPrimitive};

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

#[derive(Clone, Debug, PartialEq)]
pub enum LimitDirection {
    Both,
    Left,
    Right,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LimitResult {
    Finite(ExactNum),
    PosInfinity,
    NegInfinity,
}

impl fmt::Display for LimitResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LimitResult::Finite(v) => write!(f, "{}", Node::Num(v.clone())),
            LimitResult::PosInfinity => write!(f, "+\\infty"),
            LimitResult::NegInfinity => write!(f, "-\\infty"),
        }
    }
}

fn contains_var(node: &Node, var: &str) -> bool {
    match node {
        Node::Variable(v) => v == var,
        Node::Num(_) => false,
        Node::Add(l, r)
        | Node::Subtract(l, r)
        | Node::Multiply(l, r)
        | Node::Divide(l, r)
        | Node::Power(l, r) => contains_var(l, var) || contains_var(r, var),
        Node::Negate(inner) | Node::Sqrt(inner) | Node::Abs(inner) => contains_var(inner, var),
        Node::Function(_, args) => args.iter().any(|a| contains_var(a, var)),
        Node::Equation(l, r) => contains_var(l, var) || contains_var(r, var),
        _ => false,
    }
}

const MAX_LHOPITAL_ITERATIONS: usize = 6;

#[derive(Clone, Debug)]
pub enum LimitPoint {
    Finite(ExactNum),
    PosInfinity,
    NegInfinity,
}

/// Compute the limit of expr as var → point.
pub fn compute_limit(expr: &Node, var: &str, point: &ExactNum) -> Result<ExactNum, String> {
    compute_limit_general(expr, var, &LimitPoint::Finite(point.clone()))
}

/// Compute the limit of expr as var → point (finite or ±∞).
pub fn compute_limit_general(
    expr: &Node,
    var: &str,
    point: &LimitPoint,
) -> Result<ExactNum, String> {
    let env = Environment::new();
    let simplified = expr.simplify(&env).unwrap_or_else(|_| expr.clone());
    let result = match point {
        LimitPoint::Finite(p) => limit_internal(&simplified, var, p, 0)?,
        LimitPoint::PosInfinity => limit_at_infinity(&simplified, var, true, 0)?,
        LimitPoint::NegInfinity => limit_at_infinity(&simplified, var, false, 0)?,
    };
    Ok(try_rationalize(&result))
}

/// Compute the limit of expr as var → point with direction control.
/// Returns `LimitResult` which can represent ±∞ as proper values.
pub fn compute_limit_directed(
    expr: &Node,
    var: &str,
    point: &LimitPoint,
    direction: &LimitDirection,
) -> Result<LimitResult, String> {
    let env = Environment::new();
    let simplified = expr.simplify(&env).unwrap_or_else(|_| expr.clone());
    directed_internal(&simplified, var, point, direction)
}

fn directed_internal(
    expr: &Node,
    var: &str,
    point: &LimitPoint,
    direction: &LimitDirection,
) -> Result<LimitResult, String> {
    match direction {
        LimitDirection::Both => directed_both(expr, var, point),
        LimitDirection::Left | LimitDirection::Right => {
            let from_right = matches!(direction, LimitDirection::Right);
            directed_onesided(expr, var, point, from_right)
        }
    }
}

fn directed_both(expr: &Node, var: &str, point: &LimitPoint) -> Result<LimitResult, String> {
    match point {
        LimitPoint::Finite(p) => {
            match limit_internal(expr, var, p, 0) {
                Ok(val) => Ok(LimitResult::Finite(try_rationalize(&val))),
                Err(msg) => {
                    if let Some(result) = parse_infinity_error(&msg) {
                        return Ok(result);
                    }
                    // Try one-sided limits to determine if DNE
                    let left = directed_onesided(expr, var, point, false);
                    let right = directed_onesided(expr, var, point, true);
                    match (left, right) {
                        (Ok(ref l), Ok(ref r)) if l == r => Ok(l.clone()),
                        (Ok(l), Ok(r)) => Err(format!(
                            "Limit does not exist: {} from the left, {} from the right",
                            l, r
                        )),
                        // If one side computed but the other didn't, propagate original error
                        _ => Err(msg),
                    }
                }
            }
        }
        LimitPoint::PosInfinity | LimitPoint::NegInfinity => {
            let positive = matches!(point, LimitPoint::PosInfinity);
            match limit_at_infinity(expr, var, positive, 0) {
                Ok(val) => Ok(LimitResult::Finite(try_rationalize(&val))),
                Err(msg) => parse_infinity_error(&msg).ok_or(msg),
            }
        }
    }
}

fn directed_onesided(
    expr: &Node,
    var: &str,
    point: &LimitPoint,
    from_right: bool,
) -> Result<LimitResult, String> {
    match point {
        LimitPoint::Finite(p) => {
            match limit_internal(expr, var, p, 0) {
                Ok(val) => Ok(LimitResult::Finite(try_rationalize(&val))),
                Err(msg) => {
                    if let Some(result) = parse_infinity_error(&msg) {
                        return Ok(result);
                    }
                    // Probe numerically to determine sign of divergence
                    if let Some(result) = probe_divergence(expr, var, p, from_right) {
                        return Ok(result);
                    }
                    Err(msg)
                }
            }
        }
        LimitPoint::PosInfinity | LimitPoint::NegInfinity => {
            // Direction is inherent for ±∞
            directed_both(expr, var, point)
        }
    }
}

fn parse_infinity_error(msg: &str) -> Option<LimitResult> {
    if msg.contains("+∞") {
        Some(LimitResult::PosInfinity)
    } else if msg.contains("-∞") {
        Some(LimitResult::NegInfinity)
    } else {
        None
    }
}

/// Numerically probe the expression at two points approaching from one side
/// to detect divergence and determine sign. Uses widely-separated probe points
/// so even logarithmic divergence (like ln(x) → 0+) is detected.
fn probe_divergence(
    expr: &Node,
    var: &str,
    point: &ExactNum,
    from_right: bool,
) -> Option<LimitResult> {
    let sign = if from_right { 1i64 } else { -1i64 };
    // Wide spacing: 10^-1 and 10^-8 — catches logarithmic divergence
    let eps1 = ExactNum::rational(sign, 10);
    let eps2 = ExactNum::rational(sign, 100_000_000);

    let shifted1 = point.clone() + eps1;
    let shifted2 = point.clone() + eps2;

    let mut env1 = Environment::new();
    env1.set_exact(var, shifted1);
    let val1 = Evaluator::evaluate_exact(expr, &env1).ok()?;

    let mut env2 = Environment::new();
    env2.set_exact(var, shifted2);
    let val2 = Evaluator::evaluate_exact(expr, &env2).ok()?;

    if val1.is_nan_or_inf() || val2.is_nan_or_inf() {
        return None;
    }

    let f1 = val1.to_f64();
    let f2 = val2.to_f64();

    // Same sign and magnitude growing → divergence
    if f1.signum() == f2.signum() && f2.abs() > f1.abs() * 2.0 {
        if f2 > 0.0 {
            Some(LimitResult::PosInfinity)
        } else {
            Some(LimitResult::NegInfinity)
        }
    } else {
        None
    }
}

fn diverges_at_infinity(expr: &Node, var: &str) -> bool {
    if let Ok(p) = Polynomial::from_node(expr, var) {
        return p.degree().is_some_and(|d| d >= 1);
    }
    if let Node::Function(name, args) = expr {
        if (name == "ln" || name == "log") && args.len() == 1 {
            return contains_var(&args[0], var);
        }
    }
    false
}

fn poly_sign_at_infinity(p: &Polynomial, positive: bool) -> i8 {
    let lc = match p.leading_coeff() {
        Some(c) => c,
        None => return 0,
    };
    let deg = p.degree().unwrap_or(0);
    let lc_sign: i8 = if lc.is_positive() { 1 } else { -1 };
    #[allow(clippy::manual_is_multiple_of)]
    if positive || deg % 2 == 0 {
        lc_sign
    } else {
        -lc_sign
    }
}

fn limit_at_infinity(
    expr: &Node,
    var: &str,
    positive: bool,
    depth: usize,
) -> Result<ExactNum, String> {
    if depth > MAX_LHOPITAL_ITERATIONS {
        return Err("Limit at infinity did not converge".to_string());
    }

    let env = Environment::new();
    let simplified = expr.simplify(&env).unwrap_or_else(|_| expr.clone());

    if !contains_var(&simplified, var) {
        let eval_env = Environment::new();
        if let Ok(val) = Evaluator::evaluate_exact(&simplified, &eval_env) {
            return Ok(val);
        }
    }

    if let Node::Divide(num, den) = &simplified {
        return limit_quotient_at_infinity(num, den, var, positive, depth);
    }

    if let Node::Function(name, args) = &simplified {
        if name == "exp" && args.len() == 1 {
            return limit_exp_at_infinity(&args[0], var, positive, depth);
        }
    }

    // e^f(x) via Power node with base e
    if let Node::Power(base, exp) = &simplified {
        if let Node::Variable(v) = base.as_ref() {
            if v == "e" {
                return limit_exp_at_infinity(exp, var, positive, depth);
            }
        }
    }

    // Polynomial → diverges (nonzero degree) or constant
    if let Ok(p) = Polynomial::from_node(&simplified, var) {
        return match p.degree() {
            Some(0) | None => Ok(ExactNum::Rational(p.coeff(0))),
            _ => Err(format!(
                "Limit is {}∞",
                if poly_sign_at_infinity(&p, positive) > 0 {
                    "+"
                } else {
                    "-"
                }
            )),
        };
    }

    // Product: check for polynomial × decaying exponential
    if let Node::Multiply(a, b) = &simplified {
        if let Ok(result) = limit_product_at_infinity(a, b, var, positive, depth) {
            return Ok(result);
        }
    }

    // Exponential indeterminate forms at infinity (e.g. (1+1/x)^x → e)
    // For 1^∞: use lim f^g = e^{lim g·(f-1)} when f→1
    if let Node::Power(base, exp) = &simplified {
        if contains_var(base, var) && contains_var(exp, var) {
            // Check if base → 1 at infinity
            if let Ok(base_lim) = limit_at_infinity(base, var, positive, depth + 1) {
                if (base_lim.to_f64() - 1.0).abs() < 1e-14 {
                    // 1^∞ form: lim f^g = e^{lim g·(f-1)}
                    let f_minus_1 = Node::Subtract(
                        Box::new(*base.clone()),
                        Box::new(Node::Num(ExactNum::integer(1))),
                    );
                    let g_f1 = Node::Multiply(Box::new(*exp.clone()), Box::new(f_minus_1));
                    let env_s = Environment::new();
                    let g_f1_s = g_f1.simplify(&env_s).unwrap_or(g_f1);
                    if let Ok(val) = limit_at_infinity(&g_f1_s, var, positive, depth + 1) {
                        let f = val.to_f64().exp();
                        return Ok(ExactNum::from_f64(f));
                    }
                }
            }
        }
    }

    // Substitution fallback: x = 1/t (for +∞) or x = -1/t (for -∞), lim t→0
    if let Ok(result) = limit_via_substitution(&simplified, var, positive) {
        return Ok(result);
    }

    Err(format!(
        "Cannot compute limit of {} as {} → {}∞",
        simplified,
        var,
        if positive { "+" } else { "-" }
    ))
}

fn limit_quotient_at_infinity(
    num: &Node,
    den: &Node,
    var: &str,
    positive: bool,
    depth: usize,
) -> Result<ExactNum, String> {
    // Fast path: polynomial degree comparison
    if let (Ok(p), Ok(q)) = (
        Polynomial::from_node(num, var),
        Polynomial::from_node(den, var),
    ) {
        let dp = p.degree();
        let dq = q.degree();
        match (dp, dq) {
            (None, _) | (Some(0), Some(_)) => return Ok(ExactNum::zero()),
            (Some(dp), Some(dq)) if dp < dq => return Ok(ExactNum::zero()),
            (Some(dp), Some(dq)) if dp == dq => {
                let ratio = p.leading_coeff().unwrap() / q.leading_coeff().unwrap();
                return Ok(ExactNum::Rational(ratio));
            }
            (Some(_), Some(_)) => {
                let sign =
                    poly_sign_at_infinity(&p, positive) * poly_sign_at_infinity(&q, positive);
                return Err(format!("Limit is {}∞", if sign > 0 { "+" } else { "-" }));
            }
            _ => {}
        }
    }

    // L'Hopital at infinity: both num and den must diverge
    if diverges_at_infinity(num, var) && diverges_at_infinity(den, var) {
        let env = Environment::new();
        if let (Ok(n_prime), Ok(d_prime)) = (differentiate(num, var), differentiate(den, var)) {
            let n_s = n_prime.simplify(&env).unwrap_or(n_prime);
            let d_s = d_prime.simplify(&env).unwrap_or(d_prime);
            return limit_at_infinity(
                &Node::Divide(Box::new(n_s), Box::new(d_s)),
                var,
                positive,
                depth + 1,
            );
        }
    }

    // Finite numerator / divergent denominator → 0
    if !diverges_at_infinity(num, var) && diverges_at_infinity(den, var) {
        return Ok(ExactNum::zero());
    }

    Err("Cannot compute limit of quotient at infinity".to_string())
}

fn limit_exp_at_infinity(
    exponent: &Node,
    var: &str,
    positive: bool,
    depth: usize,
) -> Result<ExactNum, String> {
    if let Ok(p) = Polynomial::from_node(exponent, var) {
        if p.degree().is_none_or(|d| d < 1) {
            let val = p.coeff(0).to_f64().unwrap_or(0.0);
            return Ok(ExactNum::from_f64(val.exp()));
        }
        let sign = poly_sign_at_infinity(&p, positive);
        if sign < 0 {
            return Ok(ExactNum::zero());
        }
        return Err("Limit is +∞ (exponential growth)".to_string());
    }

    match limit_at_infinity(exponent, var, positive, depth + 1) {
        Ok(val) => {
            let f = val.to_f64().exp();
            Ok(ExactNum::from_f64(f))
        }
        Err(msg) if msg.contains("-∞") => Ok(ExactNum::zero()),
        Err(msg) if msg.contains("+∞") => Err("Limit is +∞ (exponential growth)".to_string()),
        Err(e) => Err(e),
    }
}

fn limit_product_at_infinity(
    a: &Node,
    b: &Node,
    var: &str,
    positive: bool,
    depth: usize,
) -> Result<ExactNum, String> {
    // Check for patterns like x^n · e^{-x} → 0
    let (poly_part, exp_part) = if is_decaying_exp(a, var, positive) {
        (b, a)
    } else if is_decaying_exp(b, var, positive) {
        (a, b)
    } else {
        return Err("Not a polynomial × decaying exponential".to_string());
    };

    if Polynomial::from_node(poly_part, var).is_ok() {
        return Ok(ExactNum::zero());
    }

    // Try rewriting as quotient: f·g = f / (1/g)
    let recip = Node::Divide(
        Box::new(Node::Num(ExactNum::integer(1))),
        Box::new(exp_part.clone()),
    );
    let env = Environment::new();
    let recip_s = recip.simplify(&env).unwrap_or(recip);
    limit_quotient_at_infinity(poly_part, &recip_s, var, positive, depth)
}

fn is_decaying_exp(expr: &Node, var: &str, positive: bool) -> bool {
    if let Node::Function(name, args) = expr {
        if name == "exp" && args.len() == 1 {
            if let Ok(p) = Polynomial::from_node(&args[0], var) {
                if p.degree().is_some_and(|d| d >= 1) {
                    return poly_sign_at_infinity(&p, positive) < 0;
                }
            }
        }
    }
    if let Node::Power(base, exp) = expr {
        if let Node::Variable(v) = base.as_ref() {
            if v == "e" {
                if let Ok(p) = Polynomial::from_node(exp, var) {
                    if p.degree().is_some_and(|d| d >= 1) {
                        return poly_sign_at_infinity(&p, positive) < 0;
                    }
                }
            }
        }
    }
    false
}

fn evaluates_to_zero(expr: &Node, var: &str, point: &ExactNum) -> bool {
    let mut env = Environment::new();
    env.set_exact(var, point.clone());
    Evaluator::evaluate_exact(expr, &env)
        .map(|v| v.is_zero())
        .unwrap_or(false)
}

fn evaluates_to_inf(expr: &Node, var: &str, point: &ExactNum) -> bool {
    let mut env = Environment::new();
    env.set_exact(var, point.clone());
    Evaluator::evaluate_exact(expr, &env)
        .map(|v| v.is_nan_or_inf())
        .unwrap_or(true)
}

fn try_rewrite_product(
    expr: &Node,
    var: &str,
    point: &ExactNum,
    depth: usize,
) -> Option<Result<ExactNum, String>> {
    let (factor_a, factor_b) = match expr {
        Node::Multiply(a, b) => (a.as_ref(), b.as_ref()),
        _ => return None,
    };

    let a_zero = evaluates_to_zero(factor_a, var, point);
    let b_zero = evaluates_to_zero(factor_b, var, point);
    let a_inf = evaluates_to_inf(factor_a, var, point);
    let b_inf = evaluates_to_inf(factor_b, var, point);

    if a_zero && b_inf {
        // 0·∞: try expanding f·g as a quotient where g has a known reciprocal
        // For cot(x) = cos(x)/sin(x), rewrite x·cot(x) as x·cos(x)/sin(x)
        if let Some((num, den)) = extract_quotient(factor_b) {
            let new_num = Node::Multiply(Box::new(factor_a.clone()), Box::new(num));
            let env = Environment::new();
            let num_s = new_num.simplify(&env).unwrap_or(new_num);
            return Some(limit_quotient(&num_s, &den, var, point, depth));
        }
        // Fallback: rewrite as a / (1/b)
        let recip = Node::Divide(
            Box::new(Node::Num(ExactNum::integer(1))),
            Box::new(factor_b.clone()),
        );
        let env = Environment::new();
        let recip_s = recip.simplify(&env).unwrap_or(recip);
        return Some(limit_quotient(factor_a, &recip_s, var, point, depth));
    }

    if b_zero && a_inf {
        if let Some((num, den)) = extract_quotient(factor_a) {
            let new_num = Node::Multiply(Box::new(factor_b.clone()), Box::new(num));
            let env = Environment::new();
            let num_s = new_num.simplify(&env).unwrap_or(new_num);
            return Some(limit_quotient(&num_s, &den, var, point, depth));
        }
        let recip = Node::Divide(
            Box::new(Node::Num(ExactNum::integer(1))),
            Box::new(factor_a.clone()),
        );
        let env = Environment::new();
        let recip_s = recip.simplify(&env).unwrap_or(recip);
        return Some(limit_quotient(factor_b, &recip_s, var, point, depth));
    }

    None
}

fn extract_quotient(expr: &Node) -> Option<(Node, Node)> {
    if let Node::Divide(n, d) = expr {
        return Some((*n.clone(), *d.clone()));
    }
    match expr {
        Node::Function(name, args) if args.len() == 1 => {
            let arg = &args[0];
            match name.as_str() {
                "cot" => Some((
                    Node::Function("cos".to_string(), vec![arg.clone()]),
                    Node::Function("sin".to_string(), vec![arg.clone()]),
                )),
                "tan" => Some((
                    Node::Function("sin".to_string(), vec![arg.clone()]),
                    Node::Function("cos".to_string(), vec![arg.clone()]),
                )),
                "csc" => Some((
                    Node::Num(ExactNum::integer(1)),
                    Node::Function("sin".to_string(), vec![arg.clone()]),
                )),
                "sec" => Some((
                    Node::Num(ExactNum::integer(1)),
                    Node::Function("cos".to_string(), vec![arg.clone()]),
                )),
                _ => None,
            }
        }
        _ => None,
    }
}

thread_local! {
    static IN_SUBSTITUTION: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn limit_via_substitution(expr: &Node, var: &str, positive: bool) -> Result<ExactNum, String> {
    if IN_SUBSTITUTION.with(|f| f.get()) {
        return Err("Already inside substitution".to_string());
    }

    let t_var = "_limit_t";
    let replacement = if positive {
        Node::Divide(
            Box::new(Node::Num(ExactNum::integer(1))),
            Box::new(Node::Variable(t_var.to_string())),
        )
    } else {
        Node::Divide(
            Box::new(Node::Num(ExactNum::integer(-1))),
            Box::new(Node::Variable(t_var.to_string())),
        )
    };

    let substituted = match crate::substitute::substitute_variable(expr, var, &replacement) {
        Ok(s) => s,
        Err(_) => return Err("Substitution failed".to_string()),
    };
    let env = Environment::new();
    let simplified = substituted.simplify(&env).unwrap_or(substituted);

    IN_SUBSTITUTION.with(|f| f.set(true));
    let result = limit_internal(&simplified, t_var, &ExactNum::zero(), 0);
    IN_SUBSTITUTION.with(|f| f.set(false));
    result
}

fn try_exp_indeterminate(
    expr: &Node,
    var: &str,
    point: &LimitPoint,
    depth: usize,
) -> Option<Result<ExactNum, String>> {
    let (base, exponent) = match expr {
        Node::Power(b, e) => (b.as_ref(), e.as_ref()),
        _ => return None,
    };

    if depth > MAX_LHOPITAL_ITERATIONS {
        return None;
    }

    // Rewrite f^g as exp(g·ln(f))
    let ln_base = Node::Function("ln".to_string(), vec![base.clone()]);
    let g_ln_f = Node::Multiply(Box::new(exponent.clone()), Box::new(ln_base));

    let env = Environment::new();
    let g_ln_f_s = g_ln_f.simplify(&env).unwrap_or(g_ln_f);

    let exponent_limit = match point {
        LimitPoint::Finite(p) => limit_internal(&g_ln_f_s, var, p, depth + 1),
        LimitPoint::PosInfinity | LimitPoint::NegInfinity => {
            let positive = matches!(point, LimitPoint::PosInfinity);
            limit_via_substitution(&g_ln_f_s, var, positive)
        }
    };

    match exponent_limit {
        Ok(val) => {
            let f = val.to_f64().exp();
            Some(Ok(ExactNum::from_f64(f)))
        }
        Err(msg) if msg.contains("-∞") => Some(Ok(ExactNum::zero())),
        Err(msg) if msg.contains("+∞") => Some(Err("Limit is +∞".to_string())),
        Err(_) => None,
    }
}

fn limit_via_series(expr: &Node, var: &str, point: &ExactNum) -> Result<ExactNum, String> {
    // Expand the expression as a Taylor series around the point
    // and extract the constant term (the value at the limit point)
    for order in [4, 8, 12] {
        if let Ok(taylor) = crate::series::taylor_series(expr, var, point, order) {
            let mut eval_env = Environment::new();
            eval_env.set_exact(var, point.clone());
            if let Ok(val) = Evaluator::evaluate_exact(&taylor, &eval_env) {
                if !val.is_nan_or_inf() {
                    return Ok(val);
                }
            }
        }
    }
    Err("Series expansion did not resolve limit".to_string())
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

    // Step 3: rewrite 0·∞ products as quotients
    if let Some(result) = try_rewrite_product(expr, var, point, depth) {
        return result;
    }

    // Step 3b: exponential indeterminate forms (0^0, 1^∞, ∞^0)
    if let Some(result) =
        try_exp_indeterminate(expr, var, &LimitPoint::Finite(point.clone()), depth)
    {
        return result;
    }

    // Step 4: try simplifying harder and retrying
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

    // Step 5: series expansion — expand as Taylor and extract constant term
    if let Ok(result) = limit_via_series(expr, var, point) {
        return Ok(result);
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

    // Strategy 2: series expansion — expand both as Taylor, find leading terms
    if let Ok(result) = limit_quotient_via_series(numer, denom, var, point) {
        return Ok(result);
    }

    // Strategy 3: L'Hôpital's rule
    let env = Environment::new();
    let n_prime = differentiate(numer, var).and_then(|d| d.simplify(&env).or(Ok(d)))?;
    let d_prime = differentiate(denom, var).and_then(|d| d.simplify(&env).or(Ok(d)))?;

    let new_expr = Node::Divide(Box::new(n_prime), Box::new(d_prime));
    limit_internal(&new_expr, var, point, depth + 1)
}

fn limit_quotient_via_series(
    numer: &Node,
    denom: &Node,
    var: &str,
    point: &ExactNum,
) -> Result<ExactNum, String> {
    for order in [6, 10] {
        let n_taylor = crate::series::taylor_series(numer, var, point, order)
            .map_err(|e| format!("Taylor failed: {}", e))?;
        let d_taylor = crate::series::taylor_series(denom, var, point, order)
            .map_err(|e| format!("Taylor failed: {}", e))?;

        if let Some(result) = try_polynomial_cancel(&n_taylor, &d_taylor, var, point) {
            return result;
        }
    }
    Err("Series expansion did not resolve quotient limit".to_string())
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

/// Parse a limit point string into a LimitPoint and direction.
/// Accepts directional suffixes: "0+", "0-", "3+", "3-", "-1+", "-1-".
/// A trailing '+' or '-' after a digit indicates direction (right or left).
/// Plain "0", "3", "inf" etc. mean both sides.
pub fn parse_limit_point(s: &str) -> Result<(LimitPoint, LimitDirection), String> {
    let trimmed = s.trim();

    // Check for directional suffix: trailing '+' or '-' after a digit
    if trimmed.len() >= 2 {
        let bytes = trimmed.as_bytes();
        let last = bytes[bytes.len() - 1];
        let prev = bytes[bytes.len() - 2];
        if (last == b'+' || last == b'-') && prev.is_ascii_digit() {
            let direction = if last == b'+' {
                LimitDirection::Right
            } else {
                LimitDirection::Left
            };
            let point = parse_limit_point_value(&trimmed[..trimmed.len() - 1])?;
            return Ok((point, direction));
        }
    }

    let point = parse_limit_point_value(trimmed)?;
    Ok((point, LimitDirection::Both))
}

fn parse_limit_point_value(s: &str) -> Result<LimitPoint, String> {
    match s {
        "inf" | "\\infty" | "+inf" | "+\\infty" | "∞" | "+∞" => Ok(LimitPoint::PosInfinity),
        "-inf" | "-\\infty" | "-∞" => Ok(LimitPoint::NegInfinity),
        _ => {
            let f: f64 = s
                .parse()
                .map_err(|_| format!("Cannot parse limit point: {}", s))?;
            let exact = if f == 0.0 {
                ExactNum::zero()
            } else if f == f.floor() && f.abs() < 1e15 {
                ExactNum::integer(f as i64)
            } else {
                ExactNum::from_f64(f)
            };
            Ok(LimitPoint::Finite(exact))
        }
    }
}

/// Compute limit from LaTeX expression (f64 point — backward compatible).
pub fn limit_latex(expr_latex: &str, var: &str, point: f64) -> Result<String, String> {
    let point_exact = if point == 0.0 {
        ExactNum::zero()
    } else if point == point.floor() && point.abs() < 1e15 {
        ExactNum::integer(point as i64)
    } else {
        ExactNum::from_f64(point)
    };
    limit_latex_directed(
        expr_latex,
        var,
        &LimitPoint::Finite(point_exact),
        &LimitDirection::Both,
    )
}

/// Compute limit from LaTeX expression with string point (supports ∞ and direction).
/// Point accepts: "0", "0+", "0-", "3+", "3-", "inf", "-inf", etc.
pub fn limit_latex_str(expr_latex: &str, var: &str, point_str: &str) -> Result<String, String> {
    let (point, direction) = parse_limit_point(point_str)?;
    limit_latex_directed(expr_latex, var, &point, &direction)
}

fn limit_latex_directed(
    expr_latex: &str,
    var: &str,
    point: &LimitPoint,
    direction: &LimitDirection,
) -> Result<String, String> {
    let mut tokenizer = Tokenizer::new(expr_latex);
    let tokens = tokenizer.tokenize();
    let expr = build_expression_tree(tokens)?;
    let result = compute_limit_directed(&expr, var, point, direction)?;
    Ok(format!("{}", result))
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

    // === Series expansion limits ===

    #[test]
    fn test_limit_x_cot_x() {
        // lim_{x→0} x·cot(x) = 1 (0·∞ form)
        let result = limit_latex_str("x \\cdot \\cot(x)", "x", "0").unwrap();
        assert!((result.parse::<f64>().unwrap() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_limit_sinx_minus_x_over_x3() {
        // lim_{x→0} (sin(x)-x)/x³ = -1/6 (higher-order 0/0)
        let result = limit_latex_str("\\frac{\\sin(x) - x}{x^3}", "x", "0").unwrap();
        assert_eq!(result, "-\\frac{1}{6}");
    }

    #[test]
    fn test_limit_exp_higher_order() {
        // lim_{x→0} (e^x - 1 - x - x²/2)/x³ = 1/6
        let result =
            limit_latex_str("\\frac{\\exp(x) - 1 - x - \\frac{x^2}{2}}{x^3}", "x", "0").unwrap();
        assert_eq!(result, "\\frac{1}{6}");
    }

    // === Exponential indeterminate forms ===

    #[test]
    fn test_limit_1_plus_1_over_x_to_x() {
        // lim_{x→∞} (1+1/x)^x = e (1^∞ form)
        let result = limit_latex_str("(1 + \\frac{1}{x})^x", "x", "inf").unwrap();
        let val: f64 = result.parse().unwrap_or_else(|_| {
            Evaluator::evaluate_exact(
                &build_expression_tree(Tokenizer::new(&result).tokenize()).unwrap(),
                &Environment::new(),
            )
            .unwrap()
            .to_f64()
        });
        assert!(
            (val - std::f64::consts::E).abs() < 1e-6,
            "expected e, got {}",
            val
        );
    }

    #[test]
    fn test_limit_x_to_x_at_zero() {
        // lim_{x→0+} x^x = 1 (0^0 form)
        let x = Node::Variable("x".to_string());
        let expr = Node::Power(Box::new(x.clone()), Box::new(x));
        let result = compute_limit(&expr, "x", &ExactNum::zero()).unwrap();
        assert!(
            (result.to_f64() - 1.0).abs() < 1e-10,
            "expected 1, got {}",
            result.to_f64()
        );
    }

    #[test]
    fn test_limit_x_to_sinx_at_zero() {
        // lim_{x→0+} x^{sin(x)} = 1 (0^0 form)
        let x = Node::Variable("x".to_string());
        let expr = Node::Power(
            Box::new(x.clone()),
            Box::new(Node::Function("sin".to_string(), vec![x])),
        );
        let result = compute_limit(&expr, "x", &ExactNum::zero()).unwrap();
        assert!(
            (result.to_f64() - 1.0).abs() < 1e-10,
            "expected 1, got {}",
            result.to_f64()
        );
    }

    // === Limits at infinity ===

    #[test]
    fn test_limit_rational_same_degree_at_infinity() {
        // lim_{x→∞} (3x²+x)/(2x²+1) = 3/2
        let result = limit_latex_str("\\frac{3x^2 + x}{2x^2 + 1}", "x", "inf").unwrap();
        assert_eq!(result, "\\frac{3}{2}");
    }

    #[test]
    fn test_limit_rational_lower_degree_at_infinity() {
        // lim_{x→∞} x/(x²+1) = 0
        let result = limit_latex_str("\\frac{x}{x^2 + 1}", "x", "inf").unwrap();
        assert_eq!(result, "0");
    }

    #[test]
    fn test_limit_exp_decay_at_infinity() {
        // lim_{x→∞} e^{-x} = 0
        let result = compute_limit_general(
            &Node::Function(
                "exp".to_string(),
                vec![Node::Negate(Box::new(Node::Variable("x".to_string())))],
            ),
            "x",
            &LimitPoint::PosInfinity,
        )
        .unwrap();
        assert_eq!(result.to_f64(), 0.0);
    }

    #[test]
    fn test_limit_ln_over_x_at_infinity() {
        // lim_{x→∞} ln(x)/x = 0
        let result = limit_latex_str("\\frac{\\ln(x)}{x}", "x", "inf").unwrap();
        assert_eq!(result, "0");
    }

    #[test]
    fn test_limit_constant_at_infinity() {
        // lim_{x→∞} 5 = 5
        let result = limit_latex_str("5", "x", "inf").unwrap();
        assert_eq!(result, "5");
    }

    #[test]
    fn test_limit_1_over_x_at_infinity() {
        // lim_{x→∞} 1/x = 0
        let result = limit_latex_str("\\frac{1}{x}", "x", "inf").unwrap();
        assert_eq!(result, "0");
    }

    #[test]
    fn test_limit_parse_infty_variants() {
        let (p1, d1) = parse_limit_point("inf").unwrap();
        let (p2, d2) = parse_limit_point("\\infty").unwrap();
        let (p3, d3) = parse_limit_point("-inf").unwrap();
        let (p4, d4) = parse_limit_point("3").unwrap();
        assert!(matches!(p1, LimitPoint::PosInfinity));
        assert!(matches!(p2, LimitPoint::PosInfinity));
        assert!(matches!(p3, LimitPoint::NegInfinity));
        assert!(matches!(p4, LimitPoint::Finite(_)));
        assert_eq!(d1, LimitDirection::Both);
        assert_eq!(d2, LimitDirection::Both);
        assert_eq!(d3, LimitDirection::Both);
        assert_eq!(d4, LimitDirection::Both);
    }

    #[test]
    fn test_limit_parse_direction() {
        let (p, d) = parse_limit_point("0+").unwrap();
        assert!(matches!(p, LimitPoint::Finite(ref v) if v.is_zero()));
        assert_eq!(d, LimitDirection::Right);

        let (p, d) = parse_limit_point("0-").unwrap();
        assert!(matches!(p, LimitPoint::Finite(ref v) if v.is_zero()));
        assert_eq!(d, LimitDirection::Left);

        let (p, d) = parse_limit_point("3+").unwrap();
        assert!(matches!(p, LimitPoint::Finite(ref v) if v.to_f64() == 3.0));
        assert_eq!(d, LimitDirection::Right);

        let (p, d) = parse_limit_point("-1-").unwrap();
        assert!(matches!(p, LimitPoint::Finite(ref v) if v.to_f64() == -1.0));
        assert_eq!(d, LimitDirection::Left);

        // "-3" is not directional — it's the number -3
        let (p, d) = parse_limit_point("-3").unwrap();
        assert!(matches!(p, LimitPoint::Finite(ref v) if v.to_f64() == -3.0));
        assert_eq!(d, LimitDirection::Both);
    }

    // === One-sided limits ===

    #[test]
    fn test_limit_1_over_x_right() {
        // lim_{x→0+} 1/x = +∞
        let result = limit_latex_str("\\frac{1}{x}", "x", "0+").unwrap();
        assert_eq!(result, "+\\infty");
    }

    #[test]
    fn test_limit_1_over_x_left() {
        // lim_{x→0-} 1/x = -∞
        let result = limit_latex_str("\\frac{1}{x}", "x", "0-").unwrap();
        assert_eq!(result, "-\\infty");
    }

    #[test]
    fn test_limit_ln_x_right() {
        // lim_{x→0+} ln(x) = -∞
        let result = limit_latex_str("\\ln(x)", "x", "0+").unwrap();
        assert_eq!(result, "-\\infty");
    }

    #[test]
    fn test_limit_1_over_x_both_dne() {
        // lim_{x→0} 1/x does not exist (diverges differently from each side)
        let result = limit_latex_str("\\frac{1}{x}", "x", "0");
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("does not exist"),
            "Expected 'does not exist', got: {}",
            msg
        );
    }

    #[test]
    fn test_limit_1_over_x2_both_posinf() {
        // lim_{x→0} 1/x² = +∞ (same from both sides)
        let result = limit_latex_str("\\frac{1}{x^2}", "x", "0").unwrap();
        assert_eq!(result, "+\\infty");
    }

    #[test]
    fn test_limit_1_over_x2_right() {
        // lim_{x→0+} 1/x² = +∞
        let result = limit_latex_str("\\frac{1}{x^2}", "x", "0+").unwrap();
        assert_eq!(result, "+\\infty");
    }
}
