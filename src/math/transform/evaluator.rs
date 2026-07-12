use crate::environment::Environment;
use crate::exact::ExactNum;
use crate::functions::call_function;
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
                crate::integer::factorial(&value)
                    .ok_or_else(|| "factorial requires a non-negative integer.".to_string())
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
            Node::Floor(operand) => {
                let value = Self::evaluate_exact(operand, env)?;
                Ok(value.floor())
            }
            Node::Ceil(operand) => {
                let value = Self::evaluate_exact(operand, env)?;
                Ok(value.ceil())
            }
            Node::Round(operand) => {
                let value = Self::evaluate_exact(operand, env)?;
                Ok(value.round())
            }
            Node::Trunc(operand) => {
                let value = Self::evaluate_exact(operand, env)?;
                Ok(value.trunc())
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

                let (start_i, end_i) = Self::integer_range_bounds(&start_val, &end_val, "sum")?;

                let mut sum_env = env.clone();
                let mut sum = ExactNum::zero();

                for i in start_i..=end_i {
                    sum_env.set_exact(index_var, ExactNum::integer(i));
                    let value = Self::evaluate_exact(body, &sum_env)?;
                    sum = sum + value;
                }

                Ok(sum)
            }
            Node::Product(ref index_var, start, end, body) => {
                let start_val = Self::evaluate_exact(start, env)?;
                let end_val = Self::evaluate_exact(end, env)?;

                let (start_i, end_i) = Self::integer_range_bounds(&start_val, &end_val, "product")?;

                let mut prod_env = env.clone();
                let mut product = ExactNum::one();

                for i in start_i..=end_i {
                    prod_env.set_exact(index_var, ExactNum::integer(i));
                    let value = Self::evaluate_exact(body, &prod_env)?;
                    product = product * value;
                }

                Ok(product)
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
                    evaluated_args.push(Self::evaluate_exact(arg, env)?);
                }
                call_function(name, evaluated_args)
            }
        }
    }

    pub fn simplify(node: &Node, env: &Environment) -> Result<Node, String> {
        node.simplify(env)
    }

    /// Σ/Π range bounds must be integers. Truncating (0.5 → empty range → 0,
    /// 2.7 → 2) would manufacture a value the expression never had — which
    /// numeric samplers then serialize inside "counterexamples". An empty
    /// integer range (start > end) is legitimate and yields the identity
    /// element; a non-integer bound is an error.
    fn integer_range_bounds(
        start: &ExactNum,
        end: &ExactNum,
        kind: &str,
    ) -> Result<(i64, i64), String> {
        let start_i = start
            .to_i64()
            .ok_or_else(|| format!("{kind} lower bound is not an integer: {start}"))?;
        let end_i = end
            .to_i64()
            .ok_or_else(|| format!("{kind} upper bound is not an integer: {end}"))?;
        Ok((start_i, end_i))
    }
}
