#[cfg(test)]
mod algebra_tests {
    use arithma::{build_expression_tree, solve_for_variable, Environment, Evaluator, Tokenizer};

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

    fn approx_eq(a: f64, b: f64, epsilon: f64) -> bool {
        (a - b).abs() < epsilon
    }

    // 1. Basic Arithmetic and Operations
    #[test]
    fn test_basic_operations() {
        // Addition: 3 + 7
        let result = evaluate_expression("3 + 7").unwrap();
        assert_eq!(result, 10.0);

        // Subtraction: 10 - 4
        let result = evaluate_expression("10 - 4").unwrap();
        assert_eq!(result, 6.0);

        // Multiplication: 5 * 6
        let result = evaluate_expression("5 * 6").unwrap();
        assert_eq!(result, 30.0);

        // Division: 12 / 4
        let result = evaluate_expression("12 / 4").unwrap();
        assert_eq!(result, 3.0);

        // Division: 12 / 0
        let result = evaluate_expression("12 / 0").unwrap();
        assert!(
            result.is_nan(),
            "Expected NaN for division by zero (12 / 0), got {:?}",
            result
        );

        // Power: 2^3
        let result = evaluate_expression("2^{3}").unwrap();
        assert_eq!(result, 8.0);

        // Square Root: sqrt(16)
        let result = evaluate_expression("\\sqrt{16}").unwrap();
        assert_eq!(result, 4.0);
    }

    #[test]
    fn test_fractions() {
        // Addition with fraction
        let result: f64 = evaluate_expression("1+\\frac{2}{3}").unwrap();
        assert!(
            approx_eq(result, 1.6666666666, 1e-9),
            "Expected approximately {}, got {}",
            1.6666666666,
            result
        );

        // Abbreviated fraction syntax
        let result: f64 = evaluate_expression("1+\\frac23").unwrap();
        assert!(
            approx_eq(result, 1.6666666666, 1e-9),
            "Expected approximately {}, got {}",
            1.6666666666,
            result
        );
    }

    #[test]
    fn test_frac_function_incorrect_args() {
        let result = evaluate_expression("\\frac{3}").unwrap_err();
        assert_eq!(result, "Not enough operands for function frac");

        // TODO: improve error message for this case
        // let result = evaluate_expression("\\frac{3}{4}{5}").unwrap_err();
        // assert!(result.contains("too many arguments"));
    }

    // 2. Polynomials
    #[test]
    fn test_polynomials() {
        let mut env = Environment::new();

        // Polynomial: x^2 + 5x + 6
        env.set("x", 2.0); // Set x = 2
        let result = evaluate_expression_with_env("x^{2} + 5 * x + 6", &env).unwrap();
        assert_eq!(result, 20.0);
    }

    // 3. Rational Expressions
    #[test]
    fn test_rational_expression() {
        let mut env = Environment::new();

        // Rational Expression: (x^2 - 1) / (x - 1)
        env.set("x", 2.0); // Set x = 2
        let result = evaluate_expression_with_env("(x^{2} -1) / (x - 1)", &env).unwrap();
        assert_eq!(result, 3.0);
    }

    // 4. Linear Equations and Systems
    #[test]
    fn test_linear_equation() -> Result<(), Box<dyn std::error::Error>> {
        // Create an instance of the Tokenizer
        let mut tokenizer = Tokenizer::new("2 * x + 5 = 11"); // Pass input as a reference

        // Tokenize and parse the input
        let tokens = tokenizer.tokenize(); // Call the instance method on tokenizer
        let parsed_expr = build_expression_tree(tokens)?;
        let solution = solve_for_variable(&parsed_expr, "x").unwrap();
        assert_eq!(solution, 3.0);

        Ok(())
    }

    // 5. Quadratic Equations
    #[test]
    #[ignore]
    fn test_quadratic_equation() {
        let mut env = Environment::new();

        // Quadratic Equation: x^2 - 4 = 0
        env.set("x", 2.0); // Set x = 2
        let result = evaluate_expression_with_env("x^{2} - 4 = 0", &env).unwrap();
        assert_eq!(result, 0.0);
    }

    // 6. Exponential and Logarithmic Functions
    #[test]
    fn test_exponential_function() {
        let mut env = Environment::new();

        // Exponential: e^x (approximation, using e ≈ 2.718)
        env.set("x", 2.0); // Set x = 2
        let result: f64 = evaluate_expression_with_env("e^{x}", &env).unwrap();
        let expected = std::f64::consts::E.powf(2.0); // e^2
        assert!(
            approx_eq(result, expected, 1e-9),
            "Expected approximately {}, got {}",
            expected,
            result
        );
    }

    #[test]
    fn test_logarithmic_function() {
        let env = Environment::new();
        let result: f64 = evaluate_expression_with_env("\\ln{20.08553692318767}", &env).unwrap();
        assert_eq!(result, 3.0);

        let result = evaluate_expression("\\log{100}").unwrap();
        assert_eq!(result, 2.0); // log10(100) = 2

        let result = evaluate_expression("\\lg{8}").unwrap();
        assert_eq!(result, 3.0); // log2(8) = 3
    }

    #[test]
    fn test_pi() {
        let pi_expr_1 = "\\pi*2";
        let env = Environment::new();

        let result_1 = evaluate_expression_with_env(pi_expr_1, &env).unwrap();
        assert_eq!(result_1, std::f64::consts::PI * 2.0)
    }

    #[test]
    fn test_eulers_number() {
        let latex_expr_1 = "e^2"; // Plain 'e'
        let latex_expr_2 = "\\mathrm{e}^2"; // LaTeX \mathrm{e}

        let env = Environment::new();

        let result_1 = evaluate_expression_with_env(latex_expr_1, &env).unwrap();
        let result_2 = evaluate_expression_with_env(latex_expr_2, &env).unwrap();

        assert_eq!(result_1, std::f64::consts::E.powf(2.0)); // e^2
        assert_eq!(result_2, std::f64::consts::E.powf(2.0)); // e^2
    }

    #[test]
    fn test_combination_of_constants() {
        let result = evaluate_expression("\\pi + e").unwrap();
        assert_eq!(result, std::f64::consts::E + std::f64::consts::PI);
    }

    // 7. Radicals and Rational Exponents
    #[test]
    fn test_rational_exponent() {
        let mut env = Environment::new();

        // Rational Exponent: x^(1/2) = sqrt(x)
        env.set("x", 9.0);
        let result: f64 = evaluate_expression_with_env("x^{1/2}", &env).unwrap();
        assert_eq!(result, 3.0);
    }

    #[test]
    fn test_inequality() {
        let mut env = Environment::new();

        // Inequality: x + 2 > 5
        env.set("x", 4.0);
        let result: f64 = evaluate_expression_with_env("x + 2 > 5", &env).unwrap();
        assert_eq!(result, 1.0); // 1.0 for true
    }

    #[test]
    fn test_greater_than() {
        // 5 > 3
        let result: f64 = evaluate_expression("5 > 3").unwrap();
        assert_eq!(result, 1.0); // 1.0 for true
    }

    #[test]
    fn test_greater_equal() {
        // 5 >= 5
        let result: f64 = evaluate_expression("5 >= 5").unwrap();
        assert_eq!(result, 1.0); // 1.0 for true
    }

    #[test]
    fn test_lesser_than() {
        // 2 < 4
        let result: f64 = evaluate_expression("2 < 4").unwrap();
        assert_eq!(result, 1.0); // 1.0 for true
    }

    #[test]
    fn test_less_equal() {
        // 3 <= 3
        let result: f64 = evaluate_expression("3 <= 3").unwrap();
        assert_eq!(result, 1.0); // 1.0 for true
    }

    #[test]
    fn test_false_inequality() {
        // 10 < 5
        let result: f64 = evaluate_expression("10 < 5").unwrap();
        assert_eq!(result, 0.0); // 0.0 for false
    }

    #[test]
    fn test_combined_negative_numbers() {
        let result: f64 = evaluate_expression("5 + -3").unwrap();
        assert_eq!(result, 2.0);
    }

    #[test]
    fn test_negative_numbers_neg_result() {
        let result: f64 = evaluate_expression("-5").unwrap();
        assert_eq!(result, -5.0);
    }

    #[test]
    fn test_absolute_value() {
        let mut env = Environment::new();
        env.set("x", -5.0); // Set x = -5

        let result = evaluate_expression_with_env("\\left|x + 2\\right|", &env).unwrap();
        assert_eq!(result, 3.0); // | -5 + 2 | = | -3 | = 3

        let result = evaluate_expression_with_env("\\left|-5\\right|", &env).unwrap();
        assert_eq!(result, 5.0); // | -5 | = 5
    }

    // #[test]
    // fn test_sec_function() {
    //     let env = Environment::new();
    //
    //     // sec(π/3) = 1/cos(π/3) = 2
    //     let result = evaluate_expression_with_env("\\sec{\\frac{\\pi}{3}}", &env).unwrap();
    //     assert!((result - 2.0).abs() < 1e-6);
    //
    //     // sec(π/2) is undefined, cos(π/2) = 0
    //     assert!(evaluate_expression_with_env("\\sec{\\frac{\\pi}{2}}", &env).is_err());
    // }

    #[test]
    #[ignore]
    fn test_sec_function() {
        let sec_expr = "\\sec(1.5708)"; // Approximate value of pi/2, where sec(x) is undefined
        let env = Environment::new();

        let result = evaluate_expression_with_env(sec_expr, &env).unwrap();
        assert!(
            result.is_nan(),
            "Expected NaN for \\sec(pi/2), got {:?}",
            result
        );
    }

    // #[test]
    // fn test_csc_function() {
    //     let env = Environment::new();
    //
    //     // csc(π/3) = 1/sin(π/3) = 2/sqrt(3)
    //     let result = evaluate_expression_with_env("\\csc{\\frac{\\pi}{3}}", &env).unwrap();
    //     assert!((result - (2.0 / 3f64.sqrt())).abs() < 1e-6);
    //
    //     // csc(π) is undefined, sin(π) = 0
    //     assert!(evaluate_expression_with_env("\\csc{\\pi}", &env).is_err());
    // }

    #[test]
    fn test_csc_function() {
        let csc_expr = "\\csc(0)"; // Cosecant is undefined at 0
        let env = Environment::new();

        let result = evaluate_expression_with_env(csc_expr, &env).unwrap();
        assert!(
            result.is_nan(),
            "Expected NaN for \\csc(0), got {:?}",
            result
        );
    }

    #[test]
    fn test_coth_function() {
        let env = Environment::new();

        // coth(1) = 1/tanh(1)
        let result = evaluate_expression_with_env("\\coth{1}", &env).unwrap();
        assert!((result - (1.0 / 1.0f64.tanh())).abs() < 1e-6);

        // coth(0) is undefined, tanh(0) = 0
        let result_1 = evaluate_expression_with_env("\\coth(0)", &env).unwrap();
        assert!(
            result_1.is_nan(),
            "Expected NaN for coth(0), got {:?}",
            result_1
        );
    }
    
    #[test]
    fn test_summation() {
        let env = Environment::new();
        
        // Sum of integers from 1 to 5
        let result = evaluate_expression_with_env("\\sum_{i=1}^{5} i", &env).unwrap();
        assert_eq!(result, 15.0);
        
        // Sum of squares from 1 to 4
        let result = evaluate_expression_with_env("\\sum_{i=1}^{4} i^2", &env).unwrap();
        assert_eq!(result, 30.0);
        
        // Sum of cubes from 1 to 3
        let result = evaluate_expression_with_env("\\sum_{i=1}^{3} i^3", &env).unwrap();
        assert_eq!(result, 36.0);
        
        // Gauss's formula: sum of 1 to n = n(n+1)/2
        let n = 10;
        let result = evaluate_expression_with_env("\\sum_{i=1}^{10} i", &env).unwrap();
        assert_eq!(result, (n * (n + 1)) as f64 / 2.0);
    }

    #[test]
    fn test_min_and_max_functions() {
        let env = Environment::new();

        // min(3, 1, 4, 2) = 1
        let result = evaluate_expression_with_env("\\min{3, 1, 4, 2}", &env).unwrap();
        assert!((result - 1.0).abs() < 1e-6);

        // min(5) = 5 (single argument)
        let result = evaluate_expression_with_env("\\min{5}", &env).unwrap();
        assert!((result - 5.0).abs() < 1e-6);

        // min() should panic or return an error
        assert!(evaluate_expression_with_env("\\min{}", &env).is_err());

        // Test max with no arguments
        let result = evaluate_expression("\\max{}").unwrap_err();
        assert!(result.contains("max requires at least one argument"));

        // max(3, 1, 4, 2) = 4
        let result = evaluate_expression("\\max{3, 1, 4, 2}").unwrap();
        assert_eq!(result, 4.0);
    }

    #[test]
    #[ignore]
    fn test_max_function() {
        let env = Environment::new();

        // max(3, 1, 4, 2) = 4
        let result = evaluate_expression_with_env("\\max{3, 1, 4, 2}", &env).unwrap();
        assert!((result - 4.0).abs() < 1e-6);

        // max(7) = 7 (single argument)
        let result = evaluate_expression_with_env("\\max{7}", &env).unwrap();
        assert!((result - 7.0).abs() < 1e-6);

        // max() should panic or return an error
        assert!(evaluate_expression_with_env("\\max{}", &env).is_err());
    }

    #[test]
    #[ignore]
    fn test_det_function() {
        let env = Environment::new();

        // det(2, 3, 4) = 2 * 3 * 4 = 24
        let result = evaluate_expression_with_env("\\det{2, 3, 4}", &env).unwrap();
        assert!((result - 24.0).abs() < 1e-6);

        // det(5) = 5 (single argument)
        let result = evaluate_expression_with_env("\\det{5}", &env).unwrap();
        assert!((result - 5.0).abs() < 1e-6);

        // det() should panic or return an error
        assert!(evaluate_expression_with_env("\\det{}", &env).is_err());
    }
}
