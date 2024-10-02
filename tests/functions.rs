#[cfg(test)]
mod function_tests {
    use arithma::{build_expression_tree, Environment, Evaluator, Tokenizer};

    fn evaluate_expression_with_env(latex: &str, env: &Environment) -> Result<f64, String> {
        // Create an instance of the Tokenizer
        let mut tokenizer = Tokenizer::new(latex); // Pass input as a reference

        // Tokenize and parse the input
        let tokens = tokenizer.tokenize(); // Call the instance method on tokenizer
        let parsed_expr = build_expression_tree(tokens)?;
        Evaluator::evaluate(&parsed_expr, &env)
    }

    // Helper function to evaluate LaTeX expression and return the result
    fn evaluate_expression(latex: &str) -> Result<f64, String> {
        let env = Environment::new();
        evaluate_expression_with_env(latex, &env)
    }

    #[test]
    fn test_function_arg_validation() {
        // Test sin function with incorrect number of arguments
        let result = evaluate_expression("\\sin{0, 1}").unwrap_err();
        assert!(result.contains("The expression did not resolve into a single tree."));

        // Test log function with missing arguments
        let result = evaluate_expression("\\log{}").unwrap_err();
        assert!(result.contains("Not enough operands for function log"));
    }

    #[test]
    #[ignore]
    fn test_sec_function_undefined() {
        // Test sec(π/2), which should result in an undefined value (NaN)
        let result = evaluate_expression("\\sec{\\frac{\\pi}{2}}").unwrap();
        assert!(
            result.is_nan(),
            "Expected NaN for \\sec(π/2), got {:?}",
            result
        );
    }

    #[test]
    fn test_csc_function_undefined() {
        // Test csc(0), which should result in an undefined value (NaN)
        let result = evaluate_expression("\\csc{0}").unwrap();
        assert!(
            result.is_nan(),
            "Expected NaN for \\csc(0), got {:?}",
            result
        );
    }

    #[test]
    fn test_nested_functions() {
        // Test a nested function call: sin(log(100)) where log(100) = 2
        let result = evaluate_expression("\\sin{\\log{100}}").unwrap();
        assert_eq!(result, 2.0f64.sin());
    }

    #[test]
    #[ignore]
    fn test_cot_function() {
        let result = evaluate_expression("\\cot{\\frac{\\pi}{4}}").unwrap(); // cot(π/4) = 1
        assert_eq!(result, 1.0);
    }

    #[test]
    fn test_exp_function() {
        let result = evaluate_expression("\\exp{1}").unwrap(); // exp(1) = e
        assert_eq!(result, std::f64::consts::E);
    }

    #[test]
    #[ignore]
    fn test_inf_function() {
        let result = evaluate_expression("\\inf{3, 1, 4, 2}").unwrap(); // inf(3, 1, 4, 2) = 1
        assert_eq!(result, 1.0);
    }

    #[test]
    fn test_sup_function() {
        let result = evaluate_expression("\\sup{3, 1, 4, 2}").unwrap(); // sup(3, 1, 4, 2) = 4
        assert_eq!(result, 4.0);
    }

    #[test]
    fn test_gcd_function() {
        let result = evaluate_expression("\\gcd{24, 36}").unwrap(); // gcd(24, 36) = 12
        assert_eq!(result, 12.0);
    }

    #[test]
    #[ignore]
    fn test_lim_function() {
        let result = evaluate_expression("\\lim{f(x), 0}").unwrap(); // Evaluate at a point
        assert_eq!(result, 0.0); // Placeholder
    }

    #[test]
    fn test_limsup_function() {
        let result = evaluate_expression("\\limsup{1, 3, 2, 5}").unwrap(); // limsup(1, 3, 2, 5) = 5
        assert_eq!(result, 5.0);
    }
}
