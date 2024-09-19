// src/evaluator.rs
use crate::node::Node;
use crate::environment::Environment;

pub struct Evaluator;

impl Evaluator {
    // Evaluate a Node with the given environment
    pub fn evaluate(node: &Node, env: &Environment) -> Result<f64, String> {
        match node {
            Node::Number(n) => Ok(*n),
            Node::Variable(ref var) => {
                if let Some(val) = env.get(var) {
                    Ok(val)
                } else {
                    Err(format!("Variable '{}' is not defined.", var))
                }
            }
            Node::Rational(numerator, denominator) => {
                if *denominator == 0 {
                    Err("Division by zero in Rational".to_string())
                } else {
                    Ok(*numerator as f64 / *denominator as f64)
                }
            }
            Node::Add(left, right) => {
                let left_val = Self::evaluate(left, env)?;
                let right_val = Self::evaluate(right, env)?;
                Ok(left_val + right_val)
            }
            Node::Subtract(left, right) => {
                let left_val = Self::evaluate(left, env)?;
                let right_val = Self::evaluate(right, env)?;
                Ok(left_val - right_val)
            }
            Node::Multiply(left, right) => {
                let left_val = Self::evaluate(left, env)?;
                let right_val = Self::evaluate(right, env)?;
                Ok(left_val * right_val)
            }
            Node::Divide(left, right) => {
                let left_val = Self::evaluate(left, env)?;
                let right_val = Self::evaluate(right, env)?;
                if right_val == 0.0 {
                    Err("Division by zero.".to_string())
                } else {
                    Ok(left_val / right_val)
                }
            }
            Node::Power(left, right) => {
                let left_val = Self::evaluate(left, env)?;
                let right_val = Self::evaluate(right, env)?;
                Ok(left_val.powf(right_val))
            }
            Node::Sqrt(operand) => {
                let value = Self::evaluate(operand, env)?;
                if value < 0.0 {
                    Err("Square root of negative number is not supported.".to_string())
                } else {
                    Ok(value.sqrt())
                }
            }
        }
    }
}