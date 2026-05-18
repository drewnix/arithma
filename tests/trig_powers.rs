#[cfg(test)]
mod trig_power_tests {
    use arithma::integration::integrate;
    use arithma::{build_expression_tree, Environment, Evaluator, Tokenizer};

    fn parse_raw(latex: &str) -> arithma::Node {
        let mut tokenizer = Tokenizer::new(latex);
        let tokens = tokenizer.tokenize();
        build_expression_tree(tokens).expect(&format!("Failed to parse: {}", latex))
    }

    fn verify_antiderivative(integrand_latex: &str, var: &str, test_points: &[f64]) {
        let expr = parse_raw(integrand_latex);
        let integral =
            integrate(&expr, var).expect(&format!("Failed to integrate: {}", integrand_latex));
        let env_base = Environment::new();
        let integral_simplified =
            arithma::simplify::Simplifiable::simplify(&integral, &env_base).unwrap_or(integral);
        eprintln!("∫({}) d{} = {}", integrand_latex, var, integral_simplified);

        for &x_val in test_points {
            let h = 1e-6;
            let mut env_plus = Environment::new();
            env_plus.set(var, x_val + h);
            let mut env_minus = Environment::new();
            env_minus.set(var, x_val - h);
            let f_plus = Evaluator::evaluate(&integral_simplified, &env_plus).unwrap();
            let f_minus = Evaluator::evaluate(&integral_simplified, &env_minus).unwrap();
            let numerical_deriv = (f_plus - f_minus) / (2.0 * h);

            let mut env = Environment::new();
            env.set(var, x_val);
            let expected = Evaluator::evaluate(&parse_raw(integrand_latex), &env).unwrap();

            assert!(
                (numerical_deriv - expected).abs() < 1e-4,
                "∫({}) d{}: d/d{}[F] at {}={:.2} is {:.6}, expected {:.6}",
                integrand_latex,
                var,
                var,
                var,
                x_val,
                numerical_deriv,
                expected
            );
        }
    }

    static POINTS: &[f64] = &[0.3, 0.7, 1.2, 2.0, -0.5];

    // === Even powers (reduction formula) ===

    #[test]
    fn test_sin_squared() {
        verify_antiderivative("\\sin(x)^{2}", "x", POINTS);
    }

    #[test]
    fn test_cos_squared() {
        verify_antiderivative("\\cos(x)^{2}", "x", POINTS);
    }

    #[test]
    fn test_sin_fourth() {
        verify_antiderivative("\\sin(x)^{4}", "x", POINTS);
    }

    #[test]
    fn test_cos_fourth() {
        verify_antiderivative("\\cos(x)^{4}", "x", POINTS);
    }

    // === Odd powers (Pythagorean + u-sub) ===

    #[test]
    fn test_sin_cubed() {
        verify_antiderivative("\\sin(x)^{3}", "x", POINTS);
    }

    #[test]
    fn test_cos_cubed() {
        verify_antiderivative("\\cos(x)^{3}", "x", POINTS);
    }

    #[test]
    fn test_sin_fifth() {
        verify_antiderivative("\\sin(x)^{5}", "x", POINTS);
    }

    #[test]
    fn test_cos_fifth() {
        verify_antiderivative("\\cos(x)^{5}", "x", POINTS);
    }

    // === Mixed products (one exponent odd) ===

    #[test]
    fn test_sin2_cos3() {
        // cos is odd → peel cos, convert rest to sin
        verify_antiderivative("\\sin(x)^{2} \\cdot \\cos(x)^{3}", "x", POINTS);
    }

    #[test]
    fn test_sin3_cos2() {
        // sin is odd → peel sin, convert rest to cos
        verify_antiderivative("\\sin(x)^{3} \\cdot \\cos(x)^{2}", "x", POINTS);
    }

    #[test]
    fn test_sin_cos4() {
        // sin^1 is odd
        verify_antiderivative("\\sin(x) \\cdot \\cos(x)^{4}", "x", POINTS);
    }

    #[test]
    fn test_sin4_cos() {
        // cos^1 is odd
        verify_antiderivative("\\sin(x)^{4} \\cdot \\cos(x)", "x", POINTS);
    }

    // === Output form checks ===

    #[test]
    fn test_sin2_produces_result() {
        let expr = parse_raw("\\sin(x)^{2}");
        let integral = integrate(&expr, "x").unwrap();
        let form = format!("{}", integral);
        assert!(
            form.contains("sin") || form.contains("cos"),
            "Result should contain trig functions: {}",
            form
        );
    }

    #[test]
    fn test_cos3_form() {
        let expr = parse_raw("\\cos(x)^{3}");
        let integral = integrate(&expr, "x").unwrap();
        let form = format!("{}", integral);
        assert!(
            form.contains("sin"),
            "∫cos³(x)dx should contain sin: {}",
            form
        );
    }
}
