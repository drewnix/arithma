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

                // If both sides are the same variable, combine like terms (e.g., x + x -> 2x)
                if let (Node::Variable(ref l_var), Node::Variable(ref r_var)) =
                    (&left_simplified, &right_simplified)
                {
                    if l_var == r_var {
                        return Ok(Node::Multiply(
                            Box::new(Node::Number(2.0)),
                            Box::new(Node::Variable(l_var.clone())),
                        ));
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

                // If both are numbers, multiply them directly
                if let (Node::Number(l), Node::Number(r)) = (&left_simplified, &right_simplified) {
                    return Ok(Node::Number(l * r));
                }

                // Multiply rational numbers
                if let (Node::Rational(l_num, l_den), Node::Rational(r_num, r_den)) =
                    (&left_simplified, &right_simplified)
                {
                    let new_num = l_num * r_num;
                    let new_den = l_den * r_den;
                    let (simplified_num, simplified_den) = simplify_fraction(new_num, new_den);
                    return Ok(Node::Rational(simplified_num, simplified_den));
                }

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
