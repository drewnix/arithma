#[cfg(test)]
mod function_tests {
    use arithma::{build_expression_tree, Environment, Evaluator, Tokenizer};

    fn evaluate_expression_with_env(latex: &str, env: &Environment) -> Result<f64, String> {
        // Create an instance of the Tokenizer
        let mut tokenizer = Tokenizer::new(latex); // Pass input as a reference

        // Tokenize and parse the input
        let tokens = tokenizer.tokenize(); // Call the instance method on tokenizer
        let parsed_expr = build_expression_tree(tokens)?;
        Evaluator::evaluate(&parsed_expr, env)
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
    fn test_circular_trig_basic() {
        assert_eq!(evaluate_expression("\\sin{0}").unwrap(), 0.0);
        assert_eq!(evaluate_expression("\\cos{0}").unwrap(), 1.0);
        let tan = evaluate_expression("\\tan{\\frac{\\pi}{4}}").unwrap();
        assert!(
            (tan - 1.0).abs() < 1e-10,
            "tan(π/4) should be 1.0, got {tan}"
        );
    }

    #[test]
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
    fn test_cot_function() {
        let result = evaluate_expression("\\cot{\\frac{\\pi}{4}}").unwrap();
        assert!(
            (result - 1.0).abs() < 1e-10,
            "cot(π/4) should be 1.0, got {}",
            result
        );
    }

    #[test]
    fn test_inverse_circular_trig() {
        assert_eq!(evaluate_expression("\\arcsin{0}").unwrap(), 0.0);
        assert_eq!(evaluate_expression("\\arccos{1}").unwrap(), 0.0);
        let arctan = evaluate_expression("\\arctan{1}").unwrap();
        assert!(
            (arctan - std::f64::consts::FRAC_PI_4).abs() < 1e-10,
            "arctan(1) should be π/4, got {arctan}"
        );
    }

    #[test]
    fn test_inverse_reciprocal_trig() {
        let arccot = evaluate_expression("\\arccot{1}").unwrap();
        assert!(
            (arccot - std::f64::consts::FRAC_PI_4).abs() < 1e-10,
            "arccot(1) should be π/4, got {arccot}"
        );
        assert_eq!(
            evaluate_expression("\\arccot{0}").unwrap(),
            std::f64::consts::FRAC_PI_2
        );
        let arcsec = evaluate_expression("\\arcsec{2}").unwrap();
        assert!(
            (arcsec - std::f64::consts::FRAC_PI_3).abs() < 1e-10,
            "arcsec(2) should be π/3, got {arcsec}"
        );
        assert!(evaluate_expression("\\arcsec{0}").unwrap().is_nan());
        assert!(evaluate_expression("\\arccsc{0}").unwrap().is_nan());
    }

    #[test]
    fn test_hyperbolic_basic() {
        assert_eq!(evaluate_expression("\\sinh{0}").unwrap(), 0.0);
        assert_eq!(evaluate_expression("\\cosh{0}").unwrap(), 1.0);
        assert_eq!(evaluate_expression("\\tanh{0}").unwrap(), 0.0);
    }

    #[test]
    fn test_reciprocal_hyperbolic() {
        assert_eq!(evaluate_expression("\\sech{0}").unwrap(), 1.0);
        let csch = evaluate_expression("\\csch{1}").unwrap();
        assert!(
            (csch - 1.0 / 1.0f64.sinh()).abs() < 1e-10,
            "csch(1) mismatch, got {csch}"
        );
        assert!(evaluate_expression("\\csch{0}").unwrap().is_nan());
        assert!(evaluate_expression("\\coth{0}").unwrap().is_nan());
    }

    #[test]
    fn test_inverse_hyperbolic() {
        assert_eq!(evaluate_expression("\\arcsinh{0}").unwrap(), 0.0);
        assert_eq!(evaluate_expression("\\arccosh{1}").unwrap(), 0.0);
        assert_eq!(evaluate_expression("\\arctanh{0}").unwrap(), 0.0);
        assert!(evaluate_expression("\\arccosh{0}").unwrap().is_nan());
        assert!(evaluate_expression("\\arctanh{1}").unwrap().is_nan());
    }

    #[test]
    fn test_inverse_reciprocal_hyperbolic() {
        let arcsech = evaluate_expression("\\arcsech{0.5}").unwrap();
        assert!(
            (arcsech - (2.0f64).acosh()).abs() < 1e-10,
            "arcsech(0.5) mismatch, got {arcsech}"
        );
        let arccoth = evaluate_expression("\\arccoth{2}").unwrap();
        assert!(
            (arccoth - (0.5f64).atanh()).abs() < 1e-10,
            "arccoth(2) mismatch, got {arccoth}"
        );
        assert!(evaluate_expression("\\arccsch{0}").unwrap().is_nan());
        assert!(evaluate_expression("\\arcsech{2}").unwrap().is_nan());
    }

    #[test]
    fn test_nested_functions() {
        // Test a nested function call: sin(log(100)) where log(100) = 2
        let result = evaluate_expression("\\sin{\\log{100}}").unwrap();
        assert_eq!(result, 2.0f64.sin());
    }

    #[test]
    fn test_exp_function() {
        let result = evaluate_expression("\\exp{1}").unwrap(); // exp(1) = e
        assert_eq!(result, std::f64::consts::E);
    }

    #[test]
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
    fn test_lim_sinx_over_x() {
        // lim_{x→0} sin(x)/x = 1
        let result = arithma::compute_limit(
            &arithma::Node::Divide(
                Box::new(arithma::Node::Function(
                    "sin".to_string(),
                    vec![arithma::Node::Variable("x".to_string())],
                )),
                Box::new(arithma::Node::Variable("x".to_string())),
            ),
            "x",
            &arithma::ExactNum::zero(),
        )
        .unwrap();
        assert!((result.to_f64() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_limsup_function() {
        let result = evaluate_expression("\\limsup{1, 3, 2, 5}").unwrap(); // limsup(1, 3, 2, 5) = 5
        assert_eq!(result, 5.0);
    }
}
