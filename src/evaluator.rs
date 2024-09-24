// src/evaluator.rs
use crate::environment::Environment;
use crate::node::Node;

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
                // Assuming unary functions for now (single argument)
                if args.len() != 1 {
                    return Err(format!("Function '{}' requires exactly one argument", name));
                }

                let arg_value = Self::evaluate(&args[0], env)?;
                match name.as_str() {
                    "sin" => Ok(arg_value.sin()),
                    "cos" => Ok(arg_value.cos()),
                    "ln" => Ok(arg_value.ln()),     // Natural logarithm (base 'e')
                    "log" => Ok(arg_value.log10()), // Common logarithm (base 10)
                    "lg" => Ok(arg_value.log2()),   // Binary logarithm (base 2)
                    "exp" => Ok(arg_value.exp()), // e^x
                    "sqrt" => {
                        if arg_value < 0.0 {
                            Err("Square root of a negative number is not supported.".to_string())
                        } else {
                            Ok(arg_value.sqrt())
                        }
                    }
                    _ => Err(format!("Unsupported function '{}'", name)),
                }
            }
        }
    }
}
