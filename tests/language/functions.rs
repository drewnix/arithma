#[cfg(test)]
mod function_tests {
    use arithma::{build_expression_tree, Environment, Evaluator, ExactNum, Node, Tokenizer};

    fn evaluate_expression_with_env(latex: &str, env: &Environment) -> Result<f64, String> {
        // Create an instance of the Tokenizer
        let mut tokenizer = Tokenizer::new(latex); // Pass input as a reference

        // Tokenize and parse the input
        let tokens = tokenizer.tokenize(); // Call the instance method on tokenizer
        let parsed_expr = build_expression_tree(tokens)?;
        Evaluator::evaluate(&parsed_expr, env)
    }

    fn simplify_expression(latex: &str) -> Result<Node, String> {
        let mut tokenizer = Tokenizer::new(latex);
        let tokens = tokenizer.tokenize();
        let parsed = build_expression_tree(tokens)?;
        let env = Environment::new();
        Evaluator::simplify(&parsed, &env)
    }

    // Helper function to evaluate LaTeX expression and return the result
    fn evaluate_expression(latex: &str) -> Result<f64, String> {
        let env = Environment::new();
        evaluate_expression_with_env(latex, &env)
    }

    fn evaluate_exact_expression(latex: &str) -> Result<ExactNum, String> {
        let env = Environment::new();
        let mut tokenizer = Tokenizer::new(latex);
        let parsed = build_expression_tree(tokenizer.tokenize())?;
        Evaluator::evaluate_exact(&parsed, &env)
    }

    // Integer arithmetic

    #[test]
    fn test_gcd_function() {
        assert_eq!(evaluate_expression("\\gcd{24, 36}").unwrap(), 12.0);
        assert_eq!(evaluate_expression("\\gcd{48, 18}").unwrap(), 6.0);
        assert_eq!(evaluate_expression("\\gcd{2, 3, 4}").unwrap(), 1.0);
        assert_eq!(evaluate_expression("\\gcd{12, 18, 24}").unwrap(), 6.0);
        assert!(evaluate_expression("\\gcd{6}")
            .unwrap_err()
            .contains("at least two"));
    }

    #[test]
    fn test_lcm_function() {
        assert_eq!(evaluate_expression("\\lcm{4, 6}").unwrap(), 12.0);
        assert_eq!(evaluate_expression("\\lcm{12, 18}").unwrap(), 36.0);
        assert_eq!(evaluate_expression("\\lcm{2, 3, 4}").unwrap(), 12.0);
        assert!(evaluate_expression("\\lcm{6}")
            .unwrap_err()
            .contains("at least two"));
    }

    #[test]
    fn test_factorial_function() {
        assert_eq!(evaluate_expression("\\factorial{5}").unwrap(), 120.0);
        assert_eq!(evaluate_expression("\\factorial{0}").unwrap(), 1.0);
        assert!(evaluate_expression("\\factorial{-1}")
            .unwrap_err()
            .contains("non-negative"));
        // 21! exceeds f64 exact-integer range; evaluate_exact stays exact.
        assert_eq!(
            evaluate_exact_expression("\\factorial{21}")
                .unwrap()
                .to_rational()
                .unwrap()
                .numer()
                .to_string(),
            "51090942171709440000"
        );
    }

    #[test]
    fn test_factorial_postfix() {
        assert_eq!(evaluate_expression("5!").unwrap(), 120.0);
        assert_eq!(evaluate_expression("(3+2)!").unwrap(), 120.0);
        assert_eq!(evaluate_expression("5! + 1").unwrap(), 121.0);
        assert_eq!(
            evaluate_exact_expression("21!")
                .unwrap()
                .to_rational()
                .unwrap()
                .numer()
                .to_string(),
            "51090942171709440000"
        );
    }

    #[test]
    fn test_binom_function() {
        assert_eq!(evaluate_expression("\\binom{5}{2}").unwrap(), 10.0);
        assert_eq!(evaluate_expression("\\binom{5}{0}").unwrap(), 1.0);
        assert_eq!(evaluate_expression("\\binom{0}{0}").unwrap(), 1.0);
        assert_eq!(evaluate_expression("\\binom{5}{5}").unwrap(), 1.0);
        assert_eq!(evaluate_expression("\\binom{3}{5}").unwrap(), 0.0);
        assert_eq!(evaluate_expression("\\binom{7}{3}").unwrap(), 35.0);
        assert_eq!(evaluate_expression("\\binom{10}{5}").unwrap(), 252.0);
        assert_eq!(evaluate_expression("\\binom{2+3}{2}").unwrap(), 10.0);
        assert!(evaluate_expression("\\binom{-1}{2}")
            .unwrap_err()
            .contains("non-negative"));
        assert_eq!(
            evaluate_exact_expression("\\binom{68}{34}")
                .unwrap()
                .to_rational()
                .unwrap()
                .numer()
                .to_string(),
            "28453041475240576740"
        );
    }

    #[test]
    fn test_binom_simplify() {
        let simplified = simplify_expression("\\binom{2+3}{2}").unwrap();
        assert_eq!(simplified, Node::Num(ExactNum::integer(10)));

        assert_eq!(
            simplify_expression("\\binom{0}{0}").unwrap(),
            Node::Num(ExactNum::integer(1))
        );

        let large = simplify_expression("\\binom{68}{34}").unwrap();
        if let Node::Num(n) = large {
            assert!(n.is_integer());
            assert_eq!(
                n.to_rational().unwrap().numer().to_string(),
                "28453041475240576740"
            );
        } else {
            panic!("expected numeric binom simplify result");
        }
    }

    #[test]
    fn test_factorial_simplify_parity() {
        let postfix = simplify_expression("5!").unwrap();
        let command = simplify_expression("\\factorial{5}").unwrap();
        assert_eq!(postfix, command);
        assert_eq!(postfix, Node::Num(ExactNum::integer(120)));
        assert!(matches!(postfix, Node::Num(ExactNum::Rational(_))));
    }

    #[test]
    fn test_gcd_simplify() {
        assert_eq!(
            simplify_expression("\\gcd{24, 36}").unwrap(),
            Node::Num(ExactNum::integer(12))
        );
        assert_eq!(
            simplify_expression("\\gcd{12+12, 36}").unwrap(),
            Node::Num(ExactNum::integer(12))
        );
    }

    #[test]
    fn test_gcd_simplify_bigint() {
        let simplified =
            simplify_expression("\\gcd{10000000000000000000, 30000000000000000000}").unwrap();
        if let Node::Num(n) = simplified {
            assert_eq!(
                n.to_rational().unwrap().numer().to_string(),
                "10000000000000000000"
            );
        } else {
            panic!("expected numeric gcd simplify result");
        }
    }

    #[test]
    fn test_lcm_simplify() {
        assert_eq!(
            simplify_expression("\\lcm{4, 6}").unwrap(),
            Node::Num(ExactNum::integer(12))
        );
    }

    #[test]
    fn test_lcm_simplify_bigint() {
        let simplified =
            simplify_expression("\\lcm{10000000000000000000, 30000000000000000000}").unwrap();
        if let Node::Num(n) = simplified {
            assert_eq!(
                n.to_rational().unwrap().numer().to_string(),
                "30000000000000000000"
            );
        } else {
            panic!("expected numeric lcm simplify result");
        }
    }

    #[test]
    fn test_factorial_simplify_large_exact() {
        let expected_23 = "25852016738884976640000";
        for input in ["23!", "\\factorial{23}"] {
            let simplified = simplify_expression(input).unwrap();
            if let Node::Num(n) = simplified {
                assert!(
                    matches!(n, ExactNum::Rational(_)),
                    "{input} should simplify to exact rational, not float"
                );
                assert!(n.is_integer());
                assert_eq!(n.to_rational().unwrap().numer().to_string(), expected_23);
            } else {
                panic!("expected numeric factorial simplify result for {input}");
            }
        }

        let expected_30 = "265252859812191058636308480000000";
        for input in ["30!", "\\factorial{30}"] {
            let simplified = simplify_expression(input).unwrap();
            if let Node::Num(n) = simplified {
                assert!(matches!(n, ExactNum::Rational(_)));
                assert_eq!(n.to_rational().unwrap().numer().to_string(), expected_30);
            } else {
                panic!("expected numeric factorial simplify result for {input}");
            }
        }
    }

    #[test]
    fn test_factorial_evaluate_exact_parity() {
        let env = Environment::new();
        for input in ["21!", "23!", "\\factorial{21}", "\\factorial{23}"] {
            let mut tokenizer = Tokenizer::new(input);
            let parsed = build_expression_tree(tokenizer.tokenize()).unwrap();
            let exact = Evaluator::evaluate_exact(&parsed, &env).unwrap();
            assert!(matches!(exact, ExactNum::Rational(_)));
            assert!(exact.is_integer());
        }
        assert_eq!(
            evaluate_exact_expression("21!")
                .unwrap()
                .to_rational()
                .unwrap()
                .numer()
                .to_string(),
            "51090942171709440000"
        );
        assert_eq!(
            evaluate_exact_expression("23!")
                .unwrap()
                .to_rational()
                .unwrap()
                .numer()
                .to_string(),
            "25852016738884976640000"
        );
    }

    #[test]
    fn test_factorial_precedence() {
        assert_eq!(evaluate_expression("2^3!").unwrap(), 64.0);
        assert_eq!(evaluate_expression("(2^3)!").unwrap(), 40320.0);
    }

    #[test]
    fn test_factorial_negation() {
        assert_eq!(evaluate_expression("-5!").unwrap(), -120.0);
        assert!(evaluate_expression("(-5)!")
            .unwrap_err()
            .contains("non-negative"));
    }

    #[test]
    fn test_factorial_non_integer() {
        assert!(evaluate_expression("\\factorial{5.5}")
            .unwrap_err()
            .contains("non-negative"));
    }

    #[test]
    fn test_factorial_substitution() {
        let mut env = Environment::new();
        env.set("x", 4.0);
        assert_eq!(evaluate_expression_with_env("x!", &env).unwrap(), 24.0);
    }

    #[test]
    fn test_factorial_nested_bang() {
        // 3!! parses as (3!)!, not double factorial
        assert_eq!(evaluate_expression("3!!").unwrap(), 720.0);
    }

    // Absolute value and rounding

    #[test]
    fn test_floor_ceiling_delimiters() {
        assert_eq!(evaluate_expression("\\lfloor 3.7 \\rfloor").unwrap(), 3.0);
        assert_eq!(evaluate_expression("\\lceil 3.7 \\rceil").unwrap(), 4.0);
        assert_eq!(evaluate_expression("\\lfloor -3.7 \\rfloor").unwrap(), -4.0);
        assert_eq!(evaluate_expression("\\lceil -3.7 \\rceil").unwrap(), -3.0);
    }

    #[test]
    fn test_floor_ceiling_functions() {
        assert_eq!(evaluate_expression("\\floor{3.7}").unwrap(), 3.0);
        assert_eq!(evaluate_expression("\\ceil{3.7}").unwrap(), 4.0);
        assert_eq!(evaluate_expression("\\floor{-3.7}").unwrap(), -4.0);
        assert_eq!(evaluate_expression("\\ceil{\\frac{7}{2}}").unwrap(), 4.0);
    }

    #[test]
    fn test_round_trunc_functions() {
        assert_eq!(evaluate_expression("\\round{3.5}").unwrap(), 4.0);
        assert_eq!(evaluate_expression("\\round{-3.5}").unwrap(), -4.0);
        assert_eq!(evaluate_expression("\\trunc{-3.7}").unwrap(), -3.0);
        assert_eq!(evaluate_expression("\\round{\\frac{7}{2}}").unwrap(), 4.0);
    }

    #[test]
    fn test_round_simplify_fold() {
        assert_eq!(
            simplify_expression("\\round{5}").unwrap(),
            Node::Num(ExactNum::integer(5))
        );
    }

    #[test]
    fn test_floor_simplify_fold() {
        assert_eq!(
            simplify_expression("\\floor{\\frac{7}{2}}").unwrap(),
            Node::Num(ExactNum::integer(3))
        );
    }

    #[test]
    fn test_ceil_simplify_fold() {
        assert_eq!(
            simplify_expression("\\ceil{\\frac{7}{2}}").unwrap(),
            Node::Num(ExactNum::integer(4))
        );
    }

    #[test]
    fn test_trunc_simplify_fold() {
        assert_eq!(
            simplify_expression("\\trunc{\\frac{7}{2}}").unwrap(),
            Node::Num(ExactNum::integer(3))
        );
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
        let arccsc = evaluate_expression("\\arccsc{2}").unwrap();
        assert!(
            (arccsc - std::f64::consts::FRAC_PI_6).abs() < 1e-10,
            "arccsc(2) should be π/6, got {arccsc}"
        );
        let arcsec = evaluate_expression("\\arcsec{2}").unwrap();
        assert!(
            (arcsec - std::f64::consts::FRAC_PI_3).abs() < 1e-10,
            "arcsec(2) should be π/3, got {arcsec}"
        );
        assert!(evaluate_expression("\\arcsec{0}").unwrap().is_nan());
        assert!(evaluate_expression("\\arccsc{0}").unwrap().is_nan());
        let arccot = evaluate_expression("\\arccot{1}").unwrap();
        assert!(
            (arccot - std::f64::consts::FRAC_PI_4).abs() < 1e-10,
            "arccot(1) should be π/4, got {arccot}"
        );
        assert_eq!(
            evaluate_expression("\\arccot{0}").unwrap(),
            std::f64::consts::FRAC_PI_2
        );
    }

    #[test]
    fn test_hyperbolic_basic() {
        assert_eq!(evaluate_expression("\\sinh{0}").unwrap(), 0.0);
        assert_eq!(evaluate_expression("\\cosh{0}").unwrap(), 1.0);
        assert_eq!(evaluate_expression("\\tanh{0}").unwrap(), 0.0);
    }

    #[test]
    fn test_reciprocal_hyperbolic() {
        let csch = evaluate_expression("\\csch{1}").unwrap();
        assert!(
            (csch - 1.0 / 1.0f64.sinh()).abs() < 1e-10,
            "csch(1) mismatch, got {csch}"
        );
        assert!(evaluate_expression("\\csch{0}").unwrap().is_nan());
        assert_eq!(evaluate_expression("\\sech{0}").unwrap(), 1.0);
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
        let arccsch = evaluate_expression("\\arccsch{2}").unwrap();
        assert!(
            (arccsch - (0.5f64).asinh()).abs() < 1e-10,
            "arccsch(2) mismatch, got {arccsch}"
        );
        assert!(evaluate_expression("\\arccsch{0}").unwrap().is_nan());
        let arcsech = evaluate_expression("\\arcsech{0.5}").unwrap();
        assert!(
            (arcsech - (2.0f64).acosh()).abs() < 1e-10,
            "arcsech(0.5) mismatch, got {arcsech}"
        );
        assert!(evaluate_expression("\\arcsech{2}").unwrap().is_nan());
        let arccoth = evaluate_expression("\\arccoth{2}").unwrap();
        assert!(
            (arccoth - (0.5f64).atanh()).abs() < 1e-10,
            "arccoth(2) mismatch, got {arccoth}"
        );
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
