#[cfg(test)]
mod substitute_tests {
    use arithma::{
        build_expression_tree, substitute, substitute_latex, Environment, Evaluator, Tokenizer,
    };

    fn parse_expression(latex: &str) -> Result<arithma::Node, String> {
        let mut tokenizer = Tokenizer::new(latex);
        let tokens = tokenizer.tokenize();
        build_expression_tree(tokens)
    }

    fn evaluate_expression(latex: &str, env: &Environment) -> Result<f64, String> {
        let expr = parse_expression(latex)?;
        Evaluator::evaluate(&expr, env)
    }

    fn approx_eq(a: f64, b: f64, epsilon: f64) -> bool {
        (a - b).abs() < epsilon
    }

    #[test]
    fn test_polynomial_substitution() {
        // Test x^2 + 3x + 2 with x = a + 1
        let expr = parse_expression("x^2 + 3*x + 2").unwrap();
        let replacement = parse_expression("a + 1").unwrap();

        let result = substitute(&expr, &[("x".to_string(), replacement)]).unwrap();

        // After substitution, result should be (a+1)^2 + 3(a+1) + 2
        // Expanding: a^2 + 2a + 1 + 3a + 3 + 2 = a^2 + 5a + 6

        // Check using evaluation at multiple points
        let mut env = Environment::new();

        for a_val in -5..=5 {
            env.set("a", a_val as f64);
            let eval_result = Evaluator::evaluate(&result, &env).unwrap();

            // Original expression with x = a + 1 substituted
            let x_val = a_val as f64 + 1.0;
            let expected = x_val * x_val + 3.0 * x_val + 2.0;

            assert!(
                approx_eq(eval_result, expected, 1e-10),
                "Failed at a = {}: result = {}, expected = {}",
                a_val,
                eval_result,
                expected
            );
        }
    }

    #[test]
    fn test_nested_substitutions() {
        // Start with f(x, y) = x^2 + y^2
        let expr = parse_expression("x^2 + y^2").unwrap();

        // Substitute x = a*cos(t), y = a*sin(t)
        // This gives us the equation of a circle in parametric form
        let x_replacement = parse_expression("a*\\cos{t}").unwrap();
        let y_replacement = parse_expression("a*\\sin{t}").unwrap();

        let result = substitute(
            &expr,
            &[
                ("x".to_string(), x_replacement),
                ("y".to_string(), y_replacement),
            ],
        )
        .unwrap();

        // The result should be (a*cos(t))^2 + (a*sin(t))^2 = a^2*cos^2(t) + a^2*sin^2(t) = a^2

        // Test with various values of a and t
        let mut env = Environment::new();

        env.set("a", 2.0); // Circle of radius 2

        for t_deg in (0..=360).step_by(30) {
            let t_rad = t_deg as f64 * std::f64::consts::PI / 180.0;
            env.set("t", t_rad);

            let eval_result = Evaluator::evaluate(&result, &env).unwrap();

            // Expected result is a^2 = 4
            assert!(
                approx_eq(eval_result, 4.0, 1e-10),
                "Failed at t = {} degrees: result = {}, expected = 4.0",
                t_deg,
                eval_result
            );
        }
    }

    #[test]
    fn test_equation_substitution() {
        // Test with an equation: ax + b = c
        let expr = parse_expression("a*x + b = c").unwrap();

        // Substitute with a = 2, b = 3, c = 7
        let a_replacement = parse_expression("2").unwrap();
        let b_replacement = parse_expression("3").unwrap();
        let c_replacement = parse_expression("7").unwrap();

        let result = substitute(
            &expr,
            &[
                ("a".to_string(), a_replacement),
                ("b".to_string(), b_replacement),
                ("c".to_string(), c_replacement),
            ],
        )
        .unwrap();

        // Result should be 2x + 3 = 7

        // Solve for x
        let mut env = Environment::new();
        env.set("x", 2.0); // The solution is x = 2

        let eval_result = Evaluator::evaluate(&result, &env).unwrap();

        // If the equation is true, the evaluator will return 0 (difference between sides)
        assert!(
            approx_eq(eval_result, 0.0, 1e-10),
            "Equation not satisfied at x = 2: result = {}",
            eval_result
        );
    }

    #[test]
    fn test_latex_variable_substitution() {
        // Test the LaTeX substitution API with a simpler expression first
        let result = substitute_latex(
            "a*x + b",
            &[
                ("a".to_string(), "2".to_string()),
                ("b".to_string(), "3".to_string()),
                ("x".to_string(), "4".to_string()),
            ],
        )
        .unwrap();

        // The result should evaluate to 2*4 + 3 = 11
        let eval_result = evaluate_expression(&result, &Environment::new()).unwrap();
        assert_eq!(eval_result, 11.0);

        // Test with a simple trigonometric function where we know the exact result
        let expr = parse_expression("\\sin{x}").unwrap();
        let x_replacement = parse_expression("0").unwrap();

        let result = substitute(&expr, &[("x".to_string(), x_replacement)]).unwrap();

        // sin(0) = 0
        let eval_result = Evaluator::evaluate(&result, &Environment::new()).unwrap();
        assert_eq!(eval_result, 0.0);

        // Verify basic sine and cosine evaluations
        let mut env = Environment::new();
        env.set("x", std::f64::consts::PI / 4.0);

        // Test sine and cosine separately
        let sin_expr = parse_expression("\\sin{x}").unwrap();
        let sin_result = Evaluator::evaluate(&sin_expr, &env).unwrap();
        let sin_expected = (std::f64::consts::PI / 4.0).sin();

        let cos_expr = parse_expression("\\cos{x}").unwrap();
        let cos_result = Evaluator::evaluate(&cos_expr, &env).unwrap();
        let cos_expected = (std::f64::consts::PI / 4.0).cos();

        // For debugging, print the values
        println!("sin(π/4): expected={}, got={}", sin_expected, sin_result);
        println!("cos(π/4): expected={}, got={}", cos_expected, cos_result);

        assert!(
            approx_eq(sin_result, sin_expected, 1e-10),
            "sin(π/4): got {}, expected approximately {}",
            sin_result,
            sin_expected
        );

        assert!(
            approx_eq(cos_result, cos_expected, 1e-10),
            "cos(π/4): got {}, expected approximately {}",
            cos_result,
            cos_expected
        );
    }

    #[test]
    fn test_complex_latex_substitution() {
        // Test with a more complex expression involving fractions and multiple variables
        // Using the direct substitution API instead of LaTeX
        let expr = parse_expression("\\frac{x + y}{z - w}").unwrap();

        let x_replacement = parse_expression("a^2").unwrap();
        let y_replacement = parse_expression("2*a*b").unwrap();
        let z_replacement = parse_expression("b^2").unwrap();
        let w_replacement = parse_expression("a^2").unwrap();

        let result = substitute(
            &expr,
            &[
                ("x".to_string(), x_replacement),
                ("y".to_string(), y_replacement),
                ("z".to_string(), z_replacement),
                ("w".to_string(), w_replacement),
            ],
        )
        .unwrap();

        // Result should be (a^2 + 2ab)/(b^2 - a^2) = (a^2 + 2ab)/((b-a)(b+a))

        // Check with numerical values
        let mut env = Environment::new();
        env.set("a", 2.0);
        env.set("b", 3.0);

        let eval_result = Evaluator::evaluate(&result, &env).unwrap();

        // Expected: (2^2 + 2*2*3)/(3^2 - 2^2) = (4 + 12)/(9 - 4) = 16/5 = 3.2
        let expected = (4.0 + 12.0) / (9.0 - 4.0);
        assert!(
            approx_eq(eval_result, expected, 1e-10),
            "Unexpected result: got {}, expected approximately {}",
            eval_result,
            expected
        );
    }

    #[test]
    fn test_summation_with_substitution() {
        // Test summation with substitution: Σ_{i=1}^{n} (i+k)
        let expr = parse_expression("\\sum_{i=1}^{n} {i+k}").unwrap();

        // Substitute n = 5, k = 3
        let n_replacement = parse_expression("5").unwrap();
        let k_replacement = parse_expression("3").unwrap();

        let result = substitute(
            &expr,
            &[
                ("n".to_string(), n_replacement),
                ("k".to_string(), k_replacement),
            ],
        )
        .unwrap();

        // Result should be Σ_{i=1}^{5} (i+3)
        // = (1+3) + (2+3) + (3+3) + (4+3) + (5+3)
        // = 4 + 5 + 6 + 7 + 8
        // = 30

        let eval_result = Evaluator::evaluate(&result, &Environment::new()).unwrap();
        assert_eq!(eval_result, 30.0);
    }
}
