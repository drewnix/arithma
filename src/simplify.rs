use crate::environment::Environment;
use crate::exact::ExactNum;
use crate::function_meta::is_transcendental_function;
use crate::integer::{extract_square_factors, prime_factorize};
use crate::multipoly::MultiPoly;
use crate::node::Node;
use crate::polynomial::Polynomial;
use crate::simplify_literal::try_normalize_pi_multiple;
use num_bigint::BigInt;
use num_integer::Integer;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};
use std::collections::HashMap;

/// Attempt to convert an f64 to an exact rational p/q with small denominator.
/// Checks denominators 1..100; returns the first match within 1e-10 tolerance.
fn try_rationalize(f: f64) -> Option<BigRational> {
    if !f.is_finite() {
        return None;
    }
    if f.fract() == 0.0 && f.abs() < i64::MAX as f64 {
        return Some(BigRational::from_integer(BigInt::from(f as i64)));
    }
    for denom in 2..=100i64 {
        let numer_f = f * denom as f64;
        let numer_rounded = numer_f.round();
        if (numer_f - numer_rounded).abs() < 1e-10 && numer_rounded.abs() < i64::MAX as f64 {
            return Some(BigRational::new(
                BigInt::from(numer_rounded as i64),
                BigInt::from(denom),
            ));
        }
    }
    None
}

/// True when `exp` is a non-zero even integer (e.g. 2, -2, 4).
fn is_even_integer_exponent(exp: &Node) -> bool {
    match exp {
        Node::Num(n) => n.to_i64().is_some_and(|e| e != 0 && e % 2 == 0),
        Node::Negate(inner) => {
            if let Node::Num(n) = inner.as_ref() {
                n.to_i64().is_some_and(|e| e != 0 && e % 2 == 0)
            } else {
                false
            }
        }
        _ => false,
    }
}

fn is_literal_positive(node: &Node) -> bool {
    matches!(
        node,
        Node::Num(n) if n.to_rational().is_some_and(|r| r.is_positive())
    )
}

fn is_known_positive(node: &Node, env: &Environment) -> bool {
    match node {
        Node::Num(_) => is_literal_positive(node),
        Node::Variable(v) => env.assumptions().is_positive(v),
        _ => false,
    }
}

fn try_fold_factorial_num(n: &ExactNum) -> Option<Node> {
    let v = n.to_i64()?;
    Some(Node::Num(crate::integer::factorial_exact(
        (v >= 0).then_some(v as usize)?,
    )))
}

fn try_fold_binom(args: &[Node]) -> Option<Node> {
    if args.len() != 2 {
        return None;
    }
    let (Node::Num(n), Node::Num(k)) = (&args[0], &args[1]) else {
        return None;
    };
    let n = n.to_i64()?;
    let k = k.to_i64()?;
    Some(Node::Num(crate::integer::binom_exact(
        (n >= 0).then_some(n as usize)?,
        (k >= 0).then_some(k as usize)?,
    )))
}

/// Argument for `ln` inside `ln(a^b)`: use `|a|` when `b` is an even integer so
/// `ln(x^2)` and `ln((-x)^2)` both become `2·ln|x|` on the real line.
fn ln_power_log_arg(base: &Node, exp: &Node, env: &Environment) -> Node {
    if is_even_integer_exponent(exp) {
        if let Node::Variable(v) = base {
            if env.assumptions().is_nonneg(v) {
                return base.clone();
            }
        }
        if is_literal_positive(base) {
            return base.clone();
        }
        return Node::Abs(Box::new(base.clone()));
    }
    base.clone()
}

fn try_exact_function_value(name: &str, args: &[Node]) -> Option<Node> {
    if name == "binom" {
        return try_fold_binom(args);
    }

    if args.len() == 1 {
        let arg = &args[0];
        match name {
            "abs" => {
                if let Node::Num(n) = arg {
                    return Some(Node::Num(n.abs()));
                }
            }
            "floor" => {
                if let Node::Num(n) = arg {
                    return Some(Node::Num(n.floor()));
                }
            }
            "ceil" => {
                if let Node::Num(n) = arg {
                    return Some(Node::Num(n.ceil()));
                }
            }
            "round" => {
                if let Node::Num(n) = arg {
                    return Some(Node::Num(n.round()));
                }
            }
            "trunc" => {
                if let Node::Num(n) = arg {
                    return Some(Node::Num(n.trunc()));
                }
            }
            _ => {}
        }
    }

    crate::simplify_literal::try_exact_function_value(name, args)
}

fn as_positive_integer_big(node: &Node) -> Option<BigInt> {
    let r = match node {
        Node::Num(n) => n.to_rational()?,
        _ => return None,
    };
    if !r.is_integer() || r <= BigRational::one() {
        return None;
    }
    Some(r.numer().clone())
}

/// Rewrite `log(n)` for positive integer `n` as a sum of `e·log(p)` terms using prime
/// factorization, keeping the result symbolic instead of evaluating to a float.
fn factor_log_integer(log_name: &str, arg: &Node) -> Option<Node> {
    let bigint = as_positive_integer_big(arg)?;
    // TODO: factor integers larger than u64::MAX (BigInt prime factorization); until then
    // `log(10^k)` / `lg(2^k)` for huge `k` still work via simplify_literal.
    let n = bigint.to_u64()?;
    if n <= 1 {
        return None;
    }

    let factors = prime_factorize(n);
    let is_non_trivial = factors.len() > 1 || factors.iter().any(|&(_, e)| e > 1);
    if factors.is_empty() || !is_non_trivial {
        return None;
    }

    let terms: Vec<Node> = factors
        .iter()
        .map(|&(prime, exponent)| {
            let log_prime = Node::Function(
                log_name.to_string(),
                vec![Node::Num(ExactNum::integer(prime as i64))],
            );
            if exponent == 1 {
                log_prime
            } else {
                Node::Multiply(
                    Box::new(Node::Num(ExactNum::integer(exponent as i64))),
                    Box::new(log_prime),
                )
            }
        })
        .collect();

    let mut iter = terms.into_iter();
    let mut result = iter.next()?;
    for term in iter {
        result = Node::Add(Box::new(result), Box::new(term));
    }
    Some(result)
}

fn is_named_log_base(log_name: &str, base: &Node) -> bool {
    match log_name {
        "ln" => match base {
            Node::Variable(v) => v == "e",
            Node::Num(b) => (b.to_f64() - std::f64::consts::E).abs() < 1e-14,
            _ => false,
        },
        "log" => matches!(
            base,
            Node::Num(n) if n.to_rational() == Some(BigRational::from_integer(BigInt::from(10)))
        ),
        "lg" => matches!(
            base,
            Node::Num(n) if n.to_rational() == Some(BigRational::from_integer(BigInt::from(2)))
        ),
        _ => false,
    }
}

/// Algebraic simplify rules shared by `ln`, `log`, and `lg`.
fn simplify_log_function(log_name: &str, arg: &Node, env: &Environment) -> Option<Node> {
    if let Node::Power(base, exp) = arg {
        if is_named_log_base(log_name, base) {
            return Some(*exp.clone());
        }
        let log_arg = ln_power_log_arg(base, exp, env);
        let inner = Node::Function(log_name.to_string(), vec![log_arg])
            .simplify(env)
            .ok()?;
        return Node::Multiply(exp.clone(), Box::new(inner))
            .simplify(env)
            .ok();
    }
    if log_name == "ln" {
        if let Node::Function(inner_name, inner_args) = arg {
            if inner_name == "exp" && inner_args.len() == 1 {
                return Some(inner_args[0].clone());
            }
        }
    }
    if let Node::Multiply(a, b) = arg {
        if !is_known_positive(a, env) || !is_known_positive(b, env) {
            return None;
        }
        let log_a = Node::Function(log_name.to_string(), vec![*a.clone()])
            .simplify(env)
            .ok()?;
        let log_b = Node::Function(log_name.to_string(), vec![*b.clone()])
            .simplify(env)
            .ok()?;
        return Node::Add(Box::new(log_a), Box::new(log_b))
            .simplify(env)
            .ok();
    }
    if let Node::Divide(a, b) = arg {
        if !is_known_positive(a, env) || !is_known_positive(b, env) {
            return None;
        }
        let log_a = Node::Function(log_name.to_string(), vec![*a.clone()])
            .simplify(env)
            .ok()?;
        let log_b = Node::Function(log_name.to_string(), vec![*b.clone()])
            .simplify(env)
            .ok()?;
        return Node::Subtract(Box::new(log_a), Box::new(log_b))
            .simplify(env)
            .ok();
    }
    factor_log_integer(log_name, arg)?.simplify(env).ok()
}

fn is_zero_node(node: &Node) -> bool {
    match node {
        Node::Num(n) => n.is_zero(),
        Node::Negate(inner) => is_zero_node(inner),
        _ => false,
    }
}

fn extract_func_factor(node: &Node) -> Option<(Node, Node)> {
    match node {
        Node::Function(_, _) => Some((Node::Num(ExactNum::integer(1)), node.clone())),
        Node::Multiply(a, b) => {
            if matches!(b.as_ref(), Node::Function(_, _)) {
                Some((*a.clone(), *b.clone()))
            } else if matches!(a.as_ref(), Node::Function(_, _)) {
                Some((*b.clone(), *a.clone()))
            } else {
                None
            }
        }
        Node::Negate(inner) => {
            let (coeff, func) = extract_func_factor(inner)?;
            Some((Node::Negate(Box::new(coeff)), func))
        }
        Node::Divide(numer, denom) => {
            if matches!(numer.as_ref(), Node::Function(_, _)) {
                Some((
                    Node::Divide(Box::new(Node::Num(ExactNum::integer(1))), denom.clone()),
                    *numer.clone(),
                ))
            } else {
                let (coeff, func) = extract_func_factor(numer)?;
                Some((Node::Divide(Box::new(coeff), denom.clone()), func))
            }
        }
        _ => None,
    }
}

fn try_combine_function_terms(
    left: &Node,
    right: &Node,
    is_sub: bool,
    env: &Environment,
) -> Option<Node> {
    let (lc, lf) = extract_func_factor(left)?;
    let (rc, rf) = extract_func_factor(right)?;

    if format!("{}", lf) != format!("{}", rf) {
        return None;
    }

    let combined_coeff = if is_sub {
        Node::Subtract(Box::new(lc), Box::new(rc))
    } else {
        Node::Add(Box::new(lc), Box::new(rc))
    };
    let simplified_coeff = combined_coeff.simplify(env).ok()?;

    if is_zero_node(&simplified_coeff) {
        return Some(Node::Num(ExactNum::integer(0)));
    }
    if matches!(&simplified_coeff, Node::Num(n) if n.is_one()) {
        return Some(lf);
    }
    Node::Multiply(Box::new(simplified_coeff), Box::new(lf))
        .simplify(env)
        .ok()
}

pub trait Simplifiable {
    fn simplify(&self, env: &Environment) -> Result<Node, String>;
}

impl Simplifiable for Node {
    fn simplify(&self, env: &Environment) -> Result<Node, String> {
        match self {
            Node::Add(left, right) => {
                let left_simplified = left.simplify(env)?;
                let right_simplified = right.simplify(env)?;

                if let (Node::Num(ref l), Node::Num(ref r)) = (&left_simplified, &right_simplified)
                {
                    return Ok(Node::Num(l + r));
                }

                if let Node::Num(ref n) = left_simplified {
                    if n.is_zero() {
                        return Ok(right_simplified);
                    }
                }
                if let Node::Num(ref n) = right_simplified {
                    if n.is_zero() {
                        return Ok(left_simplified);
                    }
                }

                // sin²(x) + cos²(x) → 1
                if let Some(result) = try_pythagorean(&left_simplified, &right_simplified) {
                    return Ok(result);
                }

                // a·f(x) + b·f(x) → (a+b)·f(x) (before fraction combination)
                if let Some(combined) =
                    try_combine_function_terms(&left_simplified, &right_simplified, false, env)
                {
                    return Ok(combined);
                }

                // a/d + b/d → (a+b)/d
                if let Some(combined) =
                    try_combine_fractions(&left_simplified, &right_simplified, false, env)
                {
                    return Ok(combined);
                }

                // a√X + b√X → (a+b)√X (and like terms in flat n-ary sums)
                let result = Node::Add(Box::new(left_simplified), Box::new(right_simplified));
                if let Some(combined) = try_combine_like_radicals(&result, env) {
                    return Ok(combined);
                }
                let mut term_map: HashMap<String, ExactNum> = HashMap::new();
                if collect_terms(&result, &mut term_map, env).is_ok() {
                    Ok(rebuild_expression(term_map))
                } else if let Some(normalized) = try_polynomial_normalize(&result) {
                    Ok(normalized)
                } else if let Some(normalized) = try_rational_normalize(&result, env) {
                    Ok(normalized)
                } else {
                    Ok(result)
                }
            }
            Node::Num(n) => {
                if let ExactNum::Float(f) = n {
                    if let Some(r) = try_rationalize(*f) {
                        return Ok(Node::Num(ExactNum::Rational(r)));
                    }
                }
                Ok(self.clone())
            }
            Node::Multiply(left, right) => {
                let left_simplified = left.simplify(env)?;
                let right_simplified = right.simplify(env)?;

                // Handle multiplication by zero
                if let Node::Num(ref n) = left_simplified {
                    if n.is_zero() {
                        return Ok(Node::Num(ExactNum::zero()));
                    }
                }
                if let Node::Num(ref n) = right_simplified {
                    if n.is_zero() {
                        return Ok(Node::Num(ExactNum::zero()));
                    }
                }

                // Multiplying by one
                if let Node::Num(ref n) = left_simplified {
                    if n.is_one() {
                        return Ok(right_simplified);
                    }
                }
                if let Node::Num(ref n) = right_simplified {
                    if n.is_one() {
                        return Ok(left_simplified);
                    }
                }

                // If both are numbers, multiply them directly
                if let (Node::Num(ref l), Node::Num(ref r)) = (&left_simplified, &right_simplified)
                {
                    return Ok(Node::Num(l * r));
                }

                // k * (-f) → (-k) * f — absorb negation into coefficient
                if let Node::Num(ref k) = left_simplified {
                    if let Node::Negate(inner) = right_simplified {
                        return Node::Multiply(Box::new(Node::Num(-k.clone())), inner)
                            .simplify(env);
                    }
                }
                // (-f) * k → (-k) * f
                if let Node::Negate(inner) = &left_simplified {
                    if let Node::Num(ref k) = right_simplified {
                        return Node::Multiply(Box::new(Node::Num(-k.clone())), inner.clone())
                            .simplify(env);
                    }
                }
                // f * (-g) → -(f * g), (-f) * g → -(f * g) — general negation extraction
                if let Node::Negate(inner) = &right_simplified {
                    return Node::Negate(Box::new(Node::Multiply(
                        Box::new(left_simplified),
                        inner.clone(),
                    )))
                    .simplify(env);
                }
                if let Node::Negate(inner) = &left_simplified {
                    return Node::Negate(Box::new(Node::Multiply(
                        inner.clone(),
                        Box::new(right_simplified),
                    )))
                    .simplify(env);
                }

                // **Handle implicit multiplication of number and variable (e.g., 5 * x -> 5x)**
                if let (Node::Num(ref l_coef), Node::Variable(ref var)) =
                    (&left_simplified, &right_simplified)
                {
                    let result = Node::Multiply(
                        Box::new(Node::Num(l_coef.clone())),
                        Box::new(Node::Variable(var.clone())),
                    );
                    return Ok(try_normalize_pi_multiple(&result).unwrap_or(result));
                }
                if let (Node::Variable(ref var), Node::Num(ref r_coef)) =
                    (&left_simplified, &right_simplified)
                {
                    let result = Node::Multiply(
                        Box::new(Node::Num(r_coef.clone())),
                        Box::new(Node::Variable(var.clone())),
                    );
                    return Ok(try_normalize_pi_multiple(&result).unwrap_or(result));
                }

                // x^a * x^b → x^(a+b)
                if let (Node::Power(ref base1, ref exp1), Node::Power(ref base2, ref exp2)) =
                    (&left_simplified, &right_simplified)
                {
                    if base1 == base2 {
                        if let (Node::Num(ref a), Node::Num(ref b)) = (exp1.as_ref(), exp2.as_ref())
                        {
                            return Ok(Node::Power(base1.clone(), Box::new(Node::Num(a + b))));
                        }
                    }
                }

                // x * x^a → x^(a+1)
                if let Node::Power(ref base, ref exp) = right_simplified {
                    if *base.as_ref() == left_simplified {
                        if let Node::Num(ref a) = exp.as_ref() {
                            return Ok(Node::Power(
                                base.clone(),
                                Box::new(Node::Num(a + &ExactNum::one())),
                            ));
                        }
                    }
                }
                // x^a * x → x^(a+1)
                if let Node::Power(ref base, ref exp) = left_simplified {
                    if *base.as_ref() == right_simplified {
                        if let Node::Num(ref a) = exp.as_ref() {
                            return Ok(Node::Power(
                                base.clone(),
                                Box::new(Node::Num(a + &ExactNum::one())),
                            ));
                        }
                    }
                }

                let result = Node::Multiply(Box::new(left_simplified), Box::new(right_simplified));
                // Flatten n-ary products first (e.g. √2·3·√2 → 6) before binary √ matching.
                if let Some(combined) = try_combine_flat_multiply(&result, env) {
                    return Ok(combined);
                }

                // √a · √a → a (binary fallback when flat combine does not apply)
                if let Node::Multiply(ref left, ref right) = result {
                    if let Some(combined) = try_simplify_matching_sqrt_product(left, right, env) {
                        return Ok(combined);
                    }

                    // x * x → x^2
                    if left == right && !matches!(**left, Node::Num(_)) {
                        return Ok(Node::Power(
                            left.clone(),
                            Box::new(Node::Num(ExactNum::two())),
                        ));
                    }
                }
                let result = if let Some(normalized) = try_polynomial_normalize(&result) {
                    normalized
                } else if let Some(normalized) = try_rational_normalize(&result, env) {
                    normalized
                } else {
                    result
                };
                Ok(try_normalize_pi_multiple(&result).unwrap_or(result))
            }
            Node::Power(base, exponent) => {
                let base_simplified = base.simplify(env)?;
                let exponent_simplified = exponent.simplify(env)?;

                // e^x → exp(x)
                if let Node::Variable(ref v) = base_simplified {
                    if v == "e" {
                        return Node::Function("exp".to_string(), vec![exponent_simplified])
                            .simplify(env);
                    }
                }

                // 0^n → 0 for n > 0, 1^n → 1
                if let Node::Num(ref b) = base_simplified {
                    if b.is_zero() {
                        if let Node::Num(ref e) = exponent_simplified {
                            if !e.is_negative() {
                                return Ok(Node::Num(ExactNum::zero()));
                            }
                        }
                    }
                    if b.is_one() {
                        return Ok(Node::Num(ExactNum::one()));
                    }
                }

                // x^0 → 1
                if let Node::Num(ref n) = exponent_simplified {
                    if n.is_zero() {
                        return Ok(Node::Num(ExactNum::one()));
                    }
                }

                // x^1 → x
                if let Node::Num(ref n) = exponent_simplified {
                    if n.is_one() {
                        return Ok(base_simplified);
                    }
                }

                // (√x)² → x (|x| when sign unknown)
                if let Node::Num(ref e) = exponent_simplified {
                    if e == &ExactNum::two() {
                        if let Some(radicand) = extract_sqrt_radicand(&base_simplified) {
                            if let Some(result) = simplify_sqrt_squared(radicand, env) {
                                return Ok(result);
                            }
                        }
                    }
                }

                // If both the base and exponent are numbers, evaluate the power
                if let (Node::Num(ref b), Node::Num(ref e)) =
                    (&base_simplified, &exponent_simplified)
                {
                    return Ok(Node::Num(b.powf(e)));
                }

                // (x^a)^b → x^(a*b) when both exponents are numeric
                if let Node::Power(inner_base, inner_exp) = &base_simplified {
                    if let (Node::Num(ref a), Node::Num(ref b)) =
                        (&**inner_exp, &exponent_simplified)
                    {
                        return Ok(Node::Power(inner_base.clone(), Box::new(Node::Num(a * b))));
                    }
                }

                // (-1)^(2n) → 1 when n is integer (even exponent of -1)
                if is_neg_one(&base_simplified) && is_even_integer_expr(&exponent_simplified, env) {
                    return Ok(Node::Num(ExactNum::one()));
                }

                Ok(Node::Power(
                    Box::new(base_simplified),
                    Box::new(exponent_simplified),
                ))
            }
            Node::Subtract(left, right) => {
                let left_simplified = left.simplify(env)?;
                let right_simplified = right.simplify(env)?;

                if let (Node::Num(ref l), Node::Num(ref r)) = (&left_simplified, &right_simplified)
                {
                    return Ok(Node::Num(l - r));
                }

                if let Node::Num(ref n) = right_simplified {
                    if n.is_zero() {
                        return Ok(left_simplified);
                    }
                }
                if let Node::Num(ref n) = left_simplified {
                    if n.is_zero() {
                        return Ok(Node::Negate(Box::new(right_simplified)));
                    }
                }

                // 1 - sin²(x) → cos²(x), 1 - cos²(x) → sin²(x)
                if let Node::Num(ref n) = left_simplified {
                    if n.is_one() {
                        if let Some(args) = is_trig_squared(&right_simplified, "sin") {
                            return Ok(Node::Power(
                                Box::new(Node::Function("cos".to_string(), args)),
                                Box::new(Node::Num(ExactNum::two())),
                            ));
                        }
                        if let Some(args) = is_trig_squared(&right_simplified, "cos") {
                            return Ok(Node::Power(
                                Box::new(Node::Function("sin".to_string(), args)),
                                Box::new(Node::Num(ExactNum::two())),
                            ));
                        }
                    }
                }

                // sin²(x) - 1 → -cos²(x), cos²(x) - 1 → -sin²(x)
                if let Node::Num(ref n) = right_simplified {
                    if n.is_one() {
                        if let Some(args) = is_trig_squared(&left_simplified, "sin") {
                            return Ok(Node::Negate(Box::new(Node::Power(
                                Box::new(Node::Function("cos".to_string(), args)),
                                Box::new(Node::Num(ExactNum::two())),
                            ))));
                        }
                        if let Some(args) = is_trig_squared(&left_simplified, "cos") {
                            return Ok(Node::Negate(Box::new(Node::Power(
                                Box::new(Node::Function("sin".to_string(), args)),
                                Box::new(Node::Num(ExactNum::two())),
                            ))));
                        }
                    }
                }

                // a·f(x) - b·f(x) → (a-b)·f(x) (before fraction combination)
                if let Some(combined) =
                    try_combine_function_terms(&left_simplified, &right_simplified, true, env)
                {
                    return Ok(combined);
                }

                // a/d - b/d → (a-b)/d
                if let Some(combined) =
                    try_combine_fractions(&left_simplified, &right_simplified, true, env)
                {
                    return Ok(combined);
                }

                // a√X - b√X → (a-b)√X (and like terms in flat n-ary sums)
                let result = Node::Subtract(Box::new(left_simplified), Box::new(right_simplified));
                if let Some(combined) = try_combine_like_radicals(&result, env) {
                    return Ok(combined);
                }
                let mut term_map: HashMap<String, ExactNum> = HashMap::new();
                if collect_terms(&result, &mut term_map, env).is_ok() {
                    Ok(rebuild_expression(term_map))
                } else if let Some(normalized) = try_polynomial_normalize(&result) {
                    Ok(normalized)
                } else if let Some(normalized) = try_rational_normalize(&result, env) {
                    Ok(normalized)
                } else {
                    Ok(result)
                }
            }
            Node::Negate(operand) => {
                let simplified = operand.simplify(env)?;
                if let Node::Num(ref n) = simplified {
                    return Ok(Node::Num(-n.clone()));
                }
                if let Node::Negate(inner) = simplified {
                    return Ok(*inner);
                }
                // -(a + b) → (-a) - b, -(a - b) → b - a
                if let Node::Add(a, b) = simplified {
                    return Node::Subtract(Box::new(Node::Negate(a)), b).simplify(env);
                }
                if let Node::Subtract(a, b) = simplified {
                    return Node::Subtract(b, a).simplify(env);
                }
                Ok(Node::Negate(Box::new(simplified)))
            }
            Node::Factorial(operand) => {
                let simplified = operand.simplify(env)?;
                if let Node::Num(ref n) = simplified {
                    if let Some(folded) = try_fold_factorial_num(n) {
                        return Ok(folded);
                    }
                }
                Ok(Node::Factorial(Box::new(simplified)))
            }
            Node::Divide(left, right) => {
                let left_simplified = left.simplify(env)?;
                let right_simplified = right.simplify(env)?;

                // Cancel common leading negative signs in fractions
                if has_leading_negative(&left_simplified) && has_leading_negative(&right_simplified)
                {
                    let pos_num = negate_leading(&left_simplified);
                    let pos_den = negate_leading(&right_simplified);
                    return Node::Divide(Box::new(pos_num), Box::new(pos_den)).simplify(env);
                }

                if let Node::Num(ref n) = right_simplified {
                    if n.is_one() {
                        return Ok(left_simplified);
                    }
                }

                if let (Node::Num(ref l), Node::Num(ref r)) = (&left_simplified, &right_simplified)
                {
                    return Ok(Node::Num(l / r));
                }

                // 1 / (n/m) → m/n — invert nested rational fraction
                if let Node::Num(ref l) = left_simplified {
                    if l.is_one() {
                        if let Node::Divide(ref inner_num, ref inner_den) = right_simplified {
                            if let (Node::Num(ref n), Node::Num(ref d)) =
                                (&**inner_num, &**inner_den)
                            {
                                return Ok(Node::Num(d / n));
                            }
                        }
                    }
                }

                // (n/expr) / m → (n/m) / expr — collapse nested numeric divisions
                if let Node::Num(ref r) = right_simplified {
                    if let Node::Divide(ref inner_num, ref inner_den) = left_simplified {
                        if let Node::Num(ref l) = **inner_num {
                            let combined = l / r;
                            return Ok(Node::Divide(
                                Box::new(Node::Num(combined)),
                                inner_den.clone(),
                            ));
                        }
                    }
                }

                // (k·expr) / m → (k/m)·expr — cancel common numeric factors
                // Also: -(k·expr) / m → -((k/m)·expr)
                if let Node::Num(ref denom_num) = right_simplified {
                    if !denom_num.is_zero() {
                        // Handle -(k·expr) / m
                        if let Node::Negate(ref inner) = left_simplified {
                            if let Node::Multiply(_, _) = **inner {
                                let inner_div =
                                    Node::Divide(inner.clone(), Box::new(right_simplified.clone()));
                                let inner_result = inner_div.simplify(env)?;
                                return Ok(Node::Negate(Box::new(inner_result)));
                            }
                        }
                        if let Node::Multiply(ref ml, ref mr) = left_simplified {
                            if let Node::Num(ref numer_coeff) = **ml {
                                let reduced = numer_coeff / denom_num;
                                if reduced.is_one() {
                                    return Ok(*mr.clone());
                                }
                                let neg_one = ExactNum::integer(-1);
                                if reduced == neg_one {
                                    return Ok(Node::Negate(mr.clone()));
                                }
                                return Node::Multiply(Box::new(Node::Num(reduced)), mr.clone())
                                    .simplify(env);
                            }
                            if let Node::Num(ref numer_coeff) = **mr {
                                let reduced = numer_coeff / denom_num;
                                if reduced.is_one() {
                                    return Ok(*ml.clone());
                                }
                                let neg_one = ExactNum::integer(-1);
                                if reduced == neg_one {
                                    return Ok(Node::Negate(ml.clone()));
                                }
                                return Node::Multiply(ml.clone(), Box::new(Node::Num(reduced)))
                                    .simplify(env);
                            }
                        }
                    }
                }

                // k / (m·expr) → (k/m) / expr — cancel numeric factor in denominator
                if let Node::Num(ref numer_num) = left_simplified {
                    if let Node::Multiply(ref dl, ref dr) = right_simplified {
                        if let Node::Num(ref denom_coeff) = **dl {
                            if !denom_coeff.is_zero() {
                                let reduced = numer_num / denom_coeff;
                                return Node::Divide(Box::new(Node::Num(reduced)), dr.clone())
                                    .simplify(env);
                            }
                        }
                        if let Node::Num(ref denom_coeff) = **dr {
                            if !denom_coeff.is_zero() {
                                let reduced = numer_num / denom_coeff;
                                return Node::Divide(Box::new(Node::Num(reduced)), dl.clone())
                                    .simplify(env);
                            }
                        }
                    }
                }

                // (k·expr1) / (m·expr2) → cancel common integer factor
                if let Node::Multiply(ref nl, ref nr) = left_simplified {
                    if let Node::Multiply(ref dl, ref dr) = right_simplified {
                        let num_coeff = if let Node::Num(ref k) = **nl {
                            Some((k, nr.as_ref()))
                        } else if let Node::Num(ref k) = **nr {
                            Some((k, nl.as_ref()))
                        } else {
                            None
                        };
                        let den_coeff = if let Node::Num(ref m) = **dl {
                            Some((m, dr.as_ref()))
                        } else if let Node::Num(ref m) = **dr {
                            Some((m, dl.as_ref()))
                        } else {
                            None
                        };
                        if let (Some((k, e1)), Some((m, e2))) = (num_coeff, den_coeff) {
                            if let (ExactNum::Rational(ref kr), ExactNum::Rational(ref mr)) = (k, m)
                            {
                                if kr.is_integer() && mr.is_integer() {
                                    let ki = kr.to_integer();
                                    let mi = mr.to_integer();
                                    let g = ki.gcd(&mi);
                                    let abs_g = if g.is_negative() { -&g } else { g.clone() };
                                    if abs_g > num_bigint::BigInt::from(1) {
                                        let new_k = &ki / &g;
                                        let new_m = &mi / &g;
                                        let one = num_bigint::BigInt::from(1);
                                        let neg_one = num_bigint::BigInt::from(-1);
                                        let make_num = |n: num_bigint::BigInt| {
                                            use num_rational::BigRational;
                                            ExactNum::Rational(BigRational::from_integer(n))
                                        };
                                        let new_num = if new_k == one {
                                            e1.clone()
                                        } else if new_k == neg_one {
                                            Node::Negate(Box::new(e1.clone()))
                                        } else {
                                            Node::Multiply(
                                                Box::new(Node::Num(make_num(new_k))),
                                                Box::new(e1.clone()),
                                            )
                                        };
                                        let new_den = if new_m == one {
                                            e2.clone()
                                        } else if new_m == neg_one {
                                            Node::Negate(Box::new(e2.clone()))
                                        } else {
                                            Node::Multiply(
                                                Box::new(Node::Num(make_num(new_m))),
                                                Box::new(e2.clone()),
                                            )
                                        };
                                        return Node::Divide(Box::new(new_num), Box::new(new_den))
                                            .simplify(env);
                                    }
                                }
                            }
                        }
                    }
                }

                // x / x → 1
                if left_simplified == right_simplified && !matches!(left_simplified, Node::Num(_)) {
                    return Ok(Node::Num(ExactNum::one()));
                }

                // (a · b · … · f) / f → a · b · …  (skip when denominator is zero)
                if !is_zero_node(&right_simplified) {
                    let (negated, numer) = match &left_simplified {
                        Node::Negate(inner) => (true, inner.as_ref()),
                        _ => (false, &left_simplified),
                    };
                    let mut factors = Vec::new();
                    collect_multiply_factors(numer, &mut factors);
                    if factors.len() > 1 {
                        if let Some(idx) = factors.iter().position(|f| f == &right_simplified) {
                            factors.remove(idx);
                            let mut cancelled = rebuild_multiply_product(factors);
                            if negated {
                                cancelled = Node::Negate(Box::new(cancelled));
                            }
                            return cancelled.simplify(env);
                        }
                    }
                }

                // sin(x) / cos(x) → tan(x), cos(x) / sin(x) → cot(x)
                if let (
                    Node::Function(ref fname1, ref args1),
                    Node::Function(ref fname2, ref args2),
                ) = (&left_simplified, &right_simplified)
                {
                    if fname1 == "sin" && fname2 == "cos" && args1 == args2 {
                        return Ok(Node::Function("tan".to_string(), args1.clone()));
                    }
                    if fname1 == "cos" && fname2 == "sin" && args1 == args2 {
                        return Ok(Node::Function("cot".to_string(), args1.clone()));
                    }
                }

                // k·sin(x) / cos(x) → k·tan(x), k·cos(x) / sin(x) → k·cot(x)
                if let Node::Multiply(ref ml, ref mr) = left_simplified {
                    if let Node::Function(ref fname2, ref args2) = right_simplified {
                        let (coeff, func) = if matches!(**ml, Node::Function(_, _)) {
                            (mr, ml)
                        } else {
                            (ml, mr)
                        };
                        if let Node::Function(ref fname1, ref args1) = **func {
                            if args1 == args2 {
                                if fname1 == "sin" && fname2 == "cos" {
                                    return Node::Multiply(
                                        coeff.clone(),
                                        Box::new(Node::Function("tan".to_string(), args1.clone())),
                                    )
                                    .simplify(env);
                                }
                                if fname1 == "cos" && fname2 == "sin" {
                                    return Node::Multiply(
                                        coeff.clone(),
                                        Box::new(Node::Function("cot".to_string(), args1.clone())),
                                    )
                                    .simplify(env);
                                }
                            }
                        }
                    }
                }

                // 1 / sin(x) → csc(x), 1 / cos(x) → sec(x), 1 / tan(x) → cot(x)
                if let Node::Num(ref n) = left_simplified {
                    if n.is_one() {
                        if let Node::Function(ref fname, ref args) = right_simplified {
                            let recip = match fname.as_str() {
                                "sin" => Some("csc"),
                                "cos" => Some("sec"),
                                "tan" => Some("cot"),
                                _ => None,
                            };
                            if let Some(recip_name) = recip {
                                return Ok(Node::Function(recip_name.to_string(), args.clone()));
                            }
                        }
                    }
                }

                // x^a / x^b → x^(a-b)
                if let (Node::Power(ref base1, ref exp1), Node::Power(ref base2, ref exp2)) =
                    (&left_simplified, &right_simplified)
                {
                    if base1 == base2 {
                        if let (Node::Num(ref a), Node::Num(ref b)) = (exp1.as_ref(), exp2.as_ref())
                        {
                            let diff = a - b;
                            if diff.is_zero() {
                                return Ok(Node::Num(ExactNum::one()));
                            } else if diff.is_one() {
                                return Ok(*base1.clone());
                            }
                            return Ok(Node::Power(base1.clone(), Box::new(Node::Num(diff))));
                        }
                    }
                }

                // x^a / x → x^(a-1)
                if let Node::Power(ref base, ref exp) = left_simplified {
                    if *base.as_ref() == right_simplified {
                        if let Node::Num(ref a) = exp.as_ref() {
                            let diff = a - &ExactNum::one();
                            if diff.is_zero() {
                                return Ok(Node::Num(ExactNum::one()));
                            } else if diff.is_one() {
                                return Ok(*base.clone());
                            }
                            return Ok(Node::Power(base.clone(), Box::new(Node::Num(diff))));
                        }
                    }
                }

                // x / x^a → x^(1-a)
                if let Node::Power(ref base, ref exp) = right_simplified {
                    if *base.as_ref() == left_simplified {
                        if let Node::Num(ref a) = exp.as_ref() {
                            let diff = &ExactNum::one() - a;
                            if diff.is_zero() {
                                return Ok(Node::Num(ExactNum::one()));
                            } else if diff.is_one() {
                                return Ok(*base.clone());
                            }
                            return Ok(Node::Power(base.clone(), Box::new(Node::Num(diff))));
                        }
                    }
                }

                if let Some(simplified) = try_polynomial_divide(&left_simplified, &right_simplified)
                {
                    return Ok(simplified);
                }

                let result = Node::Divide(Box::new(left_simplified), Box::new(right_simplified));
                if let Some(normalized) = try_normalize_pi_multiple(&result) {
                    Ok(normalized)
                } else {
                    Ok(result)
                }
            }

            Node::Summation(index_var, start, end, body) => {
                let start_simplified = start.simplify(env)?;
                let end_simplified = end.simplify(env)?;

                // Try telescoping on the unsimplified body (before simplification
                // merges the difference into a single fraction)
                if let Some(result) =
                    try_telescoping_sum(index_var, &start_simplified, &end_simplified, body, env)
                {
                    return result;
                }

                let body_simplified = body.simplify(env)?;

                // Try to evaluate if bounds are constant values
                if let (Node::Num(ref start_n), Node::Num(ref end_n)) =
                    (&start_simplified, &end_simplified)
                {
                    if start_n.is_integer() && end_n.is_integer() {
                        let start_val = start_n.to_f64();
                        let end_val = end_n.to_f64();

                        let range_size = (end_val - start_val + 1.0) as usize;
                        if range_size <= 10 {
                            let mut sum_node = Node::Num(ExactNum::zero());
                            let mut sum_env = env.clone();

                            let start_i = start_val as i64;
                            let end_i = end_val as i64;

                            for i in start_i..=end_i {
                                sum_env.set_exact(index_var, ExactNum::integer(i));

                                let substituted_body = crate::substitute::substitute_variable(
                                    &body_simplified,
                                    index_var,
                                    &Node::Num(ExactNum::integer(i)),
                                )?;

                                sum_node =
                                    Node::Add(Box::new(sum_node), Box::new(substituted_body));
                            }

                            return sum_node.simplify(env);
                        }
                    }
                }

                // Try symbolic closed form (Faulhaber, geometric, constant)
                if let Some(result) = try_symbolic_summation(
                    index_var,
                    &start_simplified,
                    &end_simplified,
                    &body_simplified,
                    env,
                ) {
                    return result;
                }

                // If we can't find a closed form, return with simplified components
                Ok(Node::Summation(
                    index_var.clone(),
                    Box::new(start_simplified),
                    Box::new(end_simplified),
                    Box::new(body_simplified),
                ))
            }
            Node::Product(index_var, start, end, body) => {
                let start_simplified = start.simplify(env)?;
                let end_simplified = end.simplify(env)?;
                let body_simplified = body.simplify(env)?;

                // Try to evaluate if bounds are constant values
                if let (Node::Num(ref start_n), Node::Num(ref end_n)) =
                    (&start_simplified, &end_simplified)
                {
                    if start_n.is_integer() && end_n.is_integer() {
                        let start_val = start_n.to_f64();
                        let end_val = end_n.to_f64();

                        let range_size = (end_val - start_val + 1.0) as usize;
                        if range_size <= 10 {
                            let mut prod_node = Node::Num(ExactNum::one());
                            let mut prod_env = env.clone();

                            let start_i = start_val as i64;
                            let end_i = end_val as i64;

                            for i in start_i..=end_i {
                                prod_env.set_exact(index_var, ExactNum::integer(i));

                                let substituted_body = crate::substitute::substitute_variable(
                                    &body_simplified,
                                    index_var,
                                    &Node::Num(ExactNum::integer(i)),
                                )?;

                                prod_node =
                                    Node::Multiply(Box::new(prod_node), Box::new(substituted_body));
                            }

                            return prod_node.simplify(env);
                        }
                    }
                }

                if let Some(result) = try_symbolic_product(
                    index_var,
                    &start_simplified,
                    &end_simplified,
                    &body_simplified,
                    env,
                ) {
                    return result;
                }

                // If we can't find a closed form, return with simplified components
                Ok(Node::Product(
                    index_var.clone(),
                    Box::new(start_simplified),
                    Box::new(end_simplified),
                    Box::new(body_simplified),
                ))
            }
            Node::Abs(operand) => {
                let simplified = operand.simplify(env)?;
                if let Node::Num(ref n) = simplified {
                    return Ok(Node::Num(n.abs()));
                }
                // |x| → x when x is nonnegative or a known positive constant
                if let Node::Variable(ref v) = simplified {
                    if v == "e" || v == "π" || env.assumptions().is_nonneg(v) {
                        return Ok(simplified);
                    }
                    if env.assumptions().is_negative(v) {
                        return Ok(Node::Negate(Box::new(simplified)));
                    }
                }
                // |-x| → |x|
                if let Node::Negate(inner) = simplified {
                    return Ok(Node::Abs(inner));
                }
                // ||x|| → |x|
                if let Node::Abs(_) = simplified {
                    return Ok(simplified);
                }
                Ok(Node::Abs(Box::new(simplified)))
            }
            Node::Floor(operand) => {
                let simplified = operand.simplify(env)?;
                if let Node::Num(ref n) = simplified {
                    return Ok(Node::Num(n.floor()));
                }
                Ok(Node::Floor(Box::new(simplified)))
            }
            Node::Ceil(operand) => {
                let simplified = operand.simplify(env)?;
                if let Node::Num(ref n) = simplified {
                    return Ok(Node::Num(n.ceil()));
                }
                Ok(Node::Ceil(Box::new(simplified)))
            }
            Node::Round(operand) => {
                let simplified = operand.simplify(env)?;
                if let Node::Num(ref n) = simplified {
                    return Ok(Node::Num(n.round()));
                }
                Ok(Node::Round(Box::new(simplified)))
            }
            Node::Trunc(operand) => {
                let simplified = operand.simplify(env)?;
                if let Node::Num(ref n) = simplified {
                    return Ok(Node::Num(n.trunc()));
                }
                Ok(Node::Trunc(Box::new(simplified)))
            }
            Node::Sqrt(operand) => {
                let simplified = operand.simplify(env)?;
                if let Node::Num(ref n) = simplified {
                    let s = n.sqrt();
                    if matches!(s, ExactNum::Rational(_)) {
                        return Ok(Node::Num(s));
                    }
                    // Non-perfect-square integer: extract square factors
                    if let Some(val) = n.to_i64() {
                        if val > 0 {
                            let (outside, inside) = extract_square_factors(val as u64);
                            if inside == 1 {
                                return Ok(Node::Num(ExactNum::integer(outside as i64)));
                            }
                            let sqrt_inside =
                                Node::Sqrt(Box::new(Node::Num(ExactNum::integer(inside as i64))));
                            if outside == 1 {
                                return Ok(sqrt_inside);
                            }
                            return Ok(Node::Multiply(
                                Box::new(Node::Num(ExactNum::integer(outside as i64))),
                                Box::new(sqrt_inside),
                            ));
                        }
                    }
                    // Non-perfect-square rational: try numerator and denominator separately
                    if let ExactNum::Rational(ref r) = n {
                        if !r.is_negative() {
                            if let (Some(num), Some(den)) = (r.numer().to_i64(), r.denom().to_i64())
                            {
                                if num > 0 && den > 0 {
                                    let (num_out, num_in) = extract_square_factors(num as u64);
                                    let (den_out, den_in) = extract_square_factors(den as u64);
                                    if num_in == 1 && den_in == 1 {
                                        return Ok(Node::Num(ExactNum::rational(
                                            num_out as i64,
                                            den_out as i64,
                                        )));
                                    }
                                    let sqrt_part = Node::Sqrt(Box::new(Node::Num(
                                        ExactNum::rational(num_in as i64, den_in as i64),
                                    )));
                                    if num_out == 1 && den_out == 1 {
                                        return Ok(sqrt_part);
                                    }
                                    return Ok(Node::Multiply(
                                        Box::new(Node::Num(ExactNum::rational(
                                            num_out as i64,
                                            den_out as i64,
                                        ))),
                                        Box::new(sqrt_part),
                                    ));
                                }
                            }
                        }
                    }
                    // Keep symbolic — do NOT fall back to float
                    return Ok(Node::Sqrt(Box::new(simplified)));
                }
                // Product radicand: decompose and extract
                if let Some(result) = try_simplify_sqrt_product(&simplified, env, true) {
                    return Ok(result);
                }
                // sqrt(x²) → x when x positive, |x| otherwise
                if let Node::Power(ref base, ref exp) = simplified {
                    if let Node::Num(ref e) = **exp {
                        if e == &ExactNum::two() {
                            if let Node::Variable(ref v) = **base {
                                if env.assumptions().is_nonneg(v) {
                                    return Ok(*base.clone());
                                }
                            }
                            return Ok(Node::Abs(base.clone()));
                        }
                    }
                }
                Ok(Node::Sqrt(Box::new(simplified)))
            }
            Node::Function(name, args) => {
                // Fold `exp(ln x)` and `exp(k·ln a)` before inner rewrites (e.g. `log(10) → 1`).
                if name == "exp" && args.len() == 1 {
                    if let Some(result) = try_exact_function_value(name, args) {
                        return result.simplify(env);
                    }
                }

                let simplified_args: Vec<Node> = args
                    .iter()
                    .map(|a| a.simplify(env))
                    .collect::<Result<Vec<_>, _>>()?;

                if simplified_args.len() == 1 {
                    if let Some(exact) = try_exact_function_value(name, &simplified_args) {
                        return Ok(exact);
                    }

                    if name == "abs" {
                        return Ok(Node::Abs(Box::new(simplified_args[0].clone())));
                    }
                    if name == "floor" {
                        return Ok(Node::Floor(Box::new(simplified_args[0].clone())));
                    }
                    if name == "ceil" {
                        return Ok(Node::Ceil(Box::new(simplified_args[0].clone())));
                    }
                    if name == "round" {
                        return Ok(Node::Round(Box::new(simplified_args[0].clone())));
                    }
                    if name == "trunc" {
                        return Ok(Node::Trunc(Box::new(simplified_args[0].clone())));
                    }

                    let arg = &simplified_args[0];
                    match name.as_str() {
                        "ln" | "log" | "lg" => {
                            if let Some(result) = simplify_log_function(name, arg, env) {
                                return Ok(result);
                            }
                        }
                        "sqrt" => {
                            // Numeric: extract square factors to avoid float fallback
                            if let Node::Num(ref n) = arg {
                                // Perfect rational square → exact result
                                let s = n.sqrt();
                                if matches!(s, ExactNum::Rational(_)) {
                                    return Ok(Node::Num(s));
                                }
                                // Non-perfect-square integer: √n = outside · √inside
                                if let Some(val) = n.to_i64() {
                                    if val > 0 {
                                        let (outside, inside) = extract_square_factors(val as u64);
                                        if inside == 1 {
                                            return Ok(Node::Num(ExactNum::integer(
                                                outside as i64,
                                            )));
                                        }
                                        let sqrt_inside = Node::Sqrt(Box::new(Node::Num(
                                            ExactNum::integer(inside as i64),
                                        )));
                                        if outside == 1 {
                                            return Ok(sqrt_inside);
                                        }
                                        return Ok(Node::Multiply(
                                            Box::new(Node::Num(ExactNum::integer(outside as i64))),
                                            Box::new(sqrt_inside),
                                        ));
                                    }
                                }
                                // Non-perfect-square rational
                                if let ExactNum::Rational(ref r) = n {
                                    if !r.is_negative() {
                                        if let (Some(num), Some(den)) =
                                            (r.numer().to_i64(), r.denom().to_i64())
                                        {
                                            if num > 0 && den > 0 {
                                                let (num_out, num_in) =
                                                    extract_square_factors(num as u64);
                                                let (den_out, den_in) =
                                                    extract_square_factors(den as u64);
                                                if num_in == 1 && den_in == 1 {
                                                    return Ok(Node::Num(ExactNum::rational(
                                                        num_out as i64,
                                                        den_out as i64,
                                                    )));
                                                }
                                                let sqrt_part = Node::Sqrt(Box::new(Node::Num(
                                                    ExactNum::rational(
                                                        num_in as i64,
                                                        den_in as i64,
                                                    ),
                                                )));
                                                if num_out == 1 && den_out == 1 {
                                                    return Ok(sqrt_part);
                                                }
                                                return Ok(Node::Multiply(
                                                    Box::new(Node::Num(ExactNum::rational(
                                                        num_out as i64,
                                                        den_out as i64,
                                                    ))),
                                                    Box::new(sqrt_part),
                                                ));
                                            }
                                        }
                                    }
                                }
                                // Fallback: keep symbolic as Sqrt node
                                return Ok(Node::Sqrt(Box::new(arg.clone())));
                            }
                            // Product radicand: decompose and extract
                            if let Some(result) = try_simplify_sqrt_product(arg, env, false) {
                                return Ok(result);
                            }
                            // sqrt(x²) → x when x nonneg, |x| otherwise
                            if let Node::Power(base, exp) = arg {
                                if let Node::Num(ref e) = **exp {
                                    if e == &ExactNum::two() {
                                        if let Node::Variable(ref v) = **base {
                                            if env.assumptions().is_nonneg(v) {
                                                return Ok(*base.clone());
                                            }
                                        }
                                        return Ok(Node::Abs(base.clone()));
                                    }
                                }
                            }
                        }
                        // Odd functions: f(-x) → -f(x)
                        "sin" | "tan" | "sinh" | "tanh" | "csc" | "cot" | "csch" | "coth"
                        | "arcsin" | "arctan" | "arccot" | "arcsinh" | "arctanh" | "arccsch"
                        | "arccoth" => {
                            if let Node::Negate(inner) = arg {
                                let inner_val = Node::Function(name.clone(), vec![*inner.clone()])
                                    .simplify(env)?;
                                return Ok(Node::Negate(Box::new(inner_val)));
                            }
                        }
                        // Even functions: f(-x) → f(x)
                        "cos" | "cosh" | "sec" | "sech" => {
                            if let Node::Negate(inner) = arg {
                                return Node::Function(name.clone(), vec![*inner.clone()])
                                    .simplify(env);
                            }
                        }
                        // arccos(-x) → π − arccos(x), arcsec(-x) → π − arcsec(x)
                        "arccos" | "arcsec" => {
                            if let Node::Negate(inner) = arg {
                                let inner_val = Node::Function(name.clone(), vec![*inner.clone()])
                                    .simplify(env)?;
                                return Node::Subtract(
                                    Box::new(Node::Variable("π".to_string())),
                                    Box::new(inner_val),
                                )
                                .simplify(env);
                            }
                        }
                        _ => {}
                    }
                }

                if simplified_args.len() != 1 {
                    if let Some(exact) = try_exact_function_value(name, &simplified_args) {
                        return Ok(exact);
                    }
                }

                // Keep log of positive integers symbolic (primes and already-factored bases).
                if matches!(name.as_str(), "ln" | "log" | "lg")
                    && simplified_args.len() == 1
                    && matches!(
                        &simplified_args[0],
                        Node::Num(n) if n.to_rational()
                            .is_some_and(|r| r.is_integer() && r > BigRational::one())
                    )
                {
                    return Ok(Node::Function(name.clone(), simplified_args));
                }

                let all_numeric = simplified_args.iter().all(|a| matches!(a, Node::Num(_)));
                // Preserve exactness: don't collapse a transcendental function of an
                // exact argument to a float (e.g. sin(2) stays sin(2), not 0.909…).
                // Fold only when some argument is already inexact, or the function
                // yields a rational result (gcd, min, max, …).
                let any_inexact = simplified_args
                    .iter()
                    .any(|a| matches!(a, Node::Num(ExactNum::Float(_))));
                let keep_symbolic = is_transcendental_function(name) && !any_inexact;
                if all_numeric && !keep_symbolic {
                    let f64_args: Vec<f64> = simplified_args
                        .iter()
                        .map(|a| {
                            if let Node::Num(n) = a {
                                n.to_f64()
                            } else {
                                unreachable!()
                            }
                        })
                        .collect();
                    if let Ok(result) = crate::functions::call_function(name, f64_args) {
                        if result.is_finite() {
                            return Ok(Node::Num(ExactNum::from_f64(result)));
                        }
                    }
                }

                Ok(Node::Function(name.clone(), simplified_args))
            }
            _ => Ok(self.clone()),
        }
    }
}

fn has_leading_negative(node: &Node) -> bool {
    match node {
        Node::Negate(_) => true,
        Node::Num(n) => n.is_negative(),
        Node::Multiply(a, _) => has_leading_negative(a),
        Node::Add(a, _) | Node::Subtract(a, _) => has_leading_negative(a),
        _ => false,
    }
}

fn negate_leading(node: &Node) -> Node {
    match node {
        Node::Negate(inner) => *inner.clone(),
        Node::Num(n) => Node::Num(-n.clone()),
        Node::Multiply(a, b) => Node::Multiply(Box::new(negate_leading(a)), b.clone()),
        Node::Subtract(a, b) => Node::Subtract(b.clone(), a.clone()),
        _ => Node::Negate(Box::new(node.clone())),
    }
}

fn collect_terms_inner(
    node: &Node,
    term_map: &mut HashMap<String, ExactNum>,
    sign: &ExactNum,
) -> Result<(), String> {
    match node {
        Node::Add(left, right) => {
            collect_terms_inner(left, term_map, sign)?;
            collect_terms_inner(right, term_map, sign)?;
        }
        Node::Subtract(left, right) => {
            collect_terms_inner(left, term_map, sign)?;
            let neg_sign = sign.clone() * ExactNum::integer(-1);
            collect_terms_inner(right, term_map, &neg_sign)?;
        }
        Node::Negate(inner) => {
            let neg_sign = sign.clone() * ExactNum::integer(-1);
            collect_terms_inner(inner, term_map, &neg_sign)?;
        }
        Node::Multiply(left, right) => {
            if let (Node::Num(ref coef), Node::Variable(ref var)) = (&**left, &**right) {
                let entry = term_map.entry(var.clone()).or_insert_with(ExactNum::zero);
                *entry = entry.clone() + coef.clone() * sign.clone();
            } else {
                return Err("Unsupported multiply form in collect_terms".to_string());
            }
        }
        Node::Variable(var) => {
            let entry = term_map.entry(var.clone()).or_insert_with(ExactNum::zero);
            *entry = entry.clone() + sign.clone();
        }
        Node::Num(num) => {
            let entry = term_map
                .entry("".to_string())
                .or_insert_with(ExactNum::zero);
            *entry = entry.clone() + num.clone() * sign.clone();
        }
        _ => return Err("Unsupported node type in collect_terms".to_string()),
    }
    Ok(())
}

fn collect_terms(
    node: &Node,
    term_map: &mut HashMap<String, ExactNum>,
    _env: &Environment,
) -> Result<(), String> {
    collect_terms_inner(node, term_map, &ExactNum::one())
}

fn rebuild_expression(term_map: HashMap<String, ExactNum>) -> Node {
    let mut terms: Vec<(String, ExactNum)> = term_map.into_iter().collect();

    // Sort: variables alphabetically first, constant term last
    terms.sort_by(|a, b| match (a.0.is_empty(), b.0.is_empty()) {
        (true, false) => std::cmp::Ordering::Greater,
        (false, true) => std::cmp::Ordering::Less,
        _ => a.0.cmp(&b.0),
    });

    // Build (abs_node, is_negative) pairs for non-zero terms
    let mut signed_terms: Vec<(Node, bool)> = vec![];

    for (var, coef) in terms {
        if coef.is_zero() {
            continue;
        }
        let negative = coef.is_negative();
        let abs_coef = if negative {
            -coef.clone()
        } else {
            coef.clone()
        };

        let node = if var.is_empty() {
            Node::Num(abs_coef)
        } else if abs_coef.is_one() {
            Node::Variable(var)
        } else {
            Node::Multiply(Box::new(Node::Num(abs_coef)), Box::new(Node::Variable(var)))
        };
        signed_terms.push((node, negative));
    }

    if signed_terms.is_empty() {
        return Node::Num(ExactNum::zero());
    }

    let (first_node, first_neg) = signed_terms.remove(0);
    let mut result = if first_neg {
        Node::Negate(Box::new(first_node))
    } else {
        first_node
    };

    for (node, negative) in signed_terms {
        result = if negative {
            Node::Subtract(Box::new(result), Box::new(node))
        } else {
            Node::Add(Box::new(result), Box::new(node))
        };
    }

    result
}

fn find_single_variable(node: &Node) -> Option<String> {
    let mut vars = std::collections::HashSet::new();
    collect_variables(node, &mut vars);
    if vars.len() == 1 {
        vars.into_iter().next()
    } else {
        None
    }
}

fn collect_variables(node: &Node, vars: &mut std::collections::HashSet<String>) {
    match node {
        Node::Variable(v) => {
            vars.insert(v.clone());
        }
        Node::Add(l, r)
        | Node::Subtract(l, r)
        | Node::Multiply(l, r)
        | Node::Divide(l, r)
        | Node::Power(l, r) => {
            collect_variables(l, vars);
            collect_variables(r, vars);
        }
        Node::Negate(inner) | Node::Sqrt(inner) | Node::Abs(inner) => {
            collect_variables(inner, vars);
        }
        _ => {}
    }
}

fn contains_divide(node: &Node) -> bool {
    match node {
        Node::Divide(_, _) => true,
        Node::Add(a, b) | Node::Subtract(a, b) | Node::Multiply(a, b) => {
            contains_divide(a) || contains_divide(b)
        }
        Node::Negate(a) => contains_divide(a),
        _ => false,
    }
}

fn try_rational_normalize(node: &Node, env: &Environment) -> Option<Node> {
    if !contains_divide(node) {
        return None;
    }
    let (num, den) = crate::expression::to_rational_form(node)?;
    let num_simplified = num.simplify(env).ok()?;
    let den_simplified = den.simplify(env).ok()?;
    if let Node::Num(ref n) = den_simplified {
        if n.is_one() {
            return Some(num_simplified);
        }
    }
    Some(Node::Divide(
        Box::new(num_simplified),
        Box::new(den_simplified),
    ))
}

fn try_polynomial_normalize(node: &Node) -> Option<Node> {
    if let Some(var) = find_single_variable(node) {
        let poly = Polynomial::from_node(node, &var).ok()?;
        return Some(poly.to_node());
    }
    // Multivariate fallback
    let mp = MultiPoly::from_node(node).ok()?;
    Some(mp.to_node())
}

fn try_factored_display(poly: &Polynomial) -> Option<Node> {
    use crate::mod_poly::factor_over_q;
    use crate::polynomial::{lcm_bigint, rational_to_node};
    use num_bigint::BigInt;
    use num_traits::One;

    // Only factor degree > 1 polynomials
    let deg = poly.degree()?;
    if deg <= 1 {
        return None;
    }

    let (content, factors) = factor_over_q(poly);
    // Only use factored form when there are multiple factors (distinct or repeated)
    if factors.len() <= 1 {
        return None;
    }

    // Group identical monic factors and convert to integer-coefficient form
    let mut grouped: Vec<(Polynomial, usize)> = Vec::new();
    let mut adjusted_content = content;

    for f in &factors {
        // Clear denominators: find LCM of coefficient denominators
        let mut lcm = BigInt::one();
        for i in 0..=f.degree().unwrap_or(0) {
            let c = f.coeff(i);
            lcm = lcm_bigint(&lcm, c.denom());
        }
        let scale = num_rational::BigRational::from_integer(lcm);
        let f_int = f.scalar_mul(&scale);
        adjusted_content /= &scale;

        // Group identical factors
        let found = grouped.iter_mut().find(|(p, _)| {
            p.degree() == f_int.degree()
                && (0..=p.degree().unwrap_or(0)).all(|i| p.coeff(i) == f_int.coeff(i))
        });
        if let Some(entry) = found {
            entry.1 += 1;
        } else {
            grouped.push((f_int, 1));
        }
    }

    // Build the factored node
    let mut parts: Vec<Node> = Vec::new();

    // Add content coefficient if not 1
    if !adjusted_content.is_one() {
        parts.push(rational_to_node(&adjusted_content));
    }

    for (factor, mult) in &grouped {
        let factor_node = factor.to_node();
        let term = if *mult > 1 {
            Node::Power(
                Box::new(factor_node),
                Box::new(Node::Num(ExactNum::integer(*mult as i64))),
            )
        } else {
            factor_node
        };
        parts.push(term);
    }

    if parts.is_empty() {
        return Some(Node::Num(ExactNum::one()));
    }

    let mut result = parts.remove(0);
    for part in parts {
        result = Node::Multiply(Box::new(result), Box::new(part));
    }

    Some(result)
}

fn try_polynomial_divide(numer: &Node, denom: &Node) -> Option<Node> {
    let mut vars = std::collections::HashSet::new();
    collect_variables(numer, &mut vars);
    collect_variables(denom, &mut vars);

    if vars.len() == 1 {
        let var = vars.into_iter().next()?;
        return try_univariate_divide(numer, denom, &var);
    }

    if vars.len() >= 2 {
        return try_multivariate_divide(numer, denom);
    }

    None
}

fn try_univariate_divide(numer: &Node, denom: &Node, var: &str) -> Option<Node> {
    use crate::polynomial::rational_gcd;
    let n = Polynomial::from_node(numer, var).ok()?;
    let d = Polynomial::from_node(denom, var).ok()?;

    if d.is_zero() {
        return None;
    }

    // Step 1: Polynomial GCD cancellation
    let g = n.gcd(&d);
    let (n_red, d_red) = if g.degree().unwrap_or(0) > 0 {
        let (nr, nr_rem) = n.div_rem(&g).ok()?;
        let (dr, dr_rem) = d.div_rem(&g).ok()?;
        if !nr_rem.is_zero() || !dr_rem.is_zero() {
            return None;
        }
        (nr, dr)
    } else {
        (n.clone(), d.clone())
    };

    // Step 2: Content simplification — cancel GCD of rational coefficients
    let c_n = n_red.content();
    let c_d = d_red.content();
    let c_gcd = rational_gcd(&c_n, &c_d);
    let one = num_rational::BigRational::from_integer(num_bigint::BigInt::from(1));
    let (n_final, d_final) = if c_gcd != one {
        let inv = num_rational::BigRational::from_integer(num_bigint::BigInt::from(1)) / &c_gcd;
        (n_red.scalar_mul(&inv), d_red.scalar_mul(&inv))
    } else {
        (n_red, d_red)
    };

    // Step 3: Try factored display for numerator and denominator
    let denom_factored = try_factored_display(&d_final);
    let numer_factored = try_factored_display(&n_final);

    // Check if anything changed (GCD/content simplification or factored display)
    let changed =
        n_final != n || d_final != d || denom_factored.is_some() || numer_factored.is_some();
    if !changed {
        return None;
    }

    // Step 4: Build result
    if d_final.is_constant() {
        let d_val = d_final.coeff(0);
        if d_val == one {
            return Some(numer_factored.unwrap_or_else(|| n_final.to_node()));
        }
    }

    let denom_node = denom_factored.unwrap_or_else(|| d_final.to_node());
    let numer_node = numer_factored.unwrap_or_else(|| n_final.to_node());

    Some(Node::Divide(Box::new(numer_node), Box::new(denom_node)))
}

fn try_multivariate_divide(numer: &Node, denom: &Node) -> Option<Node> {
    let n = MultiPoly::from_node(numer).ok()?;
    let d = MultiPoly::from_node(denom).ok()?;

    if d.is_zero() {
        return None;
    }

    // Step 1: Polynomial GCD cancellation
    let g = MultiPoly::gcd(&n, &d);
    let (n_red, d_red) = if !g.is_constant() {
        (n.exact_div(&g), d.exact_div(&g))
    } else {
        (n.clone(), d.clone())
    };

    // Step 2: Rational content simplification
    use crate::polynomial::rational_gcd;
    use num_traits::One;
    let c_n = n_red.rational_content();
    let c_d = d_red.rational_content();
    let c_gcd = rational_gcd(&c_n, &c_d);
    let (n_final, d_final) = if !c_gcd.is_one() {
        (
            n_red.scalar_div_rational(&c_gcd),
            d_red.scalar_div_rational(&c_gcd),
        )
    } else {
        (n_red, d_red)
    };

    // Step 3: Check if anything changed
    if n_final == n && d_final == d {
        return None;
    }

    if d_final.is_one() {
        return Some(n_final.to_node());
    }
    if let Some(d_val) = d_final.as_constant() {
        if !num_traits::Zero::is_zero(d_val) {
            let inv = num_rational::BigRational::from_integer(num_bigint::BigInt::from(1)) / d_val;
            return Some(n_final.scalar_mul(&inv).to_node());
        }
    }

    Some(Node::Divide(
        Box::new(n_final.to_node()),
        Box::new(d_final.to_node()),
    ))
}

fn is_trig_squared(node: &Node, func_name: &str) -> Option<Vec<Node>> {
    if let Node::Power(base, exp) = node {
        if let Node::Num(ref e) = **exp {
            if e == &ExactNum::two() {
                if let Node::Function(name, args) = base.as_ref() {
                    if name == func_name {
                        return Some(args.clone());
                    }
                }
            }
        }
    }
    None
}

fn try_pythagorean(left: &Node, right: &Node) -> Option<Node> {
    // sin²(x) + cos²(x) → 1
    if let (Some(sin_args), Some(cos_args)) =
        (is_trig_squared(left, "sin"), is_trig_squared(right, "cos"))
    {
        if sin_args == cos_args {
            return Some(Node::Num(ExactNum::one()));
        }
    }
    // cos²(x) + sin²(x) → 1
    if let (Some(cos_args), Some(sin_args)) =
        (is_trig_squared(left, "cos"), is_trig_squared(right, "sin"))
    {
        if cos_args == sin_args {
            return Some(Node::Num(ExactNum::one()));
        }
    }

    // a·sin²(x) + a·cos²(x) → a (with coefficient)
    if let (Some((coeff_l, sin_args)), Some((coeff_r, cos_args))) = (
        extract_coeff_trig_sq(left, "sin"),
        extract_coeff_trig_sq(right, "cos"),
    ) {
        if sin_args == cos_args && coeff_l == coeff_r {
            return Some(Node::Num(coeff_l));
        }
    }
    if let (Some((coeff_l, cos_args)), Some((coeff_r, sin_args))) = (
        extract_coeff_trig_sq(left, "cos"),
        extract_coeff_trig_sq(right, "sin"),
    ) {
        if cos_args == sin_args && coeff_l == coeff_r {
            return Some(Node::Num(coeff_l));
        }
    }

    None
}

fn extract_coeff_trig_sq(node: &Node, func_name: &str) -> Option<(ExactNum, Vec<Node>)> {
    if let Some(args) = is_trig_squared(node, func_name) {
        return Some((ExactNum::one(), args));
    }
    if let Node::Multiply(coeff, power) = node {
        if let Node::Num(ref c) = **coeff {
            if let Some(args) = is_trig_squared(power, func_name) {
                return Some((c.clone(), args));
            }
        }
        if let Node::Num(ref c) = **power {
            if let Some(args) = is_trig_squared(coeff, func_name) {
                return Some((c.clone(), args));
            }
        }
    }
    None
}

fn is_neg_one(node: &Node) -> bool {
    match node {
        Node::Num(n) => n == &ExactNum::integer(-1),
        Node::Negate(inner) => {
            matches!(&**inner, Node::Num(n) if n.is_one())
        }
        _ => false,
    }
}

fn is_even_integer_expr(node: &Node, env: &Environment) -> bool {
    // 2n, 2*n, k*n where k is even and n is integer
    if let Node::Multiply(left, right) = node {
        match (&**left, &**right) {
            (Node::Num(k), Node::Variable(v)) | (Node::Variable(v), Node::Num(k)) => {
                return k.is_even() && env.assumptions().is_integer(v);
            }
            _ => {}
        }
    }
    // A numeric even integer by itself
    if let Node::Num(n) = node {
        return n.is_even();
    }
    false
}

/// Extract radicand from √X whether stored as `Node::Sqrt` or `Function("sqrt", …)`.
fn extract_sqrt_radicand(node: &Node) -> Option<Node> {
    match node {
        Node::Sqrt(inner) => Some(*inner.clone()),
        Node::Function(name, args) if name == "sqrt" && args.len() == 1 => Some(args[0].clone()),
        _ => None,
    }
}

fn radicals_match(left: &Node, right: &Node) -> bool {
    match (extract_sqrt_radicand(left), extract_sqrt_radicand(right)) {
        (Some(l), Some(r)) => format!("{l}") == format!("{r}"),
        _ => format!("{left}") == format!("{right}"),
    }
}

fn simplify_sqrt_squared(radicand: Node, env: &Environment) -> Option<Node> {
    if let Node::Num(ref n) = radicand {
        if n.is_negative() {
            return None;
        }
    }
    if let Node::Variable(ref v) = radicand {
        if env.assumptions().is_nonneg(v) {
            return Some(radicand);
        }
        return Some(Node::Abs(Box::new(radicand)));
    }
    radicand.simplify(env).ok()
}

/// Attach a scalar coefficient to a simplified term (folds into `Num` when possible).
fn attach_scalar_to_term(scalar: ExactNum, term: Node) -> Node {
    if scalar.is_zero() {
        return Node::Num(ExactNum::zero());
    }
    if scalar.is_one() {
        return term;
    }
    if let Node::Num(ref n) = term {
        return Node::Num(scalar * n.clone());
    }
    Node::Multiply(Box::new(Node::Num(scalar)), Box::new(term))
}

/// Collapse `pairs` pairs of √X with combined coefficient `coeff`.
/// Each pair contributes one factor of the radicand (√X·√X → X).
fn collapse_sqrt_pair_product(
    radicand: Node,
    pairs: u32,
    coeff: ExactNum,
    env: &Environment,
) -> Option<Node> {
    if pairs == 0 {
        return None;
    }

    if let Node::Num(ref n) = radicand {
        if n.is_negative() {
            return None;
        }
        let mut radicand_power = ExactNum::one();
        for _ in 0..pairs {
            radicand_power = radicand_power * n.clone();
        }
        return Some(Node::Num(coeff * radicand_power));
    }

    let squared = simplify_sqrt_squared(radicand, env)?;
    let radicand_factor = if pairs == 1 {
        squared
    } else {
        Node::Power(
            Box::new(squared),
            Box::new(Node::Num(ExactNum::integer(pairs as i64))),
        )
    };
    let result = attach_scalar_to_term(coeff, radicand_factor);
    Some(result.simplify(env).unwrap_or(result))
}

/// √a · √a → a (and (a√X)(b√X) → ab·X when X matches).
fn try_simplify_matching_sqrt_product(
    left: &Node,
    right: &Node,
    env: &Environment,
) -> Option<Node> {
    let (l_coeff, l_radical) = extract_radical_parts(left)?;
    let (r_coeff, r_radical) = extract_radical_parts(right)?;

    if !radicals_match(&l_radical, &r_radical) {
        return None;
    }

    let radicand = extract_sqrt_radicand(&l_radical)?;
    collapse_sqrt_pair_product(radicand, 1, l_coeff * r_coeff, env)
}

/// Extract (coefficient, radical) from a term that's either a bare √X or coeff·√X.
fn extract_radical_parts(node: &Node) -> Option<(ExactNum, Node)> {
    if extract_sqrt_radicand(node).is_some() {
        return Some((ExactNum::one(), node.clone()));
    }
    match node {
        Node::Multiply(left, right) => {
            if let Node::Num(ref coeff) = **left {
                if extract_sqrt_radicand(right).is_some() {
                    return Some((coeff.clone(), *right.clone()));
                }
            }
            if let Node::Num(ref coeff) = **right {
                if extract_sqrt_radicand(left).is_some() {
                    return Some((coeff.clone(), *left.clone()));
                }
            }
            None
        }
        _ => None,
    }
}

/// Build a√X from combined coefficient; returns None when the term vanishes.
fn build_coeff_radical_term(coeff: ExactNum, radical: Node) -> Option<Node> {
    if coeff.is_zero() {
        return None;
    }
    if coeff.is_one() {
        return Some(radical);
    }
    if coeff == ExactNum::integer(-1) {
        return Some(Node::Negate(Box::new(radical)));
    }
    Some(Node::Multiply(
        Box::new(Node::Num(coeff)),
        Box::new(radical),
    ))
}

/// Flatten a nested Add/Subtract tree into signed summands.
fn flatten_add_sub_terms(node: &Node, terms: &mut Vec<(Node, bool)>, negative: bool) {
    match node {
        Node::Add(left, right) => {
            flatten_add_sub_terms(left, terms, negative);
            flatten_add_sub_terms(right, terms, negative);
        }
        Node::Subtract(left, right) => {
            flatten_add_sub_terms(left, terms, negative);
            flatten_add_sub_terms(right, terms, !negative);
        }
        Node::Negate(inner) => {
            flatten_add_sub_terms(inner, terms, !negative);
        }
        other => terms.push((other.clone(), negative)),
    }
}

/// Flatten a nested Multiply tree into factors (owned clones).
fn flatten_multiply_factors_owned(node: &Node, factors: &mut Vec<Node>) {
    match node {
        Node::Multiply(left, right) => {
            flatten_multiply_factors_owned(left, factors);
            flatten_multiply_factors_owned(right, factors);
        }
        other => factors.push(other.clone()),
    }
}

/// Combine factors in a flat n-ary product: numeric · … · √X · … with scalars between radicals.
///
/// Also folds pure numeric products (e.g. `2·3·x → 6x`) when factor count shrinks.
fn try_combine_flat_multiply(node: &Node, env: &Environment) -> Option<Node> {
    if !matches!(node, Node::Multiply(_, _)) {
        return None;
    }

    let mut flat = Vec::new();
    flatten_multiply_factors_owned(node, &mut flat);
    if flat.len() < 2 {
        return None;
    }
    let flat_len = flat.len();

    let mut numeric = ExactNum::one();
    let mut radical_entries: Vec<(ExactNum, Node)> = Vec::new();
    let mut other_factors: Vec<Node> = Vec::new();

    for factor in flat {
        if let Node::Num(n) = factor {
            numeric = numeric * n;
            continue;
        }
        if let Some((coeff, radical)) = extract_radical_parts(&factor) {
            radical_entries.push((coeff, radical));
            continue;
        }
        other_factors.push(factor);
    }

    // Group by matching radical; keep per-entry coeffs for failure recovery.
    let mut groups: Vec<(Node, Vec<(ExactNum, Node)>)> = Vec::new();
    for (coeff, radical) in radical_entries {
        if let Some((_, entries)) = groups
            .iter_mut()
            .find(|(existing, _)| radicals_match(existing, &radical))
        {
            entries.push((coeff, radical));
        } else {
            groups.push((radical.clone(), vec![(coeff, radical)]));
        }
    }

    let mut rebuilt: Vec<Node> = other_factors;
    let mut any_pairs = false;

    for (template, entries) in groups {
        let count = entries.len() as u32;
        let pairs = count / 2;
        let remainder = count % 2;

        if pairs > 0 {
            let coeff_product = entries
                .iter()
                .fold(ExactNum::one(), |acc, (c, _)| acc * c.clone());

            match extract_sqrt_radicand(&template) {
                Some(radicand) => {
                    match collapse_sqrt_pair_product(radicand, pairs, coeff_product, env) {
                        Some(collapsed) => {
                            any_pairs = true;
                            rebuilt.push(collapsed);
                        }
                        None => {
                            for (c, r) in entries {
                                if let Some(term) = build_coeff_radical_term(c, r) {
                                    rebuilt.push(term);
                                }
                            }
                            continue;
                        }
                    }
                }
                None => {
                    for (c, r) in entries {
                        if let Some(term) = build_coeff_radical_term(c, r) {
                            rebuilt.push(term);
                        }
                    }
                    continue;
                }
            }
        } else {
            for (c, r) in entries {
                if let Some(term) = build_coeff_radical_term(c, r) {
                    rebuilt.push(term);
                }
            }
            continue;
        }

        if remainder > 0 {
            if let Some(term) = build_coeff_radical_term(ExactNum::one(), template) {
                rebuilt.push(term);
            }
        }
    }

    if !numeric.is_one() {
        rebuilt.insert(0, Node::Num(numeric));
    }

    let mut num_prod = ExactNum::one();
    let mut non_numeric = Vec::new();
    for factor in rebuilt {
        if let Node::Num(n) = factor {
            num_prod = num_prod * n;
        } else {
            non_numeric.push(factor);
        }
    }

    let mut factors = non_numeric;
    if !num_prod.is_one() {
        factors.insert(0, Node::Num(num_prod));
    }

    if !any_pairs && factors.len() >= flat_len {
        return None;
    }

    if factors.is_empty() {
        return Some(Node::Num(ExactNum::one()));
    }
    if factors.len() == 1 {
        return factors.pop();
    }

    let mut result = factors.remove(0);
    for factor in factors {
        result = Node::Multiply(Box::new(result), Box::new(factor));
    }
    Some(result.simplify(env).unwrap_or(result))
}

/// Combine like radicals in an Add/Subtract sum: a√X + b√X + … → (a+b+…)√X
fn try_combine_like_radicals(node: &Node, _env: &Environment) -> Option<Node> {
    let mut flat = Vec::new();
    flatten_add_sub_terms(node, &mut flat, false);

    let mut changed = false;
    let mut radical_groups: Vec<(Node, ExactNum)> = Vec::new();
    let mut other_terms: Vec<Node> = Vec::new();

    for (term, negative) in flat {
        if let Some((coeff, radical)) = extract_radical_parts(&term) {
            let signed_coeff = if negative { -coeff } else { coeff };
            if let Some((_, total)) = radical_groups
                .iter_mut()
                .find(|(existing, _)| radicals_match(existing, &radical))
            {
                *total = total.clone() + signed_coeff;
                changed = true;
            } else {
                radical_groups.push((radical, signed_coeff));
            }
        } else if negative {
            other_terms.push(Node::Negate(Box::new(term)));
        } else {
            other_terms.push(term);
        }
    }

    if !changed {
        return None;
    }

    let mut rebuilt = other_terms;
    for (radical, coeff) in radical_groups {
        if let Some(term) = build_coeff_radical_term(coeff, radical) {
            rebuilt.push(term);
        }
    }

    if rebuilt.is_empty() {
        return Some(Node::Num(ExactNum::zero()));
    }
    if rebuilt.len() == 1 {
        return rebuilt.pop();
    }

    let mut result = rebuilt.remove(0);
    for term in rebuilt {
        result = Node::Add(Box::new(result), Box::new(term));
    }
    Some(result)
}

/// Combine fractions with the same denominator: a/d ± b/d → (a±b)/d
fn try_combine_fractions(
    left: &Node,
    right: &Node,
    subtract: bool,
    env: &Environment,
) -> Option<Node> {
    let left_is_frac = matches!(left, Node::Divide(_, _));
    let right_is_frac = matches!(right, Node::Divide(_, _));
    if !left_is_frac && !right_is_frac {
        return None;
    }

    let (ln, ld) = match left {
        Node::Divide(n, d) => (n.as_ref().clone(), d.as_ref().clone()),
        _ => (left.clone(), Node::Num(ExactNum::one())),
    };
    let (rn, rd) = match right {
        Node::Divide(n, d) => (n.as_ref().clone(), d.as_ref().clone()),
        _ => (right.clone(), Node::Num(ExactNum::one())),
    };

    if ld == rd {
        let combined_num = if subtract {
            Node::Subtract(Box::new(ln), Box::new(rn))
        } else {
            Node::Add(Box::new(ln), Box::new(rn))
        };
        let simplified_num = combined_num.simplify(env).ok()?;
        let result = Node::Divide(Box::new(simplified_num), Box::new(ld));
        return result.simplify(env).ok();
    }

    let left_num = Node::Multiply(Box::new(ln), Box::new(rd.clone()));
    let right_num = Node::Multiply(Box::new(rn), Box::new(ld.clone()));
    let combined_num = if subtract {
        Node::Subtract(Box::new(left_num), Box::new(right_num))
    } else {
        Node::Add(Box::new(left_num), Box::new(right_num))
    };
    let combined_den = Node::Multiply(Box::new(ld), Box::new(rd));

    let simplified_num = combined_num.simplify(env).ok()?;
    let simplified_den = combined_den.simplify(env).ok()?;

    if let Node::Num(ref n) = simplified_den {
        if n.is_one() {
            return Some(simplified_num);
        }
    }

    let result = Node::Divide(Box::new(simplified_num), Box::new(simplified_den));
    result.simplify(env).ok()
}

/// Collect all multiplicative factors from a nested Multiply tree.
fn collect_multiply_factors(node: &Node, factors: &mut Vec<Node>) {
    if let Node::Multiply(left, right) = node {
        collect_multiply_factors(left, factors);
        collect_multiply_factors(right, factors);
    } else {
        factors.push(node.clone());
    }
}

fn rebuild_multiply_product(mut factors: Vec<Node>) -> Node {
    let mut result = factors.remove(0);
    for factor in factors {
        result = Node::Multiply(Box::new(result), Box::new(factor));
    }
    result
}

/// Try to simplify √(product) by extracting numeric square factors and even powers.
/// Returns Some(simplified) if any extraction succeeded, None otherwise.
fn try_simplify_sqrt_product(
    radicand: &Node,
    env: &Environment,
    use_node_sqrt: bool,
) -> Option<Node> {
    let mut factors = Vec::new();
    collect_multiply_factors(radicand, &mut factors);

    if factors.len() < 2 {
        return None;
    }

    let mut outside: Vec<Node> = Vec::new();
    let mut inside: Vec<Node> = Vec::new();
    let mut changed = false;

    for factor in &factors {
        match factor {
            Node::Num(n) => {
                if let Some(val) = n.to_i64() {
                    if val > 0 {
                        let (out, inn) = extract_square_factors(val as u64);
                        if out > 1 {
                            outside.push(Node::Num(ExactNum::integer(out as i64)));
                            changed = true;
                        }
                        if inn > 1 {
                            inside.push(Node::Num(ExactNum::integer(inn as i64)));
                        }
                        continue;
                    }
                }
                inside.push((*factor).clone());
            }
            Node::Power(base, exp) => {
                if let Node::Num(ref e) = **exp {
                    if let Some(exp_val) = e.to_i64() {
                        if exp_val >= 2 && exp_val % 2 == 0 {
                            let half_exp = exp_val / 2;
                            let base_factor = if half_exp == 1 {
                                if let Node::Variable(ref v) = **base {
                                    if env.assumptions().is_nonneg(v) {
                                        *base.clone()
                                    } else {
                                        Node::Abs(base.clone())
                                    }
                                } else {
                                    Node::Abs(base.clone())
                                }
                            } else {
                                Node::Power(
                                    base.clone(),
                                    Box::new(Node::Num(ExactNum::integer(half_exp))),
                                )
                            };
                            outside.push(base_factor);
                            changed = true;
                            continue;
                        }
                        if exp_val > 2 && exp_val % 2 == 1 {
                            let half = exp_val / 2;
                            if let Node::Variable(ref v) = **base {
                                if env.assumptions().is_nonneg(v) {
                                    if half == 1 {
                                        outside.push(*base.clone());
                                    } else {
                                        outside.push(Node::Power(
                                            base.clone(),
                                            Box::new(Node::Num(ExactNum::integer(half))),
                                        ));
                                    }
                                } else {
                                    let abs_base = Node::Abs(base.clone());
                                    if half == 1 {
                                        outside.push(abs_base);
                                    } else {
                                        outside.push(Node::Power(
                                            Box::new(abs_base),
                                            Box::new(Node::Num(ExactNum::integer(half))),
                                        ));
                                    }
                                }
                                inside.push(*base.clone());
                                changed = true;
                                continue;
                            }
                        }
                    }
                }
                inside.push((*factor).clone());
            }
            _ => {
                inside.push((*factor).clone());
            }
        }
    }

    if !changed {
        return None;
    }

    let outside_node = if outside.is_empty() {
        None
    } else {
        Some(
            outside
                .into_iter()
                .reduce(|a, b| Node::Multiply(Box::new(a), Box::new(b)))
                .unwrap(),
        )
    };

    let inside_node = if inside.is_empty() {
        None
    } else {
        let inner = inside
            .into_iter()
            .reduce(|a, b| Node::Multiply(Box::new(a), Box::new(b)))
            .unwrap();
        let sqrt_node = if use_node_sqrt {
            Node::Sqrt(Box::new(inner))
        } else {
            Node::Function("sqrt".to_string(), vec![inner])
        };
        Some(sqrt_node)
    };

    let result = match (outside_node, inside_node) {
        (Some(out), Some(sqrt)) => Node::Multiply(Box::new(out), Box::new(sqrt)),
        (Some(out), None) => out,
        (None, Some(sqrt)) => sqrt,
        (None, None) => Node::Num(ExactNum::one()),
    };

    let simplified = crate::simplify::Simplifiable::simplify(&result, env).unwrap_or(result);
    Some(simplified)
}

/// Build S_p(n) = Σ_{k=1}^{n} k^p as a Node tree using Faulhaber's formulas.
fn faulhaber_sum_node(p: usize, n: &Node) -> Option<Node> {
    let nb = || Box::new(n.clone());
    let np1 = || Box::new(Node::Add(nb(), Box::new(Node::Num(ExactNum::integer(1)))));
    let int = |v: i64| Box::new(Node::Num(ExactNum::integer(v)));

    match p {
        0 => Some(n.clone()),
        1 => Some(Node::Divide(Box::new(Node::Multiply(nb(), np1())), int(2))),
        2 => {
            let two_n_plus_1 = Box::new(Node::Add(Box::new(Node::Multiply(int(2), nb())), int(1)));
            Some(Node::Divide(
                Box::new(Node::Multiply(
                    Box::new(Node::Multiply(nb(), np1())),
                    two_n_plus_1,
                )),
                int(6),
            ))
        }
        3 => Some(Node::Divide(
            Box::new(Node::Multiply(
                Box::new(Node::Power(nb(), int(2))),
                Box::new(Node::Power(np1(), int(2))),
            )),
            int(4),
        )),
        4 => {
            let two_n_plus_1 = Box::new(Node::Add(Box::new(Node::Multiply(int(2), nb())), int(1)));
            let poly = Box::new(Node::Subtract(
                Box::new(Node::Add(
                    Box::new(Node::Multiply(int(3), Box::new(Node::Power(nb(), int(2))))),
                    Box::new(Node::Multiply(int(3), nb())),
                )),
                int(1),
            ));
            Some(Node::Divide(
                Box::new(Node::Multiply(
                    Box::new(Node::Multiply(
                        Box::new(Node::Multiply(nb(), np1())),
                        two_n_plus_1,
                    )),
                    poly,
                )),
                int(30),
            ))
        }
        _ => None,
    }
}

/// Evaluate a polynomial summation using Faulhaber's formulas.
/// Σ_{k=start}^{end} f(k) = Σ_j c_j · [S_j(end) − S_j(start−1)]
fn try_faulhaber_sum(
    poly: &Polynomial,
    start: &Node,
    end: &Node,
    env: &Environment,
) -> Option<Result<Node, String>> {
    let degree = poly.degree()?;
    if degree > 4 {
        return None;
    }

    let start_minus_1 = Node::Subtract(
        Box::new(start.clone()),
        Box::new(Node::Num(ExactNum::integer(1))),
    );

    let mut terms: Vec<Node> = Vec::new();

    for j in 0..=degree {
        let c_j = poly.coeff(j);
        if c_j.is_zero() {
            continue;
        }

        let s_j_end = faulhaber_sum_node(j, end)?;
        let s_j_start = faulhaber_sum_node(j, &start_minus_1)?;
        let diff = Node::Subtract(Box::new(s_j_end), Box::new(s_j_start));

        if c_j == BigRational::one() {
            terms.push(diff);
        } else {
            let coeff_node = Node::Num(ExactNum::Rational(c_j));
            terms.push(Node::Multiply(Box::new(coeff_node), Box::new(diff)));
        }
    }

    if terms.is_empty() {
        return Some(Ok(Node::Num(ExactNum::zero())));
    }

    let mut result = terms.remove(0);
    for term in terms {
        result = Node::Add(Box::new(result), Box::new(term));
    }

    Some(result.simplify(env))
}

/// Detect geometric pattern: coefficient · base^index_var.
fn detect_geometric(body: &Node, index_var: &str) -> Option<(Node, Node)> {
    match body {
        Node::Power(base, exp) => {
            if let Node::Variable(v) = exp.as_ref() {
                if v == index_var && !base.contains_variable(index_var) {
                    return Some((Node::Num(ExactNum::one()), *base.clone()));
                }
            }
            None
        }
        Node::Multiply(left, right) => {
            if let Some((inner_coeff, base)) = detect_geometric(right, index_var) {
                if !left.contains_variable(index_var) {
                    let coeff = Node::Multiply(Box::new(*left.clone()), Box::new(inner_coeff));
                    return Some((coeff, base));
                }
            }
            if let Some((inner_coeff, base)) = detect_geometric(left, index_var) {
                if !right.contains_variable(index_var) {
                    let coeff = Node::Multiply(Box::new(inner_coeff), Box::new(*right.clone()));
                    return Some((coeff, base));
                }
            }
            None
        }
        _ => None,
    }
}

/// Geometric series: Σ_{k=start}^{end} a·r^k = a·(r^{end+1} − r^{start})/(r − 1)
fn try_geometric_sum(
    index_var: &str,
    start: &Node,
    end: &Node,
    body: &Node,
    env: &Environment,
) -> Option<Result<Node, String>> {
    let (coeff, base) = detect_geometric(body, index_var)?;

    let end_plus_1 = Node::Add(
        Box::new(end.clone()),
        Box::new(Node::Num(ExactNum::integer(1))),
    );

    let numerator = Node::Subtract(
        Box::new(Node::Power(Box::new(base.clone()), Box::new(end_plus_1))),
        Box::new(Node::Power(Box::new(base.clone()), Box::new(start.clone()))),
    );
    let denominator = Node::Subtract(Box::new(base), Box::new(Node::Num(ExactNum::integer(1))));

    let result = Node::Multiply(
        Box::new(coeff),
        Box::new(Node::Divide(Box::new(numerator), Box::new(denominator))),
    );

    Some(result.simplify(env))
}

/// Telescoping: detect f(k) = g(k) − g(k±1) and collapse.
fn try_telescoping_sum(
    index_var: &str,
    start: &Node,
    end: &Node,
    body: &Node,
    env: &Environment,
) -> Option<Result<Node, String>> {
    if let Node::Subtract(ref left, ref right) = body {
        let k_plus_1 = Node::Add(
            Box::new(Node::Variable(index_var.to_string())),
            Box::new(Node::Num(ExactNum::integer(1))),
        );

        // Pattern: g(k) − g(k+1) → sum = g(start) − g(end+1)
        if let Ok(left_shifted) = crate::substitute::substitute_variable(left, index_var, &k_plus_1)
        {
            let ls = left_shifted.simplify(env).unwrap_or(left_shifted);
            let rs = right.simplify(env).unwrap_or(*right.clone());
            if ls == rs {
                let a_start =
                    crate::substitute::substitute_variable(left, index_var, start).ok()?;
                let end_plus_1 = Node::Add(
                    Box::new(end.clone()),
                    Box::new(Node::Num(ExactNum::integer(1))),
                );
                let a_end =
                    crate::substitute::substitute_variable(left, index_var, &end_plus_1).ok()?;
                let result = Node::Subtract(Box::new(a_start), Box::new(a_end));
                return Some(result.simplify(env));
            }
        }

        // Pattern: g(k+1) − g(k) → sum = g(end+1) − g(start)
        if let Ok(right_shifted) =
            crate::substitute::substitute_variable(right, index_var, &k_plus_1)
        {
            let rs = right_shifted.simplify(env).unwrap_or(right_shifted);
            let ls = left.simplify(env).unwrap_or(*left.clone());
            if rs == ls {
                let end_plus_1 = Node::Add(
                    Box::new(end.clone()),
                    Box::new(Node::Num(ExactNum::integer(1))),
                );
                let b_end =
                    crate::substitute::substitute_variable(right, index_var, &end_plus_1).ok()?;
                let b_start =
                    crate::substitute::substitute_variable(right, index_var, start).ok()?;
                let result = Node::Subtract(Box::new(b_end), Box::new(b_start));
                return Some(result.simplify(env));
            }
        }
    }

    None
}

/// Collect additive terms, flattening Add/Subtract into a list.
/// Subtracted terms are wrapped in Negate.
fn collect_additive_terms(node: &Node) -> Vec<Node> {
    match node {
        Node::Add(left, right) => {
            let mut terms = collect_additive_terms(left);
            terms.extend(collect_additive_terms(right));
            terms
        }
        Node::Subtract(left, right) => {
            let mut terms = collect_additive_terms(left);
            let right_terms = collect_additive_terms(right);
            for t in right_terms {
                terms.push(Node::Negate(Box::new(t)));
            }
            terms
        }
        _ => vec![node.clone()],
    }
}

/// Separate a multiplicative expression into (symbolic_coeff, index_part)
/// where symbolic_coeff does not contain index_var.
fn separate_index_factors(node: &Node, index_var: &str) -> (Node, Node) {
    match node {
        Node::Multiply(left, right) => {
            let (l_coeff, l_index) = separate_index_factors(left, index_var);
            let (r_coeff, r_index) = separate_index_factors(right, index_var);
            let coeff = mul_nodes(l_coeff, r_coeff);
            let index = mul_nodes(l_index, r_index);
            (coeff, index)
        }
        Node::Negate(inner) => {
            let (coeff, index) = separate_index_factors(inner, index_var);
            (Node::Negate(Box::new(coeff)), index)
        }
        Node::Divide(num, den) if !den.contains_variable(index_var) => {
            let (n_coeff, n_index) = separate_index_factors(num, index_var);
            (Node::Divide(Box::new(n_coeff), den.clone()), n_index)
        }
        _ => {
            if node.contains_variable(index_var) {
                (Node::Num(ExactNum::one()), node.clone())
            } else {
                (node.clone(), Node::Num(ExactNum::one()))
            }
        }
    }
}

/// Multiply two nodes, simplifying away multiplication by 1.
fn mul_nodes(a: Node, b: Node) -> Node {
    let a_is_one = matches!(&a, Node::Num(n) if n.is_one());
    let b_is_one = matches!(&b, Node::Num(n) if n.is_one());
    if a_is_one {
        b
    } else if b_is_one {
        a
    } else {
        Node::Multiply(Box::new(a), Box::new(b))
    }
}

/// Symbolic coefficient summation: Σ (a·k² + b·k) → a·Faulhaber(k²) + b·Faulhaber(k).
/// Handles bodies where each additive term is a symbolic coefficient times a polynomial in the index var.
fn try_symbolic_coeff_summation(
    index_var: &str,
    start: &Node,
    end: &Node,
    body: &Node,
    env: &Environment,
) -> Option<Result<Node, String>> {
    let terms = collect_additive_terms(body);
    if terms.is_empty() {
        return None;
    }

    let mut result_terms: Vec<Node> = Vec::new();
    let mut any_symbolic = false;

    for term in &terms {
        let (coeff, index_part) = separate_index_factors(term, index_var);

        // If the coefficient has no symbolic content, and the index part is
        // the whole term, the pure polynomial path would have handled it already
        let coeff_is_one = matches!(&coeff, Node::Num(n) if n.is_one());
        if !coeff_is_one {
            any_symbolic = true;
        }

        let poly = Polynomial::from_node(&index_part, index_var).ok()?;
        let faulhaber_result = try_faulhaber_sum(&poly, start, end, env)?;
        let faulhaber_node = match faulhaber_result {
            Ok(node) => node,
            Err(e) => return Some(Err(e)),
        };

        let term_result = mul_nodes(coeff, faulhaber_node);
        result_terms.push(term_result);
    }

    if !any_symbolic || result_terms.is_empty() {
        return None;
    }

    let mut result = result_terms.remove(0);
    for term in result_terms {
        result = Node::Add(Box::new(result), Box::new(term));
    }

    Some(result.simplify(env))
}

/// Telescoping via partial fractions: decompose a rational body like 1/(k(k+1))
/// into partial fractions, then check if the result telescopes.
/// The PF body is NOT simplified — the simplifier would recombine fractions.
fn try_telescoping_via_partial_fractions(
    index_var: &str,
    start: &Node,
    end: &Node,
    body: &Node,
    env: &Environment,
) -> Option<Result<Node, String>> {
    use crate::partial_fractions::partial_fraction_decomposition;

    let (num_node, den_node) = match body {
        Node::Divide(n, d) => (n.as_ref(), d.as_ref()),
        _ => return None,
    };

    let num_poly = Polynomial::from_node(num_node, index_var).ok()?;
    let den_poly = Polynomial::from_node(den_node, index_var).ok()?;

    if den_poly.degree()? < 2 {
        return None;
    }

    let decomp = partial_fraction_decomposition(&num_poly, &den_poly).ok()?;

    if !decomp.polynomial_part.is_zero() || decomp.terms.len() < 2 {
        return None;
    }

    // Build PF body with Add/Subtract structure (not simplification-safe Add-of-negatives).
    // Terms with negative numerators become Subtract nodes so the telescoping detector sees them.
    let mut pf_body: Option<Node> = None;

    for term in &decomp.terms {
        let den = if term.power == 1 {
            term.denominator.to_node()
        } else {
            Node::Power(
                Box::new(term.denominator.to_node()),
                Box::new(Node::Num(ExactNum::integer(term.power as i64))),
            )
        };

        let is_negative = term
            .numerator
            .leading_coeff()
            .is_some_and(|c| c < &BigRational::zero());

        let (abs_num, subtract) = if is_negative {
            ((-&term.numerator).to_node(), true)
        } else {
            (term.numerator.to_node(), false)
        };

        let frac = Node::Divide(Box::new(abs_num), Box::new(den));

        pf_body = Some(match pf_body {
            None => {
                if subtract {
                    Node::Negate(Box::new(frac))
                } else {
                    frac
                }
            }
            Some(existing) => {
                if subtract {
                    Node::Subtract(Box::new(existing), Box::new(frac))
                } else {
                    Node::Add(Box::new(existing), Box::new(frac))
                }
            }
        });
    }

    let pf_body = pf_body?;

    try_telescoping_sum(index_var, start, end, &pf_body, env)
}

/// Try to find a symbolic closed form for a summation.
fn try_symbolic_summation(
    index_var: &str,
    start: &Node,
    end: &Node,
    body: &Node,
    env: &Environment,
) -> Option<Result<Node, String>> {
    // Constant body: Σ_{k=a}^{b} c = c · (b − a + 1)
    if !body.contains_variable(index_var) {
        let count = Node::Add(
            Box::new(Node::Subtract(
                Box::new(end.clone()),
                Box::new(start.clone()),
            )),
            Box::new(Node::Num(ExactNum::integer(1))),
        );
        let result = Node::Multiply(Box::new(body.clone()), Box::new(count));
        return Some(result.simplify(env));
    }

    // Polynomial body: Faulhaber's formulas
    if let Ok(poly) = Polynomial::from_node(body, index_var) {
        if let Some(result) = try_faulhaber_sum(&poly, start, end, env) {
            return Some(result);
        }
    }

    // Symbolic coefficient summation: Σ a·k² + b·k → a·Faulhaber(k²) + b·Faulhaber(k)
    if let Some(result) = try_symbolic_coeff_summation(index_var, start, end, body, env) {
        return Some(result);
    }

    // Geometric series: a · r^k
    if let Some(result) = try_geometric_sum(index_var, start, end, body, env) {
        return Some(result);
    }

    // Telescoping sum
    if let Some(result) = try_telescoping_sum(index_var, start, end, body, env) {
        return Some(result);
    }

    // Telescoping via partial fractions: 1/(k(k+1)) → PF → 1/k - 1/(k+1) → telescoping
    if let Some(result) = try_telescoping_via_partial_fractions(index_var, start, end, body, env) {
        return Some(result);
    }

    None
}

/// Geometric product: ∏_{k=start}^{end} a·r^k = a^count · r^{(start+end)·count/2}
fn try_geometric_product(
    index_var: &str,
    start: &Node,
    end: &Node,
    body: &Node,
    env: &Environment,
) -> Option<Result<Node, String>> {
    let (coeff, base) = detect_geometric(body, index_var)?;

    let count = Node::Add(
        Box::new(Node::Subtract(
            Box::new(end.clone()),
            Box::new(start.clone()),
        )),
        Box::new(Node::Num(ExactNum::integer(1))),
    );
    let coeff_power = Node::Power(Box::new(coeff), Box::new(count.clone()));
    let exponent_sum = Node::Divide(
        Box::new(Node::Multiply(
            Box::new(Node::Add(Box::new(start.clone()), Box::new(end.clone()))),
            Box::new(count),
        )),
        Box::new(Node::Num(ExactNum::integer(2))),
    );
    let result = Node::Multiply(
        Box::new(coeff_power),
        Box::new(Node::Power(Box::new(base), Box::new(exponent_sum))),
    );

    Some(result.simplify(env))
}

/// ∏_{k=1}^{m} k as a product node (used for closed forms like ∏(2k-1)).
fn factorial_product_node(index_var: &str, upper: Node) -> Node {
    Node::Product(
        index_var.to_string(),
        Box::new(Node::Num(ExactNum::one())),
        Box::new(upper),
        Box::new(Node::Variable(index_var.to_string())),
    )
}

/// ∏_{k=1}^{n} (2k-1) = (2n)! / (2^n · n!) — analogue of Σ(2k-1) = n².
fn try_product_of_odd_numbers(
    index_var: &str,
    start: &Node,
    end: &Node,
    body: &Node,
    env: &Environment,
) -> Option<Result<Node, String>> {
    if !matches!(start, Node::Num(n) if n.is_one()) {
        return None;
    }

    let poly = Polynomial::from_node(body, index_var).ok()?;
    if poly.degree()? != 1 {
        return None;
    }
    if poly.coeff(1) != BigRational::from_integer(2.into())
        || poly.coeff(0) != BigRational::from_integer(BigInt::from(-1))
    {
        return None;
    }

    if let Node::Num(end_n) = end {
        if end_n.is_integer() {
            let n = end_n.to_f64() as i64;
            if n < 1 {
                return Some(Ok(Node::Num(ExactNum::one())));
            }
            let mut product = ExactNum::one();
            for k in 1..=n {
                product = product * ExactNum::integer(2 * k - 1);
            }
            return Some(Ok(Node::Num(product)));
        }
    }

    let two_n = Node::Multiply(
        Box::new(Node::Num(ExactNum::integer(2))),
        Box::new(end.clone()),
    );
    let result = Node::Divide(
        Box::new(factorial_product_node("i", two_n)),
        Box::new(Node::Multiply(
            Box::new(Node::Power(
                Box::new(Node::Num(ExactNum::integer(2))),
                Box::new(end.clone()),
            )),
            Box::new(factorial_product_node("j", end.clone())),
        )),
    );
    Some(result.simplify(env))
}

/// Try to find a symbolic closed form for a product.
fn try_symbolic_product(
    index_var: &str,
    start: &Node,
    end: &Node,
    body: &Node,
    env: &Environment,
) -> Option<Result<Node, String>> {
    // Constant body: ∏_{k=a}^{b} c = c^(b − a + 1)
    if !body.contains_variable(index_var) {
        let count = Node::Add(
            Box::new(Node::Subtract(
                Box::new(end.clone()),
                Box::new(start.clone()),
            )),
            Box::new(Node::Num(ExactNum::integer(1))),
        );
        let result = Node::Power(Box::new(body.clone()), Box::new(count));
        return Some(result.simplify(env));
    }

    // ∏_{k=1}^{n} k when body is the index variable
    if let Node::Variable(v) = body {
        if v == index_var {
            if let Node::Num(start_n) = start {
                if start_n == &ExactNum::one() {
                    if let Node::Num(end_n) = end {
                        if end_n.is_integer() {
                            let end_i = end_n.to_f64() as i64;
                            if end_i >= 0 {
                                let mut fact = ExactNum::one();
                                for i in 2..=end_i {
                                    fact = fact * ExactNum::integer(i);
                                }
                                return Some(Ok(Node::Num(fact)));
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some(result) = try_product_of_odd_numbers(index_var, start, end, body, env) {
        return Some(result);
    }

    if let Some(result) = try_geometric_product(index_var, start, end, body, env) {
        return Some(result);
    }

    None
}

#[cfg(test)]
mod tests {
    fn simplify_latex(input: &str) -> String {
        use super::Simplifiable;
        use crate::environment::Environment;
        use crate::parser::build_expression_tree;
        use crate::tokenizer::Tokenizer;
        let mut tok = Tokenizer::new(input);
        let expr = build_expression_tree(tok.tokenize()).unwrap();
        let env = Environment::new();
        format!("{}", expr.simplify(&env).unwrap())
    }

    // --- Circular trigonometric ---
    #[test]
    fn test_sin_zero_is_zero() {
        assert_eq!(simplify_latex("\\sin(0)"), "0");
    }

    #[test]
    fn test_cos_zero_is_one() {
        assert_eq!(simplify_latex("\\cos(0)"), "1");
    }

    #[test]
    fn test_tan_zero_is_zero() {
        assert_eq!(simplify_latex("\\tan(0)"), "0");
    }

    #[test]
    fn test_sin_pi_is_zero() {
        assert_eq!(simplify_latex("\\sin(\\pi)"), "0");
    }

    #[test]
    fn test_cos_pi_is_neg_one() {
        assert_eq!(simplify_latex("\\cos(\\pi)"), "-1");
    }

    #[test]
    fn test_sin_pi_2_is_one() {
        assert_eq!(simplify_latex("\\sin(\\frac{\\pi}{2})"), "1");
    }

    #[test]
    fn test_cos_pi_2_is_zero() {
        assert_eq!(simplify_latex("\\cos(\\frac{\\pi}{2})"), "0");
    }

    #[test]
    fn test_sin_2pi_is_zero() {
        assert_eq!(simplify_latex("\\sin(2\\pi)"), "0");
    }

    #[test]
    fn test_cos_2pi_is_one() {
        assert_eq!(simplify_latex("\\cos(2\\pi)"), "1");
    }

    #[test]
    fn test_tan_pi_4_is_one() {
        assert_eq!(simplify_latex("\\tan(\\frac{\\pi}{4})"), "1");
    }

    #[test]
    fn test_sin_pi_6_is_half() {
        assert_eq!(simplify_latex("\\sin(\\frac{\\pi}{6})"), "\\frac{1}{2}");
    }

    #[test]
    fn test_sin_pi_8() {
        let result = simplify_latex("\\sin(\\frac{\\pi}{8})");
        assert!(
            result.contains("sqrt"),
            "sin(pi/8) should be exact sqrt form, got: {}",
            result
        );
    }

    #[test]
    fn test_cos_pi_12() {
        let result = simplify_latex("\\cos(\\frac{\\pi}{12})");
        assert!(
            result.contains("sqrt"),
            "cos(pi/12) should be exact sqrt form, got: {}",
            result
        );
    }

    #[test]
    fn test_cos_pi_3_is_half() {
        assert_eq!(simplify_latex("\\cos(\\frac{\\pi}{3})"), "\\frac{1}{2}");
    }

    #[test]
    fn test_sin_pi_4_is_sqrt2_over_2() {
        assert_eq!(
            simplify_latex("\\sin(\\frac{\\pi}{4})"),
            "\\frac{\\sqrt{2}}{2}"
        );
    }

    #[test]
    fn test_two_pi_over_8_displays_as_pi_over_4() {
        assert_eq!(simplify_latex("\\frac{2\\pi}{8}"), "\\frac{\\pi}{4}");
        assert_eq!(
            simplify_latex("\\sin(\\frac{2\\pi}{8})"),
            "\\frac{\\sqrt{2}}{2}"
        );
    }

    #[test]
    fn test_exact_trig_pi_fifths() {
        assert_eq!(
            simplify_latex("\\sin(\\frac{\\pi}{5})"),
            "\\frac{\\sqrt{10 - 2\\sqrt{5}}}{4}"
        );
        assert_eq!(
            simplify_latex("\\cos(\\frac{\\pi}{5})"),
            "\\frac{\\sqrt{5} + 1}{4}"
        );
        assert_eq!(
            simplify_latex("\\sin(\\frac{\\pi}{10})"),
            "\\frac{\\sqrt{5} - 1}{4}"
        );
        assert_eq!(
            simplify_latex("\\sin(\\frac{2\\pi}{5})"),
            "\\frac{\\sqrt{10 + 2\\sqrt{5}}}{4}"
        );
        assert_eq!(
            simplify_latex("\\tan(\\frac{\\pi}{5})"),
            "\\sqrt{5 - 2\\sqrt{5}}"
        );
    }

    // Quadrant/period sign correctness (angles outside the principal range).
    #[test]
    fn test_sin_7pi_6_is_neg_half() {
        // Quadrant III: sin(7π/6) = −1/2
        assert_eq!(simplify_latex("\\sin(\\frac{7\\pi}{6})"), "-\\frac{1}{2}");
    }

    #[test]
    fn test_cos_7pi_6_is_neg_sqrt3_over_2() {
        // Quadrant III: cos(7π/6) = −√3/2
        assert_eq!(
            simplify_latex("\\cos(\\frac{7\\pi}{6})"),
            "-\\frac{\\sqrt{3}}{2}"
        );
    }

    #[test]
    fn test_cos_2pi_3_is_neg_half() {
        // Quadrant II: cos(2π/3) = −1/2
        assert_eq!(simplify_latex("\\cos(\\frac{2\\pi}{3})"), "-\\frac{1}{2}");
    }

    #[test]
    fn test_sin_11pi_6_is_neg_half() {
        // Quadrant IV: sin(11π/6) = −1/2
        assert_eq!(simplify_latex("\\sin(\\frac{11\\pi}{6})"), "-\\frac{1}{2}");
    }

    #[test]
    fn test_cos_11pi_12_is_negative() {
        // Quadrant II: cos(11π/12) = −(√6+√2)/4
        assert_eq!(
            simplify_latex("\\cos(\\frac{11\\pi}{12})"),
            "-\\frac{\\sqrt{6} + \\sqrt{2}}{4}"
        );
    }

    #[test]
    fn test_sin_5pi_2_is_one() {
        // Coterminal with π/2 (5π/2 = 2π + π/2): sin = 1
        assert_eq!(simplify_latex("\\sin(\\frac{5\\pi}{2})"), "1");
    }

    #[test]
    fn test_sin_3pi_2_is_neg_one() {
        assert_eq!(simplify_latex("\\sin(\\frac{3\\pi}{2})"), "-1");
    }

    #[test]
    fn test_tan_3pi_4_is_neg_one() {
        // tan(π − x) = −tan(x): tan(3π/4) = −1
        assert_eq!(simplify_latex("\\tan(\\frac{3\\pi}{4})"), "-1");
    }

    #[test]
    fn test_tan_5pi_4_is_one() {
        // tan has period π: tan(5π/4) = tan(π/4) = 1
        assert_eq!(simplify_latex("\\tan(\\frac{5\\pi}{4})"), "1");
    }

    #[test]
    fn test_sin_neg_pi_6_is_neg_half() {
        assert_eq!(simplify_latex("\\sin(-\\frac{\\pi}{6})"), "-\\frac{1}{2}");
    }

    #[test]
    fn test_tan_pi_6_is_one_over_sqrt3() {
        assert_eq!(
            simplify_latex("\\tan(\\frac{\\pi}{6})"),
            "\\frac{1}{\\sqrt{3}}"
        );
    }

    #[test]
    fn test_tan_pi_12() {
        let r = simplify_latex("\\tan(\\frac{\\pi}{12})");
        assert!(r.contains("2") && r.contains("\\sqrt{3}"), "got {}", r);
    }

    #[test]
    fn test_arcsin_sqrt2_over_2_is_pi_4() {
        assert_eq!(
            simplify_latex("\\arcsin(\\frac{\\sqrt{2}}{2})"),
            "\\frac{\\pi}{4}"
        );
    }

    #[test]
    fn test_arcsin_sqrt3_over_2_is_pi_3() {
        assert_eq!(
            simplify_latex("\\arcsin(\\frac{\\sqrt{3}}{2})"),
            "\\frac{\\pi}{3}"
        );
    }

    #[test]
    fn test_arccos_neg_half_is_two_pi_thirds() {
        assert_eq!(
            simplify_latex("\\arccos(-\\frac{1}{2})"),
            "\\frac{2\\pi}{3}"
        );
    }

    #[test]
    fn test_arccos_sqrt3_over_2_is_pi_6() {
        assert_eq!(
            simplify_latex("\\arccos(\\frac{\\sqrt{3}}{2})"),
            "\\frac{\\pi}{6}"
        );
    }

    #[test]
    fn test_arctan_one_over_sqrt3_is_pi_6() {
        assert_eq!(
            simplify_latex("\\arctan(\\frac{1}{\\sqrt{3}})"),
            "\\frac{\\pi}{6}"
        );
    }

    #[test]
    fn test_arctan_sqrt3_is_pi_3() {
        assert_eq!(simplify_latex("\\arctan(\\sqrt{3})"), "\\frac{\\pi}{3}");
    }

    #[test]
    fn test_log_power_ten() {
        assert_eq!(simplify_latex("\\log(10^x)"), "x");
    }

    #[test]
    fn test_log_product() {
        assert_eq!(simplify_latex("\\log(2 \\cdot 3)"), "\\log(2) + \\log(3)");
    }

    #[test]
    fn test_log_quotient() {
        assert_eq!(
            simplify_latex("\\log(\\frac{30}{5})"),
            "\\log(2) + \\log(3)"
        );
    }

    #[test]
    fn test_log_composite_factorization() {
        assert_eq!(simplify_latex("\\log(50)"), "\\log(2) + 2\\log(5)");
    }

    #[test]
    fn test_lg_power_two() {
        assert_eq!(simplify_latex("\\lg(2^x)"), "x");
    }

    #[test]
    fn test_lg_product() {
        assert_eq!(simplify_latex("\\lg(3 \\cdot 5)"), "\\lg(3) + \\lg(5)");
    }

    #[test]
    fn test_lg_x_squared_uses_abs() {
        assert_eq!(simplify_latex("\\lg(x^2)"), "2\\lg(|x|)");
    }

    // --- BigInt log, exp powers, inverse hyperbolic surds ---
    #[test]
    fn test_exp_ln_two_is_two() {
        assert_eq!(simplify_latex("\\exp(\\ln(2))"), "2");
    }

    #[test]
    fn test_exp_two_ln_three_is_nine() {
        assert_eq!(simplify_latex("\\exp(2 \\cdot \\ln(3))"), "9");
    }

    #[test]
    fn test_log_ten_to_the_twenty() {
        assert_eq!(simplify_latex("\\log(100000000000000000000)"), "20");
    }

    #[test]
    fn test_arcsinh_half() {
        let r = simplify_latex("\\arcsinh(\\frac{1}{2})");
        assert!(r.contains("ln") && r.contains("5"), "got {}", r);
    }

    #[test]
    fn test_arccosh_three() {
        let r = simplify_latex("\\arccosh(3)");
        assert!(r.contains("ln") && r.contains("2"), "got {}", r);
    }

    #[test]
    fn test_arctanh_inv_sqrt3() {
        assert_eq!(
            simplify_latex("\\arctanh(\\frac{1}{\\sqrt{3}})"),
            "\\frac{1}{2} \\cdot \\ln(2 + \\sqrt{3})"
        );
    }

    #[test]
    fn test_sin_pi_twelfth_via_unreduced_angle() {
        // 2π/24 = π/12
        assert_eq!(
            simplify_latex("\\sin(\\frac{2\\pi}{24})"),
            simplify_latex("\\sin(\\frac{\\pi}{12})")
        );
    }

    // --- Parity rules and unified inverse surd exact values ---
    #[test]
    fn test_csc_neg_pi_6_is_neg_two() {
        assert_eq!(simplify_latex("\\csc(-\\frac{\\pi}{6})"), "-2");
    }

    #[test]
    fn test_sec_neg_pi_3_is_two() {
        assert_eq!(simplify_latex("\\sec(-\\frac{\\pi}{3})"), "2");
    }

    #[test]
    fn test_cot_neg_pi_4_is_neg_one() {
        assert_eq!(simplify_latex("\\cot(-\\frac{\\pi}{4})"), "-1");
    }

    #[test]
    fn test_csch_neg_ln_two_is_neg_four_thirds() {
        assert_eq!(simplify_latex("\\csch(-\\ln(2))"), "-\\frac{4}{3}");
    }

    #[test]
    fn test_sech_neg_x_is_sech_x() {
        assert_eq!(simplify_latex("\\sech(-x)"), "\\sech(x)");
    }

    #[test]
    fn test_arcsin_neg_x_is_neg_arcsin_x() {
        assert_eq!(simplify_latex("\\arcsin(-x)"), "-\\arcsin(x)");
    }

    #[test]
    fn test_arccos_neg_x_is_pi_minus_arccos_x() {
        assert_eq!(simplify_latex("\\arccos(-x)"), "\\pi - \\arccos(x)");
    }

    #[test]
    fn test_arccsc_sqrt2_is_pi_4() {
        assert_eq!(simplify_latex("\\arccsc(\\sqrt{2})"), "\\frac{\\pi}{4}");
    }

    #[test]
    fn test_arcsec_sqrt2_is_pi_4() {
        assert_eq!(simplify_latex("\\arcsec(\\sqrt{2})"), "\\frac{\\pi}{4}");
    }

    #[test]
    fn test_arccot_sqrt3_is_pi_6() {
        assert_eq!(simplify_latex("\\arccot(\\sqrt{3})"), "\\frac{\\pi}{6}");
    }

    #[test]
    fn test_arccot_inv_sqrt3_is_pi_3() {
        assert_eq!(
            simplify_latex("\\arccot(\\frac{1}{\\sqrt{3}})"),
            "\\frac{\\pi}{3}"
        );
    }

    #[test]
    fn test_arcsin_inv_sqrt2_is_pi_4() {
        assert_eq!(
            simplify_latex("\\arcsin(\\frac{1}{\\sqrt{2}})"),
            "\\frac{\\pi}{4}"
        );
    }

    // --- Reciprocal trigonometric ---
    #[test]
    fn test_csc_pi_2_is_one() {
        assert_eq!(simplify_latex("\\csc(\\frac{\\pi}{2})"), "1");
    }

    #[test]
    fn test_sec_zero_is_one() {
        assert_eq!(simplify_latex("\\sec(0)"), "1");
    }

    #[test]
    fn test_cot_pi_4_is_one() {
        assert_eq!(simplify_latex("\\cot(\\frac{\\pi}{4})"), "1");
    }

    // --- Inverse circular trigonometric ---
    #[test]
    fn test_arctan_zero_is_zero() {
        assert_eq!(simplify_latex("\\arctan(0)"), "0");
    }

    #[test]
    fn test_arctan_one_is_pi_4() {
        assert_eq!(simplify_latex("\\arctan(1)"), "\\frac{\\pi}{4}");
    }

    #[test]
    fn test_arccos_zero_is_pi_2() {
        assert_eq!(simplify_latex("\\arccos(0)"), "\\frac{\\pi}{2}");
    }

    #[test]
    fn test_arccos_one_is_zero() {
        assert_eq!(simplify_latex("\\arccos(1)"), "0");
    }

    #[test]
    fn test_arccos_neg_one_is_pi() {
        assert_eq!(simplify_latex("\\arccos(-1)"), "\\pi");
    }

    #[test]
    fn test_sin_inv_one_is_pi_2() {
        assert_eq!(simplify_latex("\\sin^{-1}(1)"), "\\frac{\\pi}{2}");
    }

    #[test]
    fn test_sin_inv_unbraced_is_arcsin() {
        assert_eq!(simplify_latex("\\sin^-1(1)"), "\\frac{\\pi}{2}");
    }

    #[test]
    fn test_sin_neg_one_point_zero_is_power_not_arcsin() {
        let result = simplify_latex("\\sin^{-1.0}(x)");
        assert!(
            result.contains("\\sin") && !result.contains("arcsin"),
            "sin^{{-1.0}}(x) should be a power, not arcsin, got: {result}"
        );
    }

    #[test]
    fn test_sin_neg_two_is_power() {
        assert_eq!(simplify_latex("\\sin^{-2}(x)"), "\\sin(x)^{-2}");
    }

    // --- Inverse reciprocal trigonometric ---
    #[test]
    fn test_arccsc_two_is_pi_6() {
        assert_eq!(simplify_latex("\\arccsc(2)"), "\\frac{\\pi}{6}");
    }

    #[test]
    fn test_arcsec_two_is_pi_3() {
        assert_eq!(simplify_latex("\\arcsec(2)"), "\\frac{\\pi}{3}");
    }

    #[test]
    fn test_arccot_zero_is_pi_2() {
        assert_eq!(simplify_latex("\\arccot(0)"), "\\frac{\\pi}{2}");
    }

    #[test]
    fn test_arccot_neg_one_is_neg_pi_4() {
        assert_eq!(simplify_latex("\\arccot(-1)"), "-\\frac{\\pi}{4}");
    }

    // --- Reciprocal hyperbolic ---
    #[test]
    fn test_sech_zero_is_one() {
        assert_eq!(simplify_latex("\\sech(0)"), "1");
    }

    // --- Hyperbolic exact (ln and inverse) ---
    #[test]
    fn test_sinh_ln_two_is_three_quarters() {
        assert_eq!(simplify_latex("\\sinh(\\ln(2))"), "\\frac{3}{4}");
    }

    #[test]
    fn test_cosh_ln_three_is_five_thirds() {
        assert_eq!(simplify_latex("\\cosh(\\ln(3))"), "\\frac{5}{3}");
    }

    #[test]
    fn test_tanh_ln_two_is_three_fifths() {
        assert_eq!(simplify_latex("\\tanh(\\ln(2))"), "\\frac{3}{5}");
    }

    #[test]
    fn test_csch_ln_two_is_four_thirds() {
        assert_eq!(simplify_latex("\\csch(\\ln(2))"), "\\frac{4}{3}");
    }

    #[test]
    fn test_arcsinh_one_is_ln_one_plus_sqrt2() {
        assert_eq!(simplify_latex("\\arcsinh(1)"), "\\ln(1 + \\sqrt{2})");
    }

    #[test]
    fn test_arccosh_two_is_ln_two_plus_sqrt3() {
        assert_eq!(simplify_latex("\\arccosh(2)"), "\\ln(2 + \\sqrt{3})");
    }

    #[test]
    fn test_arctanh_half_is_half_ln_three() {
        assert_eq!(
            simplify_latex("\\arctanh(\\frac{1}{2})"),
            "\\frac{1}{2} \\cdot \\ln(3)"
        );
    }

    #[test]
    fn test_arctanh_third_is_half_ln_two() {
        assert_eq!(
            simplify_latex("\\arctanh(\\frac{1}{3})"),
            "\\frac{1}{2} \\cdot \\ln(2)"
        );
    }

    // --- Inverse hyperbolic ---
    #[test]
    fn test_arccosh_one_is_zero() {
        assert_eq!(simplify_latex("\\arccosh(1)"), "0");
    }

    // --- Inverse reciprocal hyperbolic ---
    #[test]
    fn test_arcsech_one_is_zero() {
        assert_eq!(simplify_latex("\\arcsech(1)"), "0");
    }

    // --- Logarithmic and exponential ---
    #[test]
    fn test_log_one_is_zero() {
        assert_eq!(simplify_latex("\\log(1)"), "0");
    }

    #[test]
    fn test_log_ten_is_one() {
        assert_eq!(simplify_latex("\\log(10)"), "1");
    }

    #[test]
    fn test_log_hundred_is_two() {
        assert_eq!(simplify_latex("\\log(100)"), "2");
    }

    #[test]
    fn test_lg_two_is_one() {
        assert_eq!(simplify_latex("\\lg(2)"), "1");
    }

    #[test]
    fn test_lg_eight_is_three() {
        assert_eq!(simplify_latex("\\lg(8)"), "3");
    }

    #[test]
    fn test_ln_one_is_zero() {
        assert_eq!(simplify_latex("\\ln(1)"), "0");
    }

    #[test]
    fn test_lg_one_is_zero() {
        assert_eq!(simplify_latex("\\lg(1)"), "0");
    }

    #[test]
    fn test_ln_exp_x_is_x() {
        assert_eq!(simplify_latex("\\ln(\\exp(x))"), "x");
    }

    #[test]
    fn test_exp_ln_x_is_x() {
        assert_eq!(simplify_latex("\\exp(\\ln(x))"), "x");
    }

    #[test]
    fn test_exp_log_x_stays_symbolic() {
        assert_eq!(simplify_latex("\\exp(\\log(x))"), "\\exp(\\log(x))");
    }

    #[test]
    fn test_exp_lg_x_stays_symbolic() {
        assert_eq!(simplify_latex("\\exp(\\lg(x))"), "\\exp(\\lg(x))");
    }

    #[test]
    fn test_exp_log_ten_folds_inner_log_only() {
        // exp(log(10)) = exp(1) = e; must not cancel to 10 (exp is e^x, not 10^x).
        assert_eq!(simplify_latex("\\exp(\\log(10))"), "\\exp(1)");
    }

    #[test]
    fn test_ln_x_squared_uses_abs() {
        assert_eq!(simplify_latex("\\ln(x^2)"), "2\\ln(|x|)");
    }

    #[test]
    fn test_ln_neg_x_squared_uses_abs() {
        assert_eq!(simplify_latex("\\ln((-x)^2)"), "2\\ln(|x|)");
    }

    #[test]
    fn test_ln_e_power_x_is_x() {
        assert_eq!(simplify_latex("\\ln(e^x)"), "x");
    }

    #[test]
    fn test_pi_displays_as_latex() {
        assert_eq!(simplify_latex("\\pi"), "\\pi");
    }

    #[test]
    fn test_pi_times_two() {
        assert_eq!(simplify_latex("2\\pi"), "2\\pi");
    }

    #[test]
    fn test_combine_numeric_coeff_functions() {
        assert_eq!(simplify_latex("3\\exp(x) + 5\\exp(x)"), "8\\exp(x)");
    }

    #[test]
    fn test_combine_symbolic_coeff_functions() {
        assert_eq!(
            simplify_latex("a \\cdot \\sin(x) + b \\cdot \\sin(x)"),
            "(a + b) \\cdot \\sin(x)"
        );
    }

    #[test]
    fn test_combine_rational_coeff_functions() {
        let r = simplify_latex("\\frac{-1}{x} \\cdot \\exp(x) + \\frac{\\exp(x)}{x^2}");
        assert!(r.contains("\\exp(x)"), "Should factor out exp(x): {}", r);
        assert_eq!(
            r.matches("\\exp").count(),
            1,
            "exp(x) should appear exactly once (factored out): {}",
            r
        );
    }

    #[test]
    fn test_subtract_same_function() {
        assert_eq!(simplify_latex("\\sin(x) - \\sin(x)"), "0");
    }

    #[test]
    fn test_combine_function_subtract() {
        assert_eq!(simplify_latex("5\\cos(x) - 3\\cos(x)"), "2\\cos(x)");
    }
}
