#[cfg(test)]
mod trig_sub_tests {
    use arithma::integration::integrate;
    use arithma::{build_expression_tree, Environment, Evaluator, Tokenizer};

    fn parse_raw(latex: &str) -> arithma::Node {
        let mut tokenizer = Tokenizer::new(latex);
        let tokens = tokenizer.tokenize();
        build_expression_tree(tokens).unwrap_or_else(|_| panic!("Failed to parse: {}", latex))
    }

    fn verify_antiderivative(integrand_latex: &str, var: &str, test_points: &[f64]) {
        let expr = parse_raw(integrand_latex);
        let integral =
            integrate(&expr, var).unwrap_or_else(|_| panic!("Failed to integrate: {}", integrand_latex));
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
                (numerical_deriv - expected).abs() < 1e-3,
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

    // --- Form 1: √(a²-x²) — test points must be in (-a, a) ---

    #[test]
    fn test_sqrt_1_minus_x2() {
        // ∫√(1-x²) dx — semicircle area
        verify_antiderivative("\\sqrt{1 - x^2}", "x", &[0.0, 0.3, 0.5, -0.3, -0.7]);
    }

    #[test]
    fn test_sqrt_4_minus_x2() {
        // ∫√(4-x²) dx with a=2
        verify_antiderivative("\\sqrt{4 - x^2}", "x", &[0.0, 0.5, 1.0, -0.5, -1.0]);
    }

    // --- Form 2: √(x²+a²) — all x valid ---

    #[test]
    fn test_sqrt_x2_plus_1() {
        // ∫√(x²+1) dx
        verify_antiderivative("\\sqrt{x^2 + 1}", "x", &[0.0, 0.5, 1.0, 2.0, -1.0]);
    }

    #[test]
    fn test_sqrt_x2_plus_4() {
        // ∫√(x²+4) dx with a=2
        verify_antiderivative("\\sqrt{x^2 + 4}", "x", &[0.0, 1.0, 2.0, -1.0]);
    }

    // --- Form 3: √(x²-a²) — test points must satisfy |x| > a ---

    #[test]
    fn test_sqrt_x2_minus_1() {
        // ∫√(x²-1) dx, valid for |x| > 1
        verify_antiderivative("\\sqrt{x^2 - 1}", "x", &[1.5, 2.0, 3.0]);
    }

    #[test]
    fn test_sqrt_x2_minus_4() {
        // ∫√(x²-4) dx with a=2, valid for |x| > 2
        verify_antiderivative("\\sqrt{x^2 - 4}", "x", &[2.5, 3.0, 4.0]);
    }
}
