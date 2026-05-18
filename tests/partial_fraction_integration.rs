#[cfg(test)]
mod pf_integration_tests {
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

    #[allow(dead_code)]
    static POINTS: &[f64] = &[0.3, 0.7, 1.5, 2.5, -0.5, -1.5];

    #[test]
    fn test_pf_1_over_x2_minus_1() {
        // ∫1/(x²-1) dx = (1/2)ln|x-1| - (1/2)ln|x+1|
        // Test away from x=±1
        verify_antiderivative("\\frac{1}{x^{2} - 1}", "x", &[1.5, 2.0, 2.5, -1.5, -2.0]);
    }

    #[test]
    fn test_pf_x_over_x2_minus_1() {
        // ∫x/(x²-1) dx = (1/2)ln|x²-1|
        verify_antiderivative("\\frac{x}{x^{2} - 1}", "x", &[1.5, 2.0, 3.0, -1.5, -2.0]);
    }

    #[test]
    fn test_pf_1_over_cubic() {
        // ∫1/(x³-x) dx = ∫1/(x(x-1)(x+1)) dx
        // = -1/x + (1/2)/(x-1) + (1/2)/(x+1) type decomposition
        verify_antiderivative(
            "\\frac{1}{x^{3} - x}",
            "x",
            &[0.3, 0.5, 1.5, 2.0, -0.5, -1.5],
        );
    }

    #[test]
    fn test_pf_with_irreducible_quadratic() {
        // ∫1/(x³-1) dx where x³-1 = (x-1)(x²+x+1)
        // The x²+x+1 part produces an arctan term
        verify_antiderivative("\\frac{1}{x^{3} - 1}", "x", &[1.5, 2.0, 3.0, -0.5]);
    }

    #[test]
    fn test_pf_polynomial_part() {
        // ∫(x³+1)/(x²-1) dx — has a polynomial part from long division
        verify_antiderivative(
            "(x^3 + 1)/(x^2 - 1)",
            "x",
            &[1.5, 2.0, 3.0, -1.5, -2.0],
        );
    }
}
