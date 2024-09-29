// src/evaluator.rs
use crate::environment::Environment;
use crate::functions::call_function;
use crate::node::Node;
use crate::simplify::Simplifiable;

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
                    Ok(f64::NAN) // Return NaN for division by zero
                } else {
                    Ok(*numerator as f64 / *denominator as f64)
                }
            }
            Node::Negate(expr) => {
                let value = Self::evaluate(expr, env)?;
                Ok(-value)
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
                    Ok(f64::NAN) // Return NaN for division by zero
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
            Node::Abs(operand) => {
                let value = Self::evaluate(operand, env)?;
                Ok(value.abs())
            }
            Node::Greater(left, right) => {
                let left_val = Self::evaluate(left, env)?;
                let right_val = Self::evaluate(right, env)?;
                Ok(if left_val > right_val { 1.0 } else { 0.0 })
            }
            Node::Less(left, right) => {
                let left_val = Self::evaluate(left, env)?;
                let right_val = Self::evaluate(right, env)?;
                Ok(if left_val < right_val { 1.0 } else { 0.0 })
            }
            Node::GreaterEqual(left, right) => {
                let left_val = Self::evaluate(left, env)?;
                let right_val = Self::evaluate(right, env)?;
                Ok(if left_val >= right_val { 1.0 } else { 0.0 })
            }
            Node::LessEqual(left, right) => {
                let left_val = Self::evaluate(left, env)?;
                let right_val = Self::evaluate(right, env)?;
                Ok(if left_val <= right_val { 1.0 } else { 0.0 })
            }
            Node::Equal(left, right) => {
                let left_val = Self::evaluate(left, env)?;
                let right_val = Self::evaluate(right, env)?;
                Ok(if left_val == right_val { 1.0 } else { 0.0 })
            }
            Node::Piecewise(conditions) => {
                for (expr, cond) in conditions {
                    let cond_val = Self::evaluate(cond, env)?;
                    if cond_val == 1.0 {
                        return Self::evaluate(expr, env);
                    }
                }
                Err("No condition in Piecewise expression evaluated to true.".to_string())
            }
            Node::Function(ref name, ref args) => {
                // Evaluate the arguments first
                let mut evaluated_args = Vec::new();
                for arg in args {
                    evaluated_args.push(Self::evaluate(arg, env)?);
                }

                // Call the function using the centralized registry
                call_function(name, evaluated_args)
            }
            Node::ClosingParen | Node::ClosingBrace => {
                Err("Unexpected closing delimiter.".to_string())
            } // Add this match arm to return error
        }
    }

    pub fn simplify(node: &Node, env: &Environment) -> Result<Node, String> {
        node.simplify(env) // Delegate simplification to the node
    }
}
