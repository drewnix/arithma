#[cfg(test)]
mod integration_tests {
    use arithma::{
        build_expression_tree, definite_integral_latex, integrate_latex, Environment, Evaluator,
        Tokenizer,
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

    fn evaluate_integral(expr: &str, var: &str, env: &Environment) -> Result<f64, String> {
        let integral_latex = integrate_latex(expr, var)?;
        let integral_expr = integral_latex.replace(" + C", "");
        evaluate_expression(&integral_expr, env)
    }

    fn approx_eq(a: f64, b: f64, epsilon: f64) -> bool {
        (a - b).abs() < epsilon
    }

    #[test]
    fn test_constant_integration() {
        // ∫5 dx = 5x
        let mut env = Environment::new();
        env.set("x", 3.0);
        let result = evaluate_integral("5", "x", &env).unwrap();
        assert_eq!(
            result, 15.0,
            "Integration of 5 with respect to x at x=3 should be 15"
        );
    }

    #[test]
    fn test_variable_integration() {
        // ∫x dx = x²/2
        let mut env = Environment::new();
        env.set("x", 4.0);
        let result = evaluate_integral("x", "x", &env).unwrap();
        assert_eq!(
            result, 8.0,
            "Integration of x with respect to x at x=4 should be 8"
        );
    }

    #[test]
    fn test_power_rule() {
        // ∫x^n dx = x^(n+1)/(n+1)

        // Test x^2
        let mut env = Environment::new();
        env.set("x", 2.0);
        let result = evaluate_integral("x^2", "x", &env).unwrap();
        assert!(
            approx_eq(result, 2.67, 0.01),
            "Integration of x^2 with respect to x at x=2 should be 2.67 ≈ 8/3"
        );

        // Test x^3
        env.set("x", 2.0);
        let result = evaluate_integral("x^3", "x", &env).unwrap();
        assert_eq!(
            result, 4.0,
            "Integration of x^3 with respect to x at x=2 should be 4"
        );

        // Test x^(-2)
        env.set("x", 2.0);
        let result = evaluate_integral("x^(-2)", "x", &env).unwrap();
        assert_eq!(
            result, -0.5,
            "Integration of x^(-2) with respect to x at x=2 should be -0.5"
        );
    }

    #[test]
    fn test_logarithmic_integration() {
        // ∫(1/x) dx = ln|x|
        let mut env = Environment::new();
        env.set("x", 2.0);
        let result = evaluate_integral("1/x", "x", &env).unwrap();
        assert!(
            approx_eq(result, 2.0_f64.ln(), 1e-10),
            "Integration of 1/x with respect to x at x=2 should be ln(2)"
        );
    }

    #[test]
    fn test_sum_integration() {
        // ∫(x^2 + 2x + 1) dx = x³/3 + x² + x
        let mut env = Environment::new();
        env.set("x", 2.0);
        let result = evaluate_integral("x^2 + 2*x + 1", "x", &env).unwrap();

        // At x=2: 2³/3 + 2² + 2 = 8/3 + 4 + 2 = 2.67 + 6 = 8.67
        assert!(
            approx_eq(result, 8.67, 0.01),
            "Integration of x^2 + 2x + 1 with respect to x at x=2 should be approximately 8.67"
        );
    }

    #[test]
    fn test_definite_integrals() {
        // ∫₁² x² dx = [x³/3]₁² = 8/3 - 1/3 = 7/3 ≈ 2.33
        let result = definite_integral_latex("x^2", "x", 1.0, 2.0).unwrap();
        let value = result.parse::<f64>().unwrap_or(0.0);
        assert!(
            approx_eq(value, 7.0 / 3.0, 0.01),
            "Definite integral of x^2 from 1 to 2 should be approximately 2.33"
        );

        // ∫₀¹ (2x + 1) dx = [x² + x]₀¹ = (1 + 1) - (0 + 0) = 2
        let result = definite_integral_latex("2*x + 1", "x", 0.0, 1.0).unwrap();
        let value = result.parse::<f64>().unwrap_or(0.0);
        assert!(
            approx_eq(value, 2.0, 0.01),
            "Definite integral of 2x + 1 from 0 to 1 should be 2"
        );
    }

    #[test]
    fn test_polynomial_integration() {
        // ∫(3x⁴ - 2x² + 4) dx = (3x⁵/5) - (2x³/3) + 4x
        let mut env = Environment::new();
        env.set("x", 2.0);
        let result = evaluate_integral("3*x^4 - 2*x^2 + 4", "x", &env).unwrap();

        // At x=2: (3*2⁵/5) - (2*2³/3) + 4*2 = (3*32/5) - (2*8/3) + 8 = 19.2 - 5.33 + 8 = 21.87
        assert!(
            approx_eq(result, 21.87, 0.01),
            "Integration of 3x⁴ - 2x² + 4 with respect to x at x=2 should be approximately 21.87"
        );
    }

    #[test]
    fn test_composite_terms() {
        // Test integration with coefficient and power: ∫(2x³) dx = 2∫x³ dx = 2(x⁴/4) = x⁴/2
        let mut env = Environment::new();
        env.set("x", 2.0);
        let result = evaluate_integral("2*x^3", "x", &env).unwrap();

        // At x=2: 2⁴/2 = 16/2 = 8
        assert_eq!(
            result, 8.0,
            "Integration of 2x³ with respect to x at x=2 should be 8"
        );
    }

    #[test]
    fn test_complex_integrals() {
        // ∫(x³ + x² - 2x + 1) dx = x⁴/4 + x³/3 - x² + x
        let result = integrate_latex("x^3 + x^2 - 2*x + 1", "x").unwrap();

        assert!(
            result.contains("x^{4}")
                && result.contains("x^{3}")
                && result.contains("x^{2}")
                && result.contains("+ C"),
            "Integration result should have the correct form, got: {}",
            result
        );

        // Verify with a definite integral
        let def_result = definite_integral_latex("x^3 + x^2 - 2*x + 1", "x", 0.0, 1.0).unwrap();
        let value = def_result.parse::<f64>().unwrap_or(0.0);

        // [x⁴/4 + x³/3 - x² + x]₀¹ = (1/4 + 1/3 - 1 + 1) - 0 = 0.583
        assert!(
            approx_eq(value, 0.583, 0.01),
            "Definite integral of x³ + x² - 2x + 1 from 0 to 1 should be approximately 0.583"
        );
    }

    #[test]
    fn test_trig_integration() {
        let result = integrate_latex("\\sin(x)", "x").unwrap();
        assert!(
            result.contains("cos"),
            "∫sin(x) should contain cos: {}",
            result
        );
    }

    #[test]
    fn test_exp_integration() {
        let result = integrate_latex("e^x", "x").unwrap();
        assert!(result.contains("e"), "∫e^x should contain e: {}", result);
    }

    #[test]
    fn test_tabular_x_sin() {
        // ∫x·sin(x)dx = -x·cos(x) + sin(x)
        // Verify via definite integral: ∫₀^π x·sin(x)dx = π
        let result = definite_integral_latex("x \\sin(x)", "x", 0.0, std::f64::consts::PI).unwrap();
        let value = result.parse::<f64>().unwrap_or(0.0);
        assert!(
            approx_eq(value, std::f64::consts::PI, 0.01),
            "∫₀^π x·sin(x)dx should be π ≈ 3.14, got {}",
            value
        );
    }

    #[test]
    fn test_tabular_x2_exp() {
        // ∫x²·eˣdx = eˣ(x² - 2x + 2)
        // Verify: at x=0, antiderivative = e⁰(0 - 0 + 2) = 2
        // at x=1, antiderivative = e¹(1 - 2 + 2) = e
        let result = definite_integral_latex("x^2 e^x", "x", 0.0, 1.0).unwrap();
        let value = result.parse::<f64>().unwrap_or(0.0);
        let expected = std::f64::consts::E * 1.0 - 2.0; // e(1-2+2) - 1(0-0+2) = e - 2
        assert!(
            approx_eq(value, expected, 0.01),
            "∫₀¹ x²·eˣdx should be e-2 ≈ 0.718, got {}",
            value
        );
    }

    #[test]
    fn test_log_integration() {
        // ∫x·ln(x)dx = x²/2·ln(x) - x²/4
        use arithma::integration::integrate;
        use arithma::simplify::Simplifiable;
        use arithma::Environment;
        let x = arithma::Node::Variable("x".to_string());
        let ln_x = arithma::Node::Function("ln".to_string(), vec![x.clone()]);
        let expr = arithma::Node::Multiply(Box::new(x), Box::new(ln_x));
        let env = Environment::new();
        eprintln!("expr: {:?}", expr);
        let simplified = expr.simplify(&env).unwrap_or(expr);
        eprintln!("simplified: {:?}", simplified);
        let result = integrate(&simplified, "x");
        eprintln!("result: {:?}", result);
        assert!(result.is_ok(), "∫x·ln(x)dx should succeed: {:?}", result);
    }

    #[test]
    fn test_integrate_latex_non_elementary_message() {
        // The non-elementary error should contain a helpful explanation
        let result = integrate_latex("\\exp(-x^2)", "x");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.starts_with("NON_ELEMENTARY:"));
        assert!(
            err.contains("no elementary antiderivative")
                || err.contains("No elementary antiderivative"),
            "Expected non-elementary explanation, got: {}",
            err
        );
    }

    #[test]
    fn test_integrate_latex_exp_x_cubed_non_elementary() {
        let result = integrate_latex("\\exp(x^3)", "x");
        assert!(result.is_err());
        assert!(result.unwrap_err().starts_with("NON_ELEMENTARY:"));
    }

    #[test]
    fn test_integrate_exp_x_still_elementary() {
        // Basic ∫e^x dx = e^x should still work normally
        let result = integrate_latex("\\exp(x)", "x").unwrap();
        assert!(result.contains("+ C"));
    }

    // ===== Rothstein-Trager logarithmic integration =====

    #[test]
    fn test_integrate_1_over_x_ln_x() {
        // ∫1/(x·ln(x))dx = ln(ln(x)) + C
        let result = integrate_latex("\\frac{1}{x \\cdot \\ln(x)}", "x");
        assert!(
            result.is_ok(),
            "∫1/(x·ln(x))dx should succeed: {:?}",
            result
        );
        let s = result.unwrap();
        assert!(s.contains("\\ln"), "Result should contain ln: {}", s);
        assert!(s.contains("+ C"), "Result should contain + C: {}", s);
    }

    #[test]
    fn test_integrate_1_over_ln_x_non_elementary() {
        // ∫1/ln(x)dx — non-elementary (logarithmic integral)
        let result = integrate_latex("\\frac{1}{\\ln(x)}", "x");
        assert!(result.is_err(), "∫1/ln(x)dx should be non-elementary");
        let err = result.unwrap_err();
        assert!(
            err.starts_with("NON_ELEMENTARY:"),
            "Expected NON_ELEMENTARY, got: {}",
            err
        );
    }

    #[test]
    fn test_integrate_1_over_x_ln_x_minus_1() {
        // ∫1/(x·(ln(x)-1))dx = ln(ln(x)-1) + C
        let result = integrate_latex("\\frac{1}{x \\cdot (\\ln(x) - 1)}", "x");
        assert!(
            result.is_ok(),
            "∫1/(x·(ln(x)-1))dx should succeed: {:?}",
            result
        );
        let s = result.unwrap();
        assert!(s.contains("\\ln"), "Result should contain ln: {}", s);
    }

    #[test]
    fn test_integrate_1_over_1_plus_ln_x_non_elementary() {
        // ∫1/(1+ln(x))dx — non-elementary (gives Ei)
        let result = integrate_latex("\\frac{1}{1 + \\ln(x)}", "x");
        assert!(result.is_err(), "∫1/(1+ln(x))dx should be non-elementary");
        assert!(result.unwrap_err().starts_with("NON_ELEMENTARY:"));
    }

    #[test]
    fn test_integrate_1_over_x_ln_x_numerical() {
        // Verify: d/dx[ln(ln(x))] = 1/(x·ln(x))
        // At x = e² ≈ 7.389: ln(x) = 2, ln(ln(x)) = ln(2) ≈ 0.693
        let result = integrate_latex("\\frac{1}{x \\cdot \\ln(x)}", "x").unwrap();
        let integral_expr = result.replace(" + C", "");

        let mut env = Environment::new();
        let x_val = std::f64::consts::E * std::f64::consts::E; // e²
        env.set("x", x_val);

        let integral_val = evaluate_expression(&integral_expr, &env).unwrap();
        let expected = (x_val.ln()).ln(); // ln(ln(e²)) = ln(2)
        assert!(
            approx_eq(integral_val, expected, 0.01),
            "ln(ln(e²)) should be ln(2) ≈ {:.4}, got {:.4}",
            expected,
            integral_val
        );
    }

    #[test]
    fn test_integrate_1_over_x_ln_x_minus_1_numerical() {
        // Verify at x = e³: ln(x) = 3, ln(x)-1 = 2, ln(ln(x)-1) = ln(2) ≈ 0.693
        let result = integrate_latex("\\frac{1}{x \\cdot (\\ln(x) - 1)}", "x").unwrap();
        let integral_expr = result.replace(" + C", "");

        let mut env = Environment::new();
        let x_val = std::f64::consts::E.powi(3); // e³
        env.set("x", x_val);

        let integral_val = evaluate_expression(&integral_expr, &env).unwrap();
        let expected = (x_val.ln() - 1.0).ln(); // ln(3-1) = ln(2)
        assert!(
            approx_eq(integral_val, expected, 0.01),
            "Expected {:.4}, got {:.4}",
            expected,
            integral_val
        );
    }

    // ===== Tower builder: exp-rational integration =====

    #[test]
    fn test_integrate_exp_over_1_plus_exp() {
        // ∫exp(x)/(1+exp(x))dx = ln(1+exp(x)) + C
        let result = integrate_latex("\\frac{\\exp(x)}{1 + \\exp(x)}", "x");
        assert!(
            result.is_ok(),
            "∫exp(x)/(1+exp(x))dx should succeed: {:?}",
            result
        );
        let s = result.unwrap();
        assert!(s.contains("\\ln"), "Result should contain ln: {}", s);
        assert!(s.contains("+ C"), "Result should contain + C: {}", s);
    }

    #[test]
    fn test_integrate_1_over_1_plus_exp() {
        // ∫1/(1+exp(x))dx = x - ln(1+exp(x)) + C
        let result = integrate_latex("\\frac{1}{1 + \\exp(x)}", "x");
        assert!(
            result.is_ok(),
            "∫1/(1+exp(x))dx should succeed: {:?}",
            result
        );
        let s = result.unwrap();
        assert!(s.contains("\\ln"), "Result should contain ln: {}", s);
    }

    #[test]
    fn test_integrate_exp_over_1_plus_exp_numerical() {
        // Verify: d/dx[ln(1+exp(x))] = exp(x)/(1+exp(x))
        let result = integrate_latex("\\frac{\\exp(x)}{1 + \\exp(x)}", "x").unwrap();
        let integral_expr = result.replace(" + C", "");
        let mut env = Environment::new();
        env.set("x", 1.0);
        let val = evaluate_expression(&integral_expr, &env).unwrap();
        let expected = (1.0 + std::f64::consts::E).ln();
        assert!(
            approx_eq(val, expected, 0.01),
            "Expected {:.4}, got {:.4}",
            expected,
            val
        );
    }

    #[test]
    fn test_definite_sin() {
        let result = definite_integral_latex("\\sin(x)", "x", 0.0, std::f64::consts::PI).unwrap();
        let value = result.parse::<f64>().unwrap_or(0.0);
        assert!(
            approx_eq(value, 2.0, 0.01),
            "∫₀^π sin(x)dx should be 2, got {}",
            value
        );
    }

    // ===== Two-level tower: exp + ln integration =====

    #[test]
    fn test_integrate_exp_x_times_ln_x_non_elementary() {
        // ∫exp(x)·ln(x) dx → non-elementary (reduces to Ei)
        let result = integrate_latex("\\exp(x) \\cdot \\ln(x)", "x");
        assert!(result.is_err(), "∫exp(x)·ln(x)dx should be non-elementary");
        assert!(result.unwrap_err().starts_with("NON_ELEMENTARY:"));
    }

    #[test]
    fn test_integrate_exp_x_ln_x_plus_exp_x_over_x() {
        // ∫(exp(x)·ln(x) + exp(x)/x) dx = exp(x)·ln(x) + C
        // Build as (ln(x) + 1/x) * exp(x) to avoid Add-splitting
        use arithma::integration::integrate;
        let x = arithma::Node::Variable("x".to_string());
        let ln_x = arithma::Node::Function("ln".to_string(), vec![x.clone()]);
        let one = arithma::Node::Num(arithma::ExactNum::integer(1));
        let one_over_x = arithma::Node::Divide(Box::new(one), Box::new(x.clone()));
        let sum = arithma::Node::Add(Box::new(ln_x), Box::new(one_over_x));
        let exp_x = arithma::Node::Function("exp".to_string(), vec![x]);
        let expr = arithma::Node::Multiply(Box::new(sum), Box::new(exp_x));
        let result = integrate(&expr, "x");
        assert!(
            result.is_ok(),
            "∫(ln(x) + 1/x)·exp(x) dx should succeed: {:?}",
            result,
        );
    }

    #[test]
    fn test_integrate_exp_x_ln_x_sq_correction_numerical() {
        // ∫(exp(x)·ln(x)² + 2·exp(x)·ln(x)/x) dx = exp(x)·ln(x)² + C
        // d/dx[exp(x)·ln(x)²] = exp(x)·ln(x)² + 2·exp(x)·ln(x)/x
        // Use product form: (ln(x)² + 2·ln(x)/x) · exp(x)
        use arithma::integration::integrate;
        use arithma::ExactNum;
        use arithma::Node;

        let ln_x = Node::Function("ln".to_string(), vec![Node::Variable("x".to_string())]);
        let ln_x_sq = Node::Power(
            Box::new(ln_x.clone()),
            Box::new(Node::Num(ExactNum::integer(2))),
        );
        let two = Node::Num(ExactNum::integer(2));
        let x = Node::Variable("x".to_string());
        let two_ln_over_x = Node::Divide(
            Box::new(Node::Multiply(Box::new(two), Box::new(ln_x))),
            Box::new(x),
        );
        let inner_sum = Node::Add(Box::new(ln_x_sq), Box::new(two_ln_over_x));
        let exp_x = Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())]);
        let expr = Node::Multiply(Box::new(inner_sum), Box::new(exp_x));

        let result = integrate(&expr, "x");
        assert!(result.is_ok(), "Should succeed: {:?}", result);

        // Numerical verification at x=2: exp(2)·ln(2)²
        let result_node = result.unwrap();
        let mut env = Environment::new();
        env.set("x", 2.0);
        let val = Evaluator::evaluate(&result_node, &env).unwrap();
        let expected = 2.0_f64.exp() * 2.0_f64.ln().powi(2);
        assert!(
            approx_eq(val, expected, 0.01),
            "Expected {:.4}, got {:.4}",
            expected,
            val,
        );
    }

    #[test]
    fn test_integrate_exp_x_sq_times_ln_x_non_elementary() {
        // ∫exp(x²)·ln(x) dx → non-elementary
        let result = integrate_latex("\\exp(x^2) \\cdot \\ln(x)", "x");
        assert!(result.is_err(), "∫exp(x²)·ln(x)dx should be non-elementary",);
        assert!(result.unwrap_err().starts_with("NON_ELEMENTARY:"));
    }

    #[test]
    fn test_integrate_exp_x_ln_x_plus_exp_x_over_x_numerical() {
        // Verify: d/dx[exp(x)·ln(x)] = exp(x)·ln(x) + exp(x)/x
        // So ∫(ln(x) + 1/x)·exp(x) dx = exp(x)·ln(x)
        use arithma::integration::integrate;
        use arithma::Evaluator;
        let x = arithma::Node::Variable("x".to_string());
        let ln_x = arithma::Node::Function("ln".to_string(), vec![x.clone()]);
        let one = arithma::Node::Num(arithma::ExactNum::integer(1));
        let one_over_x = arithma::Node::Divide(Box::new(one), Box::new(x.clone()));
        let sum = arithma::Node::Add(Box::new(ln_x), Box::new(one_over_x));
        let exp_x = arithma::Node::Function("exp".to_string(), vec![x]);
        let expr = arithma::Node::Multiply(Box::new(sum), Box::new(exp_x));
        let result_node = integrate(&expr, "x").unwrap();
        let mut env = Environment::new();
        env.set("x", 2.0);
        let val = Evaluator::evaluate(&result_node, &env).unwrap();
        let expected = 2.0_f64.exp() * 2.0_f64.ln();
        assert!(
            approx_eq(val, expected, 0.01),
            "Expected {:.4}, got {:.4}",
            expected,
            val,
        );
    }

    // ===== Two-level tower: rational exp + ln integration =====

    #[test]
    fn test_integrate_ln_x_over_1_plus_exp_non_elementary() {
        // ∫ln(x)/(1+exp(x)) dx → non-elementary
        let result = integrate_latex("\\frac{\\ln(x)}{1 + \\exp(x)}", "x");
        assert!(
            result.is_err(),
            "∫ln(x)/(1+exp(x))dx should be non-elementary: {:?}",
            result,
        );
        assert!(
            result.unwrap_err().starts_with("NON_ELEMENTARY:"),
            "Should be NON_ELEMENTARY"
        );
    }

    #[test]
    fn test_integrate_exp_ln_over_1_plus_exp_non_elementary() {
        // ∫exp(x)·ln(x)/(1+exp(x)) dx → non-elementary
        let result = integrate_latex("\\frac{\\exp(x) \\cdot \\ln(x)}{1 + \\exp(x)}", "x");
        assert!(
            result.is_err(),
            "∫exp(x)·ln(x)/(1+exp(x))dx should be non-elementary: {:?}",
            result,
        );
        assert!(
            result.unwrap_err().starts_with("NON_ELEMENTARY:"),
            "Should be NON_ELEMENTARY"
        );
    }

    #[test]
    fn test_integrate_ln_x_over_1_plus_exp_2x_non_elementary() {
        // ∫ln(x)/(1+exp(2x)) dx → non-elementary (degree-2 denominator)
        let result = integrate_latex("\\frac{\\ln(x)}{1 + \\exp(2x)}", "x");
        assert!(
            result.is_err(),
            "∫ln(x)/(1+exp(2x))dx should be non-elementary: {:?}",
            result,
        );
        assert!(
            result.unwrap_err().starts_with("NON_ELEMENTARY:"),
            "Should be NON_ELEMENTARY"
        );
    }

    // ===== Log-over-exp tower integration =====

    #[test]
    fn test_integrate_ln_1_plus_exp_x_non_elementary() {
        // ∫ln(1+exp(x)) dx → non-elementary (involves Li₂)
        let result = integrate_latex("\\ln(1 + \\exp(x))", "x");
        assert!(
            result.is_err(),
            "∫ln(1+exp(x))dx should be non-elementary: {:?}",
            result,
        );
        assert!(
            result.unwrap_err().starts_with("NON_ELEMENTARY:"),
            "Should be NON_ELEMENTARY"
        );
    }
}
