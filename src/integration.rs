use crate::node::Node;
use crate::parser::build_expression_tree;
use crate::tokenizer::Tokenizer;

/// Calculates the indefinite integral of an expression with respect to a given variable
///
/// # Arguments
///
/// * `expr` - The expression to integrate
/// * `var_name` - The variable to integrate with respect to
///
/// # Returns
///
/// The indefinite integral of the expression with respect to the given variable
pub fn integrate(expr: &Node, var_name: &str) -> Result<Node, String> {
    match expr {
        // Constants: ∫k dx = k*x + C
        Node::Number(k) => {
            if *k == 0.0 {
                // ∫0 dx = 0 + C, but we'll just return 0
                Ok(Node::Number(0.0))
            } else {
                // ∫k dx = k*x + C
                Ok(Node::Multiply(
                    Box::new(Node::Number(*k)),
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
                    Box::new(Node::Number(2.0)),
                );

                Ok(Node::Divide(
                    Box::new(x_squared),
                    Box::new(Node::Number(2.0)),
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
                    if let Node::Number(n) = &**exponent {
                        if (n + 1.0).abs() < 1e-10 {
                            // Special case: n = -1, integral is ln|x|
                            return Ok(Node::Function(
                                "ln".to_string(),
                                vec![Node::Abs(Box::new(Node::Variable(var_name.to_string())))],
                            ));
                        } else {
                            // Standard power rule: ∫x^n dx = x^(n+1)/(n+1) + C
                            let new_power = Node::Power(
                                Box::new(Node::Variable(var_name.to_string())),
                                Box::new(Node::Number(n + 1.0)),
                            );

                            return Ok(Node::Divide(
                                Box::new(new_power),
                                Box::new(Node::Number(n + 1.0)),
                            ));
                        }
                    } else if let Node::Negate(inner_exp) = &**exponent {
                        // Handle x^(-n) forms
                        if let Node::Number(n) = &**inner_exp {
                            if *n == 1.0 {
                                // Special case: x^(-1) = 1/x, integral is ln|x|
                                return Ok(Node::Function(
                                    "ln".to_string(),
                                    vec![Node::Abs(Box::new(Node::Variable(var_name.to_string())))],
                                ));
                            } else {
                                // Standard power rule with negative exponent: ∫x^(-n) dx = x^(-n+1)/(-n+1) + C
                                let new_power = Node::Power(
                                    Box::new(Node::Variable(var_name.to_string())),
                                    Box::new(Node::Number(1.0 - *n)),
                                );

                                return Ok(Node::Divide(
                                    Box::new(new_power),
                                    Box::new(Node::Number(1.0 - *n)),
                                ));
                            }
                        }
                    }
                }
            }

            // Handle more complex cases or return an error
            Err("Integration of this expression is not yet implemented".to_string())
        }

        // Multiplication by a constant: ∫(k*f) dx = k*∫f dx
        Node::Multiply(left, right) => {
            if let Node::Number(k) = &**left {
                // Factor out the constant k
                let right_integral = integrate(right, var_name)?;
                return Ok(Node::Multiply(
                    Box::new(Node::Number(*k)),
                    Box::new(right_integral),
                ));
            } else if let Node::Number(k) = &**right {
                // Factor out the constant k
                let left_integral = integrate(left, var_name)?;
                return Ok(Node::Multiply(
                    Box::new(Node::Number(*k)),
                    Box::new(left_integral),
                ));
            }

            // Handle more complex cases or return an error
            Err("Integration of this product is not yet implemented".to_string())
        }

        // Division: Special case for 1/x
        Node::Divide(left, right) => {
            if let (Node::Number(k), Node::Variable(var)) = (&**left, &**right) {
                if *k == 1.0 && var == var_name {
                    // ∫(1/x) dx = ln|x|
                    return Ok(Node::Function(
                        "ln".to_string(),
                        vec![Node::Abs(Box::new(Node::Variable(var_name.to_string())))],
                    ));
                }
            }

            // Handle more complex cases or return an error
            Err("Integration of this division is not yet implemented".to_string())
        }

        // Other cases
        _ => Err("Integration of this expression is not yet implemented".to_string()),
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
    // Parse the input expression
    let mut tokenizer = Tokenizer::new(latex_expr);
    let tokens = tokenizer.tokenize();
    let expr = build_expression_tree(tokens)?;

    // Compute the integral
    let integral = integrate(&expr, var_name)?;

    // Convert back to LaTeX
    Ok(format!("{} + C", integral))
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
}
