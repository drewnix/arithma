#[cfg(test)]
mod latex_parser_tests {
    use arithma::{build_expression_tree, tokenize, Environment, Evaluator};

    fn eval_latex_expression_with_env(latex: &str, env: &Environment) -> Result<f64, String> {
        let tokens = tokenize(latex);
        let parsed_expr = build_expression_tree(tokens)?;
        Evaluator::evaluate(&parsed_expr, &env) 
    }

    // Helper function to evaluate LaTeX expression and return the result
    fn eval_latex_expression(latex: &str) -> Result<f64, String> {
        let env = Environment::new();
        eval_latex_expression_with_env(latex, &env)
    }

    fn approx_eq(a: f64, b: f64, epsilon: f64) -> bool {
        (a - b).abs() < epsilon
    }

    #[test]
    fn test_addition() {
        let result = eval_latex_expression("3 + 2").unwrap();
        assert_eq!(result, 5.0);
    }

    #[test]
    fn test_subtraction() {
        let result = eval_latex_expression("5 - 3").unwrap();
        assert_eq!(result, 2.0);
    }

    #[test]
    fn test_multiplication() {
        let result = eval_latex_expression("4 * 2").unwrap();
        assert_eq!(result, 8.0);
    }

    #[test]
    fn test_cdot_multiplication() {
        let result = eval_latex_expression("4 \\cdot 2").unwrap();
        assert_eq!(result, 8.0);
    }

    #[test]
    fn test_division() {
        let result = eval_latex_expression("10 / 2").unwrap();
        assert_eq!(result, 5.0);
    }

    #[test]
    fn test_sin_function() {
        let result = eval_latex_expression("\\sin{0}").unwrap();
        assert_eq!(result, 0.0); // sin(0) = 0
    }

    #[test]
    fn test_cos_function() {
        let result = eval_latex_expression("\\cos{0}").unwrap();
        assert_eq!(result, 1.0); // cos(0) = 1
    }

    #[test]
    fn test_square_root() {
        let result = eval_latex_expression("\\sqrt{16}").unwrap();
        assert_eq!(result, 4.0);
    }

    #[test]
    fn test_logarithm() {
        let result = eval_latex_expression("\\ln{2.718}").unwrap();
        assert!(approx_eq(result, 1.0, 1e-3)); // Allowing for a small epsilon
    }

    #[test]
    fn test_exponentiation() {
        let result = eval_latex_expression("2 ^ 3").unwrap();
        assert_eq!(result, 8.0);
    }

    #[test]
    fn test_fraction() {
        let result = eval_latex_expression("\\frac{6}{2}").unwrap();
        assert_eq!(result, 3.0); // 6 / 2 = 3
    }

    #[test]
    fn test_combined_expression() {
        let result = eval_latex_expression("3 + 2 * \\sin{\\frac{\\pi}{2}}").unwrap();
        assert_eq!(result, 5.0); // sin(π/2) = 1, so 3 + 2 * 1 = 5
    }

    #[test]
    fn test_negative_expr() {
        let result = eval_latex_expression("5 + -3").unwrap();
        assert_eq!(result, 2.0);
    }

    #[test]
    fn test_negative_unary() {
        let result = eval_latex_expression("-5").unwrap();
        assert_eq!(result, -5.0);
    }

    #[test]
    fn test_variable_expression() {
        let mut env = Environment::new();
        env.set("x", 5.0);
        let result = eval_latex_expression_with_env("2 * x + 3", &env).unwrap();
        assert_eq!(result, 13.0); // 2 * 5 + 3 = 13
    }
}
