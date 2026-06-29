#[cfg(test)]
mod algebra_tests {
    use arithma::{
        build_expression_tree, solve_for_variable, solve_for_variable_exact,
        solve_for_variable_nodes, Environment, Evaluator, Tokenizer,
    };

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
        let result = evaluate_expression("\\frac{3}");
        assert!(result.is_err(), "\\frac with one arg should error");
    }

    #[test]
    fn test_frac_addition() {
        let result = evaluate_expression("\\frac{1}{3} + \\frac{1}{6}").unwrap();
        assert!(
            approx_eq(result, 0.5, 1e-10),
            "\\frac{{1}}{{3}} + \\frac{{1}}{{6}} should be 0.5, got {}",
            result
        );
    }

    #[test]
    fn test_frac_first_operand() {
        let result = evaluate_expression("\\frac{1}{2} + 1").unwrap();
        assert!(
            approx_eq(result, 1.5, 1e-10),
            "\\frac{{1}}{{2}} + 1 should be 1.5, got {}",
            result
        );
    }

    #[test]
    fn test_frac_nested() {
        let result = evaluate_expression("\\frac{x^{2}+1}{2}").unwrap_err();
        assert!(
            result.contains("variable") || result.contains("not found") || !result.is_empty(),
            "should fail on unbound variable"
        );

        let mut env = Environment::new();
        env.set("x", 3.0);
        let result = evaluate_expression_with_env("\\frac{x^{2}+1}{2}", &env).unwrap();
        assert!(
            approx_eq(result, 5.0, 1e-10),
            "\\frac{{x^2+1}}{{2}} at x=3 should be 5.0, got {}",
            result
        );
    }

    #[test]
    fn test_function_then_operator() {
        let mut env = Environment::new();
        env.set("x", 0.0);
        let result = evaluate_expression_with_env("\\sin(x) + 1", &env).unwrap();
        assert!(
            approx_eq(result, 1.0, 1e-10),
            "\\sin(0) + 1 should be 1.0, got {}",
            result
        );
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
    fn test_sec_function() {
        let result = evaluate_expression("\\sec{\\frac{\\pi}{2}}").unwrap();
        assert!(
            result.is_nan(),
            "Expected NaN for \\sec(π/2), got {:?}",
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
        let result = evaluate_expression_with_env("\\sum_{i=1}^{4}{i^2}", &env).unwrap();
        assert_eq!(result, 30.0);

        // Sum of cubes from 1 to 3
        let result = evaluate_expression_with_env("\\sum_{i=1}^{3}{i^3}", &env).unwrap();
        assert_eq!(result, 36.0);

        // Gauss's formula: sum of 1 to n = n(n+1)/2
        let n = 10;
        let result = evaluate_expression_with_env("\\sum_{i=1}^{10} i", &env).unwrap();
        assert_eq!(result, (n * (n + 1)) as f64 / 2.0);
    }

    #[test]
    fn test_summation_edge_cases() {
        let env = Environment::new();

        // Let's test the cases individually to find the failing ones

        // Test with single term in body (should work)
        let result = evaluate_expression_with_env("\\sum_{i=1}^{5}i", &env).unwrap();
        assert_eq!(result, 15.0); // 1 + 2 + 3 + 4 + 5 = 15

        // Test with properly braced expression (should work)
        let result = evaluate_expression_with_env("\\sum_{i=1}^{3}{i^2}", &env).unwrap();
        assert_eq!(result, 14.0); // 1^2 + 2^2 + 3^2 = 14

        // Test with empty range (start > end)
        let result = evaluate_expression_with_env("\\sum_{i=5}^{3}{i}", &env).unwrap();
        assert_eq!(result, 0.0); // Empty sum should be 0

        // Test with single iteration (start = end)
        let result = evaluate_expression_with_env("\\sum_{i=7}^{7}{i^2}", &env).unwrap();
        assert_eq!(result, 49.0); // Just 7^2 = 49

        // More complex expression with proper bracing
        let result = evaluate_expression_with_env("\\sum_{i=1}^{4}{i*i + 2*i}", &env).unwrap();
        assert_eq!(result, 50.0); // (1*1+2*1) + (2*2+2*2) + (3*3+2*3) + (4*4+2*4) = 3 + 8 + 15 + 24 = 50
    }

    #[test]
    fn test_summation_complex() {
        let mut env = Environment::new();
        env.set("n", 5.0);

        // Test formula for sum of cubes: sum(i^3, i=1..n) = (n*(n+1)/2)^2
        let result = evaluate_expression_with_env("\\sum_{i=1}^{n}{i^3}", &env).unwrap();
        let expected = ((env.get("n").unwrap() * (env.get("n").unwrap() + 1.0)) / 2.0).powf(2.0);
        assert!(
            approx_eq(result, expected, 1e-9),
            "Expected sum of cubes to equal (n*(n+1)/2)^2, got {} vs {}",
            result,
            expected
        );

        // Test formula for sum of first n odd numbers = n^2
        // sum(2*i-1, i=1..n) = n^2
        let result = evaluate_expression_with_env("\\sum_{i=1}^{n}{2*i-1}", &env).unwrap();
        let expected = env.get("n").unwrap().powf(2.0);
        assert_eq!(
            result, expected,
            "Expected sum of first n odd numbers to equal n^2"
        );
    }

    #[test]
    fn test_summation_unbraced() {
        let env = Environment::new();

        // Test with unbraced upper limit - simpler case first
        let result = evaluate_expression_with_env("\\sum_{i=1}^3{i}", &env).unwrap();
        assert_eq!(result, 6.0); // 1 + 2 + 3 = 6

        // Test with no braces in the summation body but braced upper limit
        let result = evaluate_expression_with_env("\\sum_{i=1}^{3}i", &env).unwrap();
        assert_eq!(result, 6.0); // 1 + 2 + 3 = 6
    }

    #[test]
    fn test_summation_with_variables() {
        let mut env = Environment::new();
        env.set("n", 5.0);

        // Test with variable as upper bound
        let result = evaluate_expression_with_env("\\sum_{i=1}^{n}{i}", &env).unwrap();
        assert_eq!(result, 15.0); // 1 + 2 + 3 + 4 + 5 = 15

        // Test with variable in expression body
        env.set("x", 2.0);
        let result = evaluate_expression_with_env("\\sum_{i=1}^{3}{i*x}", &env).unwrap();
        assert_eq!(result, 12.0); // 1*2 + 2*2 + 3*2 = 12

        // Test manual verification of Gauss's formula
        let left = evaluate_expression_with_env("\\sum_{i=1}^{n}{i}", &env).unwrap();
        let n_val = env.get("n").unwrap();
        let expected = (n_val * (n_val + 1.0)) / 2.0;
        assert_eq!(left, expected, "Expected sum from 1 to n equals n(n+1)/2");
    }

    #[test]
    fn test_summation_error_handling() {
        // Test with invalid index variable reference
        let result = evaluate_expression("\\sum_{i=1}^{5}{j}").unwrap_err();
        assert!(result.contains("Variable 'j' is not defined"));

        // Test with invalid upper bound
        let result = evaluate_expression("\\sum_{i=1}^{k}{i}").unwrap_err();
        assert!(result.contains("Variable 'k' is not defined"));

        // Test with missing closing brace
        let result = evaluate_expression("\\sum_{i=1}^{5{i}").unwrap_err();
        assert!(
            result.contains("Unclosed upper bound brace")
                || result.contains("brace")
                || result.contains("parsing")
        );
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
    fn test_solve_quadratic_rational_roots() {
        // x² - 5x + 6 = 0  →  (x-2)(x-3) = 0  →  x = 3, x = 2
        let mut tokenizer = Tokenizer::new("x^{2} - 5*x + 6 = 0");
        let tokens = tokenizer.tokenize();
        let expr = build_expression_tree(tokens).unwrap();
        let solutions = solve_for_variable_exact(&expr, "x").unwrap();
        assert_eq!(solutions.len(), 2);
        let mut vals: Vec<f64> = solutions.iter().map(|s| s.to_f64()).collect();
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(vals[0], 2.0);
        assert_eq!(vals[1], 3.0);
    }

    #[test]
    fn test_solve_quadratic_double_root() {
        // x² - 4x + 4 = 0  →  (x-2)² = 0  →  x = 2
        let mut tokenizer = Tokenizer::new("x^{2} - 4*x + 4 = 0");
        let tokens = tokenizer.tokenize();
        let expr = build_expression_tree(tokens).unwrap();
        let solutions = solve_for_variable_exact(&expr, "x").unwrap();
        assert_eq!(solutions.len(), 1);
        assert_eq!(solutions[0].to_f64(), 2.0);
    }

    #[test]
    fn test_solve_quadratic_no_real() {
        // x² + 1 = 0  →  no real solutions
        let mut tokenizer = Tokenizer::new("x^{2} + 1 = 0");
        let tokens = tokenizer.tokenize();
        let expr = build_expression_tree(tokens).unwrap();
        let result = solve_for_variable_exact(&expr, "x");
        assert!(result.is_err());
    }

    #[test]
    fn test_solve_quadratic_irrational_roots() {
        // x² - 2 = 0  →  x = ±√2
        let mut tokenizer = Tokenizer::new("x^{2} - 2 = 0");
        let tokens = tokenizer.tokenize();
        let expr = build_expression_tree(tokens).unwrap();
        let solutions = solve_for_variable_exact(&expr, "x").unwrap();
        assert_eq!(solutions.len(), 2);
        let mut vals: Vec<f64> = solutions.iter().map(|s| s.to_f64()).collect();
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!(approx_eq(vals[0], -std::f64::consts::SQRT_2, 1e-10));
        assert!(approx_eq(vals[1], std::f64::consts::SQRT_2, 1e-10));
    }

    #[test]
    fn test_solve_linear_exact() {
        // 3x + 6 = 0  →  x = -2
        let mut tokenizer = Tokenizer::new("3*x + 6 = 0");
        let tokens = tokenizer.tokenize();
        let expr = build_expression_tree(tokens).unwrap();
        let solutions = solve_for_variable_exact(&expr, "x").unwrap();
        assert_eq!(solutions.len(), 1);
        assert_eq!(solutions[0], arithma::ExactNum::integer(-2));
    }

    #[test]
    fn test_solve_linear_fractional() {
        // 2x = 3  →  x = 3/2
        let mut tokenizer = Tokenizer::new("2*x = 3");
        let tokens = tokenizer.tokenize();
        let expr = build_expression_tree(tokens).unwrap();
        let solutions = solve_for_variable_exact(&expr, "x").unwrap();
        assert_eq!(solutions.len(), 1);
        assert_eq!(solutions[0], arithma::ExactNum::rational(3, 2));
    }

    #[test]
    fn test_solve_cubic_all_rational() {
        // x³ - 6x² + 11x - 6 = 0  →  (x-1)(x-2)(x-3) = 0
        let mut tokenizer = Tokenizer::new("x^{3} - 6*x^{2} + 11*x - 6 = 0");
        let tokens = tokenizer.tokenize();
        let expr = build_expression_tree(tokens).unwrap();
        let solutions = solve_for_variable_exact(&expr, "x").unwrap();
        let mut vals: Vec<f64> = solutions.iter().map(|s| s.to_f64()).collect();
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(vals.len(), 3);
        assert!(approx_eq(vals[0], 1.0, 1e-10));
        assert!(approx_eq(vals[1], 2.0, 1e-10));
        assert!(approx_eq(vals[2], 3.0, 1e-10));
    }

    #[test]
    fn test_solve_cubic_one_rational_two_irrational() {
        // x³ + x² - 2 = 0  →  x = 1 is a rational root; remaining x² + 2x + 2 has
        // negative discriminant, so only one real root: x = 1
        let mut tokenizer = Tokenizer::new("x^{3} + x^{2} - 2 = 0");
        let tokens = tokenizer.tokenize();
        let expr = build_expression_tree(tokens).unwrap();
        let solutions = solve_for_variable_exact(&expr, "x").unwrap();
        assert_eq!(solutions.len(), 1);
        assert_eq!(solutions[0], arithma::ExactNum::integer(1));
    }

    #[test]
    fn test_solve_cubic_cardano() {
        // x³ - 2 = 0  →  x = ∛2 ≈ 1.2599
        let mut tokenizer = Tokenizer::new("x^{3} - 2 = 0");
        let tokens = tokenizer.tokenize();
        let expr = build_expression_tree(tokens).unwrap();
        let solutions = solve_for_variable_exact(&expr, "x").unwrap();
        assert_eq!(solutions.len(), 1);
        assert!(approx_eq(solutions[0].to_f64(), 2.0_f64.cbrt(), 1e-10));
    }

    #[test]
    fn test_solve_cubic_three_real_cardano() {
        // x³ - 3x + 1 = 0  →  three irrational real roots
        let mut tokenizer = Tokenizer::new("x^{3} - 3*x + 1 = 0");
        let tokens = tokenizer.tokenize();
        let expr = build_expression_tree(tokens).unwrap();
        let solutions = solve_for_variable_exact(&expr, "x").unwrap();
        assert_eq!(solutions.len(), 3);
        for s in &solutions {
            let x = s.to_f64();
            let val = x * x * x - 3.0 * x + 1.0;
            assert!(
                val.abs() < 1e-8,
                "x={} does not satisfy x³ - 3x + 1 = 0 (got {})",
                x,
                val
            );
        }
    }

    #[test]
    fn test_solve_quartic_all_rational() {
        // x⁴ - 5x² + 4 = 0  →  (x-1)(x+1)(x-2)(x+2) = 0
        let mut tokenizer = Tokenizer::new("x^{4} - 5*x^{2} + 4 = 0");
        let tokens = tokenizer.tokenize();
        let expr = build_expression_tree(tokens).unwrap();
        let solutions = solve_for_variable_exact(&expr, "x").unwrap();
        let mut vals: Vec<f64> = solutions.iter().map(|s| s.to_f64()).collect();
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(vals.len(), 4);
        assert!(approx_eq(vals[0], -2.0, 1e-10));
        assert!(approx_eq(vals[1], -1.0, 1e-10));
        assert!(approx_eq(vals[2], 1.0, 1e-10));
        assert!(approx_eq(vals[3], 2.0, 1e-10));
    }

    #[test]
    fn test_solve_quintic_rational_roots() {
        // x⁵ - x = 0  →  x(x⁴-1) = x(x-1)(x+1)(x²+1) = 0
        // Rational roots: 0, 1, -1
        let mut tokenizer = Tokenizer::new("x^{5} - x = 0");
        let tokens = tokenizer.tokenize();
        let expr = build_expression_tree(tokens).unwrap();
        let solutions = solve_for_variable_exact(&expr, "x").unwrap();
        let mut vals: Vec<f64> = solutions.iter().map(|s| s.to_f64()).collect();
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(vals.len(), 3);
        assert!(approx_eq(vals[0], -1.0, 1e-10));
        assert!(approx_eq(vals[1], 0.0, 1e-10));
        assert!(approx_eq(vals[2], 1.0, 1e-10));
    }

    #[test]
    fn test_solve_cubic_double_root() {
        // x³ - 3x + 2 = 0  →  (x-1)²(x+2) = 0
        let mut tokenizer = Tokenizer::new("x^{3} - 3*x + 2 = 0");
        let tokens = tokenizer.tokenize();
        let expr = build_expression_tree(tokens).unwrap();
        let solutions = solve_for_variable_exact(&expr, "x").unwrap();
        let mut vals: Vec<f64> = solutions.iter().map(|s| s.to_f64()).collect();
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(vals.len(), 2);
        assert!(approx_eq(vals[0], -2.0, 1e-10));
        assert!(approx_eq(vals[1], 1.0, 1e-10));
    }

    #[test]
    fn test_solve_quartic_irrational() {
        // x⁴ - 4x² + 2 = 0 → biquadratic, roots ±√(2±√2)
        let mut tokenizer = Tokenizer::new("x^{4} - 4*x^{2} + 2 = 0");
        let tokens = tokenizer.tokenize();
        let expr = build_expression_tree(tokens).unwrap();
        let solutions = solve_for_variable_exact(&expr, "x").unwrap();
        assert_eq!(solutions.len(), 4);
        for s in &solutions {
            let x = s.to_f64();
            let val = x.powi(4) - 4.0 * x * x + 2.0;
            assert!(
                val.abs() < 1e-8,
                "x={} does not satisfy x⁴ - 4x² + 2 = 0 (got {})",
                x,
                val
            );
        }
    }

    #[test]
    fn test_solve_quartic_ferrari_general() {
        // x⁴ - x - 1 = 0 → two real roots, two complex
        let mut tokenizer = Tokenizer::new("x^{4} - x - 1 = 0");
        let tokens = tokenizer.tokenize();
        let expr = build_expression_tree(tokens).unwrap();
        let solutions = solve_for_variable_exact(&expr, "x").unwrap();
        assert_eq!(solutions.len(), 2);
        for s in &solutions {
            let x = s.to_f64();
            let val = x.powi(4) - x - 1.0;
            assert!(
                val.abs() < 1e-8,
                "x={} does not satisfy x⁴ - x - 1 = 0 (got {})",
                x,
                val
            );
        }
    }

    #[test]
    fn test_solve_quartic_no_real_roots() {
        // x⁴ + x² + 1 = 0 → no real roots
        let mut tokenizer = Tokenizer::new("x^{4} + x^{2} + 1 = 0");
        let tokens = tokenizer.tokenize();
        let expr = build_expression_tree(tokens).unwrap();
        let result = solve_for_variable_exact(&expr, "x");
        assert!(result.is_err());
    }

    #[test]
    fn test_solve_quartic_with_cubic_term() {
        // x⁴ + 2x³ - 7x² - 8x + 12 = 0 → (x-1)(x+2)(x-2)(x+3) = 0
        // But with rational roots, this is handled before Ferrari.
        // Let's verify the full path works.
        let mut tokenizer = Tokenizer::new("x^{4} + 2*x^{3} - 7*x^{2} - 8*x + 12 = 0");
        let tokens = tokenizer.tokenize();
        let expr = build_expression_tree(tokens).unwrap();
        let solutions = solve_for_variable_exact(&expr, "x").unwrap();
        let mut vals: Vec<f64> = solutions.iter().map(|s| s.to_f64()).collect();
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(vals.len(), 4);
        assert!(approx_eq(vals[0], -3.0, 1e-10));
        assert!(approx_eq(vals[1], -2.0, 1e-10));
        assert!(approx_eq(vals[2], 1.0, 1e-10));
        assert!(approx_eq(vals[3], 2.0, 1e-10));
    }

    #[test]
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

    // --- Solver with factoring (degree ≥ 5) ---

    #[test]
    fn test_solve_quintic_factors() {
        // x⁵ - x = x(x⁴-1) = x(x-1)(x+1)(x²+1) → roots: -1, 0, 1
        let expr = parse_eq("x^5 - x = 0");
        let solutions = solve_for_variable_exact(&expr, "x").unwrap();
        let vals: Vec<f64> = solutions.iter().map(|s| s.to_f64()).collect();
        assert!(
            vals.iter().any(|&v| (v - 0.0).abs() < 1e-10),
            "Missing root 0"
        );
        assert!(
            vals.iter().any(|&v| (v - 1.0).abs() < 1e-10),
            "Missing root 1"
        );
        assert!(
            vals.iter().any(|&v| (v + 1.0).abs() < 1e-10),
            "Missing root -1"
        );
    }

    #[test]
    fn test_solve_degree6() {
        // x⁶ - 1 = (x-1)(x+1)(x²+x+1)(x²-x+1)
        // Only ±1 are real (both quadratics have discriminant -3)
        let expr = parse_eq("x^6 - 1 = 0");
        let solutions = solve_for_variable_exact(&expr, "x").unwrap();
        let vals: Vec<f64> = solutions.iter().map(|s| s.to_f64()).collect();
        assert!(
            vals.iter().any(|&v| (v - 1.0).abs() < 1e-10),
            "Missing root 1"
        );
        assert!(
            vals.iter().any(|&v| (v + 1.0).abs() < 1e-10),
            "Missing root -1"
        );
        assert_eq!(solutions.len(), 2, "Should find exactly 2 real roots");
    }

    #[test]
    fn test_solve_quintic_with_rational_and_quadratic() {
        // (x-2)(x-3)(x²+1) = x⁴ - 5x³ + 7x² - 5x + 6
        // roots: 2, 3 (x²+1 has no real roots)
        let expr = parse_eq("x^4 - 5x^3 + 7x^2 - 5x + 6 = 0");
        let solutions = solve_for_variable_exact(&expr, "x").unwrap();
        let vals: Vec<f64> = solutions.iter().map(|s| s.to_f64()).collect();
        assert!(
            vals.iter().any(|&v| (v - 2.0).abs() < 1e-10),
            "Missing root 2"
        );
        assert!(
            vals.iter().any(|&v| (v - 3.0).abs() < 1e-10),
            "Missing root 3"
        );
    }

    fn parse_eq(latex: &str) -> arithma::Node {
        let mut tokenizer = Tokenizer::new(latex);
        let tokens = tokenizer.tokenize();
        build_expression_tree(tokens).unwrap_or_else(|_| panic!("Failed to parse: {}", latex))
    }

    #[test]
    fn test_solve_irrational_roots_exact() {
        // x² - 2 = 0 → ±√2, not ±1.414...
        let expr = parse_eq("x^2 - 2 = 0");
        let solutions = solve_for_variable_nodes(&expr, "x").unwrap();
        assert_eq!(solutions.len(), 2, "Should have 2 roots");
        for s in &solutions {
            let display = format!("{}", s);
            assert!(
                !display.contains('.'),
                "Should be exact, not float: {}",
                display
            );
            assert!(
                display.contains("\\sqrt{2}") || display.contains("sqrt"),
                "Should contain sqrt(2): {}",
                display
            );
        }
    }

    #[test]
    fn test_solve_rational_roots_still_work() {
        // x² - 5x + 6 = 0 → {2, 3}
        let expr = parse_eq("x^2 - 5x + 6 = 0");
        let solutions = solve_for_variable_nodes(&expr, "x").unwrap();
        assert_eq!(solutions.len(), 2);
        let displays: Vec<String> = solutions.iter().map(|s| format!("{}", s)).collect();
        assert!(displays.contains(&"2".to_string()) || displays.contains(&"3".to_string()));
    }

    #[test]
    fn test_solve_irrational_quadratic_formula() {
        // x² + x - 1 = 0 → (-1 ± √5)/2 (golden ratio related)
        let expr = parse_eq("x^2 + x - 1 = 0");
        let solutions = solve_for_variable_nodes(&expr, "x").unwrap();
        assert_eq!(solutions.len(), 2);
        for s in &solutions {
            let display = format!("{}", s);
            assert!(!display.contains('.'), "Should be exact: {}", display);
        }
    }

    #[test]
    fn test_factor_irreducible_annotation() {
        use arithma::Polynomial;
        use num_traits::One;
        // x² - 2 is irreducible over Q — single factor, degree > 1
        let expr = parse_eq("x^2 - 2");
        let poly = Polynomial::from_node(&expr, "x").unwrap();
        let (content, factors) = arithma::factor_over_q(&poly);
        assert_eq!(factors.len(), 1, "should be a single irreducible factor");
        assert!(content.is_one() || (-content.clone()).is_one());
        assert!(factors[0].degree().unwrap() > 1);
    }

    #[test]
    fn test_factor_reducible_no_annotation() {
        use arithma::Polynomial;
        // x² - 1 factors into (x-1)(x+1) — not irreducible
        let expr = parse_eq("x^2 - 1");
        let poly = Polynomial::from_node(&expr, "x").unwrap();
        let (_, factors) = arithma::factor_over_q(&poly);
        assert!(factors.len() > 1, "should have multiple factors");
    }

    #[test]
    fn test_complex_roots_omitted_cubic() {
        // x³ - 2 = 0 has 1 real root and 2 complex roots
        let expr = parse_eq("x^3 - 2 = 0");
        let result = arithma::expression::solve_full(&expr, "x").unwrap();
        assert_eq!(result.solutions.len(), 1);
        assert_eq!(result.complex_omitted, 2);
    }

    #[test]
    fn test_complex_roots_omitted_all_complex() {
        // x² + 1 = 0 has 0 real roots and 2 complex roots
        let expr = parse_eq("x^2 + 1 = 0");
        let result = arithma::expression::solve_full(&expr, "x").unwrap();
        assert_eq!(result.solutions.len(), 0);
        assert_eq!(result.complex_omitted, 2);
    }

    #[test]
    fn test_complex_roots_omitted_none() {
        // x² - 1 = 0 has 2 real roots and 0 complex roots
        let expr = parse_eq("x^2 - 1 = 0");
        let result = arithma::expression::solve_full(&expr, "x").unwrap();
        assert_eq!(result.solutions.len(), 2);
        assert_eq!(result.complex_omitted, 0);
    }

    #[test]
    fn test_nth_root_cube() {
        let env = Environment::new();
        let result = evaluate_expression_with_env("\\sqrt[3]{8}", &env).unwrap();
        assert_eq!(result, 2.0);
    }

    #[test]
    fn test_nth_root_fourth() {
        let env = Environment::new();
        let result = evaluate_expression_with_env("\\sqrt[4]{16}", &env).unwrap();
        assert_eq!(result, 2.0);
    }

    #[test]
    fn test_nth_root_preserves_sqrt() {
        let env = Environment::new();
        let result = evaluate_expression_with_env("\\sqrt{25}", &env).unwrap();
        assert_eq!(result, 5.0);
    }

    #[test]
    fn test_nth_root_symbolic() {
        let env = Environment::new();
        let result = evaluate_expression_with_env("\\sqrt[3]{27}", &env).unwrap();
        assert_eq!(result, 3.0);
    }

    #[test]
    fn test_log_base_2() {
        let env = Environment::new();
        let result = evaluate_expression_with_env("\\log_2(8)", &env).unwrap();
        assert!(
            (result - 3.0).abs() < 1e-10,
            "log_2(8) should be 3, got {}",
            result
        );
    }

    #[test]
    fn test_log_base_10_braced() {
        let env = Environment::new();
        let result = evaluate_expression_with_env("\\log_{10}(1000)", &env).unwrap();
        assert!(
            (result - 3.0).abs() < 1e-10,
            "log_10(1000) should be 3, got {}",
            result
        );
    }

    #[test]
    fn test_log_base_preserves_plain_log() {
        let env = Environment::new();
        let result = evaluate_expression_with_env("\\log(100)", &env).unwrap();
        assert_eq!(result, 2.0);
    }
}
