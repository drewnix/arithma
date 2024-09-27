use crate::environment::Environment;
use crate::node::Node;

pub trait Simplifiable {
    fn simplify(&self, env: &Environment) -> Result<Node, String>;
}

impl Simplifiable for Node {
    fn simplify(&self, env: &Environment) -> Result<Node, String> {
        match self {
            Node::Add(left, right) => {
                let left_simplified = left.simplify(env)?;
                let right_simplified = right.simplify(env)?;

                // If both sides are numbers, return the result of the addition
                if let (Node::Number(l), Node::Number(r)) = (&left_simplified, &right_simplified) {
                    return Ok(Node::Number(l + r));
                }

                // Combine like terms if both sides are the same variable multiplied by coefficients (e.g., 2x + 3x)
                if let (Node::Multiply(l_boxed_left, l_boxed_right), Node::Multiply(r_boxed_left, r_boxed_right)) =
                    (&left_simplified, &right_simplified)
                {
                    if let (Node::Number(l_coef), Node::Variable(ref l_var)) = (&**l_boxed_left, &**l_boxed_right) {
                        if let (Node::Number(r_coef), Node::Variable(ref r_var)) = (&**r_boxed_left, &**r_boxed_right) {
                            if l_var == r_var {
                                let new_coef = l_coef + r_coef;
                                return Ok(Node::Multiply(
                                    Box::new(Node::Number(new_coef)),
                                    Box::new(Node::Variable(l_var.clone())),
                                ));
                            }
                        }
                    }
                }

                // Handle adding a variable and a term with a coefficient (e.g., x + 2x)
                if let (Node::Variable(ref l_var), Node::Multiply(r_boxed_left, r_boxed_right)) =
                    (&left_simplified, &right_simplified)
                {
                    if let (Node::Number(r_coef), Node::Variable(ref r_var)) = (&**r_boxed_left, &**r_boxed_right) {
                        if l_var == r_var {
                            return Ok(Node::Multiply(
                                Box::new(Node::Number(1.0 + r_coef)), // Combine coefficients
                                Box::new(Node::Variable(l_var.clone())),
                            ));
                        }
                    }
                }

                // Handle the reverse case: 2x + x
                if let (Node::Multiply(l_boxed_left, l_boxed_right), Node::Variable(ref r_var)) =
                    (&left_simplified, &right_simplified)
                {
                    if let (Node::Number(l_coef), Node::Variable(ref l_var)) = (&**l_boxed_left, &**l_boxed_right) {
                        if l_var == r_var {
                            return Ok(Node::Multiply(
                                Box::new(Node::Number(l_coef + 1.0)), // Combine coefficients
                                Box::new(Node::Variable(l_var.clone())),
                            ));
                        }
                    }
                }

                // Otherwise, return the simplified addition node
                Ok(Node::Add(
                    Box::new(left_simplified),
                    Box::new(right_simplified),
                ))
            }
            Node::Rational(numerator, denominator) => {
                if *denominator == 0 {
                    return Ok(Node::Number(f64::NAN));
                }

                // Zero in the numerator
                if *numerator == 0 {
                    return Ok(Node::Number(0.0));
                }

                let (simplified_num, simplified_den) = simplify_fraction(*numerator, *denominator);
                if simplified_den == 1 {
                    Ok(Node::Number(simplified_num as f64))
                } else {
                    Ok(Node::Rational(simplified_num, simplified_den))
                }
            }
            Node::Multiply(left, right) => {
                let left_simplified = left.simplify(env)?;
                let right_simplified = right.simplify(env)?;

                // Handle multiplication by zero
                if let Node::Number(0.0) = left_simplified {
                    return Ok(Node::Number(0.0));
                }
                if let Node::Number(0.0) = right_simplified {
                    return Ok(Node::Number(0.0));
                }

                // Multiplying by one
                if let Node::Number(1.0) = left_simplified {
                    return Ok(right_simplified);
                }
                if let Node::Number(1.0) = right_simplified {
                    return Ok(left_simplified);
                }

                // If both are rational numbers, multiply them directly
                if let (Node::Rational(l_num, l_den), Node::Rational(r_num, r_den)) = (&left_simplified, &right_simplified) {
                    let new_num = l_num * r_num;
                    let new_den = l_den * r_den;
                    let (simplified_num, simplified_den) = simplify_fraction(new_num, new_den);
                    return Ok(Node::Rational(simplified_num, simplified_den));
                }

                // **Handle implicit multiplication of number and variable (e.g., 5 * x -> 5x)**
                if let (Node::Number(l_coef), Node::Variable(ref var)) = (&left_simplified, &right_simplified) {
                    return Ok(Node::Multiply(
                        Box::new(Node::Number(*l_coef)),
                        Box::new(Node::Variable(var.clone())),
                    ));
                }
                if let (Node::Variable(ref var), Node::Number(r_coef)) = (&left_simplified, &right_simplified) {
                    return Ok(Node::Multiply(
                        Box::new(Node::Number(*r_coef)),
                        Box::new(Node::Variable(var.clone())),
                    ));
                }

                // If no simplification is possible, return the simplified node
                Ok(Node::Multiply(
                    Box::new(left_simplified),
                    Box::new(right_simplified),
                ))
            }
            Node::Divide(left, right) => {
                let left_simplified = left.simplify(env)?;
                let right_simplified = right.simplify(env)?;

                // Division by one
                if let Node::Number(1.0) = right_simplified {
                    return Ok(left_simplified);
                }

                // If no special simplifications apply, return simplified Divide
                Ok(Node::Divide(
                    Box::new(left_simplified),
                    Box::new(right_simplified),
                ))
            }
            // Simplify exponentiation
            Node::Power(base, exponent) => {
                let base_simplified = base.simplify(env)?;
                let exponent_simplified = exponent.simplify(env)?;

                // Exponentiation by zero
                if let Node::Number(0.0) = exponent_simplified {
                    return Ok(Node::Number(1.0));
                }

                // Exponentiation by one
                if let Node::Number(1.0) = exponent_simplified {
                    return Ok(base_simplified);
                }

                // If no special simplifications apply, return simplified Power
                Ok(Node::Power(
                    Box::new(base_simplified),
                    Box::new(exponent_simplified),
                ))
            }
            _ => Ok(self.clone()),
        }
    }
}

fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 {
        a.abs()
    } else {
        gcd(b, a % b)
    }
}

fn simplify_fraction(numerator: i64, denominator: i64) -> (i64, i64) {
    let gcd_value = gcd(numerator, denominator);
    (numerator / gcd_value, denominator / gcd_value)
}
