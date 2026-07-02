use crate::exact::ExactNum;
use crate::node::Node;
use crate::polynomial::Polynomial;
use crate::Tokenizer;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{Signed, ToPrimitive, Zero};

pub fn extract_variable(expr: &str) -> Option<String> {
    let mut tokenizer = Tokenizer::new(expr);
    let tokens = tokenizer.tokenize();
    tokens
        .into_iter()
        .find(|token| token.chars().all(char::is_alphabetic))
}

#[derive(Debug)]
pub struct SolveResult {
    pub solutions: Vec<Node>,
    pub complex_omitted: usize,
}

pub fn solve_full(expr: &Node, target_var: &str) -> Result<SolveResult, String> {
    let equation_expr = if let Node::Equation(left, right) = expr {
        Node::Subtract(left.clone(), right.clone())
    } else {
        expr.clone()
    };

    let env = crate::environment::Environment::new();
    let simplified =
        crate::simplify::Simplifiable::simplify(&equation_expr, &env).unwrap_or(equation_expr);

    let poly = match Polynomial::from_node(&simplified, target_var) {
        Ok(p) => Some(p),
        Err(_) => {
            if let Some(cleared) = try_clear_denominators(&simplified, target_var) {
                let cleared_simplified =
                    crate::simplify::Simplifiable::simplify(&cleared, &env).unwrap_or(cleared);
                Polynomial::from_node(&cleared_simplified, target_var).ok()
            } else {
                None
            }
        }
    };

    if let Some(poly) = poly {
        let degree = poly.degree().unwrap_or(0);

        if degree == 0 {
            if poly.coeff(0).is_zero() {
                return Err("Equation is trivially true for all values".to_string());
            } else {
                return Err("No solution (contradiction)".to_string());
            }
        }

        let solutions = solve_polynomial_nodes(expr, target_var).unwrap_or_default();
        let complex_omitted = degree.saturating_sub(solutions.len());

        Ok(SolveResult {
            solutions,
            complex_omitted,
        })
    } else {
        let solutions = solve_polynomial_nodes(expr, target_var)?;
        Ok(SolveResult {
            solutions,
            complex_omitted: 0,
        })
    }
}

pub fn solve_for_variable(expr: &Node, target_var: &str) -> Result<f64, String> {
    let solutions = solve_polynomial(expr, target_var)?;
    if solutions.is_empty() {
        return Err("No real solutions".to_string());
    }
    Ok(solutions[0].to_f64())
}

pub fn solve_for_variable_exact(expr: &Node, target_var: &str) -> Result<Vec<ExactNum>, String> {
    solve_polynomial(expr, target_var)
}

pub fn solve_for_variable_nodes(expr: &Node, target_var: &str) -> Result<Vec<Node>, String> {
    solve_polynomial_nodes(expr, target_var)
}

fn solve_polynomial(expr: &Node, target_var: &str) -> Result<Vec<ExactNum>, String> {
    let equation_expr = if let Node::Equation(left, right) = expr {
        Node::Subtract(left.clone(), right.clone())
    } else {
        expr.clone()
    };

    let env = crate::environment::Environment::new();
    let simplified =
        crate::simplify::Simplifiable::simplify(&equation_expr, &env).unwrap_or(equation_expr);

    let poly = match Polynomial::from_node(&simplified, target_var) {
        Ok(p) => p,
        Err(orig_err) => {
            if let Some(cleared) = try_clear_denominators(&simplified, target_var) {
                let cleared_simplified =
                    crate::simplify::Simplifiable::simplify(&cleared, &env).unwrap_or(cleared);
                Polynomial::from_node(&cleared_simplified, target_var)
                    .map_err(|e| format!("Cannot convert to polynomial: {}", e))?
            } else {
                return Err(format!("Cannot convert to polynomial: {}", orig_err));
            }
        }
    };

    match poly.degree() {
        None => return Err("Equation is trivially true for all values".to_string()),
        Some(0) => return Err("No solution (contradiction)".to_string()),
        _ => {}
    }

    let mut roots: Vec<ExactNum> = Vec::new();

    let rat_roots = poly.rational_roots();
    let mut remaining = poly.clone();
    for root in &rat_roots {
        roots.push(rational_to_exact(root));
        remaining = remaining.deflate(root);
    }

    match remaining.degree() {
        None | Some(0) => {}
        Some(1) => {
            let a = remaining.coeff(1);
            let b = remaining.coeff(0);
            let root = -b / a;
            roots.push(rational_to_exact(&root));
        }
        Some(2) => {
            roots.extend(solve_quadratic(&remaining)?);
        }
        Some(3) => {
            roots.extend(solve_cubic_cardano(&remaining));
        }
        Some(4) => {
            roots.extend(solve_quartic_ferrari(&remaining));
        }
        Some(d) => {
            // Factor the remaining polynomial into irreducible pieces
            // and solve each piece that has degree ≤ 4.
            let (_, factors) = crate::mod_poly::factor_over_q(&remaining);
            let mut found_any = false;
            for factor in &factors {
                match factor.degree() {
                    Some(1) => {
                        let a = factor.coeff(1);
                        let b = factor.coeff(0);
                        let root = -b / a;
                        roots.push(rational_to_exact(&root));
                        found_any = true;
                    }
                    Some(2) => {
                        if let Ok(qr) = solve_quadratic(factor) {
                            found_any = found_any || !qr.is_empty();
                            roots.extend(qr);
                        }
                    }
                    Some(3) => {
                        let cr = solve_cubic_cardano(factor);
                        found_any = found_any || !cr.is_empty();
                        roots.extend(cr);
                    }
                    Some(4) => {
                        let qr = solve_quartic_ferrari(factor);
                        found_any = found_any || !qr.is_empty();
                        roots.extend(qr);
                    }
                    _ => {}
                }
            }
            if !found_any && roots.is_empty() {
                return Err(format!(
                    "Polynomial degree {} — irreducible factors of degree > 4 have no closed-form solution",
                    d
                ));
            }
        }
    }

    if roots.is_empty() {
        return Err("No real solutions".to_string());
    }

    let mut unique = Vec::new();
    for root in roots {
        if !unique.contains(&root) {
            unique.push(root);
        }
    }

    Ok(unique)
}

fn solve_polynomial_nodes(expr: &Node, target_var: &str) -> Result<Vec<Node>, String> {
    let equation_expr = if let Node::Equation(left, right) = expr {
        Node::Subtract(left.clone(), right.clone())
    } else {
        expr.clone()
    };

    let env = crate::environment::Environment::new();
    let simplified =
        crate::simplify::Simplifiable::simplify(&equation_expr, &env).unwrap_or(equation_expr);

    let poly = match Polynomial::from_node(&simplified, target_var) {
        Ok(p) => p,
        Err(orig_err) => {
            if let Some(cleared) = try_clear_denominators(&simplified, target_var) {
                let cleared_simplified =
                    crate::simplify::Simplifiable::simplify(&cleared, &env).unwrap_or(cleared);
                match Polynomial::from_node(&cleared_simplified, target_var) {
                    Ok(p) => p,
                    Err(_) => {
                        if let Some(roots) = try_solve_parametric(&cleared_simplified, target_var) {
                            return Ok(roots);
                        }
                        return Err(format!("Cannot convert to polynomial: {}", orig_err));
                    }
                }
            } else {
                if let Some(roots) = try_solve_parametric(&simplified, target_var) {
                    return Ok(roots);
                }
                return Err(format!("Cannot convert to polynomial: {}", orig_err));
            }
        }
    };

    match poly.degree() {
        None => return Err("Equation is trivially true for all values".to_string()),
        Some(0) => return Err("No solution (contradiction)".to_string()),
        _ => {}
    }

    let mut roots: Vec<Node> = Vec::new();

    let rat_roots = poly.rational_roots();
    let mut remaining = poly.clone();
    for root in &rat_roots {
        roots.push(Node::Num(rational_to_exact(root)));
        remaining = remaining.deflate(root);
    }

    match remaining.degree() {
        None | Some(0) => {}
        Some(1) => {
            let a = remaining.coeff(1);
            let b = remaining.coeff(0);
            let root = -b / a;
            roots.push(Node::Num(rational_to_exact(&root)));
        }
        Some(2) => {
            roots.extend(solve_quadratic_nodes(&remaining)?);
        }
        Some(3) => {
            roots.extend(solve_cubic_cardano(&remaining).into_iter().map(Node::Num));
        }
        Some(4) => {
            roots.extend(solve_quartic_ferrari(&remaining).into_iter().map(Node::Num));
        }
        Some(d) => {
            let (_, factors) = crate::mod_poly::factor_over_q(&remaining);
            let mut found_any = false;
            for factor in &factors {
                match factor.degree() {
                    Some(1) => {
                        let a = factor.coeff(1);
                        let b = factor.coeff(0);
                        let root = -b / a;
                        roots.push(Node::Num(rational_to_exact(&root)));
                        found_any = true;
                    }
                    Some(2) => {
                        if let Ok(qr) = solve_quadratic_nodes(factor) {
                            found_any = found_any || !qr.is_empty();
                            roots.extend(qr);
                        }
                    }
                    Some(3) => {
                        let cr = solve_cubic_cardano(factor);
                        found_any = found_any || !cr.is_empty();
                        roots.extend(cr.into_iter().map(Node::Num));
                    }
                    Some(4) => {
                        let qr = solve_quartic_ferrari(factor);
                        found_any = found_any || !qr.is_empty();
                        roots.extend(qr.into_iter().map(Node::Num));
                    }
                    _ => {}
                }
            }
            if !found_any && roots.is_empty() {
                return Err(format!(
                    "Polynomial degree {} — irreducible factors of degree > 4 have no closed-form solution",
                    d
                ));
            }
        }
    }

    if roots.is_empty() {
        return Err("No real solutions".to_string());
    }

    // Deduplicate by display representation
    let mut unique = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for root in roots {
        let key = format!("{}", root);
        if seen.insert(key) {
            unique.push(root);
        }
    }

    Ok(unique)
}

fn solve_quadratic(poly: &Polynomial) -> Result<Vec<ExactNum>, String> {
    let a = poly.coeff(2);
    let b = poly.coeff(1);
    let c = poly.coeff(0);
    let discriminant = &b * &b - BigRational::from_integer(BigInt::from(4)) * &a * &c;

    if discriminant.is_negative() {
        return Ok(vec![]);
    }

    if discriminant.is_zero() {
        let root = -b / (BigRational::from_integer(BigInt::from(2)) * a);
        return Ok(vec![rational_to_exact(&root)]);
    }

    let two_a = BigRational::from_integer(BigInt::from(2)) * &a;
    if let Some(sqrt_d) = exact_rational_sqrt(&discriminant) {
        let r1 = (-&b + &sqrt_d) / &two_a;
        let r2 = (-&b - &sqrt_d) / &two_a;
        Ok(vec![rational_to_exact(&r1), rational_to_exact(&r2)])
    } else {
        let disc_f64 = rational_to_f64(&discriminant);
        let sqrt_d = disc_f64.sqrt();
        let b_f64 = rational_to_f64(&b);
        let two_a_f64 = rational_to_f64(&two_a);
        let r1 = (-b_f64 + sqrt_d) / two_a_f64;
        let r2 = (-b_f64 - sqrt_d) / two_a_f64;
        Ok(vec![ExactNum::from_f64(r1), ExactNum::from_f64(r2)])
    }
}

fn solve_quadratic_nodes(poly: &Polynomial) -> Result<Vec<Node>, String> {
    let a = poly.coeff(2);
    let b = poly.coeff(1);
    let c = poly.coeff(0);
    let discriminant = &b * &b - BigRational::from_integer(BigInt::from(4)) * &a * &c;

    if discriminant.is_negative() {
        return Ok(vec![]);
    }

    if discriminant.is_zero() {
        let root = -&b / (BigRational::from_integer(BigInt::from(2)) * &a);
        return Ok(vec![Node::Num(rational_to_exact(&root))]);
    }

    let two_a = BigRational::from_integer(BigInt::from(2)) * &a;

    if let Some(sqrt_d) = exact_rational_sqrt(&discriminant) {
        // Perfect square discriminant: exact rational roots
        let r1 = (-&b + &sqrt_d) / &two_a;
        let r2 = (-&b - &sqrt_d) / &two_a;
        Ok(vec![
            Node::Num(rational_to_exact(&r1)),
            Node::Num(rational_to_exact(&r2)),
        ])
    } else {
        // Build symbolic: (-b ± √d) / (2a)
        let env = crate::environment::Environment::new();
        let neg_b = rational_to_node(&(-&b));
        let sqrt_d = Node::Sqrt(Box::new(rational_to_node(&discriminant)));
        let denom = rational_to_node(&two_a);

        let r1 = Node::Divide(
            Box::new(Node::Add(Box::new(neg_b.clone()), Box::new(sqrt_d.clone()))),
            Box::new(denom.clone()),
        );
        let r2 = Node::Divide(
            Box::new(Node::Subtract(Box::new(neg_b), Box::new(sqrt_d))),
            Box::new(denom),
        );

        // Simplify to clean up (e.g., 0 + √2 → √2, or 2/2 cancellation)
        let r1 = crate::simplify::Simplifiable::simplify(&r1, &env).unwrap_or(r1);
        let r2 = crate::simplify::Simplifiable::simplify(&r2, &env).unwrap_or(r2);
        Ok(vec![r1, r2])
    }
}

fn rational_to_node(r: &BigRational) -> Node {
    if r.is_integer() {
        if let Some(n) = r.numer().to_i64() {
            return Node::Num(ExactNum::integer(n));
        }
    }
    if let (Some(n), Some(d)) = (r.numer().to_i64(), r.denom().to_i64()) {
        Node::Num(ExactNum::rational(n, d))
    } else {
        // Fallback for large numbers
        Node::Num(ExactNum::from_f64(rational_to_f64(r)))
    }
}

/// Cardano's formula for an irreducible cubic (no rational roots).
fn solve_cubic_cardano(poly: &Polynomial) -> Vec<ExactNum> {
    solve_cubic_f64(
        rational_to_f64(&poly.coeff(3)),
        rational_to_f64(&poly.coeff(2)),
        rational_to_f64(&poly.coeff(1)),
        rational_to_f64(&poly.coeff(0)),
    )
    .into_iter()
    .map(ExactNum::from_f64)
    .collect()
}

/// Solve a3·x³ + a2·x² + a1·x + a0 = 0 using Cardano's formula.
/// Returns all real roots as f64 values.
pub fn solve_cubic_f64_pub(a3: f64, a2: f64, a1: f64, a0: f64) -> Vec<f64> {
    solve_cubic_f64(a3, a2, a1, a0)
}

pub fn solve_quartic_f64_pub(a4: f64, a3: f64, a2: f64, a1: f64, a0: f64) -> Vec<f64> {
    let b = a3 / a4;
    let c = a2 / a4;
    let d = a1 / a4;
    let e = a0 / a4;

    let shift = b / 4.0;
    let p = c - 3.0 * b * b / 8.0;
    let q = d - b * c / 2.0 + b * b * b / 8.0;
    let r = e - b * d / 4.0 + b * b * c / 16.0 - 3.0 * b.powi(4) / 256.0;

    if q.abs() < 1e-14 {
        let disc = p * p - 4.0 * r;
        let mut roots = Vec::new();
        if disc >= -1e-10 {
            let sqrt_disc = disc.max(0.0).sqrt();
            let z1 = (-p + sqrt_disc) / 2.0;
            let z2 = (-p - sqrt_disc) / 2.0;
            for z in [z1, z2] {
                if z >= -1e-10 {
                    let y = z.max(0.0).sqrt();
                    roots.push(y - shift);
                    if y.abs() > 1e-10 {
                        roots.push(-y - shift);
                    }
                }
            }
        }
        return roots;
    }

    let resolvent = solve_cubic_f64(8.0, -4.0 * p, -8.0 * r, 4.0 * p * r - q * q);
    let m = resolvent
        .iter()
        .copied()
        .find(|&m| 2.0 * m - p > 1e-14)
        .unwrap_or(resolvent[0]);

    let alpha_sq = 2.0 * m - p;
    if alpha_sq < 0.0 {
        return vec![];
    }
    let alpha = alpha_sq.sqrt();
    let beta = -q / (2.0 * alpha);

    let mut roots = Vec::new();
    let disc1 = alpha * alpha - 4.0 * (m - beta);
    if disc1 >= -1e-10 {
        let disc1 = disc1.max(0.0).sqrt();
        roots.push((alpha + disc1) / 2.0 - shift);
        roots.push((alpha - disc1) / 2.0 - shift);
    }
    let disc2 = alpha * alpha - 4.0 * (m + beta);
    if disc2 >= -1e-10 {
        let disc2 = disc2.max(0.0).sqrt();
        roots.push((-alpha + disc2) / 2.0 - shift);
        roots.push((-alpha - disc2) / 2.0 - shift);
    }
    roots
}

fn solve_cubic_f64(a3: f64, a2: f64, a1: f64, a0: f64) -> Vec<f64> {
    let shift = a2 / (3.0 * a3);
    let p = (a1 / a3) - (a2 * a2) / (3.0 * a3 * a3);
    let q = (a0 / a3) + (2.0 * a2 * a2 * a2) / (27.0 * a3 * a3 * a3) - (a2 * a1) / (3.0 * a3 * a3);

    let h = q * q / 4.0 + p * p * p / 27.0;

    let depressed_roots = if h.abs() < 1e-14 && p.abs() < 1e-14 && q.abs() < 1e-14 {
        vec![0.0]
    } else if h.abs() < 1e-14 {
        let t1 = 3.0 * q / p;
        let t2 = -3.0 * q / (2.0 * p);
        vec![t1, t2]
    } else if h > 0.0 {
        let sqrt_h = h.sqrt();
        let s = cbrt(-q / 2.0 + sqrt_h);
        let t = cbrt(-q / 2.0 - sqrt_h);
        vec![s + t]
    } else {
        let m = 2.0 * (-p / 3.0).sqrt();
        let theta = (1.0 / 3.0) * (3.0 * q / (p * m)).acos();
        let pi = std::f64::consts::PI;
        vec![
            m * theta.cos(),
            m * (theta + 2.0 * pi / 3.0).cos(),
            m * (theta + 4.0 * pi / 3.0).cos(),
        ]
    };

    depressed_roots.into_iter().map(|t| t - shift).collect()
}

/// Ferrari's method for an irreducible quartic (no rational roots).
fn solve_quartic_ferrari(poly: &Polynomial) -> Vec<ExactNum> {
    let a4 = rational_to_f64(&poly.coeff(4));
    let a3 = rational_to_f64(&poly.coeff(3));
    let a2 = rational_to_f64(&poly.coeff(2));
    let a1 = rational_to_f64(&poly.coeff(1));
    let a0 = rational_to_f64(&poly.coeff(0));

    let b = a3 / a4;
    let c = a2 / a4;
    let d = a1 / a4;
    let e = a0 / a4;

    // Depressed quartic y⁴ + py² + qy + r = 0 via x = y - b/4
    let shift = b / 4.0;
    let p = c - 3.0 * b * b / 8.0;
    let q = d - b * c / 2.0 + b * b * b / 8.0;
    let r = e - b * d / 4.0 + b * b * c / 16.0 - 3.0 * b.powi(4) / 256.0;

    if q.abs() < 1e-14 {
        return solve_biquadratic_f64(p, r, shift);
    }

    // Resolvent cubic: 8m³ - 4pm² - 8rm + (4pr - q²) = 0
    let resolvent = solve_cubic_f64(8.0, -4.0 * p, -8.0 * r, 4.0 * p * r - q * q);

    let m = resolvent
        .iter()
        .copied()
        .find(|&m| 2.0 * m - p > 1e-14)
        .unwrap_or(resolvent[0]);

    let alpha_sq = 2.0 * m - p;
    if alpha_sq < 0.0 {
        return vec![];
    }
    let alpha = alpha_sq.sqrt();
    let beta = -q / (2.0 * alpha);

    let mut roots = Vec::new();

    // First quadratic: y² - αy + (m - β) = 0
    let disc1 = alpha * alpha - 4.0 * (m - beta);
    if disc1 >= -1e-10 {
        let disc1 = disc1.max(0.0).sqrt();
        roots.push((alpha + disc1) / 2.0);
        roots.push((alpha - disc1) / 2.0);
    }

    // Second quadratic: y² + αy + (m + β) = 0
    let disc2 = alpha * alpha - 4.0 * (m + beta);
    if disc2 >= -1e-10 {
        let disc2 = disc2.max(0.0).sqrt();
        roots.push((-alpha + disc2) / 2.0);
        roots.push((-alpha - disc2) / 2.0);
    }

    roots
        .into_iter()
        .map(|y| ExactNum::from_f64(y - shift))
        .collect()
}

fn solve_biquadratic_f64(p: f64, r: f64, shift: f64) -> Vec<ExactNum> {
    let disc = p * p - 4.0 * r;
    let mut roots = Vec::new();
    if disc >= -1e-10 {
        let sqrt_disc = disc.max(0.0).sqrt();
        let z1 = (-p + sqrt_disc) / 2.0;
        let z2 = (-p - sqrt_disc) / 2.0;
        if z1 >= -1e-10 {
            let y = z1.max(0.0).sqrt();
            roots.push(ExactNum::from_f64(y - shift));
            if y.abs() > 1e-10 {
                roots.push(ExactNum::from_f64(-y - shift));
            }
        }
        if z2 >= -1e-10 && (z2 - z1).abs() > 1e-10 {
            let y = z2.max(0.0).sqrt();
            roots.push(ExactNum::from_f64(y - shift));
            if y.abs() > 1e-10 {
                roots.push(ExactNum::from_f64(-y - shift));
            }
        }
    }
    roots
}

fn cbrt(x: f64) -> f64 {
    if x >= 0.0 {
        x.powf(1.0 / 3.0)
    } else {
        -(-x).powf(1.0 / 3.0)
    }
}

fn rational_to_f64(r: &BigRational) -> f64 {
    r.to_f64()
        .unwrap_or_else(|| r.numer().to_f64().unwrap_or(0.0) / r.denom().to_f64().unwrap_or(1.0))
}

fn rational_to_exact(r: &BigRational) -> ExactNum {
    if r.is_integer() {
        ExactNum::integer(r.numer().try_into().unwrap_or(0))
    } else {
        ExactNum::rational(
            r.numer().try_into().unwrap_or(0),
            r.denom().try_into().unwrap_or(1),
        )
    }
}

fn exact_rational_sqrt(r: &BigRational) -> Option<BigRational> {
    if r.is_negative() {
        return None;
    }
    if r.is_zero() {
        return Some(BigRational::zero());
    }
    let n: i64 = r.numer().try_into().ok()?;
    let d: i64 = r.denom().try_into().ok()?;
    let nu = n.unsigned_abs();
    let du = d.unsigned_abs();
    let sn = (nu as f64).sqrt() as u64;
    let sd = (du as f64).sqrt() as u64;
    if sn * sn == nu && sd * sd == du {
        Some(BigRational::new(BigInt::from(sn), BigInt::from(sd)))
    } else {
        None
    }
}

fn is_effectively_zero(node: &Node) -> bool {
    match node {
        Node::Num(n) => n.is_zero(),
        Node::Negate(inner) => is_effectively_zero(inner),
        _ => false,
    }
}

fn try_solve_parametric(expr: &Node, var: &str) -> Option<Vec<Node>> {
    use crate::derivative::differentiate;
    use crate::simplify::Simplifiable;
    use crate::substitute::substitute_variable;

    let env = crate::environment::Environment::new();
    let zero = Node::Num(ExactNum::integer(0));

    let c0 = substitute_variable(expr, var, &zero)
        .ok()?
        .simplify(&env)
        .ok()?;

    let d1 = differentiate(expr, var).ok()?;
    let c1 = substitute_variable(&d1, var, &zero)
        .ok()?
        .simplify(&env)
        .ok()?;

    let d2 = differentiate(&d1, var).ok()?;
    let d2_simp = d2.simplify(&env).ok()?;

    if contains_var(&d2_simp, var) {
        return None;
    }

    let c2_raw = substitute_variable(&d2_simp, var, &zero)
        .ok()?
        .simplify(&env)
        .ok()?;

    if is_effectively_zero(&c2_raw) {
        if is_effectively_zero(&c1) {
            return None;
        }
        let sol = Node::Divide(Box::new(Node::Negate(Box::new(c0))), Box::new(c1))
            .simplify(&env)
            .ok()?;
        return Some(vec![sol]);
    }

    let c2 = Node::Divide(Box::new(c2_raw), Box::new(Node::Num(ExactNum::integer(2))))
        .simplify(&env)
        .ok()?;

    let disc = Node::Subtract(
        Box::new(Node::Power(
            Box::new(c1.clone()),
            Box::new(Node::Num(ExactNum::integer(2))),
        )),
        Box::new(Node::Multiply(
            Box::new(Node::Num(ExactNum::integer(4))),
            Box::new(Node::Multiply(Box::new(c2.clone()), Box::new(c0))),
        )),
    )
    .simplify(&env)
    .ok()?;

    let sqrt_disc = Node::Sqrt(Box::new(disc));
    let neg_c1 = Node::Negate(Box::new(c1));
    let two_c2 = Node::Multiply(Box::new(Node::Num(ExactNum::integer(2))), Box::new(c2))
        .simplify(&env)
        .ok()?;

    let sol1 = Node::Divide(
        Box::new(Node::Add(
            Box::new(neg_c1.clone()),
            Box::new(sqrt_disc.clone()),
        )),
        Box::new(two_c2.clone()),
    )
    .simplify(&env)
    .ok()?;

    let sol2 = Node::Divide(
        Box::new(Node::Subtract(Box::new(neg_c1), Box::new(sqrt_disc))),
        Box::new(two_c2),
    )
    .simplify(&env)
    .ok()?;

    if format!("{}", sol1) == format!("{}", sol2) {
        Some(vec![sol1])
    } else {
        Some(vec![sol1, sol2])
    }
}

fn contains_var(node: &Node, var: &str) -> bool {
    match node {
        Node::Variable(v) => v == var,
        Node::Num(_) => false,
        Node::Add(a, b)
        | Node::Subtract(a, b)
        | Node::Multiply(a, b)
        | Node::Divide(a, b)
        | Node::Power(a, b) => contains_var(a, var) || contains_var(b, var),
        Node::Negate(inner) | Node::Sqrt(inner) | Node::Abs(inner) | Node::Factorial(inner) => {
            contains_var(inner, var)
        }
        Node::Function(_, args) => args.iter().any(|a| contains_var(a, var)),
        Node::Equation(a, b)
        | Node::Greater(a, b)
        | Node::Less(a, b)
        | Node::GreaterEqual(a, b)
        | Node::LessEqual(a, b)
        | Node::Equal(a, b) => contains_var(a, var) || contains_var(b, var),
        Node::Summation(_, start, end, body) => {
            contains_var(start, var) || contains_var(end, var) || contains_var(body, var)
        }
        Node::Product(_, start, end, body) => {
            contains_var(start, var) || contains_var(end, var) || contains_var(body, var)
        }
        Node::Piecewise(cases) => cases
            .iter()
            .any(|(val, cond)| contains_var(val, var) || contains_var(cond, var)),
    }
}

pub(crate) fn to_rational_form(node: &Node) -> Option<(Node, Node)> {
    match node {
        Node::Num(_) | Node::Variable(_) => Some((node.clone(), Node::Num(ExactNum::one()))),
        Node::Divide(a, b) => Some((*a.clone(), *b.clone())),
        Node::Add(a, b) => {
            let (an, ad) = to_rational_form(a)?;
            let (bn, bd) = to_rational_form(b)?;
            let num = Node::Add(
                Box::new(Node::Multiply(Box::new(an), Box::new(bd.clone()))),
                Box::new(Node::Multiply(Box::new(bn), Box::new(ad.clone()))),
            );
            let den = Node::Multiply(Box::new(ad), Box::new(bd));
            Some((num, den))
        }
        Node::Subtract(a, b) => {
            let (an, ad) = to_rational_form(a)?;
            let (bn, bd) = to_rational_form(b)?;
            let num = Node::Subtract(
                Box::new(Node::Multiply(Box::new(an), Box::new(bd.clone()))),
                Box::new(Node::Multiply(Box::new(bn), Box::new(ad.clone()))),
            );
            let den = Node::Multiply(Box::new(ad), Box::new(bd));
            Some((num, den))
        }
        Node::Negate(inner) => {
            let (n, d) = to_rational_form(inner)?;
            Some((Node::Negate(Box::new(n)), d))
        }
        Node::Multiply(a, b) => {
            let (an, ad) = to_rational_form(a)?;
            let (bn, bd) = to_rational_form(b)?;
            Some((
                Node::Multiply(Box::new(an), Box::new(bn)),
                Node::Multiply(Box::new(ad), Box::new(bd)),
            ))
        }
        _ => None,
    }
}

fn try_clear_denominators(expr: &Node, var: &str) -> Option<Node> {
    let (num, den) = to_rational_form(expr)?;
    if contains_var(&den, var) {
        Some(num)
    } else {
        None
    }
}
