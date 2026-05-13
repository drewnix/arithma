use crate::exact::ExactNum;
use crate::node::Node;
use crate::polynomial::Polynomial;
use crate::Tokenizer;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{Signed, Zero};

pub fn extract_variable(expr: &str) -> Option<String> {
    let mut tokenizer = Tokenizer::new(expr);
    let tokens = tokenizer.tokenize();
    tokens
        .into_iter()
        .find(|token| token.chars().all(char::is_alphabetic))
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

fn solve_polynomial(expr: &Node, target_var: &str) -> Result<Vec<ExactNum>, String> {
    let equation_expr = if let Node::Equation(left, right) = expr {
        Node::Subtract(left.clone(), right.clone())
    } else {
        expr.clone()
    };

    let env = crate::environment::Environment::new();
    let simplified = crate::simplify::Simplifiable::simplify(&equation_expr, &env)
        .unwrap_or(equation_expr);

    let poly = Polynomial::from_node(&simplified, target_var)
        .map_err(|e| format!("Cannot convert to polynomial: {}", e))?;

    match poly.degree() {
        None => {
            // Zero polynomial — every value is a solution
            Err("Equation is trivially true for all values".to_string())
        }
        Some(0) => {
            // Nonzero constant — no solutions
            Err("No solution (contradiction)".to_string())
        }
        Some(1) => {
            // ax + b = 0  →  x = -b/a
            let a = poly.coeff(1);
            let b = poly.coeff(0);
            let root = -b / a;
            Ok(vec![rational_to_exact(&root)])
        }
        Some(2) => {
            // ax² + bx + c = 0  →  x = (-b ± √(b²-4ac)) / 2a
            let a = poly.coeff(2);
            let b = poly.coeff(1);
            let c = poly.coeff(0);
            let discriminant = &b * &b - BigRational::from_integer(BigInt::from(4)) * &a * &c;

            if discriminant.is_negative() {
                return Err("No real solutions (negative discriminant)".to_string());
            }

            if discriminant.is_zero() {
                let root = -b / (BigRational::from_integer(BigInt::from(2)) * a);
                return Ok(vec![rational_to_exact(&root)]);
            }

            // Check if discriminant is a perfect square
            let two_a = BigRational::from_integer(BigInt::from(2)) * &a;
            if let Some(sqrt_d) = exact_rational_sqrt(&discriminant) {
                let r1 = (-&b + &sqrt_d) / &two_a;
                let r2 = (-&b - &sqrt_d) / &two_a;
                Ok(vec![rational_to_exact(&r1), rational_to_exact(&r2)])
            } else {
                let disc_f64 = discriminant.numer().to_string().parse::<f64>().unwrap_or(0.0)
                    / discriminant.denom().to_string().parse::<f64>().unwrap_or(1.0);
                let sqrt_d = disc_f64.sqrt();
                let b_f64 = b.numer().to_string().parse::<f64>().unwrap_or(0.0)
                    / b.denom().to_string().parse::<f64>().unwrap_or(1.0);
                let two_a_f64 = two_a.numer().to_string().parse::<f64>().unwrap_or(0.0)
                    / two_a.denom().to_string().parse::<f64>().unwrap_or(1.0);
                let r1 = (-b_f64 + sqrt_d) / two_a_f64;
                let r2 = (-b_f64 - sqrt_d) / two_a_f64;
                Ok(vec![ExactNum::from_f64(r1), ExactNum::from_f64(r2)])
            }
        }
        Some(d) => Err(format!(
            "Polynomial degree {} — only linear and quadratic equations are supported",
            d
        )),
    }
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
