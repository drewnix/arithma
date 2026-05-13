use crate::environment::Environment;
use crate::exact::ExactNum;
use crate::node::Node;
use crate::polynomial::Polynomial;
use std::collections::HashMap;

pub trait Simplifiable {
    fn simplify(&self, env: &Environment) -> Result<Node, String>;
}

impl Simplifiable for Node {
    fn simplify(&self, env: &Environment) -> Result<Node, String> {
        match self {
            Node::Add(left, right) => {
                let left_simplified = left.simplify(env)?;
                let right_simplified = right.simplify(env)?;

                if let (Node::Num(ref l), Node::Num(ref r)) = (&left_simplified, &right_simplified)
                {
                    return Ok(Node::Num(l + r));
                }

                if let Node::Num(ref n) = left_simplified {
                    if n.is_zero() {
                        return Ok(right_simplified);
                    }
                }
                if let Node::Num(ref n) = right_simplified {
                    if n.is_zero() {
                        return Ok(left_simplified);
                    }
                }

                let result = Node::Add(Box::new(left_simplified), Box::new(right_simplified));
                let mut term_map: HashMap<String, ExactNum> = HashMap::new();
                if collect_terms(&result, &mut term_map, env).is_ok() {
                    Ok(rebuild_expression(term_map))
                } else if let Some(normalized) = try_polynomial_normalize(&result) {
                    Ok(normalized)
                } else {
                    Ok(result)
                }
            }
            Node::Num(_) => {
                // ExactNum::Rational is already in lowest terms (BigRational handles reduction).
                // Nothing to simplify for plain numbers.
                Ok(self.clone())
            }
            Node::Multiply(left, right) => {
                let left_simplified = left.simplify(env)?;
                let right_simplified = right.simplify(env)?;

                // Handle multiplication by zero
                if let Node::Num(ref n) = left_simplified {
                    if n.is_zero() {
                        return Ok(Node::Num(ExactNum::zero()));
                    }
                }
                if let Node::Num(ref n) = right_simplified {
                    if n.is_zero() {
                        return Ok(Node::Num(ExactNum::zero()));
                    }
                }

                // Multiplying by one
                if let Node::Num(ref n) = left_simplified {
                    if n.is_one() {
                        return Ok(right_simplified);
                    }
                }
                if let Node::Num(ref n) = right_simplified {
                    if n.is_one() {
                        return Ok(left_simplified);
                    }
                }

                // If both are numbers, multiply them directly
                if let (Node::Num(ref l), Node::Num(ref r)) = (&left_simplified, &right_simplified)
                {
                    return Ok(Node::Num(l * r));
                }

                // **Handle implicit multiplication of number and variable (e.g., 5 * x -> 5x)**
                if let (Node::Num(ref l_coef), Node::Variable(ref var)) =
                    (&left_simplified, &right_simplified)
                {
                    return Ok(Node::Multiply(
                        Box::new(Node::Num(l_coef.clone())),
                        Box::new(Node::Variable(var.clone())),
                    ));
                }
                if let (Node::Variable(ref var), Node::Num(ref r_coef)) =
                    (&left_simplified, &right_simplified)
                {
                    return Ok(Node::Multiply(
                        Box::new(Node::Num(r_coef.clone())),
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
                if let Node::Num(ref n) = exponent_simplified {
                    if n.is_zero() {
                        return Ok(Node::Num(ExactNum::one())); // Anything raised to 0 is 1
                    }
                }

                // Exponentiation by one
                if let Node::Num(ref n) = exponent_simplified {
                    if n.is_one() {
                        return Ok(base_simplified);
                    }
                }

                // If both the base and exponent are numbers, evaluate the power
                if let (Node::Num(ref b), Node::Num(ref e)) =
                    (&base_simplified, &exponent_simplified)
                {
                    return Ok(Node::Num(b.powf(e)));
                }

                // If no special simplifications apply, return simplified Power
                Ok(Node::Power(
                    Box::new(base_simplified),
                    Box::new(exponent_simplified),
                ))
            }
            Node::Subtract(left, right) => {
                let left_simplified = left.simplify(env)?;
                let right_simplified = right.simplify(env)?;

                if let (Node::Num(ref l), Node::Num(ref r)) = (&left_simplified, &right_simplified)
                {
                    return Ok(Node::Num(l - r));
                }

                Ok(Node::Subtract(
                    Box::new(left_simplified),
                    Box::new(right_simplified),
                ))
            }
            Node::Negate(operand) => {
                let simplified = operand.simplify(env)?;
                if let Node::Num(ref n) = simplified {
                    return Ok(Node::Num(-n.clone()));
                }
                Ok(Node::Negate(Box::new(simplified)))
            }
            Node::Divide(left, right) => {
                let left_simplified = left.simplify(env)?;
                let right_simplified = right.simplify(env)?;

                if let Node::Num(ref n) = right_simplified {
                    if n.is_one() {
                        return Ok(left_simplified);
                    }
                }

                if let (Node::Num(ref l), Node::Num(ref r)) = (&left_simplified, &right_simplified)
                {
                    return Ok(Node::Num(l / r));
                }

                if let Some(simplified) = try_polynomial_divide(&left_simplified, &right_simplified)
                {
                    return Ok(simplified);
                }

                Ok(Node::Divide(
                    Box::new(left_simplified),
                    Box::new(right_simplified),
                ))
            }

            Node::Summation(index_var, start, end, body) => {
                let start_simplified = start.simplify(env)?;
                let end_simplified = end.simplify(env)?;
                let body_simplified = body.simplify(env)?;

                // Try to evaluate if bounds are constant values
                if let (Node::Num(ref start_n), Node::Num(ref end_n)) =
                    (&start_simplified, &end_simplified)
                {
                    if start_n.is_integer() && end_n.is_integer() {
                        let start_val = start_n.to_f64();
                        let end_val = end_n.to_f64();

                        // For small ranges (fewer than 100 terms), we can expand inline
                        let range_size = (end_val - start_val + 1.0) as usize;
                        if range_size <= 10 {
                            let mut sum_node = Node::Num(ExactNum::zero());

                            // Create a temporary environment for each iteration
                            let mut sum_env = env.clone();

                            let start_i = start_val as i64;
                            let end_i = end_val as i64;

                            // Evaluate each term and add them together
                            for i in start_i..=end_i {
                                sum_env.set(index_var, i as f64);

                                // Create a substituted body for this iteration
                                let substituted_body = crate::substitute::substitute_variable(
                                    &body_simplified,
                                    index_var,
                                    &Node::Num(ExactNum::integer(i)),
                                )?;

                                // Add this term to our running sum
                                sum_node =
                                    Node::Add(Box::new(sum_node), Box::new(substituted_body));
                            }

                            return Ok(sum_node);
                        }
                    }
                }

                // If we can't or shouldn't evaluate the summation, return it with simplified components
                Ok(Node::Summation(
                    index_var.clone(),
                    Box::new(start_simplified),
                    Box::new(end_simplified),
                    Box::new(body_simplified),
                ))
            }
            Node::Sqrt(operand) => {
                let simplified = operand.simplify(env)?;
                if let Node::Num(ref n) = simplified {
                    return Ok(Node::Num(n.sqrt()));
                }
                Ok(Node::Sqrt(Box::new(simplified)))
            }
            Node::Function(name, args) => {
                let simplified_args: Vec<Node> = args
                    .iter()
                    .map(|a| a.simplify(env))
                    .collect::<Result<Vec<_>, _>>()?;

                let all_numeric = simplified_args.iter().all(|a| matches!(a, Node::Num(_)));
                if all_numeric {
                    let f64_args: Vec<f64> = simplified_args
                        .iter()
                        .map(|a| {
                            if let Node::Num(n) = a {
                                n.to_f64()
                            } else {
                                unreachable!()
                            }
                        })
                        .collect();
                    if let Ok(result) = crate::functions::call_function(name, f64_args) {
                        if result.is_finite() {
                            return Ok(Node::Num(ExactNum::from_f64(result)));
                        }
                    }
                }

                Ok(Node::Function(name.clone(), simplified_args))
            }
            _ => Ok(self.clone()),
        }
    }
}

fn collect_terms(
    node: &Node,
    term_map: &mut HashMap<String, ExactNum>,
    _env: &Environment,
) -> Result<(), String> {
    match node {
        Node::Add(left, right) => {
            collect_terms(left, term_map, _env)?;
            collect_terms(right, term_map, _env)?;
        }
        Node::Multiply(left, right) => {
            if let (Node::Num(ref coef), Node::Variable(ref var)) = (&**left, &**right) {
                let entry = term_map.entry(var.clone()).or_insert_with(ExactNum::zero);
                *entry = entry.clone() + coef.clone();
            }
        }
        Node::Variable(var) => {
            let entry = term_map.entry(var.clone()).or_insert_with(ExactNum::zero);
            *entry = entry.clone() + ExactNum::one();
        }
        Node::Num(num) => {
            // For constants without variables (like `+10`), store them in the `""` key
            let entry = term_map
                .entry("".to_string())
                .or_insert_with(ExactNum::zero);
            *entry = entry.clone() + num.clone();
        }
        _ => return Err("Unsupported node type in collect_terms".to_string()),
    }
    Ok(())
}

fn rebuild_expression(term_map: HashMap<String, ExactNum>) -> Node {
    let mut terms: Vec<(String, ExactNum)> = term_map.into_iter().collect();

    // Sort terms by the variable name (lexicographically)
    terms.sort_by(|a, b| a.0.cmp(&b.0));

    let mut result_terms: Vec<Node> = vec![];

    for (var, coef) in terms {
        if var.is_empty() {
            if !coef.is_zero() {
                result_terms.push(Node::Num(coef));
            }
        } else if !coef.is_zero() {
            if coef.is_one() {
                result_terms.push(Node::Variable(var));
            } else {
                result_terms.push(Node::Multiply(
                    Box::new(Node::Num(coef)),
                    Box::new(Node::Variable(var)),
                ));
            }
        }
    }

    if result_terms.is_empty() {
        return Node::Num(ExactNum::zero());
    }

    // Combine all terms into a single expression (iterate from start to end)
    let mut simplified_expr = result_terms.remove(0);
    for term in result_terms {
        simplified_expr = Node::Add(Box::new(simplified_expr), Box::new(term));
    }

    simplified_expr
}

fn find_single_variable(node: &Node) -> Option<String> {
    let mut vars = std::collections::HashSet::new();
    collect_variables(node, &mut vars);
    if vars.len() == 1 {
        vars.into_iter().next()
    } else {
        None
    }
}

fn collect_variables(node: &Node, vars: &mut std::collections::HashSet<String>) {
    match node {
        Node::Variable(v) => {
            vars.insert(v.clone());
        }
        Node::Add(l, r)
        | Node::Subtract(l, r)
        | Node::Multiply(l, r)
        | Node::Divide(l, r)
        | Node::Power(l, r) => {
            collect_variables(l, vars);
            collect_variables(r, vars);
        }
        Node::Negate(inner) | Node::Sqrt(inner) | Node::Abs(inner) => {
            collect_variables(inner, vars);
        }
        _ => {}
    }
}

fn try_polynomial_normalize(node: &Node) -> Option<Node> {
    let var = find_single_variable(node)?;
    let poly = Polynomial::from_node(node, &var).ok()?;
    Some(poly.to_node())
}

fn try_polynomial_divide(numer: &Node, denom: &Node) -> Option<Node> {
    let mut vars = std::collections::HashSet::new();
    collect_variables(numer, &mut vars);
    collect_variables(denom, &mut vars);
    if vars.len() != 1 {
        return None;
    }
    let var = vars.into_iter().next()?;

    let n = Polynomial::from_node(numer, &var).ok()?;
    let d = Polynomial::from_node(denom, &var).ok()?;

    if d.is_zero() {
        return None;
    }

    let g = n.gcd(&d);
    if g.degree()? == 0 {
        return None;
    }

    let (n_reduced, n_rem) = n.div_rem(&g).ok()?;
    let (d_reduced, d_rem) = d.div_rem(&g).ok()?;

    if !n_rem.is_zero() || !d_rem.is_zero() {
        return None;
    }

    if d_reduced.is_constant() {
        let d_val = d_reduced.coeff(0);
        if d_val == num_rational::BigRational::from_integer(num_bigint::BigInt::from(1)) {
            return Some(n_reduced.to_node());
        }
        return Some(
            n_reduced
                .scalar_mul(
                    &(num_rational::BigRational::from_integer(num_bigint::BigInt::from(1)) / d_val),
                )
                .to_node(),
        );
    }

    Some(Node::Divide(
        Box::new(n_reduced.to_node()),
        Box::new(d_reduced.to_node()),
    ))
}
