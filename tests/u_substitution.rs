#[cfg(test)]
mod u_sub_tests {
    use arithma::integration::{integrate, integrate_latex};
    use arithma::{build_expression_tree, Environment, Evaluator, Tokenizer};

    fn parse_raw(latex: &str) -> arithma::Node {
        let mut tokenizer = Tokenizer::new(latex);
        let tokens = tokenizer.tokenize();
        build_expression_tree(tokens).expect(&format!("Failed to parse: {}", latex))
    }

    fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() < eps
    }

    fn verify_integral(integrand_latex: &str, var: &str, test_points: &[(f64, f64)]) {
        let expr = parse_raw(integrand_latex);
        let integral =
            integrate(&expr, var).expect(&format!("Failed to integrate: {}", integrand_latex));
        let env_base = Environment::new();
        let integral_simplified =
            arithma::simplify::Simplifiable::simplify(&integral, &env_base).unwrap_or(integral);
        eprintln!("∫({}) d{} = {}", integrand_latex, var, integral_simplified);
        for &(x_val, expected_f_x) in test_points {
            let mut env = Environment::new();
            env.set(var, x_val);
            // Verify by numerical differentiation: d/dx[F(x)] ≈ f(x)
            let h = 1e-6;
            let mut env_plus = Environment::new();
            env_plus.set(var, x_val + h);
            let mut env_minus = Environment::new();
            env_minus.set(var, x_val - h);
            let f_plus = Evaluator::evaluate(&integral_simplified, &env_plus).unwrap();
            let f_minus = Evaluator::evaluate(&integral_simplified, &env_minus).unwrap();
            let numerical_deriv = (f_plus - f_minus) / (2.0 * h);
            assert!(
                approx_eq(numerical_deriv, expected_f_x, 1e-4),
                "For ∫({}) d{}: derivative of result at {}={} is {}, expected {}",
                integrand_latex,
                var,
                var,
                x_val,
                numerical_deriv,
                expected_f_x
            );
        }
    }

    // === Basic u-substitution: ∫f(g(x))·g'(x) dx ===

    #[test]
    fn test_usub_2x_cos_x2() {
        // ∫2x·cos(x²) dx = sin(x²)
        // f(u) = cos(u), g(x) = x², g'(x) = 2x
        let x = 1.5_f64;
        let f_x = 2.0 * x * (x * x).cos();
        verify_integral("2 \\cdot x \\cdot \\cos(x^{2})", "x", &[(1.5, f_x)]);
    }

    #[test]
    fn test_usub_cos_x_exp_sin() {
        // ∫cos(x)·e^{sin(x)} dx = e^{sin(x)}
        // f(u) = e^u, g(x) = sin(x), g'(x) = cos(x)
        let x = 0.5_f64;
        let f_x = x.cos() * x.sin().exp();
        verify_integral("\\cos(x) \\cdot e^{\\sin(x)}", "x", &[(0.5, f_x)]);
    }

    #[test]
    fn test_usub_sin_x_cos_x() {
        // ∫sin(x)·cos(x) dx = sin²(x)/2
        // u = sin(x), du = cos(x)dx, ∫u du = u²/2
        let x = 1.0_f64;
        let f_x = x.sin() * x.cos();
        verify_integral("\\sin(x) \\cdot \\cos(x)", "x", &[(1.0, f_x)]);
    }

    #[test]
    fn test_usub_x_sin_x2() {
        // ∫x·sin(x²) dx = -cos(x²)/2
        // u = x², du = 2x dx → (1/2)∫sin(u) du = -(1/2)cos(u)
        let x = 2.0_f64;
        let f_x = x * (x * x).sin();
        verify_integral("x \\cdot \\sin(x^{2})", "x", &[(2.0, f_x)]);
    }

    #[test]
    fn test_usub_3x2_cos_x3() {
        // ∫3x²·cos(x³) dx = sin(x³)
        let x = 1.0_f64;
        let f_x = 3.0 * x * x * (x * x * x).cos();
        verify_integral("3 \\cdot x^{2} \\cdot \\cos(x^{3})", "x", &[(1.0, f_x)]);
    }

    #[test]
    fn test_usub_exp_2x() {
        // ∫2·e^{2x} shouldn't infinite loop — this is really 2·(e^{2x})
        // Actually e^{2x} with linear sub should already work, but let's verify
        let x = 1.0_f64;
        let f_x = 2.0 * (2.0 * x).exp();
        verify_integral("2 \\cdot e^{2 \\cdot x}", "x", &[(1.0, f_x)]);
    }

    // === Via integrate_latex ===

    #[test]
    fn test_usub_latex_interface() {
        let result = integrate_latex("2 \\cdot x \\cdot \\cos(x^{2})", "x");
        assert!(result.is_ok(), "u-sub via LaTeX interface should succeed");
        let result_str = result.unwrap();
        eprintln!("LaTeX result: {}", result_str);
        assert!(
            result_str.contains("+ C"),
            "Should include constant of integration"
        );
    }
}
