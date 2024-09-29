use crate::environment::Environment;
use crate::node::Node;
use std::collections::HashMap;

pub trait Simplifiable {
    fn simplify(&self, env: &Environment) -> Result<Node, String>;
}

impl Simplifiable for Node {
    fn simplify(&self, env: &Environment) -> Result<Node, String> {
        match self {
            Node::Add(_, _) => {
                let mut term_map: HashMap<String, f64> = HashMap::new();
                // Collect all terms from the addition node
                collect_terms(self, &mut term_map, env)?;

                // Rebuild the expression by combining like terms
                let simplified_expr = rebuild_expression(term_map);
                Ok(simplified_expr)
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
                if let (Node::Rational(l_num, l_den), Node::Rational(r_num, r_den)) =
                    (&left_simplified, &right_simplified)
                {
                    let new_num = l_num * r_num;
                    let new_den = l_den * r_den;
                    let (simplified_num, simplified_den) = simplify_fraction(new_num, new_den);
                    return Ok(Node::Rational(simplified_num, simplified_den));
                }

                // **Handle implicit multiplication of number and variable (e.g., 5 * x -> 5x)**
                if let (Node::Number(l_coef), Node::Variable(ref var)) =
                    (&left_simplified, &right_simplified)
                {
                    return Ok(Node::Multiply(
                        Box::new(Node::Number(*l_coef)),
                        Box::new(Node::Variable(var.clone())),
                    ));
                }
                if let (Node::Variable(ref var), Node::Number(r_coef)) =
                    (&left_simplified, &right_simplified)
                {
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
            Node::Power(base, exponent) => {
                let base_simplified = base.simplify(env)?;
                let exponent_simplified = exponent.simplify(env)?;

                // Exponentiation by zero
                if let Node::Number(0.0) = exponent_simplified {
                    return Ok(Node::Number(1.0)); // Anything raised to 0 is 1
                }

                // Exponentiation by one
                if let Node::Number(1.0) = exponent_simplified {
                    return Ok(base_simplified);
                }

                // If both the base and exponent are numbers, evaluate the power
                if let (Node::Number(b), Node::Number(e)) = (&base_simplified, &exponent_simplified)
                {
                    return Ok(Node::Number(b.powf(*e))); // Use powf for floating-point exponents
                }

                // If no special simplifications apply, return simplified Power
                Ok(Node::Power(
                    Box::new(base_simplified),
                    Box::new(exponent_simplified),
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

fn collect_terms(
    node: &Node,
    term_map: &mut HashMap<String, f64>,
    env: &Environment,
) -> Result<(), String> {
    match node {
        Node::Add(left, right) => {
            collect_terms(left, term_map, env)?;
            collect_terms(right, term_map, env)?;
        }
        Node::Multiply(left, right) => {
            if let (Node::Number(coef), Node::Variable(var)) = (&**left, &**right) {
                let entry = term_map.entry(var.clone()).or_insert(0.0);
                *entry += coef;
            }
        }
        Node::Variable(var) => {
            let entry = term_map.entry(var.clone()).or_insert(0.0);
            *entry += 1.0;
        }
        Node::Number(num) => {
            // For constants without variables (like `+10`), store them in the `""` key
            let entry = term_map.entry("".to_string()).or_insert(0.0);
            *entry += num;
        }
        _ => return Err("Unsupported node type in collect_terms".to_string()),
    }
    Ok(())
}

fn rebuild_expression(term_map: HashMap<String, f64>) -> Node {
    let mut terms: Vec<(String, f64)> = term_map.into_iter().collect();

    // Sort terms by the variable name (lexicographically)
    terms.sort_by(|a, b| a.0.cmp(&b.0));

    let mut result_terms: Vec<Node> = vec![];

    for (var, coef) in terms {
        if var.is_empty() {
            if coef != 0.0 {
                result_terms.push(Node::Number(coef));
            }
        } else if coef != 0.0 {
            if coef == 1.0 {
                result_terms.push(Node::Variable(var));
            } else {
                result_terms.push(Node::Multiply(
                    Box::new(Node::Number(coef)),
                    Box::new(Node::Variable(var)),
                ));
            }
        }
    }

    // Combine all terms into a single expression (iterate from start to end)
    let mut simplified_expr = result_terms.remove(0);
    for term in result_terms {
        simplified_expr = Node::Add(Box::new(simplified_expr), Box::new(term));
    }

    simplified_expr
}
