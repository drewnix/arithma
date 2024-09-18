use crate::environment::Environment;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Node {
    // Leaf nodes: numbers or variables
    Number(f64),
    Variable(String),
    Rational(i64, i64), // Numerator and denominator

    // Internal nodes: operators with children (operands)
    Add(Box<Node>, Box<Node>),
    Subtract(Box<Node>, Box<Node>),
    Multiply(Box<Node>, Box<Node>),
    Divide(Box<Node>, Box<Node>),
    Power(Box<Node>, Box<Node>),
}

impl Node {
    pub fn evaluate(&self, env: &Environment) -> Result<f64, String> {
        match self {
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
                let left_val = left.evaluate(env)?;
                let right_val = right.evaluate(env)?;
                Ok(left_val + right_val)
            }
            Node::Subtract(left, right) => {
                let left_val = left.evaluate(env)?;
                let right_val = right.evaluate(env)?;
                Ok(left_val - right_val)
            }
            Node::Multiply(left, right) => {
                let left_val = left.evaluate(env)?;
                let right_val = right.evaluate(env)?;
                Ok(left_val * right_val)
            }
            Node::Divide(left, right) => {
                let left_val = left.evaluate(env)?;
                let right_val = right.evaluate(env)?;
                if right_val == 0.0 {
                    Err("Division by zero.".to_string())
                } else {
                    Ok(left_val / right_val)
                }
            }
            Node::Power(left, right) => {
                let left_val = left.evaluate(env)?;
                let right_val = right.evaluate(env)?;
                Ok(left_val.powf(right_val))
            }
        }
    }
}
