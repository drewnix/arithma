use crate::environment::Environment;
use crate::exact::ExactNum;
use crate::multipoly::MultiPoly;
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

                // sin²(x) + cos²(x) → 1
                if let Some(result) = try_pythagorean(&left_simplified, &right_simplified) {
                    return Ok(result);
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

                // k * (-f) → (-k) * f — absorb negation into coefficient
                if let Node::Num(ref k) = left_simplified {
                    if let Node::Negate(inner) = right_simplified {
                        return Node::Multiply(Box::new(Node::Num(-k.clone())), inner)
                            .simplify(env);
                    }
                }
                // (-f) * k → (-k) * f
                if let Node::Negate(inner) = &left_simplified {
                    if let Node::Num(ref k) = right_simplified {
                        return Node::Multiply(Box::new(Node::Num(-k.clone())), inner.clone())
                            .simplify(env);
                    }
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

                // x^a * x^b → x^(a+b)
                if let (Node::Power(ref base1, ref exp1), Node::Power(ref base2, ref exp2)) =
                    (&left_simplified, &right_simplified)
                {
                    if base1 == base2 {
                        if let (Node::Num(ref a), Node::Num(ref b)) = (exp1.as_ref(), exp2.as_ref())
                        {
                            return Ok(Node::Power(base1.clone(), Box::new(Node::Num(a + b))));
                        }
                    }
                }

                // x * x^a → x^(a+1)
                if let Node::Power(ref base, ref exp) = right_simplified {
                    if *base.as_ref() == left_simplified {
                        if let Node::Num(ref a) = exp.as_ref() {
                            return Ok(Node::Power(
                                base.clone(),
                                Box::new(Node::Num(a + &ExactNum::one())),
                            ));
                        }
                    }
                }
                // x^a * x → x^(a+1)
                if let Node::Power(ref base, ref exp) = left_simplified {
                    if *base.as_ref() == right_simplified {
                        if let Node::Num(ref a) = exp.as_ref() {
                            return Ok(Node::Power(
                                base.clone(),
                                Box::new(Node::Num(a + &ExactNum::one())),
                            ));
                        }
                    }
                }

                // x * x → x^2
                if left_simplified == right_simplified && !matches!(left_simplified, Node::Num(_)) {
                    return Ok(Node::Power(
                        Box::new(left_simplified),
                        Box::new(Node::Num(ExactNum::two())),
                    ));
                }

                let result = Node::Multiply(Box::new(left_simplified), Box::new(right_simplified));
                if let Some(normalized) = try_polynomial_normalize(&result) {
                    Ok(normalized)
                } else {
                    Ok(result)
                }
            }
            Node::Power(base, exponent) => {
                let base_simplified = base.simplify(env)?;
                let exponent_simplified = exponent.simplify(env)?;

                // 0^n → 0 for n > 0, 1^n → 1
                if let Node::Num(ref b) = base_simplified {
                    if b.is_zero() {
                        if let Node::Num(ref e) = exponent_simplified {
                            if !e.is_negative() {
                                return Ok(Node::Num(ExactNum::zero()));
                            }
                        }
                    }
                    if b.is_one() {
                        return Ok(Node::Num(ExactNum::one()));
                    }
                }

                // x^0 → 1
                if let Node::Num(ref n) = exponent_simplified {
                    if n.is_zero() {
                        return Ok(Node::Num(ExactNum::one()));
                    }
                }

                // x^1 → x
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

                // (x^a)^b → x^(a*b) when both exponents are numeric
                if let Node::Power(inner_base, inner_exp) = &base_simplified {
                    if let (Node::Num(ref a), Node::Num(ref b)) =
                        (&**inner_exp, &exponent_simplified)
                    {
                        return Ok(Node::Power(inner_base.clone(), Box::new(Node::Num(a * b))));
                    }
                }

                // (-1)^(2n) → 1 when n is integer (even exponent of -1)
                if is_neg_one(&base_simplified) {
                    if is_even_integer_expr(&exponent_simplified, env) {
                        return Ok(Node::Num(ExactNum::one()));
                    }
                }

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

                if let Node::Num(ref n) = right_simplified {
                    if n.is_zero() {
                        return Ok(left_simplified);
                    }
                }
                if let Node::Num(ref n) = left_simplified {
                    if n.is_zero() {
                        return Ok(Node::Negate(Box::new(right_simplified)));
                    }
                }

                // 1 - sin²(x) → cos²(x), 1 - cos²(x) → sin²(x)
                if let Node::Num(ref n) = left_simplified {
                    if n.is_one() {
                        if let Some(args) = is_trig_squared(&right_simplified, "sin") {
                            return Ok(Node::Power(
                                Box::new(Node::Function("cos".to_string(), args)),
                                Box::new(Node::Num(ExactNum::two())),
                            ));
                        }
                        if let Some(args) = is_trig_squared(&right_simplified, "cos") {
                            return Ok(Node::Power(
                                Box::new(Node::Function("sin".to_string(), args)),
                                Box::new(Node::Num(ExactNum::two())),
                            ));
                        }
                    }
                }

                // sin²(x) - 1 → -cos²(x), cos²(x) - 1 → -sin²(x)
                if let Node::Num(ref n) = right_simplified {
                    if n.is_one() {
                        if let Some(args) = is_trig_squared(&left_simplified, "sin") {
                            return Ok(Node::Negate(Box::new(Node::Power(
                                Box::new(Node::Function("cos".to_string(), args)),
                                Box::new(Node::Num(ExactNum::two())),
                            ))));
                        }
                        if let Some(args) = is_trig_squared(&left_simplified, "cos") {
                            return Ok(Node::Negate(Box::new(Node::Power(
                                Box::new(Node::Function("sin".to_string(), args)),
                                Box::new(Node::Num(ExactNum::two())),
                            ))));
                        }
                    }
                }

                let result = Node::Subtract(Box::new(left_simplified), Box::new(right_simplified));
                let mut term_map: HashMap<String, ExactNum> = HashMap::new();
                if collect_terms(&result, &mut term_map, env).is_ok() {
                    Ok(rebuild_expression(term_map))
                } else if let Some(normalized) = try_polynomial_normalize(&result) {
                    Ok(normalized)
                } else {
                    Ok(result)
                }
            }
            Node::Negate(operand) => {
                let simplified = operand.simplify(env)?;
                if let Node::Num(ref n) = simplified {
                    return Ok(Node::Num(-n.clone()));
                }
                if let Node::Negate(inner) = simplified {
                    return Ok(*inner);
                }
                // -(a + b) → (-a) - b, -(a - b) → b - a
                if let Node::Add(a, b) = simplified {
                    return Node::Subtract(Box::new(Node::Negate(a)), b).simplify(env);
                }
                if let Node::Subtract(a, b) = simplified {
                    return Node::Subtract(b, a).simplify(env);
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

                // x / x → 1
                if left_simplified == right_simplified && !matches!(left_simplified, Node::Num(_)) {
                    return Ok(Node::Num(ExactNum::one()));
                }

                // sin(x) / cos(x) → tan(x), cos(x) / sin(x) → cot(x)
                if let (
                    Node::Function(ref fname1, ref args1),
                    Node::Function(ref fname2, ref args2),
                ) = (&left_simplified, &right_simplified)
                {
                    if fname1 == "sin" && fname2 == "cos" && args1 == args2 {
                        return Ok(Node::Function("tan".to_string(), args1.clone()));
                    }
                    if fname1 == "cos" && fname2 == "sin" && args1 == args2 {
                        return Ok(Node::Function("cot".to_string(), args1.clone()));
                    }
                }

                // 1 / sin(x) → csc(x), 1 / cos(x) → sec(x), 1 / tan(x) → cot(x)
                if let Node::Num(ref n) = left_simplified {
                    if n.is_one() {
                        if let Node::Function(ref fname, ref args) = right_simplified {
                            let recip = match fname.as_str() {
                                "sin" => Some("csc"),
                                "cos" => Some("sec"),
                                "tan" => Some("cot"),
                                _ => None,
                            };
                            if let Some(recip_name) = recip {
                                return Ok(Node::Function(recip_name.to_string(), args.clone()));
                            }
                        }
                    }
                }

                // x^a / x^b → x^(a-b)
                if let (Node::Power(ref base1, ref exp1), Node::Power(ref base2, ref exp2)) =
                    (&left_simplified, &right_simplified)
                {
                    if base1 == base2 {
                        if let (Node::Num(ref a), Node::Num(ref b)) = (exp1.as_ref(), exp2.as_ref())
                        {
                            let diff = a - b;
                            if diff.is_zero() {
                                return Ok(Node::Num(ExactNum::one()));
                            } else if diff.is_one() {
                                return Ok(*base1.clone());
                            }
                            return Ok(Node::Power(base1.clone(), Box::new(Node::Num(diff))));
                        }
                    }
                }

                // x^a / x → x^(a-1)
                if let Node::Power(ref base, ref exp) = left_simplified {
                    if *base.as_ref() == right_simplified {
                        if let Node::Num(ref a) = exp.as_ref() {
                            let diff = a - &ExactNum::one();
                            if diff.is_zero() {
                                return Ok(Node::Num(ExactNum::one()));
                            } else if diff.is_one() {
                                return Ok(*base.clone());
                            }
                            return Ok(Node::Power(base.clone(), Box::new(Node::Num(diff))));
                        }
                    }
                }

                // x / x^a → x^(1-a)
                if let Node::Power(ref base, ref exp) = right_simplified {
                    if *base.as_ref() == left_simplified {
                        if let Node::Num(ref a) = exp.as_ref() {
                            let diff = &ExactNum::one() - a;
                            if diff.is_zero() {
                                return Ok(Node::Num(ExactNum::one()));
                            } else if diff.is_one() {
                                return Ok(*base.clone());
                            }
                            return Ok(Node::Power(base.clone(), Box::new(Node::Num(diff))));
                        }
                    }
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
                                sum_env.set_exact(index_var, ExactNum::integer(i));

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
            Node::Abs(operand) => {
                let simplified = operand.simplify(env)?;
                if let Node::Num(ref n) = simplified {
                    return Ok(Node::Num(n.abs()));
                }
                // |x| → x when x is nonnegative
                if let Node::Variable(ref v) = simplified {
                    if env.assumptions().is_nonneg(v) {
                        return Ok(simplified);
                    }
                    if env.assumptions().is_negative(v) {
                        return Ok(Node::Negate(Box::new(simplified)));
                    }
                }
                // |-x| → |x|
                if let Node::Negate(inner) = simplified {
                    return Ok(Node::Abs(inner));
                }
                // ||x|| → |x|
                if let Node::Abs(_) = simplified {
                    return Ok(simplified);
                }
                Ok(Node::Abs(Box::new(simplified)))
            }
            Node::Sqrt(operand) => {
                let simplified = operand.simplify(env)?;
                if let Node::Num(ref n) = simplified {
                    return Ok(Node::Num(n.sqrt()));
                }
                // sqrt(x²) → x when x positive, |x| otherwise
                if let Node::Power(ref base, ref exp) = simplified {
                    if let Node::Num(ref e) = **exp {
                        if e == &ExactNum::two() {
                            if let Node::Variable(ref v) = **base {
                                if env.assumptions().is_nonneg(v) {
                                    return Ok(*base.clone());
                                }
                            }
                            return Ok(Node::Abs(base.clone()));
                        }
                    }
                }
                Ok(Node::Sqrt(Box::new(simplified)))
            }
            Node::Function(name, args) => {
                let simplified_args: Vec<Node> = args
                    .iter()
                    .map(|a| a.simplify(env))
                    .collect::<Result<Vec<_>, _>>()?;

                if simplified_args.len() == 1 {
                    let arg = &simplified_args[0];
                    match name.as_str() {
                        "ln" => {
                            // ln(e^x) → x
                            if let Node::Power(base, exp) = arg {
                                if let Node::Num(ref b) = **base {
                                    if (b.to_f64() - std::f64::consts::E).abs() < 1e-14 {
                                        return Ok(*exp.clone());
                                    }
                                }
                                // ln(a^b) → b·ln(a), then re-simplify since ln(a)
                                // may itself expand (e.g. a = x·y → ln(x)+ln(y))
                                let inner_ln =
                                    Node::Function("ln".to_string(), vec![*base.clone()])
                                        .simplify(env)?;
                                return Node::Multiply(exp.clone(), Box::new(inner_ln))
                                    .simplify(env);
                            }
                            // ln(a·b) → ln(a) + ln(b), re-simplify each ln
                            if let Node::Multiply(a, b) = arg {
                                let ln_a = Node::Function("ln".to_string(), vec![*a.clone()])
                                    .simplify(env)?;
                                let ln_b = Node::Function("ln".to_string(), vec![*b.clone()])
                                    .simplify(env)?;
                                return Node::Add(Box::new(ln_a), Box::new(ln_b)).simplify(env);
                            }
                            // ln(a/b) → ln(a) - ln(b), re-simplify each ln
                            if let Node::Divide(a, b) = arg {
                                let ln_a = Node::Function("ln".to_string(), vec![*a.clone()])
                                    .simplify(env)?;
                                let ln_b = Node::Function("ln".to_string(), vec![*b.clone()])
                                    .simplify(env)?;
                                return Node::Subtract(Box::new(ln_a), Box::new(ln_b))
                                    .simplify(env);
                            }
                        }
                        "exp" => {
                            // exp(ln(x)) → x
                            if let Node::Function(inner_name, inner_args) = arg {
                                if inner_name == "ln" && inner_args.len() == 1 {
                                    return Ok(inner_args[0].clone());
                                }
                            }
                        }
                        "sqrt" => {
                            // sqrt(x²) → x when x nonneg, |x| otherwise
                            if let Node::Power(base, exp) = arg {
                                if let Node::Num(ref e) = **exp {
                                    if e == &ExactNum::two() {
                                        if let Node::Variable(ref v) = **base {
                                            if env.assumptions().is_nonneg(v) {
                                                return Ok(*base.clone());
                                            }
                                        }
                                        return Ok(Node::Abs(base.clone()));
                                    }
                                }
                            }
                        }
                        // sin(-x) → -sin(x)
                        "sin" | "tan" | "sinh" | "tanh" => {
                            if let Node::Negate(inner) = arg {
                                return Ok(Node::Negate(Box::new(Node::Function(
                                    name.clone(),
                                    vec![*inner.clone()],
                                ))));
                            }
                        }
                        // cos(-x) → cos(x)
                        "cos" | "cosh" => {
                            if let Node::Negate(inner) = arg {
                                return Ok(Node::Function(name.clone(), vec![*inner.clone()]));
                            }
                        }
                        _ => {}
                    }
                }

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

fn collect_terms_inner(
    node: &Node,
    term_map: &mut HashMap<String, ExactNum>,
    sign: &ExactNum,
) -> Result<(), String> {
    match node {
        Node::Add(left, right) => {
            collect_terms_inner(left, term_map, sign)?;
            collect_terms_inner(right, term_map, sign)?;
        }
        Node::Subtract(left, right) => {
            collect_terms_inner(left, term_map, sign)?;
            let neg_sign = sign.clone() * ExactNum::integer(-1);
            collect_terms_inner(right, term_map, &neg_sign)?;
        }
        Node::Negate(inner) => {
            let neg_sign = sign.clone() * ExactNum::integer(-1);
            collect_terms_inner(inner, term_map, &neg_sign)?;
        }
        Node::Multiply(left, right) => {
            if let (Node::Num(ref coef), Node::Variable(ref var)) = (&**left, &**right) {
                let entry = term_map.entry(var.clone()).or_insert_with(ExactNum::zero);
                *entry = entry.clone() + coef.clone() * sign.clone();
            } else {
                return Err("Unsupported multiply form in collect_terms".to_string());
            }
        }
        Node::Variable(var) => {
            let entry = term_map.entry(var.clone()).or_insert_with(ExactNum::zero);
            *entry = entry.clone() + sign.clone();
        }
        Node::Num(num) => {
            let entry = term_map
                .entry("".to_string())
                .or_insert_with(ExactNum::zero);
            *entry = entry.clone() + num.clone() * sign.clone();
        }
        _ => return Err("Unsupported node type in collect_terms".to_string()),
    }
    Ok(())
}

fn collect_terms(
    node: &Node,
    term_map: &mut HashMap<String, ExactNum>,
    _env: &Environment,
) -> Result<(), String> {
    collect_terms_inner(node, term_map, &ExactNum::one())
}

fn rebuild_expression(term_map: HashMap<String, ExactNum>) -> Node {
    let mut terms: Vec<(String, ExactNum)> = term_map.into_iter().collect();

    // Sort: variables alphabetically first, constant term last
    terms.sort_by(|a, b| match (a.0.is_empty(), b.0.is_empty()) {
        (true, false) => std::cmp::Ordering::Greater,
        (false, true) => std::cmp::Ordering::Less,
        _ => a.0.cmp(&b.0),
    });

    // Build (abs_node, is_negative) pairs for non-zero terms
    let mut signed_terms: Vec<(Node, bool)> = vec![];

    for (var, coef) in terms {
        if coef.is_zero() {
            continue;
        }
        let negative = coef.is_negative();
        let abs_coef = if negative {
            -coef.clone()
        } else {
            coef.clone()
        };

        let node = if var.is_empty() {
            Node::Num(abs_coef)
        } else if abs_coef.is_one() {
            Node::Variable(var)
        } else {
            Node::Multiply(Box::new(Node::Num(abs_coef)), Box::new(Node::Variable(var)))
        };
        signed_terms.push((node, negative));
    }

    if signed_terms.is_empty() {
        return Node::Num(ExactNum::zero());
    }

    let (first_node, first_neg) = signed_terms.remove(0);
    let mut result = if first_neg {
        Node::Negate(Box::new(first_node))
    } else {
        first_node
    };

    for (node, negative) in signed_terms {
        result = if negative {
            Node::Subtract(Box::new(result), Box::new(node))
        } else {
            Node::Add(Box::new(result), Box::new(node))
        };
    }

    result
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
    if let Some(var) = find_single_variable(node) {
        let poly = Polynomial::from_node(node, &var).ok()?;
        return Some(poly.to_node());
    }
    // Multivariate fallback
    let mp = MultiPoly::from_node(node).ok()?;
    Some(mp.to_node())
}

fn try_polynomial_divide(numer: &Node, denom: &Node) -> Option<Node> {
    let mut vars = std::collections::HashSet::new();
    collect_variables(numer, &mut vars);
    collect_variables(denom, &mut vars);

    if vars.len() == 1 {
        let var = vars.into_iter().next()?;
        return try_univariate_divide(numer, denom, &var);
    }

    if vars.len() >= 2 {
        return try_multivariate_divide(numer, denom);
    }

    None
}

fn try_univariate_divide(numer: &Node, denom: &Node, var: &str) -> Option<Node> {
    let n = Polynomial::from_node(numer, var).ok()?;
    let d = Polynomial::from_node(denom, var).ok()?;

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

fn try_multivariate_divide(numer: &Node, denom: &Node) -> Option<Node> {
    let n = MultiPoly::from_node(numer).ok()?;
    let d = MultiPoly::from_node(denom).ok()?;

    if d.is_zero() {
        return None;
    }

    let g = MultiPoly::gcd(&n, &d);
    if g.is_constant() {
        return None;
    }

    let n_reduced = n.exact_div(&g);
    let d_reduced = d.exact_div(&g);

    if d_reduced.is_one() {
        return Some(n_reduced.to_node());
    }
    if let Some(d_val) = d_reduced.as_constant() {
        if !num_traits::Zero::is_zero(d_val) {
            let inv = num_rational::BigRational::from_integer(num_bigint::BigInt::from(1)) / d_val;
            return Some(n_reduced.scalar_mul(&inv).to_node());
        }
    }

    Some(Node::Divide(
        Box::new(n_reduced.to_node()),
        Box::new(d_reduced.to_node()),
    ))
}

fn is_trig_squared(node: &Node, func_name: &str) -> Option<Vec<Node>> {
    if let Node::Power(base, exp) = node {
        if let Node::Num(ref e) = **exp {
            if e == &ExactNum::two() {
                if let Node::Function(name, args) = base.as_ref() {
                    if name == func_name {
                        return Some(args.clone());
                    }
                }
            }
        }
    }
    None
}

fn try_pythagorean(left: &Node, right: &Node) -> Option<Node> {
    // sin²(x) + cos²(x) → 1
    if let (Some(sin_args), Some(cos_args)) =
        (is_trig_squared(left, "sin"), is_trig_squared(right, "cos"))
    {
        if sin_args == cos_args {
            return Some(Node::Num(ExactNum::one()));
        }
    }
    // cos²(x) + sin²(x) → 1
    if let (Some(cos_args), Some(sin_args)) =
        (is_trig_squared(left, "cos"), is_trig_squared(right, "sin"))
    {
        if cos_args == sin_args {
            return Some(Node::Num(ExactNum::one()));
        }
    }

    // a·sin²(x) + a·cos²(x) → a (with coefficient)
    if let (Some((coeff_l, sin_args)), Some((coeff_r, cos_args))) = (
        extract_coeff_trig_sq(left, "sin"),
        extract_coeff_trig_sq(right, "cos"),
    ) {
        if sin_args == cos_args && coeff_l == coeff_r {
            return Some(Node::Num(coeff_l));
        }
    }
    if let (Some((coeff_l, cos_args)), Some((coeff_r, sin_args))) = (
        extract_coeff_trig_sq(left, "cos"),
        extract_coeff_trig_sq(right, "sin"),
    ) {
        if cos_args == sin_args && coeff_l == coeff_r {
            return Some(Node::Num(coeff_l));
        }
    }

    None
}

fn extract_coeff_trig_sq(node: &Node, func_name: &str) -> Option<(ExactNum, Vec<Node>)> {
    if let Some(args) = is_trig_squared(node, func_name) {
        return Some((ExactNum::one(), args));
    }
    if let Node::Multiply(coeff, power) = node {
        if let Node::Num(ref c) = **coeff {
            if let Some(args) = is_trig_squared(power, func_name) {
                return Some((c.clone(), args));
            }
        }
        if let Node::Num(ref c) = **power {
            if let Some(args) = is_trig_squared(coeff, func_name) {
                return Some((c.clone(), args));
            }
        }
    }
    None
}

fn is_neg_one(node: &Node) -> bool {
    match node {
        Node::Num(n) => n == &ExactNum::integer(-1),
        Node::Negate(inner) => {
            matches!(&**inner, Node::Num(n) if n.is_one())
        }
        _ => false,
    }
}

fn is_even_integer_expr(node: &Node, env: &Environment) -> bool {
    // 2n, 2*n, k*n where k is even and n is integer
    if let Node::Multiply(left, right) = node {
        match (&**left, &**right) {
            (Node::Num(k), Node::Variable(v)) | (Node::Variable(v), Node::Num(k)) => {
                return k.is_even() && env.assumptions().is_integer(v);
            }
            _ => {}
        }
    }
    // A numeric even integer by itself
    if let Node::Num(n) = node {
        return n.is_even();
    }
    false
}
