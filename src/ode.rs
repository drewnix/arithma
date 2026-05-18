use crate::environment::Environment;
use crate::exact::ExactNum;
use crate::integration::integrate;
use crate::node::Node;
use crate::simplify::Simplifiable;
use crate::{build_expression_tree, Tokenizer};

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

fn var(name: &str) -> Node {
    Node::Variable(name.to_string())
}

fn num(n: i64) -> Node {
    Node::Num(ExactNum::integer(n))
}

fn exp(arg: Node) -> Node {
    Node::Function("exp".to_string(), vec![arg])
}

fn cos(arg: Node) -> Node {
    Node::Function("cos".to_string(), vec![arg])
}

fn sin(arg: Node) -> Node {
    Node::Function("sin".to_string(), vec![arg])
}

fn mul(a: Node, b: Node) -> Node {
    Node::Multiply(Box::new(a), Box::new(b))
}

fn add(a: Node, b: Node) -> Node {
    Node::Add(Box::new(a), Box::new(b))
}

fn simplify(node: &Node) -> Node {
    let env = Environment::new();
    node.simplify(&env).unwrap_or_else(|_| node.clone())
}

/// Solve a second-order constant-coefficient ODE: ay'' + by' + cy = 0.
/// Returns the general solution as a Node with C_{1} and C_{2}.
pub fn solve_constant_coeff(
    a: &ExactNum,
    b: &ExactNum,
    c: &ExactNum,
    indep: &str,
) -> Result<Node, String> {
    let a_f = a.to_f64();
    let b_f = b.to_f64();
    let c_f = c.to_f64();

    if a_f == 0.0 {
        return Err("Coefficient a must be nonzero for second-order ODE".to_string());
    }

    let disc = b_f * b_f - 4.0 * a_f * c_f;

    let c1 = var("C_{1}");
    let c2 = var("C_{2}");
    let x = var(indep);

    if disc > 1e-12 {
        // Distinct real roots
        let r1 = (-b_f + disc.sqrt()) / (2.0 * a_f);
        let r2 = (-b_f - disc.sqrt()) / (2.0 * a_f);
        let solution = add(
            mul(c1, exp(mul(Node::Num(ExactNum::from_f64(r1)), x.clone()))),
            mul(c2, exp(mul(Node::Num(ExactNum::from_f64(r2)), x))),
        );
        Ok(simplify(&solution))
    } else if disc.abs() <= 1e-12 {
        // Repeated root
        let r = -b_f / (2.0 * a_f);
        let rx = mul(Node::Num(ExactNum::from_f64(r)), x.clone());
        let solution = mul(add(c1, mul(c2, x)), exp(rx));
        Ok(simplify(&solution))
    } else {
        // Complex roots: alpha ± beta*i
        let alpha = -b_f / (2.0 * a_f);
        let beta = (-disc).sqrt() / (2.0 * a_f);

        let bx = mul(Node::Num(ExactNum::from_f64(beta)), x.clone());
        let inner = add(mul(c1, cos(bx.clone())), mul(c2, sin(bx)));

        let solution = if alpha.abs() < 1e-14 {
            // Pure imaginary roots — no exponential envelope
            inner
        } else {
            mul(exp(mul(Node::Num(ExactNum::from_f64(alpha)), x)), inner)
        };
        Ok(simplify(&solution))
    }
}

/// Try to solve dy/dx = f(x,y) as a separable ODE.
/// Separable means f(x,y) = g(x) * h(y).
fn try_separable(rhs: &Node, indep: &str, dep: &str) -> Option<Node> {
    let has_x = contains_var(rhs, indep);
    let has_y = contains_var(rhs, dep);

    // f depends only on x: dy/dx = g(x) → y = ∫g(x)dx + C₁
    if has_x && !has_y {
        let integral = integrate(rhs, indep).ok()?;
        let solution = add(integral, var("C_{1}"));
        return Some(simplify(&solution));
    }

    // f depends only on y: dy/dx = h(y) → ∫dy/h(y) = x + C₁ (implicit)
    if !has_x && has_y {
        let one_over_h = Node::Divide(Box::new(num(1)), Box::new(rhs.clone()));
        let lhs_integral = integrate(&simplify(&one_over_h), dep).ok()?;
        let solution = Node::Equation(
            Box::new(lhs_integral),
            Box::new(add(var(indep), var("C_{1}"))),
        );
        return Some(simplify(&solution));
    }

    // f is a product: try to split into g(x)*h(y)
    if has_x && has_y {
        if let Some((g_x, h_y)) = try_factor_separable(rhs, indep, dep) {
            // ∫dy/h(y) = ∫g(x)dx + C₁
            let one_over_h = Node::Divide(Box::new(num(1)), Box::new(h_y));
            let lhs = integrate(&simplify(&one_over_h), dep).ok()?;
            let rhs_integral = integrate(&g_x, indep).ok()?;
            let solution = Node::Equation(Box::new(lhs), Box::new(add(rhs_integral, var("C_{1}"))));
            return Some(simplify(&solution));
        }
    }

    // Constant RHS: dy/dx = k → y = kx + C₁
    if !has_x && !has_y {
        let solution = add(mul(rhs.clone(), var(indep)), var("C_{1}"));
        return Some(simplify(&solution));
    }

    None
}

/// Try to factor an expression as g(indep) * h(dep).
fn try_factor_separable(node: &Node, indep: &str, dep: &str) -> Option<(Node, Node)> {
    if let Node::Multiply(left, right) = node {
        let l_has_x = contains_var(left, indep);
        let l_has_y = contains_var(left, dep);
        let r_has_x = contains_var(right, indep);
        let r_has_y = contains_var(right, dep);

        // left = g(x), right = h(y)
        if l_has_x && !l_has_y && !r_has_x && r_has_y {
            return Some((*left.clone(), *right.clone()));
        }
        // left = h(y), right = g(x)
        if !l_has_x && l_has_y && r_has_x && !r_has_y {
            return Some((*right.clone(), *left.clone()));
        }
        // left = g(x), right = constant
        if l_has_x && !l_has_y && !r_has_x && !r_has_y {
            return Some((node.clone(), num(1)));
        }
        // left = constant, right = h(y)
        if !l_has_x && !l_has_y && !r_has_x && r_has_y {
            return Some((node.clone(), num(1)));
        }
    }
    None
}

/// Try to solve dy/dx = f(x,y) as a first-order linear ODE.
/// Linear means f(x,y) = Q(x) - P(x)*y, i.e., dy/dx + P(x)*y = Q(x).
fn try_linear(rhs: &Node, indep: &str, dep: &str) -> Option<Node> {
    // Extract the coefficient of y and the remainder.
    // f(x,y) = Q(x) - P(x)*y → we need P(x) and Q(x).
    let (neg_p, q) = extract_linear_parts(rhs, dep)?;

    // P(x) = -neg_p (since f = Q - P*y, and neg_p is the coefficient of y in f)
    // So dy/dx + P(x)*y = Q(x) where P = -neg_p
    let p = simplify(&Node::Negate(Box::new(neg_p)));

    // Integrating factor: mu = e^(∫P dx)
    let p_integral = integrate(&p, indep).ok()?;
    let mu = exp(p_integral.clone());

    // Solution: y = e^(-∫P dx) * (∫ Q * e^(∫P dx) dx + C₁)
    let q_times_mu = simplify(&mul(q, mu));
    let integral_q_mu = integrate(&q_times_mu, indep).ok()?;

    let neg_p_integral = simplify(&Node::Negate(Box::new(p_integral)));
    let solution = mul(exp(neg_p_integral), add(integral_q_mu, var("C_{1}")));

    Some(simplify(&solution))
}

/// Extract the linear decomposition: given f(x,y), find (coeff_of_y, remainder)
/// such that f = remainder + coeff_of_y * y.
fn extract_linear_parts(node: &Node, dep: &str) -> Option<(Node, Node)> {
    match node {
        // y itself: coeff=1, remainder=0
        Node::Variable(v) if v == dep => Some((num(1), num(0))),

        // A number or variable that isn't dep: coeff=0, remainder=self
        Node::Num(_) => Some((num(0), node.clone())),
        Node::Variable(_) => Some((num(0), node.clone())),

        // a + b: decompose each and combine
        Node::Add(a, b) => {
            let (ca, ra) = extract_linear_parts(a, dep)?;
            let (cb, rb) = extract_linear_parts(b, dep)?;
            Some((simplify(&add(ca, cb)), simplify(&add(ra, rb))))
        }

        // a - b: decompose each and combine
        Node::Subtract(a, b) => {
            let (ca, ra) = extract_linear_parts(a, dep)?;
            let (cb, rb) = extract_linear_parts(b, dep)?;
            Some((
                simplify(&Node::Subtract(Box::new(ca), Box::new(cb))),
                simplify(&Node::Subtract(Box::new(ra), Box::new(rb))),
            ))
        }

        // -a: negate both parts
        Node::Negate(inner) => {
            let (c, r) = extract_linear_parts(inner, dep)?;
            Some((
                simplify(&Node::Negate(Box::new(c))),
                simplify(&Node::Negate(Box::new(r))),
            ))
        }

        // k * something or something * k
        Node::Multiply(a, b) => {
            let a_has_dep = contains_var(a, dep);
            let b_has_dep = contains_var(b, dep);

            if !a_has_dep && !b_has_dep {
                // No dep at all: pure remainder
                return Some((num(0), node.clone()));
            }

            // factor * y or y * factor
            if a_has_dep && !b_has_dep {
                if let Node::Variable(v) = a.as_ref() {
                    if v == dep {
                        return Some((*b.clone(), num(0)));
                    }
                }
            }
            if b_has_dep && !a_has_dep {
                if let Node::Variable(v) = b.as_ref() {
                    if v == dep {
                        return Some((*a.clone(), num(0)));
                    }
                }
            }

            None
        }

        // Function calls: if no dep, it's a remainder
        Node::Function(_, _) => {
            if !contains_var(node, dep) {
                Some((num(0), node.clone()))
            } else {
                None
            }
        }

        // Division, Power, etc. with no dep: pure remainder
        Node::Divide(_, _) | Node::Power(_, _) | Node::Sqrt(_) => {
            if !contains_var(node, dep) {
                Some((num(0), node.clone()))
            } else {
                None
            }
        }

        _ => None,
    }
}

/// Solve a first-order ODE: dy/dx = f(x,y).
/// Classification order: if f depends on dep, try linear first (gives explicit
/// solutions); otherwise try separable. Falls back to the other method if the
/// first fails.
pub fn solve_first_order(rhs: &Node, indep: &str, dep: &str) -> Result<Node, String> {
    let rhs_simplified = simplify(rhs);

    if contains_var(&rhs_simplified, dep) {
        // Has dependent variable — prefer linear (explicit solution)
        if let Some(solution) = try_linear(&rhs_simplified, indep, dep) {
            return Ok(solution);
        }
        if let Some(solution) = try_separable(&rhs_simplified, indep, dep) {
            return Ok(solution);
        }
    } else {
        // Pure function of indep (or constant) — separable is direct
        if let Some(solution) = try_separable(&rhs_simplified, indep, dep) {
            return Ok(solution);
        }
    }

    Err(format!(
        "Cannot classify ODE: dy/d{} = {}. Supported types: separable, first-order linear.",
        indep, rhs_simplified
    ))
}

/// Solve a first-order ODE from LaTeX: dy/dx = rhs_latex.
pub fn solve_ode_latex(rhs_latex: &str, indep: &str, dep: &str) -> Result<String, String> {
    let mut tokenizer = Tokenizer::new(rhs_latex);
    let tokens = tokenizer.tokenize();
    let rhs = build_expression_tree(tokens)?;
    let solution = solve_first_order(&rhs, indep, dep)?;
    if let Node::Equation(_, _) = &solution {
        Ok(format!("{}", solution))
    } else {
        Ok(format!("{} = {}", dep, solution))
    }
}

/// Solve a second-order constant-coefficient ODE from numeric coefficients.
pub fn solve_constant_coeff_latex(a: f64, b: f64, c: f64, indep: &str) -> Result<String, String> {
    let a_num = ExactNum::from_f64(a);
    let b_num = ExactNum::from_f64(b);
    let c_num = ExactNum::from_f64(c);
    let solution = solve_constant_coeff(&a_num, &b_num, &c_num, indep)?;
    Ok(format!("y = {}", solution))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solve_and_format(rhs: &str, indep: &str, dep: &str) -> String {
        solve_ode_latex(rhs, indep, dep).unwrap()
    }

    fn solve_cc(a: f64, b: f64, c: f64) -> String {
        solve_constant_coeff_latex(a, b, c, "x").unwrap()
    }

    // === Separable: dy/dx = g(x) ===

    #[test]
    fn test_separable_pure_x() {
        // dy/dx = x^2 → y = x^3/3 + C₁
        let result = solve_and_format("x^2", "x", "y");
        assert!(result.contains("C_{1}"), "Expected C_1 in: {}", result);
    }

    #[test]
    fn test_separable_constant() {
        // dy/dx = 3 → y = 3x + C₁
        let result = solve_and_format("3", "x", "y");
        assert!(result.contains("C_{1}"), "Expected C_1 in: {}", result);
    }

    // === Separable: dy/dx = h(y) ===

    #[test]
    fn test_separable_pure_y() {
        // dy/dx = y → implicit: ln|y| = x + C₁
        let result = solve_and_format("y", "x", "y");
        assert!(result.contains("C_{1}"), "Expected C_1 in: {}", result);
    }

    // === Second-order constant-coefficient ===

    #[test]
    fn test_cc_distinct_real_roots() {
        // y'' + 3y' + 2y = 0 → r² + 3r + 2 = 0 → r = -1, -2
        // y = C₁e^(-x) + C₂e^(-2x)
        let result = solve_cc(1.0, 3.0, 2.0);
        assert!(result.contains("C_{1}"), "Expected C_1 in: {}", result);
        assert!(result.contains("C_{2}"), "Expected C_2 in: {}", result);
    }

    #[test]
    fn test_cc_repeated_root() {
        // y'' + 2y' + y = 0 → r² + 2r + 1 = 0 → r = -1 (repeated)
        // y = (C₁ + C₂x)e^(-x)
        let result = solve_cc(1.0, 2.0, 1.0);
        assert!(result.contains("C_{1}"), "Expected C_1 in: {}", result);
        assert!(result.contains("C_{2}"), "Expected C_2 in: {}", result);
    }

    #[test]
    fn test_cc_complex_roots() {
        // y'' + y = 0 → r² + 1 = 0 → r = ±i
        // y = C₁cos(x) + C₂sin(x)
        let result = solve_cc(1.0, 0.0, 1.0);
        assert!(result.contains("C_{1}"), "Expected C_1 in: {}", result);
        assert!(result.contains("C_{2}"), "Expected C_2 in: {}", result);
        assert!(
            result.contains("\\cos") || result.contains("cos"),
            "Expected cos in: {}",
            result
        );
        assert!(
            result.contains("\\sin") || result.contains("sin"),
            "Expected sin in: {}",
            result
        );
    }

    #[test]
    fn test_cc_complex_roots_damped() {
        // y'' + 2y' + 5y = 0 → r² + 2r + 5 = 0 → r = -1 ± 2i
        // y = e^(-x)(C₁cos(2x) + C₂sin(2x))
        let result = solve_cc(1.0, 2.0, 5.0);
        assert!(result.contains("C_{1}"), "Expected C_1 in: {}", result);
        assert!(result.contains("C_{2}"), "Expected C_2 in: {}", result);
    }
}
