use crate::exact::ExactNum;
use crate::node::Node;
use crate::polynomial::Polynomial;
use crate::substitute::substitute_variable;

pub fn differentiate(expr: &Node, var_name: &str) -> Result<Node, String> {
    let env = crate::environment::Environment::new();
    let expr =
        &crate::simplify::Simplifiable::simplify(expr, &env).unwrap_or_else(|_| expr.clone());

    if let Ok(poly) = Polynomial::from_node(expr, var_name) {
        return Ok(poly.derivative().to_node());
    }

    match expr {
        // Constants differentiate to zero
        Node::Num(_) => Ok(Node::Num(ExactNum::zero())),

        // Variables: d/dx(x) = 1, d/dx(y) = 0
        Node::Variable(name) => {
            if name == var_name {
                Ok(Node::Num(ExactNum::one()))
            } else {
                Ok(Node::Num(ExactNum::zero()))
            }
        }

        // d/dx(f + g) = d/dx(f) + d/dx(g)
        Node::Add(left, right) => {
            let left_derivative = differentiate(left, var_name)?;
            let right_derivative = differentiate(right, var_name)?;
            Ok(Node::Add(
                Box::new(left_derivative),
                Box::new(right_derivative),
            ))
        }

        // d/dx(f - g) = d/dx(f) - d/dx(g)
        Node::Subtract(left, right) => {
            let left_derivative = differentiate(left, var_name)?;
            let right_derivative = differentiate(right, var_name)?;
            Ok(Node::Subtract(
                Box::new(left_derivative),
                Box::new(right_derivative),
            ))
        }

        // Product rule: d/dx(f*g) = f*dg/dx + g*df/dx
        Node::Multiply(left, right) => {
            let left_derivative = differentiate(left, var_name)?;
            let right_derivative = differentiate(right, var_name)?;

            // f * dg/dx
            let term1 = Node::Multiply(left.clone(), Box::new(right_derivative));

            // g * df/dx
            let term2 = Node::Multiply(right.clone(), Box::new(left_derivative));

            // f * dg/dx + g * df/dx
            Ok(Node::Add(Box::new(term1), Box::new(term2)))
        }

        // Quotient rule: d/dx(f/g) = (g*df/dx - f*dg/dx) / g^2
        Node::Divide(left, right) => {
            let left_derivative = differentiate(left, var_name)?;
            let right_derivative = differentiate(right, var_name)?;

            // g * df/dx
            let term1 = Node::Multiply(right.clone(), Box::new(left_derivative));

            // f * dg/dx
            let term2 = Node::Multiply(left.clone(), Box::new(right_derivative));

            // g * df/dx - f * dg/dx
            let numerator = Node::Subtract(Box::new(term1), Box::new(term2));

            // g^2
            let denominator = Node::Power(right.clone(), Box::new(Node::Num(ExactNum::two())));

            // (g*df/dx - f*dg/dx) / g^2
            Ok(Node::Divide(Box::new(numerator), Box::new(denominator)))
        }

        // Power rule for x^n: d/dx(x^n) = n*x^(n-1)
        Node::Power(base, exponent) => {
            // Check if the base is the variable we're differentiating with respect to
            // and the exponent is a constant
            if let (Node::Variable(base_var), Node::Num(n)) = (&**base, &**exponent) {
                if base_var == var_name {
                    // n * x^(n-1)
                    let coefficient = n.clone();
                    let new_exponent = n.clone() - ExactNum::one();

                    // Handle special cases
                    if new_exponent.is_zero() {
                        return Ok(Node::Num(coefficient));
                    } else if new_exponent.is_one() {
                        return Ok(Node::Multiply(
                            Box::new(Node::Num(coefficient)),
                            Box::new(Node::Variable(var_name.to_string())),
                        ));
                    } else {
                        return Ok(Node::Multiply(
                            Box::new(Node::Num(coefficient)),
                            Box::new(Node::Power(
                                Box::new(Node::Variable(var_name.to_string())),
                                Box::new(Node::Num(new_exponent)),
                            )),
                        ));
                    }
                }
            }

            // General case using chain rule: d/dx(f(x)^g(x)) = g*f^(g-1)*f' + f^g*ln(f)*g'
            // For now, we'll just implement the simple case where g is constant: d/dx(f(x)^n) = n*f(x)^(n-1)*f'(x)
            if let Node::Num(n) = &**exponent {
                let base_derivative = differentiate(base, var_name)?;

                // n * f(x)^(n-1)
                let new_exponent = n.clone() - ExactNum::one();
                let power_term = if new_exponent.is_zero() {
                    Node::Num(ExactNum::one())
                } else {
                    Node::Power(base.clone(), Box::new(Node::Num(new_exponent)))
                };

                let coefficient = Node::Num(n.clone());

                // n * f(x)^(n-1) * f'(x)
                Ok(Node::Multiply(
                    Box::new(Node::Multiply(Box::new(coefficient), Box::new(power_term))),
                    Box::new(base_derivative),
                ))
            } else {
                // General case: d/dx(f^g) = f^g * (g'*ln(f) + g*f'/f)
                let base_deriv = differentiate(base, var_name)?;
                let exp_deriv = differentiate(exponent, var_name)?;

                let base_is_const = matches!(base_deriv, Node::Num(ref n) if n.is_zero());
                let exp_is_const = matches!(exp_deriv, Node::Num(ref n) if n.is_zero());

                if exp_is_const && !base_is_const {
                    // d/dx(f(x)^c) = c * f^(c-1) * f' — already handled above,
                    // but as a fallback for non-Num constant exponents
                    let power_term = Node::Power(
                        base.clone(),
                        Box::new(Node::Subtract(
                            exponent.clone(),
                            Box::new(Node::Num(ExactNum::one())),
                        )),
                    );
                    Ok(Node::Multiply(
                        Box::new(Node::Multiply(exponent.clone(), Box::new(power_term))),
                        Box::new(base_deriv),
                    ))
                } else if base_is_const && !exp_is_const {
                    // d/dx(a^g(x)) = a^g(x) * ln(a) * g'(x)
                    let original = Node::Power(base.clone(), exponent.clone());
                    let ln_base = Node::Function("ln".to_string(), vec![*base.clone()]);
                    Ok(Node::Multiply(
                        Box::new(Node::Multiply(Box::new(original), Box::new(ln_base))),
                        Box::new(exp_deriv),
                    ))
                } else if base_is_const && exp_is_const {
                    Ok(Node::Num(ExactNum::zero()))
                } else {
                    // Both base and exponent depend on x:
                    // d/dx(f^g) = f^g * (g'*ln(f) + g*f'/f)
                    let original = Node::Power(base.clone(), exponent.clone());
                    let ln_base = Node::Function("ln".to_string(), vec![*base.clone()]);
                    let term1 = Node::Multiply(Box::new(exp_deriv), Box::new(ln_base));
                    let term2 = Node::Multiply(
                        exponent.clone(),
                        Box::new(Node::Divide(Box::new(base_deriv), base.clone())),
                    );
                    Ok(Node::Multiply(
                        Box::new(original),
                        Box::new(Node::Add(Box::new(term1), Box::new(term2))),
                    ))
                }
            }
        }

        // d/dx(sqrt(f)) = 1/(2*sqrt(f)) * df/dx
        Node::Sqrt(operand) => {
            let operand_derivative = differentiate(operand, var_name)?;

            // 1/(2*sqrt(f))
            let coefficient = Node::Divide(
                Box::new(Node::Num(ExactNum::one())),
                Box::new(Node::Multiply(
                    Box::new(Node::Num(ExactNum::two())),
                    Box::new(Node::Sqrt(operand.clone())),
                )),
            );

            // 1/(2*sqrt(f)) * df/dx
            Ok(Node::Multiply(
                Box::new(coefficient),
                Box::new(operand_derivative),
            ))
        }

        // d/dx(|f|) = sgn(f) * df/dx where sgn(f) = f/|f| for f != 0
        Node::Abs(operand) => {
            let operand_derivative = differentiate(operand, var_name)?;

            // sgn(f) as f/|f|
            let sign = Node::Divide(operand.clone(), Box::new(Node::Abs(operand.clone())));

            // sgn(f) * df/dx
            Ok(Node::Multiply(Box::new(sign), Box::new(operand_derivative)))
        }

        // d/dx(-f) = -df/dx
        Node::Negate(operand) => {
            let operand_derivative = differentiate(operand, var_name)?;
            Ok(Node::Negate(Box::new(operand_derivative)))
        }

        // For summation, we differentiate the body with respect to the variable
        // Note: we don't differentiate with respect to the summation index
        Node::Summation(index, start, end, body) => {
            if index == var_name {
                // If the variable we're differentiating with respect to is the summation index,
                // the derivative is zero because the index is bound by the summation
                Ok(Node::Num(ExactNum::zero()))
            } else {
                // Differentiate the start, end and body with respect to the variable
                let start_derivative = differentiate(start, var_name)?;
                let end_derivative = differentiate(end, var_name)?;
                let body_derivative = differentiate(body, var_name)?;

                // If start and end don't depend on the variable, just differentiate the body
                if matches!(&start_derivative, Node::Num(n) if n.is_zero())
                    && matches!(&end_derivative, Node::Num(n) if n.is_zero())
                {
                    Ok(Node::Summation(
                        index.clone(),
                        start.clone(),
                        end.clone(),
                        Box::new(body_derivative),
                    ))
                } else {
                    // For now, return an error for the more complex case where bounds depend on the variable
                    Err(
                        "Differentiation of summations with variable bounds not yet implemented"
                            .to_string(),
                    )
                }
            }
        }

        // Generalized product rule: d/dx ∏_{i=a}^{b} f(i,x) = Σ_k (df(k,x)/dx · ∏_{j≠k} f(j,x))
        Node::Product(index, start, end, body) => {
            if index == var_name || !body.contains_variable(var_name) {
                return Ok(Node::Num(ExactNum::zero()));
            }

            // Bounds must not depend on the differentiation variable.
            let bound_is_const = |b: &Node| -> Result<bool, String> {
                Ok(matches!(differentiate(b, var_name)?, Node::Num(n) if n.is_zero()))
            };
            if !(bound_is_const(start)? && bound_is_const(end)?) {
                return Err(
                    "Differentiation of products with variable bounds not yet implemented"
                        .to_string(),
                );
            }

            // Expand the finite product ∏_{i=a}^{b} f(i, x) into an explicit chain
            // of factors, then let the ordinary product rule (Node::Multiply) derive
            // it. This reproduces the generalized product rule without hand-rolling it.
            let bound_err = || {
                "Differentiation of product notation requires constant integer bounds when the body depends on the differentiation variable".to_string()
            };
            let (start_i, end_i) = match (start.as_ref(), end.as_ref()) {
                (Node::Num(a), Node::Num(b)) => (
                    a.to_i64().ok_or_else(bound_err)?,
                    b.to_i64().ok_or_else(bound_err)?,
                ),
                _ => return Err(bound_err()),
            };

            let expanded = (start_i..=end_i)
                .map(|i| substitute_variable(body, index, &Node::Num(ExactNum::integer(i))))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .reduce(|acc, n| Node::Multiply(Box::new(acc), Box::new(n)))
                .unwrap_or_else(|| Node::Num(ExactNum::one()));

            differentiate(&expanded, var_name)
        }

        // Function differentiation
        Node::Function(name, args) => {
            match name.as_str() {
                "sqrt" => {
                    if args.len() != 1 {
                        return Err("sqrt function requires exactly one argument".to_string());
                    }

                    // d/dx(sqrt(f)) = 1/(2*sqrt(f)) * df/dx
                    let operand = &args[0];
                    let operand_derivative = differentiate(operand, var_name)?;

                    // 1/(2*sqrt(f))
                    let coefficient = Node::Divide(
                        Box::new(Node::Num(ExactNum::one())),
                        Box::new(Node::Multiply(
                            Box::new(Node::Num(ExactNum::two())),
                            Box::new(Node::Function("sqrt".to_string(), vec![operand.clone()])),
                        )),
                    );

                    // 1/(2*sqrt(f)) * df/dx
                    Ok(Node::Multiply(
                        Box::new(coefficient),
                        Box::new(operand_derivative),
                    ))
                }
                "sin" => {
                    if args.len() != 1 {
                        return Err("sin function requires exactly one argument".to_string());
                    }

                    // d/dx(sin(f)) = cos(f) * df/dx
                    let operand = &args[0];
                    let operand_derivative = differentiate(operand, var_name)?;

                    // cos(f)
                    let coefficient = Node::Function("cos".to_string(), vec![operand.clone()]);

                    // cos(f) * df/dx
                    Ok(Node::Multiply(
                        Box::new(coefficient),
                        Box::new(operand_derivative),
                    ))
                }
                "cos" => {
                    if args.len() != 1 {
                        return Err("cos function requires exactly one argument".to_string());
                    }

                    // d/dx(cos(f)) = -sin(f) * df/dx
                    let operand = &args[0];
                    let operand_derivative = differentiate(operand, var_name)?;

                    // -sin(f)
                    let coefficient = Node::Negate(Box::new(Node::Function(
                        "sin".to_string(),
                        vec![operand.clone()],
                    )));

                    // -sin(f) * df/dx
                    Ok(Node::Multiply(
                        Box::new(coefficient),
                        Box::new(operand_derivative),
                    ))
                }
                "tan" => {
                    if args.len() != 1 {
                        return Err("tan function requires exactly one argument".to_string());
                    }

                    // d/dx(tan(f)) = sec^2(f) * df/dx
                    let operand = &args[0];
                    let operand_derivative = differentiate(operand, var_name)?;

                    // sec^2(f) = 1/cos^2(f)
                    let coefficient = Node::Divide(
                        Box::new(Node::Num(ExactNum::one())),
                        Box::new(Node::Power(
                            Box::new(Node::Function("cos".to_string(), vec![operand.clone()])),
                            Box::new(Node::Num(ExactNum::two())),
                        )),
                    );

                    // sec^2(f) * df/dx
                    Ok(Node::Multiply(
                        Box::new(coefficient),
                        Box::new(operand_derivative),
                    ))
                }
                "ln" => {
                    if args.len() != 1 {
                        return Err("ln function requires exactly one argument".to_string());
                    }

                    // d/dx(ln(f)) = 1/f * df/dx
                    let operand = &args[0];
                    let operand_derivative = differentiate(operand, var_name)?;

                    // 1/f
                    let coefficient = Node::Divide(
                        Box::new(Node::Num(ExactNum::one())),
                        Box::new(operand.clone()),
                    );

                    // 1/f * df/dx
                    Ok(Node::Multiply(
                        Box::new(coefficient),
                        Box::new(operand_derivative),
                    ))
                }
                "exp" => {
                    if args.len() != 1 {
                        return Err("exp function requires exactly one argument".to_string());
                    }

                    // d/dx(exp(f)) = exp(f) * df/dx
                    let operand = &args[0];
                    let operand_derivative = differentiate(operand, var_name)?;

                    // exp(f)
                    let coefficient = Node::Function("exp".to_string(), vec![operand.clone()]);

                    // exp(f) * df/dx
                    Ok(Node::Multiply(
                        Box::new(coefficient),
                        Box::new(operand_derivative),
                    ))
                }
                "log" => {
                    if args.len() != 1 {
                        return Err("log function requires exactly one argument".to_string());
                    }

                    // d/dx(log10(f)) = 1/(f*ln(10)) * df/dx
                    let operand = &args[0];
                    let operand_derivative = differentiate(operand, var_name)?;

                    // 1/(f*ln(10))
                    let ln10 = Node::Num(ExactNum::Float(std::f64::consts::LN_10));
                    let coefficient = Node::Divide(
                        Box::new(Node::Num(ExactNum::one())),
                        Box::new(Node::Multiply(Box::new(operand.clone()), Box::new(ln10))),
                    );

                    // 1/(f*ln(10)) * df/dx
                    Ok(Node::Multiply(
                        Box::new(coefficient),
                        Box::new(operand_derivative),
                    ))
                }
                "sec" => {
                    if args.len() != 1 {
                        return Err("sec function requires exactly one argument".to_string());
                    }
                    // d/dx(sec(f)) = sec(f)·tan(f) · f'
                    let f = &args[0];
                    let fp = differentiate(f, var_name)?;
                    Ok(Node::Multiply(
                        Box::new(Node::Multiply(
                            Box::new(Node::Function("sec".to_string(), vec![f.clone()])),
                            Box::new(Node::Function("tan".to_string(), vec![f.clone()])),
                        )),
                        Box::new(fp),
                    ))
                }
                "csc" => {
                    if args.len() != 1 {
                        return Err("csc function requires exactly one argument".to_string());
                    }
                    // d/dx(csc(f)) = -csc(f)·cot(f) · f'
                    let f = &args[0];
                    let fp = differentiate(f, var_name)?;
                    Ok(Node::Multiply(
                        Box::new(Node::Negate(Box::new(Node::Multiply(
                            Box::new(Node::Function("csc".to_string(), vec![f.clone()])),
                            Box::new(Node::Function("cot".to_string(), vec![f.clone()])),
                        )))),
                        Box::new(fp),
                    ))
                }
                "cot" => {
                    if args.len() != 1 {
                        return Err("cot function requires exactly one argument".to_string());
                    }
                    // d/dx(cot(f)) = -csc²(f) · f'
                    let f = &args[0];
                    let fp = differentiate(f, var_name)?;
                    Ok(Node::Multiply(
                        Box::new(Node::Negate(Box::new(Node::Power(
                            Box::new(Node::Function("csc".to_string(), vec![f.clone()])),
                            Box::new(Node::Num(ExactNum::two())),
                        )))),
                        Box::new(fp),
                    ))
                }
                "sinh" => {
                    if args.len() != 1 {
                        return Err("sinh function requires exactly one argument".to_string());
                    }
                    // d/dx(sinh(f)) = cosh(f) · f'
                    let f = &args[0];
                    let fp = differentiate(f, var_name)?;
                    Ok(Node::Multiply(
                        Box::new(Node::Function("cosh".to_string(), vec![f.clone()])),
                        Box::new(fp),
                    ))
                }
                "cosh" => {
                    if args.len() != 1 {
                        return Err("cosh function requires exactly one argument".to_string());
                    }
                    // d/dx(cosh(f)) = sinh(f) · f'
                    let f = &args[0];
                    let fp = differentiate(f, var_name)?;
                    Ok(Node::Multiply(
                        Box::new(Node::Function("sinh".to_string(), vec![f.clone()])),
                        Box::new(fp),
                    ))
                }
                "tanh" => {
                    if args.len() != 1 {
                        return Err("tanh function requires exactly one argument".to_string());
                    }
                    // d/dx(tanh(f)) = (1 - tanh²(f)) · f'
                    let f = &args[0];
                    let fp = differentiate(f, var_name)?;
                    Ok(Node::Multiply(
                        Box::new(Node::Subtract(
                            Box::new(Node::Num(ExactNum::one())),
                            Box::new(Node::Power(
                                Box::new(Node::Function("tanh".to_string(), vec![f.clone()])),
                                Box::new(Node::Num(ExactNum::two())),
                            )),
                        )),
                        Box::new(fp),
                    ))
                }
                "arcsin" => {
                    if args.len() != 1 {
                        return Err("arcsin function requires exactly one argument".to_string());
                    }
                    // d/dx(arcsin(f)) = 1/√(1-f²) · f'
                    let f = &args[0];
                    let fp = differentiate(f, var_name)?;
                    Ok(Node::Multiply(
                        Box::new(Node::Divide(
                            Box::new(Node::Num(ExactNum::one())),
                            Box::new(Node::Sqrt(Box::new(Node::Subtract(
                                Box::new(Node::Num(ExactNum::one())),
                                Box::new(Node::Power(
                                    Box::new(f.clone()),
                                    Box::new(Node::Num(ExactNum::two())),
                                )),
                            )))),
                        )),
                        Box::new(fp),
                    ))
                }
                "arccos" => {
                    if args.len() != 1 {
                        return Err("arccos function requires exactly one argument".to_string());
                    }
                    // d/dx(arccos(f)) = -1/√(1-f²) · f'
                    let f = &args[0];
                    let fp = differentiate(f, var_name)?;
                    Ok(Node::Multiply(
                        Box::new(Node::Negate(Box::new(Node::Divide(
                            Box::new(Node::Num(ExactNum::one())),
                            Box::new(Node::Sqrt(Box::new(Node::Subtract(
                                Box::new(Node::Num(ExactNum::one())),
                                Box::new(Node::Power(
                                    Box::new(f.clone()),
                                    Box::new(Node::Num(ExactNum::two())),
                                )),
                            )))),
                        )))),
                        Box::new(fp),
                    ))
                }
                "arctan" => {
                    if args.len() != 1 {
                        return Err("arctan function requires exactly one argument".to_string());
                    }
                    // d/dx(arctan(f)) = 1/(1+f²) · f'
                    let f = &args[0];
                    let fp = differentiate(f, var_name)?;
                    Ok(Node::Multiply(
                        Box::new(Node::Divide(
                            Box::new(Node::Num(ExactNum::one())),
                            Box::new(Node::Add(
                                Box::new(Node::Num(ExactNum::one())),
                                Box::new(Node::Power(
                                    Box::new(f.clone()),
                                    Box::new(Node::Num(ExactNum::two())),
                                )),
                            )),
                        )),
                        Box::new(fp),
                    ))
                }
                "abs" => {
                    if args.len() != 1 {
                        return Err("abs function requires exactly one argument".to_string());
                    }

                    // Same as Node::Abs case
                    let operand = &args[0];
                    let operand_derivative = differentiate(operand, var_name)?;

                    // sgn(f) as f/|f|
                    let sign = Node::Divide(
                        Box::new(operand.clone()),
                        Box::new(Node::Function("abs".to_string(), vec![operand.clone()])),
                    );

                    // sgn(f) * df/dx
                    Ok(Node::Multiply(Box::new(sign), Box::new(operand_derivative)))
                }
                _ => Err(format!(
                    "Differentiation not implemented for function: {}",
                    name
                )),
            }
        }

        // Not yet implemented for other node types
        _ => Err(format!(
            "Differentiation not implemented for this expression type: {:?}",
            expr
        )),
    }
}

/// Computes the partial derivative of an expression with respect to a variable
pub fn partial_derivative(expr: &Node, var_name: &str) -> Result<Node, String> {
    // For now, the implementation is the same as the regular derivative
    differentiate(expr, var_name)
}

/// Differentiate a LaTeX expression and evaluate at the given environment.
/// This avoids the lossy round-trip through Display formatting.
pub fn differentiate_and_evaluate(
    latex_expr: &str,
    var_name: &str,
    env: &crate::environment::Environment,
) -> Result<f64, String> {
    let mut tokenizer = crate::tokenizer::Tokenizer::new(latex_expr);
    let tokens = tokenizer.tokenize();
    let expr = crate::parser::build_expression_tree(tokens)?;
    let derivative = differentiate(&expr, var_name)?;
    crate::evaluator::Evaluator::evaluate(&derivative, env)
}

/// Differentiate a LaTeX expression with respect to a variable
///
/// # Arguments
///
/// * `latex_expr` - The LaTeX expression to differentiate
/// * `var_name` - The variable to differentiate with respect to
///
/// # Returns
///
/// The derivative of the expression as a LaTeX string
pub fn differentiate_latex(latex_expr: &str, var_name: &str) -> Result<String, String> {
    let mut tokenizer = crate::tokenizer::Tokenizer::new(latex_expr);
    let tokens = tokenizer.tokenize();
    let expr = crate::parser::build_expression_tree(tokens)?;
    let derivative = differentiate(&expr, var_name)?;
    let env = crate::environment::Environment::new();
    let simplified =
        crate::simplify::Simplifiable::simplify(&derivative, &env).unwrap_or(derivative);
    Ok(format!("{}", simplified))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluator::Evaluator;
    use crate::parser::build_expression_tree;
    use crate::tokenizer::Tokenizer;
    use crate::Environment;

    fn parse_expression(latex: &str) -> Result<Node, String> {
        let mut tokenizer = Tokenizer::new(latex);
        let tokens = tokenizer.tokenize();
        build_expression_tree(tokens)
    }

    fn evaluate_expression(expr: &Node, env: &Environment) -> Result<f64, String> {
        Evaluator::evaluate(expr, env)
    }

    #[test]
    fn test_derivative_of_constant() {
        // d/dx(5) = 0
        let expr = parse_expression("5").unwrap();
        let derivative = differentiate(&expr, "x").unwrap();

        let env = Environment::new();
        let result = evaluate_expression(&derivative, &env).unwrap();
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_derivative_of_variable() {
        // d/dx(x) = 1
        let expr = parse_expression("x").unwrap();
        let derivative = differentiate(&expr, "x").unwrap();

        let env = Environment::new();
        let result = evaluate_expression(&derivative, &env).unwrap();
        assert_eq!(result, 1.0);

        // d/dy(x) = 0
        let derivative = differentiate(&expr, "y").unwrap();
        let result = evaluate_expression(&derivative, &env).unwrap();
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_derivative_of_sum() {
        // d/dx(x + 5) = 1
        let expr = parse_expression("x + 5").unwrap();
        let derivative = differentiate(&expr, "x").unwrap();

        let env = Environment::new();
        let result = evaluate_expression(&derivative, &env).unwrap();
        assert_eq!(result, 1.0);

        // d/dx(x + y) = 1
        let expr = parse_expression("x + y").unwrap();
        let derivative = differentiate(&expr, "x").unwrap();

        let mut env = Environment::new();
        env.set("y", 3.0); // Value doesn't matter
        let result = evaluate_expression(&derivative, &env).unwrap();
        assert_eq!(result, 1.0);

        // d/dy(x + y) = 1
        let derivative = differentiate(&expr, "y").unwrap();
        let result = evaluate_expression(&derivative, &env).unwrap();
        assert_eq!(result, 1.0);
    }

    #[test]
    fn test_derivative_of_product_notation() {
        // d/dx ∏_{i=1}^{3} (x+i): at x=0 → 2·3 + 1·3 + 1·2 = 11
        let expr = parse_expression("\\prod_{i=1}^{3} {x + i}").unwrap();
        let derivative = differentiate(&expr, "x").unwrap();
        let mut env = Environment::new();
        env.set("x", 0.0);
        assert_eq!(evaluate_expression(&derivative, &env).unwrap(), 11.0);

        // ∏_{i=1}^{3} (x·i) = 6x³ → d/dx = 18x², at x=2 → 72
        let expr = parse_expression("\\prod_{i=1}^{3} {x \\cdot i}").unwrap();
        let derivative = differentiate(&expr, "x").unwrap();
        env.set("x", 2.0);
        assert_eq!(evaluate_expression(&derivative, &env).unwrap(), 72.0);

        // Index-only body: d/dx ∏_{k=1}^{5} k = 0
        let expr = parse_expression("\\prod_{k=1}^{5} k").unwrap();
        let derivative = differentiate(&expr, "x").unwrap();
        assert_eq!(evaluate_expression(&derivative, &env).unwrap(), 0.0);

        // Bound index: d/dk ∏_{k=1}^{3} k = 0
        let derivative = differentiate(&expr, "k").unwrap();
        assert_eq!(evaluate_expression(&derivative, &env).unwrap(), 0.0);
    }

    #[test]
    fn test_derivative_of_product() {
        // d/dx(x * 5) = 5
        let expr = parse_expression("x * 5").unwrap();
        let derivative = differentiate(&expr, "x").unwrap();

        // Set up environment with x defined to avoid variable not defined error
        let mut env = Environment::new();
        env.set("x", 1.0); // Value doesn't matter for this test
        let result = evaluate_expression(&derivative, &env).unwrap();
        assert_eq!(result, 5.0);

        // d/dx(x * y) = y
        let expr = parse_expression("x * y").unwrap();

        // Set the environment before differentiation to include both variables
        let mut env = Environment::new();
        env.set("x", 1.0); // Value doesn't matter for this test
        env.set("y", 3.0);

        let derivative = differentiate(&expr, "x").unwrap();
        let result = evaluate_expression(&derivative, &env).unwrap();
        assert_eq!(result, 3.0);

        // d/dx(x^2 * y) = 2x * y
        let expr = parse_expression("x^2 * y").unwrap();

        let derivative = differentiate(&expr, "x").unwrap();

        // Print the derivative for debugging
        println!("Derivative of x^2 * y: {}", derivative);

        let mut env = Environment::new();
        env.set("x", 2.0);
        env.set("y", 3.0);
        let result = evaluate_expression(&derivative, &env).unwrap();
        assert_eq!(
            result, 12.0,
            "Expected 12.0 but got {}. Derivative = {}",
            result, derivative
        ); // 2*2*3 = 12
    }

    #[test]
    fn test_derivative_of_quotient() {
        // d/dx(1/x) = -1/x^2
        let expr = parse_expression("1/x").unwrap();
        let derivative = differentiate(&expr, "x").unwrap();

        let mut env = Environment::new();
        env.set("x", 2.0);
        let result = evaluate_expression(&derivative, &env).unwrap();
        assert_eq!(result, -0.25); // -1/4

        // d/dx(y/x) = -y/x^2
        let expr = parse_expression("y/x").unwrap();
        let derivative = differentiate(&expr, "x").unwrap();

        let mut env = Environment::new();
        env.set("x", 2.0);
        env.set("y", 3.0);
        let result = evaluate_expression(&derivative, &env).unwrap();
        assert_eq!(result, -0.75); // -3/4
    }

    #[test]
    fn test_power_rule() {
        // d/dx(x^2) = 2x
        let expr = parse_expression("x^2").unwrap();
        let derivative = differentiate(&expr, "x").unwrap();

        let mut env = Environment::new();
        env.set("x", 3.0);
        let result = evaluate_expression(&derivative, &env).unwrap();
        assert_eq!(result, 6.0); // 2*3 = 6

        // d/dx(x^3) = 3x^2
        let expr = parse_expression("x^3").unwrap();
        let derivative = differentiate(&expr, "x").unwrap();

        let mut env = Environment::new();
        env.set("x", 2.0);
        let result = evaluate_expression(&derivative, &env).unwrap();
        assert_eq!(result, 12.0); // 3*2^2 = 12

        // d/dx(x^(-1)) = -x^(-2)
        let expr = parse_expression("x^{-1}").unwrap();
        let derivative = differentiate(&expr, "x").unwrap();

        let mut env = Environment::new();
        env.set("x", 2.0);
        let result = evaluate_expression(&derivative, &env).unwrap();
        assert_eq!(result, -0.25); // -1/4
    }

    #[test]
    fn test_chain_rule() {
        // d/dx((2x+1)^2) = 2*2*(2x+1)
        let expr = parse_expression("(2*x+1)^2").unwrap();
        let derivative = differentiate(&expr, "x").unwrap();

        let mut env = Environment::new();
        env.set("x", 1.0);
        let result = evaluate_expression(&derivative, &env).unwrap();
        assert_eq!(result, 12.0); // 4*(2*1+1) = 12

        // d/dx(sqrt(x)) = 1/(2*sqrt(x))
        let expr = parse_expression("\\sqrt{x}").unwrap();
        let derivative = differentiate(&expr, "x").unwrap();

        let mut env = Environment::new();
        env.set("x", 4.0);
        let result = evaluate_expression(&derivative, &env).unwrap();
        assert_eq!(result, 0.25); // 1/(2*sqrt(4)) = 1/4
    }

    #[test]
    fn test_complex_derivatives() {
        // d/dx(x^2 + 2x + 1) = 2x + 2
        let expr = parse_expression("x^2 + 2*x + 1").unwrap();
        let derivative = differentiate(&expr, "x").unwrap();

        let mut env = Environment::new();
        env.set("x", 3.0);
        let result = evaluate_expression(&derivative, &env).unwrap();
        assert_eq!(result, 8.0); // 2*3 + 2 = 8

        // d/dx(x^3 - 3x^2 + 3x - 1) = 3x^2 - 6x + 3
        let expr = parse_expression("x^3 - 3*x^2 + 3*x - 1").unwrap();
        let derivative = differentiate(&expr, "x").unwrap();

        let mut env = Environment::new();
        env.set("x", 2.0);
        let result = evaluate_expression(&derivative, &env).unwrap();

        // Print the derivative for debugging
        println!("Derivative: {}", derivative);

        // Calculate: 3*(2^2) - 6*2 + 3 = 3*4 - 12 + 3 = 12 - 12 + 3 = 3
        assert_eq!(
            result, 3.0,
            "Expected 3.0 but got {}. Derivative = {}",
            result, derivative
        );
    }

    #[test]
    fn test_latex_differentiation() {
        // d/dx(x^2) should produce a valid LaTeX expression
        let result = differentiate_latex("x^2", "x").unwrap();

        // Parse and evaluate the result to check if it's valid and correct
        let expr = parse_expression(&result).unwrap();
        let mut env = Environment::new();
        env.set("x", 3.0);
        let eval_result = evaluate_expression(&expr, &env).unwrap();
        assert_eq!(eval_result, 6.0); // 2*3 = 6
    }

    #[test]
    fn test_polynomial_derivative_canonical_form() {
        // d/dx(x^3 + 3x^2 + 3x + 1) = 3x^2 + 6x + 3
        let expr = parse_expression("x^3 + 3*x^2 + 3*x + 1").unwrap();
        let derivative = differentiate(&expr, "x").unwrap();
        assert_eq!(format!("{}", derivative), "3x^{2} + 6x + 3");
    }

    #[test]
    fn test_polynomial_derivative_single_term() {
        // d/dx(5x^4) = 20x^3
        let expr = parse_expression("5*x^4").unwrap();
        let derivative = differentiate(&expr, "x").unwrap();
        assert_eq!(format!("{}", derivative), "20x^{3}");
    }

    #[test]
    fn test_polynomial_derivative_constant() {
        // d/dx(42) = 0
        let expr = parse_expression("42").unwrap();
        let derivative = differentiate(&expr, "x").unwrap();
        assert_eq!(format!("{}", derivative), "0");
    }
}
