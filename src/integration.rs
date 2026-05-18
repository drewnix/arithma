use crate::exact::ExactNum;
use crate::node::Node;
use crate::parser::build_expression_tree;
use crate::polynomial::Polynomial;
use crate::tokenizer::Tokenizer;

pub fn integrate(expr: &Node, var_name: &str) -> Result<Node, String> {
    let env = crate::environment::Environment::new();
    let expr = &crate::simplify::Simplifiable::simplify(expr, &env).unwrap_or_else(|_| expr.clone());

    if let Ok(poly) = Polynomial::from_node(expr, var_name) {
        return Ok(poly.integral().to_node());
    }

    match expr {
        // Constants: ∫k dx = k*x + C
        Node::Num(k) => {
            if k.is_zero() {
                // ∫0 dx = 0 + C, but we'll just return 0
                Ok(Node::Num(ExactNum::zero()))
            } else {
                // ∫k dx = k*x + C
                Ok(Node::Multiply(
                    Box::new(Node::Num(k.clone())),
                    Box::new(Node::Variable(var_name.to_string())),
                ))
            }
        }

        // Variables: ∫x dx = x²/2 + C, ∫y dx = y*x + C (if y != x)
        Node::Variable(name) => {
            if name == var_name {
                // ∫x dx = x²/2 + C
                let x_squared = Node::Power(
                    Box::new(Node::Variable(name.clone())),
                    Box::new(Node::Num(ExactNum::from_f64(2.0))),
                );

                Ok(Node::Divide(
                    Box::new(x_squared),
                    Box::new(Node::Num(ExactNum::from_f64(2.0))),
                ))
            } else {
                // ∫y dx = y*x + C (y is a constant with respect to x)
                Ok(Node::Multiply(
                    Box::new(Node::Variable(name.clone())),
                    Box::new(Node::Variable(var_name.to_string())),
                ))
            }
        }

        // Addition: ∫(f+g) dx = ∫f dx + ∫g dx
        Node::Add(left, right) => {
            let left_integral = integrate(left, var_name)?;
            let right_integral = integrate(right, var_name)?;

            Ok(Node::Add(Box::new(left_integral), Box::new(right_integral)))
        }

        // Subtraction: ∫(f-g) dx = ∫f dx - ∫g dx
        Node::Subtract(left, right) => {
            let left_integral = integrate(left, var_name)?;
            let right_integral = integrate(right, var_name)?;

            Ok(Node::Subtract(
                Box::new(left_integral),
                Box::new(right_integral),
            ))
        }

        // Power of a variable: ∫x^n dx = x^(n+1)/(n+1) + C (if n ≠ -1)
        Node::Power(base, exponent) => {
            if let Node::Variable(base_var) = &**base {
                if base_var == var_name {
                    if let Node::Num(n) = &**exponent {
                        let new_exp = n.clone() + ExactNum::one();
                        if new_exp.to_f64().abs() < 1e-10 {
                            // Special case: n = -1, integral is ln|x|
                            return Ok(Node::Function(
                                "ln".to_string(),
                                vec![Node::Abs(Box::new(Node::Variable(var_name.to_string())))],
                            ));
                        } else {
                            // Standard power rule: ∫x^n dx = x^(n+1)/(n+1) + C
                            let new_power = Node::Power(
                                Box::new(Node::Variable(var_name.to_string())),
                                Box::new(Node::Num(new_exp.clone())),
                            );

                            return Ok(Node::Divide(
                                Box::new(new_power),
                                Box::new(Node::Num(new_exp)),
                            ));
                        }
                    } else if let Node::Negate(inner_exp) = &**exponent {
                        // Handle x^(-n) forms
                        if let Node::Num(n) = &**inner_exp {
                            if n.is_one() {
                                // Special case: x^(-1) = 1/x, integral is ln|x|
                                return Ok(Node::Function(
                                    "ln".to_string(),
                                    vec![Node::Abs(Box::new(Node::Variable(var_name.to_string())))],
                                ));
                            } else {
                                // Standard power rule with negative exponent: ∫x^(-n) dx = x^(-n+1)/(-n+1) + C
                                let new_exp = ExactNum::one() - n.clone();
                                let new_power = Node::Power(
                                    Box::new(Node::Variable(var_name.to_string())),
                                    Box::new(Node::Num(new_exp.clone())),
                                );

                                return Ok(Node::Divide(
                                    Box::new(new_power),
                                    Box::new(Node::Num(new_exp)),
                                ));
                            }
                        }
                    }
                }
            }

            // ∫a^x dx = a^x / ln(a) where a is a constant
            if let Node::Num(a) = &**base {
                if let Node::Variable(v) = &**exponent {
                    if v == var_name {
                        let a_to_x = Node::Power(
                            Box::new(Node::Num(a.clone())),
                            Box::new(Node::Variable(var_name.to_string())),
                        );
                        let ln_a = Node::Function(
                            "ln".to_string(),
                            vec![Node::Num(a.clone())],
                        );
                        return Ok(Node::Divide(Box::new(a_to_x), Box::new(ln_a)));
                    }
                }
            }

            Err("Integration of this expression is not yet implemented".to_string())
        }

        // Multiplication by a constant: ∫(k*f) dx = k*∫f dx
        Node::Multiply(left, right) => {
            if let Node::Num(k) = &**left {
                // Factor out the constant k
                let right_integral = integrate(right, var_name)?;
                return Ok(Node::Multiply(
                    Box::new(Node::Num(k.clone())),
                    Box::new(right_integral),
                ));
            } else if let Node::Num(k) = &**right {
                // Factor out the constant k
                let left_integral = integrate(left, var_name)?;
                return Ok(Node::Multiply(
                    Box::new(Node::Num(k.clone())),
                    Box::new(left_integral),
                ));
            }

            // Handle more complex cases or return an error
            Err("Integration of this product is not yet implemented".to_string())
        }

        // Division: Special case for 1/x
        Node::Divide(left, right) => {
            if let (Node::Num(k), Node::Variable(var)) = (&**left, &**right) {
                if k.is_one() && var == var_name {
                    // ∫(1/x) dx = ln|x|
                    return Ok(Node::Function(
                        "ln".to_string(),
                        vec![Node::Abs(Box::new(Node::Variable(var_name.to_string())))],
                    ));
                }
            }

            // ∫k/f(x) dx = k * ∫(1/f(x)) dx — factor out constant numerator
            if let Node::Num(k) = &**left {
                let one_over_right = Node::Divide(
                    Box::new(Node::Num(ExactNum::one())),
                    right.clone(),
                );
                if let Ok(inner) = integrate(&one_over_right, var_name) {
                    return Ok(Node::Multiply(
                        Box::new(Node::Num(k.clone())),
                        Box::new(inner),
                    ));
                }
            }
            // ∫f(x)/k dx = (1/k) * ∫f(x) dx — factor out constant denominator
            if let Node::Num(k) = &**right {
                if !k.is_zero() {
                    let inner = integrate(left, var_name)?;
                    let inv = ExactNum::one() / k.clone();
                    return Ok(Node::Multiply(
                        Box::new(Node::Num(inv)),
                        Box::new(inner),
                    ));
                }
            }

            Err("Integration of this division is not yet implemented".to_string())
        }

        Node::Negate(inner) => {
            let inner_integral = integrate(inner, var_name)?;
            Ok(Node::Negate(Box::new(inner_integral)))
        }

        // Standard function integrals
        Node::Function(name, args) if args.len() == 1 => {
            let arg = &args[0];
            // Only handle direct variable argument for now
            if let Node::Variable(v) = arg {
                if v == var_name {
                    return integrate_standard_function(name, var_name);
                }
            }
            // Try linear substitution: f(ax+b) where a is constant
            if let Some((a, _b)) = extract_linear_arg(arg, var_name) {
                let base_integral = integrate_standard_function(name, var_name)?;
                let inv_a = Node::Divide(
                    Box::new(Node::Num(ExactNum::one())),
                    Box::new(Node::Num(a)),
                );
                return Ok(Node::Multiply(
                    Box::new(inv_a),
                    Box::new(base_integral),
                ));
            }
            Err(format!("Integration of {}(...) with non-linear argument not yet implemented", name))
        }

        _ => Err("Integration of this expression is not yet implemented".to_string()),
    }
}

fn integrate_standard_function(name: &str, var: &str) -> Result<Node, String> {
    let x = || Node::Variable(var.to_string());
    match name {
        // ∫sin(x) = -cos(x)
        "sin" => Ok(Node::Negate(Box::new(
            Node::Function("cos".to_string(), vec![x()]),
        ))),
        // ∫cos(x) = sin(x)
        "cos" => Ok(Node::Function("sin".to_string(), vec![x()])),
        // ∫tan(x) = -ln|cos(x)|
        "tan" => Ok(Node::Negate(Box::new(Node::Function(
            "ln".to_string(),
            vec![Node::Abs(Box::new(Node::Function(
                "cos".to_string(),
                vec![x()],
            )))],
        )))),
        // ∫sec²(x) — handled if it comes through as sec*sec; skip for now
        // ∫sec(x)  = ln|sec(x) + tan(x)|
        "sec" => Ok(Node::Function(
            "ln".to_string(),
            vec![Node::Abs(Box::new(Node::Add(
                Box::new(Node::Function("sec".to_string(), vec![x()])),
                Box::new(Node::Function("tan".to_string(), vec![x()])),
            )))],
        )),
        // ∫csc(x) = -ln|csc(x) + cot(x)|
        "csc" => Ok(Node::Negate(Box::new(Node::Function(
            "ln".to_string(),
            vec![Node::Abs(Box::new(Node::Add(
                Box::new(Node::Function("csc".to_string(), vec![x()])),
                Box::new(Node::Function("cot".to_string(), vec![x()])),
            )))],
        )))),
        // ∫cot(x) = ln|sin(x)|
        "cot" => Ok(Node::Function(
            "ln".to_string(),
            vec![Node::Abs(Box::new(Node::Function(
                "sin".to_string(),
                vec![x()],
            )))],
        )),
        // ∫exp(x) = exp(x)
        "exp" => Ok(Node::Function("exp".to_string(), vec![x()])),
        // ∫ln(x) = x·ln(x) - x
        "ln" => Ok(Node::Subtract(
            Box::new(Node::Multiply(
                Box::new(x()),
                Box::new(Node::Function("ln".to_string(), vec![x()])),
            )),
            Box::new(x()),
        )),
        // ∫sinh(x) = cosh(x)
        "sinh" => Ok(Node::Function("cosh".to_string(), vec![x()])),
        // ∫cosh(x) = sinh(x)
        "cosh" => Ok(Node::Function("sinh".to_string(), vec![x()])),
        // ∫tanh(x) = ln(cosh(x))
        "tanh" => Ok(Node::Function(
            "ln".to_string(),
            vec![Node::Function("cosh".to_string(), vec![x()])],
        )),
        _ => Err(format!("Integration of {}(x) not implemented", name)),
    }
}

/// Extract (a, b) if the expression is of the form a*var + b (linear in var).
fn extract_linear_arg(expr: &Node, var: &str) -> Option<(ExactNum, ExactNum)> {
    match expr {
        Node::Variable(v) if v == var => Some((ExactNum::one(), ExactNum::zero())),
        Node::Multiply(left, right) => {
            if let (Node::Num(a), Node::Variable(v)) = (&**left, &**right) {
                if v == var {
                    return Some((a.clone(), ExactNum::zero()));
                }
            }
            if let (Node::Variable(v), Node::Num(a)) = (&**left, &**right) {
                if v == var {
                    return Some((a.clone(), ExactNum::zero()));
                }
            }
            None
        }
        Node::Add(left, right) => {
            if let Some((a, b1)) = extract_linear_arg(left, var) {
                if let Node::Num(b2) = &**right {
                    return Some((a, &b1 + b2));
                }
            }
            if let Some((a, b1)) = extract_linear_arg(right, var) {
                if let Node::Num(b2) = &**left {
                    return Some((a, &b1 + b2));
                }
            }
            None
        }
        _ => None,
    }
}

/// Integrates a LaTeX expression with respect to a variable
///
/// # Arguments
///
/// * `latex_expr` - The LaTeX expression to integrate
/// * `var_name` - The variable to integrate with respect to
///
/// # Returns
///
/// The integral of the expression as a LaTeX string
pub fn integrate_latex(latex_expr: &str, var_name: &str) -> Result<String, String> {
    let mut tokenizer = Tokenizer::new(latex_expr);
    let tokens = tokenizer.tokenize();
    let expr = build_expression_tree(tokens)?;
    let integral = integrate(&expr, var_name)?;
    let env = crate::environment::Environment::new();
    let simplified =
        crate::simplify::Simplifiable::simplify(&integral, &env).unwrap_or(integral);
    Ok(format!("{} + C", simplified))
}

/// Calculates the definite integral of an expression between two bounds
///
/// # Arguments
///
/// * `expr` - The expression to integrate
/// * `var_name` - The variable to integrate with respect to
/// * `lower` - The lower bound of integration
/// * `upper` - The upper bound of integration
///
/// # Returns
///
/// The definite integral value
pub fn definite_integral(
    expr: &Node,
    var_name: &str,
    lower: f64,
    upper: f64,
) -> Result<f64, String> {
    // First find the indefinite integral
    let indefinite = integrate(expr, var_name)?;

    // Create substitution functions to evaluate at upper and lower bounds
    let mut upper_env = crate::environment::Environment::new();
    upper_env.set(var_name, upper);

    let mut lower_env = crate::environment::Environment::new();
    lower_env.set(var_name, lower);

    // Calculate F(upper) - F(lower)
    let upper_value = crate::evaluator::Evaluator::evaluate(&indefinite, &upper_env)?;
    let lower_value = crate::evaluator::Evaluator::evaluate(&indefinite, &lower_env)?;

    Ok(upper_value - lower_value)
}

/// Calculates the definite integral of a LaTeX expression between two bounds
///
/// # Arguments
///
/// * `latex_expr` - The LaTeX expression to integrate
/// * `var_name` - The variable to integrate with respect to
/// * `lower` - The lower bound of integration
/// * `upper` - The upper bound of integration
///
/// # Returns
///
/// The definite integral value as a LaTeX string
pub fn definite_integral_latex(
    latex_expr: &str,
    var_name: &str,
    lower: f64,
    upper: f64,
) -> Result<String, String> {
    // Parse the input expression
    let mut tokenizer = Tokenizer::new(latex_expr);
    let tokens = tokenizer.tokenize();
    let expr = build_expression_tree(tokens)?;

    // Compute the definite integral
    let result = definite_integral(&expr, var_name, lower, upper)?;

    // Convert back to LaTeX
    Ok(format!("{}", result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::Environment;
    use crate::evaluator::Evaluator;

    fn parse_expression(latex: &str) -> Result<Node, String> {
        let mut tokenizer = Tokenizer::new(latex);
        let tokens = tokenizer.tokenize();
        build_expression_tree(tokens)
    }

    fn approx_eq(a: f64, b: f64, epsilon: f64) -> bool {
        (a - b).abs() < epsilon
    }

    #[test]
    fn test_integrate_constant() {
        // ∫5 dx = 5x
        let expr = parse_expression("5").unwrap();
        let integral = integrate(&expr, "x").unwrap();

        let mut env = Environment::new();
        env.set("x", 2.0);

        // Test at x=2: 5*2 = 10
        let result = Evaluator::evaluate(&integral, &env).unwrap();
        assert_eq!(result, 10.0);
    }

    #[test]
    fn test_integrate_variable() {
        // ∫x dx = x²/2
        let expr = parse_expression("x").unwrap();
        let integral = integrate(&expr, "x").unwrap();

        let mut env = Environment::new();
        env.set("x", 3.0);

        // Test at x=3: 3²/2 = 4.5
        let result = Evaluator::evaluate(&integral, &env).unwrap();
        assert_eq!(result, 4.5);
    }

    #[test]
    fn test_integrate_different_variable() {
        // ∫y dx = y*x (y is constant with respect to x)
        let expr = parse_expression("y").unwrap();
        let integral = integrate(&expr, "x").unwrap();

        let mut env = Environment::new();
        env.set("x", 2.0);
        env.set("y", 3.0);

        // Test at x=2, y=3: 3*2 = 6
        let result = Evaluator::evaluate(&integral, &env).unwrap();
        assert_eq!(result, 6.0);
    }

    #[test]
    fn test_integrate_polynomial() {
        // ∫(3x² + 2x + 1) dx = x³ + x² + x
        let expr = parse_expression("3*x^2 + 2*x + 1").unwrap();
        let integral = integrate(&expr, "x").unwrap();

        let mut env = Environment::new();
        env.set("x", 2.0);

        // Test at x=2: 2³ + 2² + 2 = 8 + 4 + 2 = 14
        let result = Evaluator::evaluate(&integral, &env).unwrap();
        assert_eq!(result, 14.0);
    }

    #[test]
    fn test_integrate_power() {
        // ∫x^3 dx = x^4/4
        let expr = parse_expression("x^3").unwrap();
        let integral = integrate(&expr, "x").unwrap();

        let mut env = Environment::new();
        env.set("x", 2.0);

        // Test at x=2: 2⁴/4 = 16/4 = 4
        let result = Evaluator::evaluate(&integral, &env).unwrap();
        assert_eq!(result, 4.0);
    }

    #[test]
    fn test_integrate_negative_power() {
        // ∫x^(-1) dx = ln|x|
        let expr = parse_expression("x^(-1)").unwrap();
        let integral = integrate(&expr, "x").unwrap();

        let mut env = Environment::new();
        env.set("x", 2.0);

        // Test at x=2: ln(2) ≈ 0.693
        let result = Evaluator::evaluate(&integral, &env).unwrap();
        assert!(approx_eq(result, 2.0_f64.ln(), 1e-10));
    }

    #[test]
    fn test_definite_integral() {
        // ∫₁³ x² dx = [x³/3]₁³ = 3³/3 - 1³/3 = 9 - 1/3 = 8.667
        let expr = parse_expression("x^2").unwrap();
        let result = definite_integral(&expr, "x", 1.0, 3.0).unwrap();

        assert!(approx_eq(result, 8.667, 0.001));
    }

    #[test]
    fn test_integrate_complex_expression() {
        // Test a more complex expression with the parts we've implemented
        // ∫(2x³ + 3x² - 4x + 5) dx = (2x⁴/4) + (3x³/3) - (4x²/2) + 5x = (x⁴/2) + x³ - 2x² + 5x
        let expr = parse_expression("2*x^3 + 3*x^2 - 4*x + 5").unwrap();
        let integral = integrate(&expr, "x").unwrap();

        let mut env = Environment::new();
        env.set("x", 2.0);

        // At x=2: (2⁴/2) + 2³ - 2*2² + 5*2 = 8 + 8 - 8 + 10 = 18
        let result = Evaluator::evaluate(&integral, &env).unwrap();
        assert_eq!(result, 18.0);
    }

    #[test]
    fn test_latex_integration() {
        // Test the LaTeX interface for integration
        let result = integrate_latex("x^2", "x").unwrap();

        // Check that it contains the expected parts, allowing for formatting variations
        assert!(
            result.contains("+ C"),
            "Result should contain constant of integration"
        );

        // Create an environment and evaluate the integral at x=2
        let mut env = Environment::new();
        env.set("x", 2.0);

        // Parse just the expression part (without "+ C")
        let expr_part = result.replace(" + C", "");
        let parsed = parse_expression(&expr_part).unwrap();

        // Evaluate at x=2: x^3/3 at x=2 should be 8/3 ≈ 2.67
        let evaluated = Evaluator::evaluate(&parsed, &env).unwrap();
        assert!(
            approx_eq(evaluated, 2.67, 0.01),
            "Integral of x^2 evaluated at x=2 should be approximately 2.67"
        );
    }

    #[test]
    fn test_polynomial_integration_canonical_form() {
        let expr = parse_expression("3*x^2 + 2*x + 1").unwrap();
        let integral = integrate(&expr, "x").unwrap();
        let form = format!("{}", integral);
        assert_eq!(form, "x^{3} + x^{2} + x");
    }

    #[test]
    fn test_polynomial_integration_single_term() {
        let expr = parse_expression("6*x^2").unwrap();
        let integral = integrate(&expr, "x").unwrap();
        let form = format!("{}", integral);
        assert_eq!(form, "2x^{3}");
    }

    #[test]
    fn test_polynomial_integration_constant() {
        let expr = parse_expression("7").unwrap();
        let integral = integrate(&expr, "x").unwrap();
        let form = format!("{}", integral);
        assert_eq!(form, "7x");
    }

    #[test]
    fn test_polynomial_integration_fractional_coeff() {
        // ∫x^2 dx = (1/3)x^3
        let expr = parse_expression("x^2").unwrap();
        let integral = integrate(&expr, "x").unwrap();
        let form = format!("{}", integral);
        assert_eq!(form, "\\frac{1}{3} \\cdot x^{3}");
    }

    #[test]
    fn test_nonpolynomial_fallback() {
        // ∫x^(-1) dx should fall through to the ln|x| path
        let expr = parse_expression("x^{-1}").unwrap();
        let integral = integrate(&expr, "x").unwrap();
        let mut env = Environment::new();
        env.set("x", std::f64::consts::E);
        let result = Evaluator::evaluate(&integral, &env).unwrap();
        assert!(approx_eq(result, 1.0, 1e-10));
    }
}
