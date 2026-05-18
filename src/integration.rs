use crate::exact::ExactNum;
use crate::node::Node;
use crate::parser::build_expression_tree;
use crate::polynomial::Polynomial;
use crate::tokenizer::Tokenizer;

pub fn integrate(expr: &Node, var_name: &str) -> Result<Node, String> {
    let env = crate::environment::Environment::new();
    let expr = &crate::simplify::Simplifiable::simplify(expr, &env).unwrap_or_else(|_| expr.clone());

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
                        let ln_a = Node::Function(
                            "ln".to_string(),
                            vec![Node::Num(a.clone())],
                        );
                        return Ok(Node::Divide(Box::new(a_to_x), Box::new(ln_a)));
                    }
                }
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

            // ∫k/f(x) dx = k * ∫(1/f(x)) dx — factor out constant numerator
            if let Node::Num(k) = &**left {
                let one_over_right = Node::Divide(
                    Box::new(Node::Num(ExactNum::one())),
                    right.clone(),
                );
                if let Ok(inner) = integrate(&one_over_right, var_name) {
                    return Ok(Node::Multiply(
                        Box::new(Node::Num(k.clone())),
                        Box::new(inner),
                    ));
                }
            }
            // ∫f(x)/k dx = (1/k) * ∫f(x) dx — factor out constant denominator
            if let Node::Num(k) = &**right {
                if !k.is_zero() {
                    let inner = integrate(left, var_name)?;
                    let inv = ExactNum::one() / k.clone();
                    return Ok(Node::Multiply(
                        Box::new(Node::Num(inv)),
                        Box::new(inner),
                    ));
                }
            }

            Err("Integration of this division is not yet implemented".to_string())
        }

        Node::Negate(inner) => {
            let inner_integral = integrate(inner, var_name)?;
            Ok(Node::Negate(Box::new(inner_integral)))
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
                let inv_a = Node::Divide(
                    Box::new(Node::Num(ExactNum::one())),
                    Box::new(Node::Num(a)),
                );
                return Ok(Node::Multiply(
                    Box::new(inv_a),
                    Box::new(base_integral),
                ));
            }
            // Try u-substitution on the full expression (may help with composed functions)
            let full_expr = Node::Function(name.clone(), args.clone());
            if let Some(result) = try_u_substitution(&full_expr, var_name) {
                return result;
            }
            Err(format!("Integration of {}(...) with non-linear argument not yet implemented", name))
        }

        _ => Err("Integration of this expression is not yet implemented".to_string()),
    }
}

fn integrate_standard_function(name: &str, var: &str) -> Result<Node, String> {
    let x = || Node::Variable(var.to_string());
    match name {
        // ∫sin(x) = -cos(x)
        "sin" => Ok(Node::Negate(Box::new(
            Node::Function("cos".to_string(), vec![x()]),
        ))),
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
        Node::Add(l, r) | Node::Subtract(l, r) | Node::Multiply(l, r)
        | Node::Divide(l, r) | Node::Power(l, r) => {
            contains_var(l, var) || contains_var(r, var)
        }
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
        v_integral = crate::simplify::Simplifiable::simplify(&v_integral, &env)
            .unwrap_or(v_integral);

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
            .unwrap_or_else(|| {
                crate::simplify::Simplifiable::simplify(&v_du, &env).unwrap_or(v_du)
            })
    };

    let remaining = match integrate(&v_du_reparsed, var) {
        Ok(r) => crate::simplify::Simplifiable::simplify(&r, &env).unwrap_or(r),
        Err(e) => return Some(Err(e)),
    };

    Some(Ok(Node::Subtract(Box::new(uv), Box::new(remaining))))
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
        let ratio_simplified = crate::simplify::Simplifiable::simplify(&ratio, &env)
            .unwrap_or(ratio);

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

        let final_simplified = crate::simplify::Simplifiable::simplify(&final_result, &env)
            .unwrap_or(final_result);
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
            args.iter().map(|a| replace_subexpr(a, target, replacement)).collect(),
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
    let simplified =
        crate::simplify::Simplifiable::simplify(&integral, &env).unwrap_or(integral);
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
