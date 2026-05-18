use crate::exact::ExactNum;
use crate::node::Node;
use crate::parser::build_expression_tree;
use crate::polynomial::Polynomial;
use crate::tokenizer::Tokenizer;
use num_traits::{One, ToPrimitive, Zero};

pub fn integrate(expr: &Node, var_name: &str) -> Result<Node, String> {
    let env = crate::environment::Environment::new();
    let expr =
        &crate::simplify::Simplifiable::simplify(expr, &env).unwrap_or_else(|_| expr.clone());

    if let Ok(poly) = Polynomial::from_node(expr, var_name) {
        return Ok(poly.integral().to_node());
    }

    match expr {
        // Constants: ∫k dx = k*x + C
        Node::Num(k) => {
            if k.is_zero() {
                // ∫0 dx = 0 + C, but we'll just return 0
                Ok(Node::Num(ExactNum::zero()))
            } else {
                // ∫k dx = k*x + C
                Ok(Node::Multiply(
                    Box::new(Node::Num(k.clone())),
                    Box::new(Node::Variable(var_name.to_string())),
                ))
            }
        }

        // Variables: ∫x dx = x²/2 + C, ∫y dx = y*x + C (if y != x)
        Node::Variable(name) => {
            if name == var_name {
                // ∫x dx = x²/2 + C
                let x_squared = Node::Power(
                    Box::new(Node::Variable(name.clone())),
                    Box::new(Node::Num(ExactNum::from_f64(2.0))),
                );

                Ok(Node::Divide(
                    Box::new(x_squared),
                    Box::new(Node::Num(ExactNum::from_f64(2.0))),
                ))
            } else {
                // ∫y dx = y*x + C (y is a constant with respect to x)
                Ok(Node::Multiply(
                    Box::new(Node::Variable(name.clone())),
                    Box::new(Node::Variable(var_name.to_string())),
                ))
            }
        }

        // Addition: ∫(f+g) dx = ∫f dx + ∫g dx
        Node::Add(left, right) => {
            let left_integral = integrate(left, var_name)?;
            let right_integral = integrate(right, var_name)?;

            Ok(Node::Add(Box::new(left_integral), Box::new(right_integral)))
        }

        // Subtraction: ∫(f-g) dx = ∫f dx - ∫g dx
        Node::Subtract(left, right) => {
            let left_integral = integrate(left, var_name)?;
            let right_integral = integrate(right, var_name)?;

            Ok(Node::Subtract(
                Box::new(left_integral),
                Box::new(right_integral),
            ))
        }

        // Power of a variable: ∫x^n dx = x^(n+1)/(n+1) + C (if n ≠ -1)
        Node::Power(base, exponent) => {
            if let Node::Variable(base_var) = &**base {
                if base_var == var_name {
                    if let Node::Num(n) = &**exponent {
                        let new_exp = n.clone() + ExactNum::one();
                        if new_exp.to_f64().abs() < 1e-10 {
                            // Special case: n = -1, integral is ln|x|
                            return Ok(Node::Function(
                                "ln".to_string(),
                                vec![Node::Abs(Box::new(Node::Variable(var_name.to_string())))],
                            ));
                        } else {
                            // Standard power rule: ∫x^n dx = x^(n+1)/(n+1) + C
                            let new_power = Node::Power(
                                Box::new(Node::Variable(var_name.to_string())),
                                Box::new(Node::Num(new_exp.clone())),
                            );

                            return Ok(Node::Divide(
                                Box::new(new_power),
                                Box::new(Node::Num(new_exp)),
                            ));
                        }
                    } else if let Node::Negate(inner_exp) = &**exponent {
                        // Handle x^(-n) forms
                        if let Node::Num(n) = &**inner_exp {
                            if n.is_one() {
                                // Special case: x^(-1) = 1/x, integral is ln|x|
                                return Ok(Node::Function(
                                    "ln".to_string(),
                                    vec![Node::Abs(Box::new(Node::Variable(var_name.to_string())))],
                                ));
                            } else {
                                // Standard power rule with negative exponent: ∫x^(-n) dx = x^(-n+1)/(-n+1) + C
                                let new_exp = ExactNum::one() - n.clone();
                                let new_power = Node::Power(
                                    Box::new(Node::Variable(var_name.to_string())),
                                    Box::new(Node::Num(new_exp.clone())),
                                );

                                return Ok(Node::Divide(
                                    Box::new(new_power),
                                    Box::new(Node::Num(new_exp)),
                                ));
                            }
                        }
                    }
                }
            }

            // ∫a^x dx = a^x / ln(a) where a is a constant
            if let Node::Num(a) = &**base {
                if let Node::Variable(v) = &**exponent {
                    if v == var_name {
                        let a_to_x = Node::Power(
                            Box::new(Node::Num(a.clone())),
                            Box::new(Node::Variable(var_name.to_string())),
                        );
                        let ln_a = Node::Function("ln".to_string(), vec![Node::Num(a.clone())]);
                        return Ok(Node::Divide(Box::new(a_to_x), Box::new(ln_a)));
                    }
                }
            }

            // ∫sin^n(x) dx, ∫cos^n(x) dx via half-angle / reduction
            if let Some(result) = try_trig_power_integral(base, exponent, var_name) {
                return result;
            }

            // Try u-substitution on power expressions
            if let Some(result) = try_u_substitution(expr, var_name) {
                return result;
            }

            Err("Integration of this expression is not yet implemented".to_string())
        }

        // Multiplication by a constant: ∫(k*f) dx = k*∫f dx
        Node::Multiply(left, right) => {
            if let Node::Num(k) = &**left {
                // Factor out the constant k
                let right_integral = integrate(right, var_name)?;
                return Ok(Node::Multiply(
                    Box::new(Node::Num(k.clone())),
                    Box::new(right_integral),
                ));
            } else if let Node::Num(k) = &**right {
                // Factor out the constant k
                let left_integral = integrate(left, var_name)?;
                return Ok(Node::Multiply(
                    Box::new(Node::Num(k.clone())),
                    Box::new(left_integral),
                ));
            }

            // Integration by parts via tabular method for polynomial × {sin, cos, exp}
            if let Some(result) = try_tabular_integration(left, right, var_name) {
                return result;
            }
            if let Some(result) = try_tabular_integration(right, left, var_name) {
                return result;
            }

            // Single-step IBP for polynomial × ln: u=ln(...), dv=polynomial
            if let Some(result) = try_log_integration(left, right, var_name) {
                return result;
            }
            if let Some(result) = try_log_integration(right, left, var_name) {
                return result;
            }

            // ∫sin^m(x)·cos^n(x) dx — mixed trig power products
            if let Some(result) = try_trig_product_integral(expr, var_name) {
                return result;
            }

            // U-substitution: f(g(x)) · g'(x) patterns
            if let Some(result) = try_u_substitution(expr, var_name) {
                return result;
            }

            Err("Integration of this product is not yet implemented".to_string())
        }

        // Division: Special case for 1/x
        Node::Divide(left, right) => {
            if let (Node::Num(k), Node::Variable(var)) = (&**left, &**right) {
                if k.is_one() && var == var_name {
                    // ∫(1/x) dx = ln|x|
                    return Ok(Node::Function(
                        "ln".to_string(),
                        vec![Node::Abs(Box::new(Node::Variable(var_name.to_string())))],
                    ));
                }
            }

            // ∫1/(ax+b) dx = (1/a)·ln|ax+b| for linear denominators
            if let Ok(den_poly) = Polynomial::from_node(right, var_name) {
                if den_poly.degree() == Some(1) {
                    if let Ok(num_poly) = Polynomial::from_node(left, var_name) {
                        if num_poly.degree().unwrap_or(0) == 0 && !num_poly.is_zero() {
                            let k = num_poly.coeff(0); // numerator constant
                            let a = den_poly.coeff(1); // coefficient of x
                            let coeff = k / a;
                            let ln_term = Node::Function(
                                "ln".to_string(),
                                vec![Node::Abs(Box::new(right.as_ref().clone()))],
                            );
                            if coeff == num_rational::BigRational::one() {
                                return Ok(ln_term);
                            } else {
                                let coeff_node = Node::Num(ExactNum::rational(
                                    coeff.numer().to_i64().unwrap_or(0),
                                    coeff.denom().to_i64().unwrap_or(1),
                                ));
                                return Ok(Node::Multiply(
                                    Box::new(coeff_node),
                                    Box::new(ln_term),
                                ));
                            }
                        }
                    }
                }
            }

            // ∫1/(1+x²) dx = arctan(x), ∫1/√(1-x²) dx = arcsin(x)
            if let Node::Num(ref n) = **left {
                if n.is_one() {
                    if let Some(result) = try_inverse_trig_integral(right, var_name) {
                        return Ok(result);
                    }
                }
            }

            // ∫k/f(x) dx = k * ∫(1/f(x)) dx — factor out constant numerator
            if let Node::Num(k) = &**left {
                if !k.is_one() {
                    let one_over_right =
                        Node::Divide(Box::new(Node::Num(ExactNum::one())), right.clone());
                    if let Ok(inner) = integrate(&one_over_right, var_name) {
                        return Ok(Node::Multiply(
                            Box::new(Node::Num(k.clone())),
                            Box::new(inner),
                        ));
                    }
                }
            }
            // ∫f(x)/k dx = (1/k) * ∫f(x) dx — factor out constant denominator
            if let Node::Num(k) = &**right {
                if !k.is_zero() {
                    let inner = integrate(left, var_name)?;
                    let inv = ExactNum::one() / k.clone();
                    return Ok(Node::Multiply(Box::new(Node::Num(inv)), Box::new(inner)));
                }
            }

            // U-substitution on division expressions
            if let Some(result) = try_u_substitution(expr, var_name) {
                return result;
            }

            // Partial fraction decomposition for rational functions
            if let Some(result) = try_partial_fraction_integration(left, right, var_name) {
                return result;
            }

            Err("Integration of this division is not yet implemented".to_string())
        }

        Node::Negate(inner) => {
            let inner_integral = integrate(inner, var_name)?;
            Ok(Node::Negate(Box::new(inner_integral)))
        }

        // Sqrt node: try trig substitution for √(quadratic)
        Node::Sqrt(inner) => {
            if let Some(result) = try_trig_substitution_sqrt(inner, var_name) {
                return result;
            }
            Err("Integration of this sqrt expression is not yet implemented".to_string())
        }

        // Standard function integrals
        Node::Function(name, args) if args.len() == 1 => {
            let arg = &args[0];
            // Only handle direct variable argument for now
            if let Node::Variable(v) = arg {
                if v == var_name {
                    return integrate_standard_function(name, var_name);
                }
            }
            // Try linear substitution: f(ax+b) where a is constant
            if let Some((a, _b)) = extract_linear_arg(arg, var_name) {
                let base_integral = integrate_standard_function(name, var_name)?;
                let inv_a =
                    Node::Divide(Box::new(Node::Num(ExactNum::one())), Box::new(Node::Num(a)));
                return Ok(Node::Multiply(Box::new(inv_a), Box::new(base_integral)));
            }
            // sqrt(quadratic) → trig substitution
            if name == "sqrt" {
                if let Some(result) = try_trig_substitution_sqrt(arg, var_name) {
                    return result;
                }
            }
            // Try u-substitution on the full expression (may help with composed functions)
            let full_expr = Node::Function(name.clone(), args.clone());
            if let Some(result) = try_u_substitution(&full_expr, var_name) {
                return result;
            }
            Err(format!(
                "Integration of {}(...) with non-linear argument not yet implemented",
                name
            ))
        }

        _ => Err("Integration of this expression is not yet implemented".to_string()),
    }
}

fn integrate_standard_function(name: &str, var: &str) -> Result<Node, String> {
    let x = || Node::Variable(var.to_string());
    match name {
        // ∫sin(x) = -cos(x)
        "sin" => Ok(Node::Negate(Box::new(Node::Function(
            "cos".to_string(),
            vec![x()],
        )))),
        // ∫cos(x) = sin(x)
        "cos" => Ok(Node::Function("sin".to_string(), vec![x()])),
        // ∫tan(x) = -ln|cos(x)|
        "tan" => Ok(Node::Negate(Box::new(Node::Function(
            "ln".to_string(),
            vec![Node::Abs(Box::new(Node::Function(
                "cos".to_string(),
                vec![x()],
            )))],
        )))),
        // ∫sec²(x) — handled if it comes through as sec*sec; skip for now
        // ∫sec(x)  = ln|sec(x) + tan(x)|
        "sec" => Ok(Node::Function(
            "ln".to_string(),
            vec![Node::Abs(Box::new(Node::Add(
                Box::new(Node::Function("sec".to_string(), vec![x()])),
                Box::new(Node::Function("tan".to_string(), vec![x()])),
            )))],
        )),
        // ∫csc(x) = -ln|csc(x) + cot(x)|
        "csc" => Ok(Node::Negate(Box::new(Node::Function(
            "ln".to_string(),
            vec![Node::Abs(Box::new(Node::Add(
                Box::new(Node::Function("csc".to_string(), vec![x()])),
                Box::new(Node::Function("cot".to_string(), vec![x()])),
            )))],
        )))),
        // ∫cot(x) = ln|sin(x)|
        "cot" => Ok(Node::Function(
            "ln".to_string(),
            vec![Node::Abs(Box::new(Node::Function(
                "sin".to_string(),
                vec![x()],
            )))],
        )),
        // ∫exp(x) = exp(x)
        "exp" => Ok(Node::Function("exp".to_string(), vec![x()])),
        // ∫ln(x) = x·ln(x) - x
        "ln" => Ok(Node::Subtract(
            Box::new(Node::Multiply(
                Box::new(x()),
                Box::new(Node::Function("ln".to_string(), vec![x()])),
            )),
            Box::new(x()),
        )),
        // ∫sinh(x) = cosh(x)
        "sinh" => Ok(Node::Function("cosh".to_string(), vec![x()])),
        // ∫cosh(x) = sinh(x)
        "cosh" => Ok(Node::Function("sinh".to_string(), vec![x()])),
        // ∫tanh(x) = ln(cosh(x))
        "tanh" => Ok(Node::Function(
            "ln".to_string(),
            vec![Node::Function("cosh".to_string(), vec![x()])],
        )),
        _ => Err(format!("Integration of {}(x) not implemented", name)),
    }
}

/// Extract (a, b) if the expression is of the form a*var + b (linear in var).
fn extract_linear_arg(expr: &Node, var: &str) -> Option<(ExactNum, ExactNum)> {
    match expr {
        Node::Variable(v) if v == var => Some((ExactNum::one(), ExactNum::zero())),
        Node::Multiply(left, right) => {
            if let (Node::Num(a), Node::Variable(v)) = (&**left, &**right) {
                if v == var {
                    return Some((a.clone(), ExactNum::zero()));
                }
            }
            if let (Node::Variable(v), Node::Num(a)) = (&**left, &**right) {
                if v == var {
                    return Some((a.clone(), ExactNum::zero()));
                }
            }
            None
        }
        Node::Add(left, right) => {
            if let Some((a, b1)) = extract_linear_arg(left, var) {
                if let Node::Num(b2) = &**right {
                    return Some((a, &b1 + b2));
                }
            }
            if let Some((a, b1)) = extract_linear_arg(right, var) {
                if let Node::Num(b2) = &**left {
                    return Some((a, &b1 + b2));
                }
            }
            None
        }
        _ => None,
    }
}

/// Tabular integration by parts for polynomial × {sin, cos, exp}.
/// `u_candidate` is tested as the polynomial side, `dv_candidate` as the
/// transcendental side. Returns None if the pattern doesn't match.
///
/// Algorithm: repeatedly differentiate u (until 0) and integrate dv,
/// then combine with alternating signs:
///   ∫u·dv = u·V₁ - u'·V₂ + u''·V₃ - ...
/// where Vₖ is the k-th iterated integral of dv.
/// Returns true if the expression is suitable as the "dv" side of tabular
/// integration — a function whose repeated integrals stay bounded in complexity.
fn is_repeatedly_integratable(expr: &Node, var: &str) -> bool {
    match expr {
        Node::Function(name, args) if args.len() == 1 => {
            matches!(name.as_str(), "sin" | "cos" | "exp" | "sinh" | "cosh")
                && is_linear_in_var(&args[0], var)
        }
        // e^x (parsed as Power with Euler's number base)
        Node::Power(base, exp) => {
            if let Node::Num(b) = &**base {
                if (b.to_f64() - std::f64::consts::E).abs() < 1e-10 {
                    return contains_var(exp, var);
                }
            }
            false
        }
        Node::Multiply(left, right) => {
            // k * f(x) where f is repeatedly integratable
            (matches!(&**left, Node::Num(_)) && is_repeatedly_integratable(right, var))
                || (matches!(&**right, Node::Num(_)) && is_repeatedly_integratable(left, var))
        }
        Node::Negate(inner) => is_repeatedly_integratable(inner, var),
        _ => false,
    }
}

fn is_linear_in_var(expr: &Node, var: &str) -> bool {
    match expr {
        Node::Variable(v) => v == var,
        Node::Multiply(left, right) => {
            (matches!(&**left, Node::Num(_)) && is_linear_in_var(right, var))
                || (matches!(&**right, Node::Num(_)) && is_linear_in_var(left, var))
        }
        Node::Add(left, right) | Node::Subtract(left, right) => {
            (is_linear_in_var(left, var) && !contains_var(right, var))
                || (!contains_var(left, var) && is_linear_in_var(right, var))
        }
        Node::Negate(inner) => is_linear_in_var(inner, var),
        _ => false,
    }
}

fn contains_var(expr: &Node, var: &str) -> bool {
    match expr {
        Node::Variable(v) => v == var,
        Node::Add(l, r)
        | Node::Subtract(l, r)
        | Node::Multiply(l, r)
        | Node::Divide(l, r)
        | Node::Power(l, r) => contains_var(l, var) || contains_var(r, var),
        Node::Negate(inner) | Node::Sqrt(inner) | Node::Abs(inner) => contains_var(inner, var),
        Node::Function(_, args) => args.iter().any(|a| contains_var(a, var)),
        _ => false,
    }
}

fn try_tabular_integration(
    u_candidate: &Node,
    dv_candidate: &Node,
    var: &str,
) -> Option<Result<Node, String>> {
    // u must be polynomial, dv must be repeatedly integratable (sin/cos/exp)
    if Polynomial::from_node(u_candidate, var).is_err() {
        return None;
    }
    if !is_repeatedly_integratable(dv_candidate, var) {
        return None;
    }

    let env = crate::environment::Environment::new();
    let mut u = crate::simplify::Simplifiable::simplify(u_candidate, &env)
        .unwrap_or_else(|_| u_candidate.clone());
    let mut v_integral = match integrate(dv_candidate, var) {
        Ok(v) => v,
        Err(_) => return None,
    };

    let mut terms: Vec<Node> = Vec::new();
    let mut positive = true;

    for _ in 0..20 {
        // Simplify v_integral
        v_integral =
            crate::simplify::Simplifiable::simplify(&v_integral, &env).unwrap_or(v_integral);

        // Term: ±u · V
        let term = Node::Multiply(Box::new(u.clone()), Box::new(v_integral.clone()));
        if positive {
            terms.push(term);
        } else {
            terms.push(Node::Negate(Box::new(term)));
        }

        // Differentiate u
        let du = match crate::derivative::differentiate(&u, var) {
            Ok(d) => crate::simplify::Simplifiable::simplify(&d, &env).unwrap_or(d),
            Err(e) => return Some(Err(e)),
        };

        // If derivative is zero, we're done
        if matches!(&du, Node::Num(n) if n.is_zero()) {
            break;
        }

        // Integrate v_integral one more time
        v_integral = match integrate(&v_integral, var) {
            Ok(v) => v,
            Err(e) => return Some(Err(e)),
        };

        u = du;
        positive = !positive;
    }

    if terms.is_empty() {
        return Some(Ok(Node::Num(ExactNum::zero())));
    }

    let mut result = terms.remove(0);
    for term in terms {
        result = Node::Add(Box::new(result), Box::new(term));
    }

    Some(Ok(result))
}

/// Single-step integration by parts for ln(x) × polynomial.
/// Uses u = ln(x), dv = polynomial. Result: uv - ∫v·du.
fn try_log_integration(
    log_candidate: &Node,
    poly_candidate: &Node,
    var: &str,
) -> Option<Result<Node, String>> {
    // Check log_candidate is ln(x) or similar
    let is_log = match log_candidate {
        Node::Function(name, args) if args.len() == 1 => {
            matches!(name.as_str(), "ln" | "log") && contains_var(&args[0], var)
        }
        _ => false,
    };
    if !is_log {
        return None;
    }

    // Check poly_candidate is a polynomial
    if Polynomial::from_node(poly_candidate, var).is_err() {
        return None;
    }

    let env = crate::environment::Environment::new();

    // u = ln_candidate, dv = poly_candidate
    // du = d/dx(ln_candidate)
    // v = ∫poly_candidate dx
    let du = match crate::derivative::differentiate(log_candidate, var) {
        Ok(d) => crate::simplify::Simplifiable::simplify(&d, &env).unwrap_or(d),
        Err(e) => return Some(Err(e)),
    };
    let v = match integrate(poly_candidate, var) {
        Ok(i) => crate::simplify::Simplifiable::simplify(&i, &env).unwrap_or(i),
        Err(e) => return Some(Err(e)),
    };

    // uv - ∫v·du
    let uv = Node::Multiply(Box::new(log_candidate.clone()), Box::new(v.clone()));

    // v·du — simplify aggressively, try polynomial path
    let v_du = Node::Multiply(Box::new(v), Box::new(du));
    let v_du_str = format!("{}", v_du);
    let v_du_reparsed = {
        let mut tok = crate::tokenizer::Tokenizer::new(&v_du_str);
        let toks = tok.tokenize();
        crate::parser::build_expression_tree(toks)
            .ok()
            .and_then(|e| crate::simplify::Simplifiable::simplify(&e, &env).ok())
            .unwrap_or_else(|| crate::simplify::Simplifiable::simplify(&v_du, &env).unwrap_or(v_du))
    };

    let remaining = match integrate(&v_du_reparsed, var) {
        Ok(r) => crate::simplify::Simplifiable::simplify(&r, &env).unwrap_or(r),
        Err(e) => return Some(Err(e)),
    };

    Some(Ok(Node::Subtract(Box::new(uv), Box::new(remaining))))
}

/// Extract (function_name, argument, exponent) from a trig power like sin^n(x).
/// Returns None if the node isn't a trig power.
fn extract_trig_power(node: &Node) -> Option<(&str, &Node, u32)> {
    if let Node::Power(base, exp) = node {
        if let Node::Function(name, args) = base.as_ref() {
            if args.len() == 1 && matches!(name.as_str(), "sin" | "cos") {
                if let Node::Num(n) = exp.as_ref() {
                    if let Some(e) = n.to_i64() {
                        if e >= 1 {
                            return Some((name.as_str(), &args[0], e as u32));
                        }
                    }
                }
            }
        }
    }
    // sin(x) alone is sin^1(x)
    if let Node::Function(name, args) = node {
        if args.len() == 1 && matches!(name.as_str(), "sin" | "cos") {
            return Some((name.as_str(), &args[0], 1));
        }
    }
    None
}

/// Build sin^n(x) or cos^n(x) as a Node.
fn trig_power_node(func: &str, arg: &Node, n: u32) -> Node {
    let f = Node::Function(func.to_string(), vec![arg.clone()]);
    if n == 1 {
        f
    } else {
        Node::Power(
            Box::new(f),
            Box::new(Node::Num(ExactNum::integer(n as i64))),
        )
    }
}

/// Integrate sin^n(x) or cos^n(x).
///
/// Even n: use half-angle identity repeatedly.
///   sin²(x) = (1 - cos(2x))/2
///   cos²(x) = (1 + cos(2x))/2
///
/// Odd n: peel off one factor, convert rest via Pythagorean identity.
///   ∫sin³(x)dx = ∫(1-cos²(x))·sin(x)dx → u = cos(x)
///   ∫cos³(x)dx = ∫(1-sin²(x))·cos(x)dx → u = sin(x)
fn try_trig_power_integral(
    base: &Node,
    exponent: &Node,
    var: &str,
) -> Option<Result<Node, String>> {
    let power_node = Node::Power(Box::new(base.clone()), Box::new(exponent.clone()));
    let (func, arg, n) = extract_trig_power(&power_node)?;

    // Only handle direct variable argument
    if !matches!(arg, Node::Variable(v) if v == var) {
        return None;
    }

    Some(integrate_trig_power(func, var, n))
}

fn integrate_trig_power(func: &str, var: &str, n: u32) -> Result<Node, String> {
    let env = crate::environment::Environment::new();
    let x = || Node::Variable(var.to_string());

    if n == 0 {
        return Ok(Node::Variable(var.to_string()));
    }
    if n == 1 {
        return integrate_standard_function(func, var);
    }

    if n.is_multiple_of(2) {
        // Even power: half-angle reduction
        // sin²(x) = (1 - cos(2x))/2, cos²(x) = (1 + cos(2x))/2
        // Apply to one pair, leaving sin^(n-2) or cos^(n-2) to recurse on.
        // For sin^2k: expand as ((1-cos(2x))/2)^k and integrate.
        // Simpler: use the reduction formula:
        //   ∫sin^n(x)dx = -sin^(n-1)(x)·cos(x)/n + (n-1)/n · ∫sin^(n-2)(x)dx
        //   ∫cos^n(x)dx = cos^(n-1)(x)·sin(x)/n + (n-1)/n · ∫cos^(n-2)(x)dx
        let n_i = n as i64;
        let (other_func, sign) = if func == "sin" {
            ("cos", ExactNum::integer(-1))
        } else {
            ("sin", ExactNum::one())
        };

        // First term: ±trig^(n-1)(x) · other_trig(x) / n
        let first_term = Node::Divide(
            Box::new(Node::Multiply(
                Box::new(Node::Num(sign)),
                Box::new(Node::Multiply(
                    Box::new(trig_power_node(func, &x(), n - 1)),
                    Box::new(Node::Function(other_func.to_string(), vec![x()])),
                )),
            )),
            Box::new(Node::Num(ExactNum::integer(n_i))),
        );

        // Second term: (n-1)/n · ∫trig^(n-2)(x) dx
        let sub_integral = integrate_trig_power(func, var, n - 2)?;
        let second_term = Node::Multiply(
            Box::new(Node::Divide(
                Box::new(Node::Num(ExactNum::integer(n_i - 1))),
                Box::new(Node::Num(ExactNum::integer(n_i))),
            )),
            Box::new(sub_integral),
        );

        let result = Node::Add(Box::new(first_term), Box::new(second_term));
        Ok(crate::simplify::Simplifiable::simplify(&result, &env).unwrap_or(result))
    } else {
        // Odd power: peel off one factor, convert rest via Pythagorean identity
        // ∫sin^(2k+1)(x)dx = ∫(1-cos²(x))^k · sin(x) dx → u = cos(x)
        // ∫cos^(2k+1)(x)dx = ∫(1-sin²(x))^k · cos(x) dx → u = sin(x)
        let k = (n - 1) / 2;
        let (u_func, du_sign) = if func == "sin" {
            ("cos", -1i64) // du = -sin(x)dx → sin(x)dx = -du
        } else {
            ("sin", 1i64) // du = cos(x)dx
        };

        // Expand (1 - u²)^k as a polynomial in u, then integrate each term
        let u_var = "_u_";
        let one_minus_u2 = Node::Subtract(
            Box::new(Node::Num(ExactNum::one())),
            Box::new(Node::Power(
                Box::new(Node::Variable(u_var.to_string())),
                Box::new(Node::Num(ExactNum::two())),
            )),
        );

        // Build (1 - u²)^k
        let mut integrand = if k == 0 {
            Node::Num(ExactNum::one())
        } else if k == 1 {
            one_minus_u2.clone()
        } else {
            Node::Power(
                Box::new(one_minus_u2),
                Box::new(Node::Num(ExactNum::integer(k as i64))),
            )
        };

        // Simplify (expands the polynomial)
        integrand = crate::simplify::Simplifiable::simplify(&integrand, &env).unwrap_or(integrand);

        // Integrate with respect to u
        let integral_in_u = integrate(&integrand, u_var)?;

        // Back-substitute u = cos(x) or u = sin(x)
        let back_sub = Node::Function(u_func.to_string(), vec![Node::Variable(var.to_string())]);
        let result = crate::substitute::substitute_variable(&integral_in_u, u_var, &back_sub)?;

        // Multiply by the du sign factor (-1 for sin case)
        let result = if du_sign == -1 {
            Node::Negate(Box::new(result))
        } else {
            result
        };

        Ok(crate::simplify::Simplifiable::simplify(&result, &env).unwrap_or(result))
    }
}

/// Integrate products sin^m(x) · cos^n(x).
fn try_trig_product_integral(expr: &Node, var: &str) -> Option<Result<Node, String>> {
    let env = crate::environment::Environment::new();

    // Flatten factors and identify trig powers
    let mut factors = Vec::new();
    collect_factors(expr, &mut factors);

    let mut sin_power: u32 = 0;
    let mut cos_power: u32 = 0;
    let mut trig_arg: Option<Node> = None;
    let mut other_factors: Vec<Node> = Vec::new();

    for f in &factors {
        if let Some((func, arg, n)) = extract_trig_power(f) {
            if !matches!(arg, Node::Variable(v) if v == var) {
                return None;
            }
            match trig_arg {
                None => trig_arg = Some(arg.clone()),
                Some(ref existing) if existing == arg => {}
                _ => return None, // different arguments
            }
            match func {
                "sin" => sin_power += n,
                "cos" => cos_power += n,
                _ => return None,
            }
        } else {
            other_factors.push(f.clone());
        }
    }

    if sin_power == 0 && cos_power == 0 {
        return None;
    }

    // Need at least two trig factors to be a "product" (single powers handled elsewhere)
    if sin_power == 0 || cos_power == 0 {
        return None;
    }

    // Strategy: if one exponent is odd, peel from that side
    let result = if cos_power % 2 == 1 {
        // cos is odd: peel one cos, convert rest to sin
        // ∫sin^m · cos^(2k+1) dx = ∫sin^m · (1-sin²)^k · cos dx → u = sin(x)
        let k = (cos_power - 1) / 2;
        integrate_mixed_odd("sin", sin_power, k, var)
    } else if sin_power % 2 == 1 {
        // sin is odd: peel one sin, convert rest to cos
        // ∫sin^(2k+1) · cos^n dx = ∫(1-cos²)^k · cos^n · sin dx → u = cos(x)
        let k = (sin_power - 1) / 2;
        integrate_mixed_odd("cos", cos_power, k, var)
    } else {
        // Both even: Pythagorean expansion + existing reduction.
        // Convert the smaller power via sin² = 1-cos² (or cos² = 1-sin²),
        // expand via binomial theorem, integrate each pure power term.
        let (keep_func, expand_half, keep_half) = if sin_power <= cos_power {
            ("cos", sin_power / 2, cos_power / 2)
        } else {
            ("sin", cos_power / 2, sin_power / 2)
        };
        integrate_mixed_even(keep_func, expand_half, keep_half, var)
    };

    match result {
        Ok(trig_result) => {
            if other_factors.is_empty() {
                Some(Ok(crate::simplify::Simplifiable::simplify(
                    &trig_result,
                    &env,
                )
                .unwrap_or(trig_result)))
            } else {
                // Multiply back the non-trig constant factors
                let mut product = other_factors.remove(0);
                for f in other_factors {
                    product = Node::Multiply(Box::new(product), Box::new(f));
                }
                let full = Node::Multiply(Box::new(product), Box::new(trig_result));
                Some(Ok(
                    crate::simplify::Simplifiable::simplify(&full, &env).unwrap_or(full)
                ))
            }
        }
        Err(e) => Some(Err(e)),
    }
}

/// Integrate sin^m(x)·cos^(2k+1)(x)dx or cos^n(x)·sin^(2k+1)(x)dx
/// by peeling one trig factor and converting the rest via Pythagorean identity.
///
/// `u_func` is the function that becomes u (e.g., "sin" when cos is odd).
/// `u_power` is the existing power of u_func.
/// `k` is the number of squared pairs to convert: (1-u²)^k.
fn integrate_mixed_odd(u_func: &str, u_power: u32, k: u32, var: &str) -> Result<Node, String> {
    let env = crate::environment::Environment::new();
    let u_var = "_u_";

    let du_sign = if u_func == "sin" { 1i64 } else { -1i64 };

    // Build u^m · (1 - u²)^k in terms of u
    let u_to_m = if u_power == 0 {
        Node::Num(ExactNum::one())
    } else if u_power == 1 {
        Node::Variable(u_var.to_string())
    } else {
        Node::Power(
            Box::new(Node::Variable(u_var.to_string())),
            Box::new(Node::Num(ExactNum::integer(u_power as i64))),
        )
    };

    let one_minus_u2 = Node::Subtract(
        Box::new(Node::Num(ExactNum::one())),
        Box::new(Node::Power(
            Box::new(Node::Variable(u_var.to_string())),
            Box::new(Node::Num(ExactNum::two())),
        )),
    );

    let pythagorean_part = if k == 0 {
        Node::Num(ExactNum::one())
    } else if k == 1 {
        one_minus_u2
    } else {
        Node::Power(
            Box::new(one_minus_u2),
            Box::new(Node::Num(ExactNum::integer(k as i64))),
        )
    };

    let mut integrand = Node::Multiply(Box::new(u_to_m), Box::new(pythagorean_part));
    integrand = crate::simplify::Simplifiable::simplify(&integrand, &env).unwrap_or(integrand);

    let integral_in_u = integrate(&integrand, u_var)?;

    let back_sub = Node::Function(u_func.to_string(), vec![Node::Variable(var.to_string())]);
    let result = crate::substitute::substitute_variable(&integral_in_u, u_var, &back_sub)?;

    let result = if du_sign == -1 {
        Node::Negate(Box::new(result))
    } else {
        result
    };

    Ok(crate::simplify::Simplifiable::simplify(&result, &env).unwrap_or(result))
}

/// Integrate sin^(2p)(x)·cos^(2q)(x) when both exponents are even.
///
/// Converts the smaller power via Pythagorean identity, expands via
/// binomial theorem, and integrates each resulting pure power term.
///
/// Example: ∫sin²(x)·cos²(x)dx with p=1, q=1, keep=cos:
///   sin²(x) = (1-cos²(x))^1 = 1 - cos²(x)
///   ∫(1 - cos²(x))·cos²(x)dx = ∫cos²(x)dx - ∫cos⁴(x)dx
fn integrate_mixed_even(
    keep_func: &str,
    expand_half: u32,
    keep_half: u32,
    var: &str,
) -> Result<Node, String> {
    let env = crate::environment::Environment::new();
    let p = expand_half as u64;
    let q = keep_half;

    // (1 - t²)^p = Σ C(p,j)·(-1)^j·t^(2j)
    // Multiply by t^(2q): Σ C(p,j)·(-1)^j · t^(2j+2q)
    // Integrate term by term via existing reduction formula.
    let mut terms: Vec<Node> = Vec::new();

    for j in 0..=p {
        let binom = binomial_coeff(p, j) as i64;
        let coeff = if j % 2 == 0 { binom } else { -binom };
        let power = 2 * j as u32 + 2 * q;

        let integral_term = integrate_trig_power(keep_func, var, power)?;

        if coeff == 1 {
            terms.push(integral_term);
        } else if coeff == -1 {
            terms.push(Node::Negate(Box::new(integral_term)));
        } else {
            terms.push(Node::Multiply(
                Box::new(Node::Num(ExactNum::integer(coeff))),
                Box::new(integral_term),
            ));
        }
    }

    if terms.is_empty() {
        return Ok(Node::Num(ExactNum::zero()));
    }

    let mut result = terms.remove(0);
    for term in terms {
        result = Node::Add(Box::new(result), Box::new(term));
    }

    Ok(crate::simplify::Simplifiable::simplify(&result, &env).unwrap_or(result))
}

fn binomial_coeff(n: u64, k: u64) -> u64 {
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    let mut result: u64 = 1;
    for i in 0..k {
        result = result * (n - i) / (i + 1);
    }
    result
}

/// Recognize inverse trig patterns in the denominator of 1/(...).
/// ∫1/(a² + x²) dx = (1/a)·arctan(x/a)
/// ∫1/√(a² - x²) dx = arcsin(x/a)
/// ∫1/√(x² - a²) dx = ln|x + √(x²-a²)| (arccosh form, but we use log form)
fn try_inverse_trig_integral(denom: &Node, var: &str) -> Option<Node> {
    let env = crate::environment::Environment::new();

    // Pattern: a² + x² or x² + a²
    if let Node::Add(l, r) = denom {
        let (const_part, var_part) = if !contains_var(l, var) && contains_var(r, var) {
            (l, r)
        } else if contains_var(l, var) && !contains_var(r, var) {
            (r, l)
        } else {
            return None;
        };

        // var_part should be x² (or simplified form of x^2)
        let is_var_squared = match var_part.as_ref() {
            Node::Power(base, exp) => {
                matches!(&**base, Node::Variable(v) if v == var)
                    && matches!(&**exp, Node::Num(n) if n == &ExactNum::two())
            }
            _ => false,
        };
        if !is_var_squared {
            return None;
        }

        // const_part should be a positive number (a²)
        let a_squared = match const_part.as_ref() {
            Node::Num(n) => {
                let val = n.to_f64();
                if val > 0.0 {
                    Some(n.clone())
                } else {
                    None
                }
            }
            _ => None,
        };
        let a_squared = a_squared?;

        let a_sq_f64 = a_squared.to_f64();
        let a_f64 = a_sq_f64.sqrt();

        if a_squared.is_one() {
            // ∫1/(1+x²) dx = arctan(x)
            return Some(Node::Function(
                "arctan".to_string(),
                vec![Node::Variable(var.to_string())],
            ));
        }

        // ∫1/(a²+x²) dx = (1/a)·arctan(x/a)
        let a_node = Node::Num(ExactNum::from_f64(a_f64));
        let result = Node::Multiply(
            Box::new(Node::Divide(
                Box::new(Node::Num(ExactNum::one())),
                Box::new(a_node.clone()),
            )),
            Box::new(Node::Function(
                "arctan".to_string(),
                vec![Node::Divide(
                    Box::new(Node::Variable(var.to_string())),
                    Box::new(a_node),
                )],
            )),
        );
        let result = crate::simplify::Simplifiable::simplify(&result, &env).unwrap_or(result);
        return Some(result);
    }

    // Pattern: √(a² - x²) — try polynomial approach to handle both
    // Subtract(a², x²) and Add(Negate(x²), a²) forms from the simplifier.
    // Parser produces Function("sqrt", [inner]), not Node::Sqrt.
    let sqrt_inner = match denom {
        Node::Sqrt(inner) => Some(inner.as_ref()),
        Node::Function(name, args) if name == "sqrt" && args.len() == 1 => Some(&args[0]),
        _ => None,
    };
    if let Some(inner) = sqrt_inner {
        if let Ok(poly) = Polynomial::from_node(inner, var) {
            // Should be -x² + a², i.e., degree 2 with negative leading coeff
            if poly.degree().unwrap_or(0) == 2 {
                let c2 = poly.coeff(2);
                let c1 = poly.coeff(1);
                let c0 = poly.coeff(0);
                if c2 == num_rational::BigRational::from_integer((-1).into())
                    && c1.is_zero()
                    && c0 > num_rational::BigRational::from_integer(0.into())
                {
                    let a_sq_f64: f64 = c0.numer().to_string().parse::<f64>().unwrap_or(0.0)
                        / c0.denom().to_string().parse::<f64>().unwrap_or(1.0);
                    if (a_sq_f64 - 1.0).abs() < 1e-14 {
                        return Some(Node::Function(
                            "arcsin".to_string(),
                            vec![Node::Variable(var.to_string())],
                        ));
                    }
                    let a_f64 = a_sq_f64.sqrt();
                    let a_node = Node::Num(ExactNum::from_f64(a_f64));
                    return Some(Node::Function(
                        "arcsin".to_string(),
                        vec![Node::Divide(
                            Box::new(Node::Variable(var.to_string())),
                            Box::new(a_node),
                        )],
                    ));
                }
            }
        }
    }

    None
}

/// Integrate a rational function P(x)/Q(x) via partial fraction decomposition.
///
/// Each partial fraction term integrates as:
/// - A/(x-a) → A·ln|x-a|
/// - A/(x-a)^k (k≥2) → A·(x-a)^(1-k)/(1-k)
/// - (Ax+B)/(x²+bx+c) → (A/2)·ln(x²+bx+c) + (2B-Ab)/√(4c-b²)·arctan((2x+b)/√(4c-b²))
fn try_partial_fraction_integration(
    numerator: &Node,
    denominator: &Node,
    var: &str,
) -> Option<Result<Node, String>> {
    let num_poly = Polynomial::from_node(numerator, var).ok()?;
    let den_poly = Polynomial::from_node(denominator, var).ok()?;

    // Need at least a non-trivial rational function
    if den_poly.degree().unwrap_or(0) < 1 {
        return None;
    }
    // Skip if numerator degree < denominator degree and denominator is linear
    // (handled by the linear denominator path above)
    if den_poly.degree().unwrap_or(0) < 2
        && num_poly.degree().unwrap_or(0) < den_poly.degree().unwrap_or(0)
    {
        return None;
    }

    let decomp = crate::partial_fractions::partial_fraction_decomposition(&num_poly, &den_poly)
        .ok()?;

    if decomp.terms.is_empty() && decomp.polynomial_part.is_zero() {
        return None;
    }

    let env = crate::environment::Environment::new();
    let mut result_terms: Vec<Node> = Vec::new();

    // Integrate polynomial part
    if !decomp.polynomial_part.is_zero() {
        match integrate(&decomp.polynomial_part.to_node(), var) {
            Ok(r) => result_terms.push(r),
            Err(_) => return None,
        }
    }

    // Integrate each partial fraction term
    for term in &decomp.terms {
        match integrate_pf_term(term, var) {
            Ok(r) => result_terms.push(r),
            Err(_) => return None,
        }
    }

    if result_terms.is_empty() {
        return Some(Ok(Node::Num(ExactNum::zero())));
    }

    let mut result = result_terms.remove(0);
    for t in result_terms {
        result = Node::Add(Box::new(result), Box::new(t));
    }

    let result = crate::simplify::Simplifiable::simplify(&result, &env).unwrap_or(result);
    Some(Ok(result))
}

/// Integrate a single partial fraction term: N(x) / q(x)^k.
fn integrate_pf_term(
    term: &crate::partial_fractions::PartialFractionTerm,
    var: &str,
) -> Result<Node, String> {
    let q = &term.denominator;
    let n = &term.numerator;
    let k = term.power;
    let q_deg = q.degree().unwrap_or(0);

    if q_deg == 1 {
        // Linear factor: q = x + a (monic)
        // N is a constant A
        let _a = q.coeff(0);
        let coeff_a = n.coeff(0); // numerator constant

        // ∫ A/(x+a)^k dx
        let x_plus_a = q.to_node();

        if k == 1 {
            // A·ln|x+a|
            let ln_term = Node::Function(
                "ln".to_string(),
                vec![Node::Abs(Box::new(x_plus_a))],
            );
            if coeff_a == num_rational::BigRational::one() {
                Ok(ln_term)
            } else {
                Ok(Node::Multiply(
                    Box::new(rational_to_node(&coeff_a)),
                    Box::new(ln_term),
                ))
            }
        } else {
            // A·(x+a)^(1-k)/(1-k)
            let exp = 1i64 - k as i64;
            let power_term = Node::Power(
                Box::new(x_plus_a),
                Box::new(Node::Num(ExactNum::integer(exp))),
            );
            use num_traits::ToPrimitive;
            let a_numer = coeff_a.numer().to_i64().unwrap_or(0);
            let a_denom = coeff_a.denom().to_i64().unwrap_or(1);
            let coeff = ExactNum::rational(a_numer, a_denom * exp);
            Ok(Node::Multiply(
                Box::new(Node::Num(coeff)),
                Box::new(power_term),
            ))
        }
    } else if q_deg == 2 && k == 1 {
        // Irreducible quadratic, power 1: (Ax+B)/(x²+bx+c)
        // Split: A/2·(2x+b)/(x²+bx+c) + (B-Ab/2)/(x²+bx+c)
        // First part: (A/2)·ln(x²+bx+c)
        // Second part: (B-Ab/2) · (2/√(4c-b²)) · arctan((2x+b)/√(4c-b²))

        let a_coeff = n.coeff(1); // coefficient of x in numerator
        let b_coeff = n.coeff(0); // constant in numerator
        let b_denom = q.coeff(1); // b in x²+bx+c
        let c_denom = q.coeff(0); // c in x²+bx+c

        let env = crate::environment::Environment::new();
        let mut terms: Vec<Node> = Vec::new();

        // Log term: (A/2)·ln|x²+bx+c|
        if !a_coeff.is_zero() {
            let half_a = &a_coeff / &num_rational::BigRational::from_integer(num_bigint::BigInt::from(2));
            let ln_arg = q.to_node();
            let ln_term = Node::Function("ln".to_string(), vec![Node::Abs(Box::new(ln_arg))]);
            if half_a == num_rational::BigRational::one() {
                terms.push(ln_term);
            } else {
                terms.push(Node::Multiply(
                    Box::new(rational_to_node(&half_a)),
                    Box::new(ln_term),
                ));
            }
        }

        // Arctan term: (B - A·b/2) · (2/√(4c-b²)) · arctan((2x+b)/√(4c-b²))
        let ab_half = &a_coeff * &b_denom
            / &num_rational::BigRational::from_integer(num_bigint::BigInt::from(2));
        let residual = &b_coeff - &ab_half;

        if !residual.is_zero() {
            // discriminant = 4c - b²
            let four = num_rational::BigRational::from_integer(num_bigint::BigInt::from(4));
            let disc = &four * &c_denom - &b_denom * &b_denom;
            let disc_f64 = disc.numer().to_string().parse::<f64>().unwrap_or(0.0)
                / disc.denom().to_string().parse::<f64>().unwrap_or(1.0);

            if disc_f64 > 0.0 {
                let sqrt_disc = disc_f64.sqrt();
                // coeff = residual · 2 / sqrt_disc
                let _two = num_rational::BigRational::from_integer(num_bigint::BigInt::from(2));
                let overall_coeff_f64 = {
                    let res_f64 = residual.numer().to_string().parse::<f64>().unwrap_or(0.0)
                        / residual.denom().to_string().parse::<f64>().unwrap_or(1.0);
                    res_f64 * 2.0 / sqrt_disc
                };

                // arctan arg: (2x + b) / sqrt(4c - b²)
                let two_x_plus_b = Node::Add(
                    Box::new(Node::Multiply(
                        Box::new(Node::Num(ExactNum::from_f64(2.0))),
                        Box::new(Node::Variable(var.to_string())),
                    )),
                    Box::new(rational_to_node(&b_denom)),
                );
                let arctan_arg = Node::Divide(
                    Box::new(two_x_plus_b),
                    Box::new(Node::Num(ExactNum::from_f64(sqrt_disc))),
                );
                let arctan_term =
                    Node::Function("arctan".to_string(), vec![arctan_arg]);

                if (overall_coeff_f64 - 1.0).abs() < 1e-14 {
                    terms.push(arctan_term);
                } else {
                    terms.push(Node::Multiply(
                        Box::new(Node::Num(ExactNum::from_f64(overall_coeff_f64))),
                        Box::new(arctan_term),
                    ));
                }
            }
        }

        if terms.is_empty() {
            return Ok(Node::Num(ExactNum::zero()));
        }
        let mut result = terms.remove(0);
        for t in terms {
            result = Node::Add(Box::new(result), Box::new(t));
        }
        let result = crate::simplify::Simplifiable::simplify(&result, &env).unwrap_or(result);
        Ok(result)
    } else {
        // Higher degree or higher power — not yet implemented
        Err(format!(
            "Integration of degree-{} factor to power {} not yet implemented",
            q_deg, k
        ))
    }
}

/// Trig substitution for ∫√(ax²+bx+c) dx.
///
/// Completes the square: ax²+bx+c = a(x+b/2a)² + (c - b²/4a).
/// Then depending on the signs of a and the remainder:
///   √(k² - u²) → u = k·sin(θ): result = (k²/2)(θ + sin(θ)cos(θ))
///   √(u² + k²) → u = k·tan(θ): result = (k²/2)(ln|sec(θ)+tan(θ)| + sec(θ)tan(θ))
///   √(u² - k²) → u = k·sec(θ): result involves sec and log terms
fn try_trig_substitution_sqrt(
    inner: &Node,
    var: &str,
) -> Option<Result<Node, String>> {
    let poly = Polynomial::from_node(inner, var).ok()?;
    if poly.degree()? != 2 {
        return None;
    }

    let a_coeff = poly.coeff(2);
    let b_coeff = poly.coeff(1);
    let c_coeff = poly.coeff(0);

    let env = crate::environment::Environment::new();

    // Complete the square: a(x + b/2a)² + (c - b²/4a)
    let two = num_rational::BigRational::from_integer(num_bigint::BigInt::from(2));
    let four = num_rational::BigRational::from_integer(num_bigint::BigInt::from(4));
    let shift = &b_coeff / (&two * &a_coeff); // b/(2a)
    let remainder = &c_coeff - &b_coeff * &b_coeff / (&four * &a_coeff); // c - b²/(4a)

    let a_f64 = rat_to_f64(&a_coeff);
    let remainder_f64 = rat_to_f64(&remainder);
    let shift_f64 = rat_to_f64(&shift);

    // u = x + shift, du = dx
    // Inner becomes: a·u² + remainder

    if a_f64 < 0.0 && remainder_f64 > 0.0 {
        // Form: √(k² - α·u²) where k² = remainder, α = -a
        // = √(remainder + a·u²) = √(remainder - |a|·u²)
        // Substitute u = √(remainder/|a|)·sin(θ)
        // √(k² - α·u²) = √remainder · cos(θ)
        // du = √(remainder/|a|) · cos(θ) dθ
        // ∫√(k²-α·u²) du = (remainder/|a|^{1/2}) · ∫cos²(θ) dθ
        //   wait, let me redo this more carefully.
        // Let |a| = α. Inner = remainder - α·u²
        // Let u = √(remainder/α) · sin(θ), then α·u² = remainder·sin²(θ)
        // √(inner) = √(remainder - remainder·sin²(θ)) = √remainder · cos(θ)
        // du = √(remainder/α) · cos(θ) dθ
        // ∫√(inner) du = √remainder · cos(θ) · √(remainder/α) · cos(θ) dθ
        //              = (remainder/√α) · ∫cos²(θ) dθ
        //              = (remainder/√α) · (θ/2 + sin(2θ)/4)
        //              = (remainder/(2√α)) · (θ + sin(θ)cos(θ))
        // Back-substitute: sin(θ) = u·√(α/remainder), so θ = arcsin(u·√(α/remainder))
        //   cos(θ) = √(1-sin²(θ)) = √(inner)/√remainder
        //   sin(θ)·cos(θ) = u·√α·√inner/remainder

        let alpha = -a_f64;
        let k_sq = remainder_f64; // k² = remainder
        let scale = k_sq / alpha.sqrt(); // remainder / √α

        // u = x + shift
        let u_node = if shift_f64.abs() < 1e-14 {
            Node::Variable(var.to_string())
        } else {
            Node::Add(
                Box::new(Node::Variable(var.to_string())),
                Box::new(Node::Num(ExactNum::from_f64(shift_f64))),
            )
        };

        // θ = arcsin(u · √(α/k²))
        let sin_arg_scale = (alpha / k_sq).sqrt();
        let theta = Node::Function(
            "arcsin".to_string(),
            vec![Node::Multiply(
                Box::new(Node::Num(ExactNum::from_f64(sin_arg_scale))),
                Box::new(u_node.clone()),
            )],
        );

        // sin(θ)·cos(θ) = u · √α · √(inner) / k²
        // where √(inner) = original sqrt argument evaluated
        let sqrt_inner = Node::Sqrt(Box::new(inner.clone()));
        let sin_cos_product = Node::Divide(
            Box::new(Node::Multiply(
                Box::new(Node::Num(ExactNum::from_f64(alpha.sqrt()))),
                Box::new(Node::Multiply(
                    Box::new(u_node),
                    Box::new(sqrt_inner),
                )),
            )),
            Box::new(Node::Num(ExactNum::from_f64(k_sq))),
        );

        // Result: (scale/2) · (θ + sin(θ)cos(θ))
        let half_scale = scale / 2.0;
        let result = Node::Multiply(
            Box::new(Node::Num(ExactNum::from_f64(half_scale))),
            Box::new(Node::Add(
                Box::new(theta),
                Box::new(sin_cos_product),
            )),
        );

        let result = crate::simplify::Simplifiable::simplify(&result, &env).unwrap_or(result);
        return Some(Ok(result));
    }

    if a_f64 > 0.0 && remainder_f64 > 0.0 {
        // Form: √(a·u² + k²) where k² = remainder
        // Substitute u = (k/√a)·tan(θ)
        // √(a·u² + k²) = k·sec(θ)
        // du = (k/√a)·sec²(θ) dθ
        // ∫√(a·u² + k²) du = (k²/√a) · ∫sec³(θ) dθ
        //   = (k²/(2√a)) · (sec(θ)tan(θ) + ln|sec(θ)+tan(θ)|)
        // Back-substitute: tan(θ) = u·√a/k
        //   sec(θ) = √(a·u²+k²)/k = √(inner)/k

        let k = remainder_f64.sqrt();
        let sqrt_a = a_f64.sqrt();
        let coeff = k * k / (2.0 * sqrt_a); // k²/(2√a)

        let u_node = if shift_f64.abs() < 1e-14 {
            Node::Variable(var.to_string())
        } else {
            Node::Add(
                Box::new(Node::Variable(var.to_string())),
                Box::new(Node::Num(ExactNum::from_f64(shift_f64))),
            )
        };

        // tan(θ) = u·√a/k
        let tan_val = Node::Multiply(
            Box::new(Node::Num(ExactNum::from_f64(sqrt_a / k))),
            Box::new(u_node),
        );
        // sec(θ) = √(inner)/k
        let sec_val = Node::Divide(
            Box::new(Node::Sqrt(Box::new(inner.clone()))),
            Box::new(Node::Num(ExactNum::from_f64(k))),
        );

        // sec(θ)·tan(θ) = √(inner)/k · u·√a/k = u·√a·√(inner)/k²
        let sec_tan = Node::Multiply(Box::new(sec_val.clone()), Box::new(tan_val.clone()));

        // ln|sec(θ) + tan(θ)|
        let log_arg = Node::Add(Box::new(sec_val), Box::new(tan_val));
        let log_term = Node::Function(
            "ln".to_string(),
            vec![Node::Abs(Box::new(log_arg))],
        );

        let result = Node::Multiply(
            Box::new(Node::Num(ExactNum::from_f64(coeff))),
            Box::new(Node::Add(
                Box::new(sec_tan),
                Box::new(log_term),
            )),
        );

        let result = crate::simplify::Simplifiable::simplify(&result, &env).unwrap_or(result);
        return Some(Ok(result));
    }

    if a_f64 > 0.0 && remainder_f64 < 0.0 {
        // Form: √(a·u² - k²) where k² = -remainder
        // Substitute u = (k/√a)·sec(θ)
        // √(a·u² - k²) = k·tan(θ)
        // du = (k/√a)·sec(θ)tan(θ) dθ
        // ∫√(a·u² - k²) du = (k²/√a) · ∫tan²(θ)sec(θ) dθ
        //   = (k²/(2√a)) · (sec(θ)tan(θ) - ln|sec(θ)+tan(θ)|)

        let k_sq = -remainder_f64;
        let k = k_sq.sqrt();
        let sqrt_a = a_f64.sqrt();
        let coeff = k_sq / (2.0 * sqrt_a);

        let u_node = if shift_f64.abs() < 1e-14 {
            Node::Variable(var.to_string())
        } else {
            Node::Add(
                Box::new(Node::Variable(var.to_string())),
                Box::new(Node::Num(ExactNum::from_f64(shift_f64))),
            )
        };

        // sec(θ) = u·√a/k
        let sec_val = Node::Multiply(
            Box::new(Node::Num(ExactNum::from_f64(sqrt_a / k))),
            Box::new(u_node),
        );
        // tan(θ) = √(inner)/k
        let tan_val = Node::Divide(
            Box::new(Node::Sqrt(Box::new(inner.clone()))),
            Box::new(Node::Num(ExactNum::from_f64(k))),
        );

        let sec_tan = Node::Multiply(Box::new(sec_val.clone()), Box::new(tan_val.clone()));
        let log_arg = Node::Add(Box::new(sec_val), Box::new(tan_val));
        let log_term = Node::Function(
            "ln".to_string(),
            vec![Node::Abs(Box::new(log_arg))],
        );

        let result = Node::Multiply(
            Box::new(Node::Num(ExactNum::from_f64(coeff))),
            Box::new(Node::Subtract(
                Box::new(sec_tan),
                Box::new(log_term),
            )),
        );

        let result = crate::simplify::Simplifiable::simplify(&result, &env).unwrap_or(result);
        return Some(Ok(result));
    }

    None
}

fn rat_to_f64(r: &num_rational::BigRational) -> f64 {
    r.numer().to_f64().unwrap_or(0.0) / r.denom().to_f64().unwrap_or(1.0)
}

fn rational_to_node(r: &num_rational::BigRational) -> Node {
    use num_traits::ToPrimitive;
    if r.is_integer() {
        Node::Num(ExactNum::integer(r.numer().to_i64().unwrap_or(0)))
    } else {
        Node::Num(ExactNum::rational(
            r.numer().to_i64().unwrap_or(0),
            r.denom().to_i64().unwrap_or(1),
        ))
    }
}

/// U-substitution: Given ∫h(x)dx, find g(x) such that h(x) = f(g(x))·g'(x)·c,
/// then result = c · F(g(x)) where F is the antiderivative of f.
fn try_u_substitution(expr: &Node, var: &str) -> Option<Result<Node, String>> {
    let env = crate::environment::Environment::new();

    // Decompose into multiplicative factors
    let mut factors = Vec::new();
    collect_factors(expr, &mut factors);

    // Collect candidates from the whole expression
    let candidates = collect_u_candidates(expr, var);

    for g_x in &candidates {
        if !contains_var(g_x, var) {
            continue;
        }
        let dg = match crate::derivative::differentiate(g_x, var) {
            Ok(d) => crate::simplify::Simplifiable::simplify(&d, &env).unwrap_or(d),
            Err(_) => continue,
        };
        if matches!(&dg, Node::Num(n) if n.is_zero()) {
            continue;
        }

        // Rebuild the product of all factors EXCEPT those that match g(x) or contain it
        // Then check if that product / g'(x) is constant
        //
        // Strategy: separate factors into "g-dependent" (contain g(x) as subexpr)
        // and "remaining" (potential g'(x) carrier).
        let mut remaining_factors: Vec<Node> = Vec::new();
        let mut g_factor: Option<Node> = None;

        for f in &factors {
            let f_with_u = replace_subexpr(f, g_x, &Node::Variable("_u_".to_string()));
            let was_changed = &f_with_u != f;
            if was_changed && !contains_var(&f_with_u, var) {
                // Factor contains g(x), and after substitution is free of var
                if g_factor.is_some() {
                    remaining_factors.push(f.clone());
                } else {
                    g_factor = Some(f.clone());
                }
            } else {
                remaining_factors.push(f.clone());
            }
        }

        let g_factor = match g_factor {
            Some(f) => f,
            None => continue,
        };

        // Build the "remaining" product and divide by g'(x)
        let remaining = if remaining_factors.is_empty() {
            Node::Num(ExactNum::one())
        } else {
            let mut prod = remaining_factors.remove(0);
            for f in remaining_factors {
                prod = Node::Multiply(Box::new(prod), Box::new(f));
            }
            prod
        };

        let ratio = Node::Divide(Box::new(remaining), Box::new(dg.clone()));
        let ratio_simplified =
            crate::simplify::Simplifiable::simplify(&ratio, &env).unwrap_or(ratio);

        if contains_var(&ratio_simplified, var) {
            continue;
        }

        // ratio_simplified is the constant c
        // g_factor with g(x)→u is f(u)
        let f_of_u = replace_subexpr(&g_factor, g_x, &Node::Variable("_u_".to_string()));

        let integral_of_f = match integrate(&f_of_u, "_u_") {
            Ok(i) => i,
            Err(_) => continue,
        };

        // Back-substitute u = g(x)
        let result = match crate::substitute::substitute_variable(&integral_of_f, "_u_", g_x) {
            Ok(r) => r,
            Err(_) => continue,
        };

        // Multiply by constant c
        let final_result = if matches!(&ratio_simplified, Node::Num(n) if n.is_one()) {
            result
        } else {
            Node::Multiply(Box::new(ratio_simplified), Box::new(result))
        };

        let final_simplified =
            crate::simplify::Simplifiable::simplify(&final_result, &env).unwrap_or(final_result);
        return Some(Ok(final_simplified));
    }
    None
}

/// Flatten a multiplicative expression into factors.
fn collect_factors(expr: &Node, factors: &mut Vec<Node>) {
    match expr {
        Node::Multiply(l, r) => {
            collect_factors(l, factors);
            collect_factors(r, factors);
        }
        _ => factors.push(expr.clone()),
    }
}

/// Collect candidate inner functions g(x) for u-substitution.
fn collect_u_candidates(expr: &Node, var: &str) -> Vec<Node> {
    let mut candidates = Vec::new();
    collect_u_candidates_inner(expr, var, &mut candidates);
    candidates
}

fn collect_u_candidates_inner(expr: &Node, var: &str, candidates: &mut Vec<Node>) {
    match expr {
        Node::Function(_, args) => {
            // The function call itself is a candidate (e.g., sin(x) for ∫sin(x)·cos(x)dx)
            if contains_var(expr, var) {
                candidates.push(expr.clone());
            }
            for arg in args {
                if contains_var(arg, var) && !matches!(arg, Node::Variable(_)) {
                    candidates.push(arg.clone());
                }
                collect_u_candidates_inner(arg, var, candidates);
            }
        }
        Node::Power(base, exp) => {
            // If the base is a function call like sin(x), it's a candidate
            if let Node::Function(_, _) = &**base {
                if contains_var(base, var) {
                    candidates.push(*base.clone());
                }
            }
            // Non-trivial base expressions are candidates
            if contains_var(base, var) && !matches!(&**base, Node::Variable(_)) {
                candidates.push(*base.clone());
            }
            // Non-trivial exponent expressions are candidates
            if contains_var(exp, var) && !matches!(&**exp, Node::Variable(_)) {
                candidates.push(*exp.clone());
            }
            collect_u_candidates_inner(base, var, candidates);
            collect_u_candidates_inner(exp, var, candidates);
        }
        Node::Multiply(l, r) | Node::Add(l, r) | Node::Subtract(l, r) | Node::Divide(l, r) => {
            collect_u_candidates_inner(l, var, candidates);
            collect_u_candidates_inner(r, var, candidates);
        }
        Node::Negate(inner) | Node::Sqrt(inner) | Node::Abs(inner) => {
            collect_u_candidates_inner(inner, var, candidates);
        }
        _ => {}
    }
}

/// Replace all occurrences of `target` subexpression with `replacement`.
fn replace_subexpr(expr: &Node, target: &Node, replacement: &Node) -> Node {
    if expr == target {
        return replacement.clone();
    }
    match expr {
        Node::Add(l, r) => Node::Add(
            Box::new(replace_subexpr(l, target, replacement)),
            Box::new(replace_subexpr(r, target, replacement)),
        ),
        Node::Subtract(l, r) => Node::Subtract(
            Box::new(replace_subexpr(l, target, replacement)),
            Box::new(replace_subexpr(r, target, replacement)),
        ),
        Node::Multiply(l, r) => Node::Multiply(
            Box::new(replace_subexpr(l, target, replacement)),
            Box::new(replace_subexpr(r, target, replacement)),
        ),
        Node::Divide(l, r) => Node::Divide(
            Box::new(replace_subexpr(l, target, replacement)),
            Box::new(replace_subexpr(r, target, replacement)),
        ),
        Node::Power(base, exp) => Node::Power(
            Box::new(replace_subexpr(base, target, replacement)),
            Box::new(replace_subexpr(exp, target, replacement)),
        ),
        Node::Negate(inner) => Node::Negate(Box::new(replace_subexpr(inner, target, replacement))),
        Node::Sqrt(inner) => Node::Sqrt(Box::new(replace_subexpr(inner, target, replacement))),
        Node::Abs(inner) => Node::Abs(Box::new(replace_subexpr(inner, target, replacement))),
        Node::Function(name, args) => Node::Function(
            name.clone(),
            args.iter()
                .map(|a| replace_subexpr(a, target, replacement))
                .collect(),
        ),
        _ => expr.clone(),
    }
}

/// Integrates a LaTeX expression with respect to a variable
///
/// # Arguments
///
/// * `latex_expr` - The LaTeX expression to integrate
/// * `var_name` - The variable to integrate with respect to
///
/// # Returns
///
/// The integral of the expression as a LaTeX string
pub fn integrate_latex(latex_expr: &str, var_name: &str) -> Result<String, String> {
    let mut tokenizer = Tokenizer::new(latex_expr);
    let tokens = tokenizer.tokenize();
    let expr = build_expression_tree(tokens)?;
    let integral = integrate(&expr, var_name)?;
    let env = crate::environment::Environment::new();
    let simplified = crate::simplify::Simplifiable::simplify(&integral, &env).unwrap_or(integral);
    Ok(format!("{} + C", simplified))
}

/// Calculates the definite integral of an expression between two bounds
///
/// # Arguments
///
/// * `expr` - The expression to integrate
/// * `var_name` - The variable to integrate with respect to
/// * `lower` - The lower bound of integration
/// * `upper` - The upper bound of integration
///
/// # Returns
///
/// The definite integral value
pub fn definite_integral(
    expr: &Node,
    var_name: &str,
    lower: f64,
    upper: f64,
) -> Result<f64, String> {
    // First find the indefinite integral
    let indefinite = integrate(expr, var_name)?;

    // Create substitution functions to evaluate at upper and lower bounds
    let mut upper_env = crate::environment::Environment::new();
    upper_env.set(var_name, upper);

    let mut lower_env = crate::environment::Environment::new();
    lower_env.set(var_name, lower);

    // Calculate F(upper) - F(lower)
    let upper_value = crate::evaluator::Evaluator::evaluate(&indefinite, &upper_env)?;
    let lower_value = crate::evaluator::Evaluator::evaluate(&indefinite, &lower_env)?;

    Ok(upper_value - lower_value)
}

/// Calculates the definite integral of a LaTeX expression between two bounds
///
/// # Arguments
///
/// * `latex_expr` - The LaTeX expression to integrate
/// * `var_name` - The variable to integrate with respect to
/// * `lower` - The lower bound of integration
/// * `upper` - The upper bound of integration
///
/// # Returns
///
/// The definite integral value as a LaTeX string
pub fn definite_integral_latex(
    latex_expr: &str,
    var_name: &str,
    lower: f64,
    upper: f64,
) -> Result<String, String> {
    // Parse the input expression
    let mut tokenizer = Tokenizer::new(latex_expr);
    let tokens = tokenizer.tokenize();
    let expr = build_expression_tree(tokens)?;

    // Compute the definite integral
    let result = definite_integral(&expr, var_name, lower, upper)?;

    // Convert back to LaTeX
    Ok(format!("{}", result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::Environment;
    use crate::evaluator::Evaluator;

    fn parse_expression(latex: &str) -> Result<Node, String> {
        let mut tokenizer = Tokenizer::new(latex);
        let tokens = tokenizer.tokenize();
        build_expression_tree(tokens)
    }

    fn approx_eq(a: f64, b: f64, epsilon: f64) -> bool {
        (a - b).abs() < epsilon
    }

    #[test]
    fn test_integrate_constant() {
        // ∫5 dx = 5x
        let expr = parse_expression("5").unwrap();
        let integral = integrate(&expr, "x").unwrap();

        let mut env = Environment::new();
        env.set("x", 2.0);

        // Test at x=2: 5*2 = 10
        let result = Evaluator::evaluate(&integral, &env).unwrap();
        assert_eq!(result, 10.0);
    }

    #[test]
    fn test_integrate_variable() {
        // ∫x dx = x²/2
        let expr = parse_expression("x").unwrap();
        let integral = integrate(&expr, "x").unwrap();

        let mut env = Environment::new();
        env.set("x", 3.0);

        // Test at x=3: 3²/2 = 4.5
        let result = Evaluator::evaluate(&integral, &env).unwrap();
        assert_eq!(result, 4.5);
    }

    #[test]
    fn test_integrate_different_variable() {
        // ∫y dx = y*x (y is constant with respect to x)
        let expr = parse_expression("y").unwrap();
        let integral = integrate(&expr, "x").unwrap();

        let mut env = Environment::new();
        env.set("x", 2.0);
        env.set("y", 3.0);

        // Test at x=2, y=3: 3*2 = 6
        let result = Evaluator::evaluate(&integral, &env).unwrap();
        assert_eq!(result, 6.0);
    }

    #[test]
    fn test_integrate_polynomial() {
        // ∫(3x² + 2x + 1) dx = x³ + x² + x
        let expr = parse_expression("3*x^2 + 2*x + 1").unwrap();
        let integral = integrate(&expr, "x").unwrap();

        let mut env = Environment::new();
        env.set("x", 2.0);

        // Test at x=2: 2³ + 2² + 2 = 8 + 4 + 2 = 14
        let result = Evaluator::evaluate(&integral, &env).unwrap();
        assert_eq!(result, 14.0);
    }

    #[test]
    fn test_integrate_power() {
        // ∫x^3 dx = x^4/4
        let expr = parse_expression("x^3").unwrap();
        let integral = integrate(&expr, "x").unwrap();

        let mut env = Environment::new();
        env.set("x", 2.0);

        // Test at x=2: 2⁴/4 = 16/4 = 4
        let result = Evaluator::evaluate(&integral, &env).unwrap();
        assert_eq!(result, 4.0);
    }

    #[test]
    fn test_integrate_negative_power() {
        // ∫x^(-1) dx = ln|x|
        let expr = parse_expression("x^(-1)").unwrap();
        let integral = integrate(&expr, "x").unwrap();

        let mut env = Environment::new();
        env.set("x", 2.0);

        // Test at x=2: ln(2) ≈ 0.693
        let result = Evaluator::evaluate(&integral, &env).unwrap();
        assert!(approx_eq(result, 2.0_f64.ln(), 1e-10));
    }

    #[test]
    fn test_definite_integral() {
        // ∫₁³ x² dx = [x³/3]₁³ = 3³/3 - 1³/3 = 9 - 1/3 = 8.667
        let expr = parse_expression("x^2").unwrap();
        let result = definite_integral(&expr, "x", 1.0, 3.0).unwrap();

        assert!(approx_eq(result, 8.667, 0.001));
    }

    #[test]
    fn test_integrate_complex_expression() {
        // Test a more complex expression with the parts we've implemented
        // ∫(2x³ + 3x² - 4x + 5) dx = (2x⁴/4) + (3x³/3) - (4x²/2) + 5x = (x⁴/2) + x³ - 2x² + 5x
        let expr = parse_expression("2*x^3 + 3*x^2 - 4*x + 5").unwrap();
        let integral = integrate(&expr, "x").unwrap();

        let mut env = Environment::new();
        env.set("x", 2.0);

        // At x=2: (2⁴/2) + 2³ - 2*2² + 5*2 = 8 + 8 - 8 + 10 = 18
        let result = Evaluator::evaluate(&integral, &env).unwrap();
        assert_eq!(result, 18.0);
    }

    #[test]
    fn test_latex_integration() {
        // Test the LaTeX interface for integration
        let result = integrate_latex("x^2", "x").unwrap();

        // Check that it contains the expected parts, allowing for formatting variations
        assert!(
            result.contains("+ C"),
            "Result should contain constant of integration"
        );

        // Create an environment and evaluate the integral at x=2
        let mut env = Environment::new();
        env.set("x", 2.0);

        // Parse just the expression part (without "+ C")
        let expr_part = result.replace(" + C", "");
        let parsed = parse_expression(&expr_part).unwrap();

        // Evaluate at x=2: x^3/3 at x=2 should be 8/3 ≈ 2.67
        let evaluated = Evaluator::evaluate(&parsed, &env).unwrap();
        assert!(
            approx_eq(evaluated, 2.67, 0.01),
            "Integral of x^2 evaluated at x=2 should be approximately 2.67"
        );
    }

    #[test]
    fn test_polynomial_integration_canonical_form() {
        let expr = parse_expression("3*x^2 + 2*x + 1").unwrap();
        let integral = integrate(&expr, "x").unwrap();
        let form = format!("{}", integral);
        assert_eq!(form, "x^{3} + x^{2} + x");
    }

    #[test]
    fn test_polynomial_integration_single_term() {
        let expr = parse_expression("6*x^2").unwrap();
        let integral = integrate(&expr, "x").unwrap();
        let form = format!("{}", integral);
        assert_eq!(form, "2x^{3}");
    }

    #[test]
    fn test_polynomial_integration_constant() {
        let expr = parse_expression("7").unwrap();
        let integral = integrate(&expr, "x").unwrap();
        let form = format!("{}", integral);
        assert_eq!(form, "7x");
    }

    #[test]
    fn test_polynomial_integration_fractional_coeff() {
        // ∫x^2 dx = (1/3)x^3
        let expr = parse_expression("x^2").unwrap();
        let integral = integrate(&expr, "x").unwrap();
        let form = format!("{}", integral);
        assert_eq!(form, "\\frac{1}{3} \\cdot x^{3}");
    }

    #[test]
    fn test_nonpolynomial_fallback() {
        // ∫x^(-1) dx should fall through to the ln|x| path
        let expr = parse_expression("x^{-1}").unwrap();
        let integral = integrate(&expr, "x").unwrap();
        let mut env = Environment::new();
        env.set("x", std::f64::consts::E);
        let result = Evaluator::evaluate(&integral, &env).unwrap();
        assert!(approx_eq(result, 1.0, 1e-10));
    }
}
