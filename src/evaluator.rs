use crate::environment::Environment;
use crate::exact::ExactNum;
use crate::functions::call_function;
use crate::integer::factorial_u64;
use crate::node::Node;
use crate::simplify::Simplifiable;

pub struct Evaluator;

impl Evaluator {
    pub fn evaluate(node: &Node, env: &Environment) -> Result<f64, String> {
        Self::evaluate_exact(node, env).map(|n| n.to_f64())
    }

    pub fn evaluate_exact(node: &Node, env: &Environment) -> Result<ExactNum, String> {
        match node {
            Node::Num(n) => Ok(n.clone()),
            Node::Variable(ref var) => {
                if let Some(val) = env.get_exact(var) {
                    Ok(val.clone())
                } else if var == "π" {
                    Ok(ExactNum::Float(std::f64::consts::PI))
                } else if var == "e" {
                    Ok(ExactNum::Float(std::f64::consts::E))
                } else {
                    Err(format!("Variable '{}' is not defined.", var))
                }
            }
            Node::Negate(expr) => {
                let value = Self::evaluate_exact(expr, env)?;
                Ok(-value)
            }
            Node::Factorial(expr) => {
                let value = Self::evaluate_exact(expr, env)?;
                let n = value.to_f64();
                if n < 0.0 || n.fract() != 0.0 {
                    return Err("factorial requires a non-negative integer.".to_string());
                }
                let result = factorial_u64(n as u64)?;
                Ok(ExactNum::integer(result as i64))
            }
            Node::Add(left, right) => {
                let l = Self::evaluate_exact(left, env)?;
                let r = Self::evaluate_exact(right, env)?;
                Ok(l + r)
            }
            Node::Subtract(left, right) => {
                let l = Self::evaluate_exact(left, env)?;
                let r = Self::evaluate_exact(right, env)?;
                Ok(l - r)
            }
            Node::Multiply(left, right) => {
                let l = Self::evaluate_exact(left, env)?;
                let r = Self::evaluate_exact(right, env)?;
                Ok(l * r)
            }
            Node::Divide(left, right) => {
                let l = Self::evaluate_exact(left, env)?;
                let r = Self::evaluate_exact(right, env)?;
                Ok(l / r)
            }
            Node::Power(left, right) => {
                let l = Self::evaluate_exact(left, env)?;
                let r = Self::evaluate_exact(right, env)?;
                Ok(l.powf(&r))
            }
            Node::Sqrt(operand) => {
                let value = Self::evaluate_exact(operand, env)?;
                if value.is_negative() {
                    Err("Square root of negative number is not supported.".to_string())
                } else {
                    Ok(value.sqrt())
                }
            }
            Node::Abs(operand) => {
                let value = Self::evaluate_exact(operand, env)?;
                Ok(value.abs())
            }
            Node::Greater(left, right) => {
                let l = Self::evaluate_exact(left, env)?;
                let r = Self::evaluate_exact(right, env)?;
                Ok(if l > r {
                    ExactNum::one()
                } else {
                    ExactNum::zero()
                })
            }
            Node::Less(left, right) => {
                let l = Self::evaluate_exact(left, env)?;
                let r = Self::evaluate_exact(right, env)?;
                Ok(if l < r {
                    ExactNum::one()
                } else {
                    ExactNum::zero()
                })
            }
            Node::GreaterEqual(left, right) => {
                let l = Self::evaluate_exact(left, env)?;
                let r = Self::evaluate_exact(right, env)?;
                Ok(if l >= r {
                    ExactNum::one()
                } else {
                    ExactNum::zero()
                })
            }
            Node::LessEqual(left, right) => {
                let l = Self::evaluate_exact(left, env)?;
                let r = Self::evaluate_exact(right, env)?;
                Ok(if l <= r {
                    ExactNum::one()
                } else {
                    ExactNum::zero()
                })
            }
            Node::Equal(left, right) => {
                let l = Self::evaluate_exact(left, env)?;
                let r = Self::evaluate_exact(right, env)?;
                Ok(if l == r {
                    ExactNum::one()
                } else {
                    ExactNum::zero()
                })
            }
            Node::Equation(left, right) => {
                let l = Self::evaluate_exact(left, env)?;
                let r = Self::evaluate_exact(right, env)?;
                Ok(l - r)
            }
            Node::Summation(ref index_var, start, end, body) => {
                let start_val = Self::evaluate_exact(start, env)?;
                let end_val = Self::evaluate_exact(end, env)?;

                let start_i = start_val.to_f64() as i64;
                let end_i = end_val.to_f64() as i64;

                let mut sum_env = env.clone();
                let mut sum = ExactNum::zero();

                for i in start_i..=end_i {
                    sum_env.set_exact(index_var, ExactNum::integer(i));
                    let value = Self::evaluate_exact(body, &sum_env)?;
                    sum = sum + value;
                }

                Ok(sum)
            }
            Node::Piecewise(conditions) => {
                for (expr, cond) in conditions {
                    let cond_val = Self::evaluate_exact(cond, env)?;
                    if cond_val.is_one() {
                        return Self::evaluate_exact(expr, env);
                    }
                }
                Err("No condition in Piecewise expression evaluated to true.".to_string())
            }
            Node::Function(ref name, ref args) => {
                let mut evaluated_args = Vec::new();
                for arg in args {
                    evaluated_args.push(Self::evaluate(arg, env)?);
                }
                let result = call_function(name, evaluated_args)?;
                Ok(ExactNum::Float(result))
            }
        }
    }

    pub fn simplify(node: &Node, env: &Environment) -> Result<Node, String> {
        node.simplify(env)
    }
}
