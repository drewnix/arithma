#[cfg(test)]
mod round_trip_tests {
    use arithma::{parse_latex, Environment, Evaluator};

    fn round_trip(latex: &str) -> (String, String) {
        let env = Environment::new();
        let expr = parse_latex(latex, &env).unwrap();
        let display1 = format!("{}", expr);
        let expr2 = parse_latex(&display1, &env).unwrap();
        let display2 = format!("{}", expr2);
        (display1, display2)
    }

    fn assert_round_trip(latex: &str) {
        let (d1, d2) = round_trip(latex);
        assert_eq!(
            d1, d2,
            "Round-trip failed for '{}': first='{}', second='{}'",
            latex, d1, d2
        );
    }

    fn assert_round_trip_value(latex: &str, expected: f64, env: &Environment) {
        let expr = parse_latex(latex, env).unwrap();
        let display1 = format!("{}", expr);
        let val1 = Evaluator::evaluate(&expr, env).unwrap();
        let expr2 = parse_latex(&display1, env).unwrap();
        let val2 = Evaluator::evaluate(&expr2, env).unwrap();
        assert!(
            (val1 - val2).abs() < 1e-10,
            "Round-trip value mismatch for '{}': {} vs {}",
            latex,
            val1,
            val2
        );
        assert!(
            (val1 - expected).abs() < 1e-10,
            "Value mismatch for '{}': expected {}, got {}",
            latex,
            expected,
            val1
        );
    }

    #[test]
    fn test_round_trip_integer() {
        assert_round_trip("42");
    }

    #[test]
    fn test_round_trip_fraction() {
        assert_round_trip("\\frac{1}{3}");
    }

    #[test]
    fn test_round_trip_addition() {
        assert_round_trip("3 + 5");
    }

    #[test]
    fn test_round_trip_polynomial() {
        let mut env = Environment::new();
        env.set("x", 2.0);
        assert_round_trip_value("x^{2} + 3x + 1", 11.0, &env);
    }

    #[test]
    fn test_round_trip_frac_addition() {
        let env = Environment::new();
        assert_round_trip_value("\\frac{1}{3} + \\frac{1}{6}", 0.5, &env);
    }

    #[test]
    fn test_round_trip_nested_frac() {
        let mut env = Environment::new();
        env.set("x", 4.0);
        assert_round_trip_value("\\frac{x + 1}{x - 1}", 5.0 / 3.0, &env);
    }

    #[test]
    fn test_round_trip_power() {
        let mut env = Environment::new();
        env.set("x", 3.0);
        assert_round_trip_value("x^{2}", 9.0, &env);
    }

    #[test]
    fn test_round_trip_negate() {
        assert_round_trip("-5");
    }

    #[test]
    fn test_round_trip_function_plus_constant() {
        let mut env = Environment::new();
        env.set("x", 0.0);
        assert_round_trip_value("\\sin(x) + 1", 1.0, &env);
    }
}

#[cfg(test)]
mod latex_parser_tests {
    use arithma::{build_expression_tree, Environment, Evaluator, Tokenizer};

    fn eval_latex_expression_with_env(latex: &str, env: &Environment) -> Result<f64, String> {
        // Create an instance of the Tokenizer
        let mut tokenizer = Tokenizer::new(latex); // Pass input as a reference
        let tokens = tokenizer.tokenize(); // Call the instance method on tokenizer

        // Tokenize and parse the input
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
