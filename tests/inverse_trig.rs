#[cfg(test)]
mod inverse_trig_tests {
    use arithma::integration::{integrate, integrate_latex};
    use arithma::{build_expression_tree, Environment, Evaluator, Tokenizer};

    fn parse_raw(latex: &str) -> arithma::Node {
        let mut tokenizer = Tokenizer::new(latex);
        let tokens = tokenizer.tokenize();
        build_expression_tree(tokens).unwrap_or_else(|_| panic!("Failed to parse: {}", latex))
    }

    fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() < eps
    }

    fn verify_by_numerical_derivative(integrand_latex: &str, var: &str, x_val: f64) {
        let expr = parse_raw(integrand_latex);
        let integral =
            integrate(&expr, var).unwrap_or_else(|_| panic!("Failed to integrate: {}", integrand_latex));
        let env_base = Environment::new();
        let integral_simplified =
            arithma::simplify::Simplifiable::simplify(&integral, &env_base).unwrap_or(integral);
        eprintln!("∫({}) d{} = {}", integrand_latex, var, integral_simplified);

        // Verify: d/dx[F(x)] ≈ f(x) by numerical differentiation
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
            approx_eq(numerical_deriv, expected, 1e-4),
            "For ∫({}) d{}: derivative of result at {}={} is {}, expected {}",
            integrand_latex,
            var,
            var,
            x_val,
            numerical_deriv,
            expected
        );
    }

    // === ∫1/(1+x²) dx = arctan(x) ===

    #[test]
    fn test_arctan_basic() {
        verify_by_numerical_derivative("\\frac{1}{1 + x^{2}}", "x", 0.5);
    }

    #[test]
    fn test_arctan_at_zero() {
        verify_by_numerical_derivative("\\frac{1}{1 + x^{2}}", "x", 0.0);
    }

    #[test]
    fn test_arctan_at_large() {
        verify_by_numerical_derivative("\\frac{1}{1 + x^{2}}", "x", 3.0);
    }

    // === ∫1/(a²+x²) dx = (1/a)arctan(x/a) ===

    #[test]
    fn test_arctan_scaled() {
        // ∫1/(4+x²) dx = (1/2)arctan(x/2)
        verify_by_numerical_derivative("\\frac{1}{4 + x^{2}}", "x", 1.0);
    }

    #[test]
    fn test_arctan_scaled_9() {
        // ∫1/(9+x²) dx = (1/3)arctan(x/3)
        verify_by_numerical_derivative("\\frac{1}{9 + x^{2}}", "x", 2.0);
    }

    // === ∫1/√(1-x²) dx = arcsin(x) ===

    #[test]
    fn test_arcsin_basic() {
        verify_by_numerical_derivative("\\frac{1}{\\sqrt{1 - x^{2}}}", "x", 0.3);
    }

    #[test]
    fn test_arcsin_at_zero() {
        verify_by_numerical_derivative("\\frac{1}{\\sqrt{1 - x^{2}}}", "x", 0.0);
    }

    // === ∫1/√(a²-x²) dx = arcsin(x/a) ===

    #[test]
    fn test_arcsin_scaled() {
        // ∫1/√(4-x²) dx = arcsin(x/2)
        verify_by_numerical_derivative("\\frac{1}{\\sqrt{4 - x^{2}}}", "x", 0.5);
    }

    // === Via integrate_latex interface ===

    #[test]
    fn test_arctan_latex() {
        let result = integrate_latex("\\frac{1}{1 + x^{2}}", "x").unwrap();
        eprintln!("LaTeX result: {}", result);
        assert!(result.contains("arctan"));
    }

    #[test]
    fn test_arcsin_latex() {
        let result = integrate_latex("\\frac{1}{\\sqrt{1 - x^{2}}}", "x").unwrap();
        eprintln!("LaTeX result: {}", result);
        assert!(result.contains("arcsin"));
    }

    // === Constant multiplier variants ===

    #[test]
    fn test_const_times_arctan() {
        // ∫3/(1+x²) dx = 3·arctan(x)
        verify_by_numerical_derivative("\\frac{3}{1 + x^{2}}", "x", 1.0);
    }
}
