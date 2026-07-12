#[cfg(test)]
mod derivative_tests {
    use arithma::{differentiate_and_evaluate, Environment};

    fn approx_eq(a: f64, b: f64, epsilon: f64) -> bool {
        (a - b).abs() < epsilon
    }

    #[test]
    fn test_power_rule_derivatives() {
        let mut env = Environment::new();

        // d/dx(42) = 0
        let result = differentiate_and_evaluate("42", "x", &env).unwrap();
        assert_eq!(result, 0.0, "Derivative of constant should be 0");

        // d/dx(x) = 1
        env.set("x", 3.0);
        let result = differentiate_and_evaluate("x", "x", &env).unwrap();
        assert_eq!(result, 1.0, "Derivative of x should be 1");

        // d/dx(5x) = 5
        env.set("x", 3.0);
        let result = differentiate_and_evaluate("5*x", "x", &env).unwrap();
        assert_eq!(result, 5.0, "Derivative of 5x should be 5");

        // d/dx(x^2) = 2x, at x=3: 6
        env.set("x", 3.0);
        let result = differentiate_and_evaluate("x^2", "x", &env).unwrap();
        assert_eq!(result, 6.0, "Derivative of x^2 at x=3 should be 6");

        // d/dx(x^3) = 3x^2, at x=2: 12
        env.set("x", 2.0);
        let result = differentiate_and_evaluate("x^3", "x", &env).unwrap();
        assert_eq!(result, 12.0, "Derivative of x^3 at x=2 should be 12");

        // d/dx(x^4) = 4x^3, at x=2: 32
        env.set("x", 2.0);
        let result = differentiate_and_evaluate("x^4", "x", &env).unwrap();
        assert_eq!(result, 32.0, "Derivative of x^4 at x=2 should be 32");

        // d/dx(x^0.5) = 0.5*x^(-0.5), at x=2: 0.5/sqrt(2) ≈ 0.3536
        env.set("x", 2.0);
        let result = differentiate_and_evaluate("x^0.5", "x", &env).unwrap();
        let expected = 0.5_f64 / 2.0_f64.sqrt();
        assert!(
            approx_eq(result, expected, 1e-10),
            "Derivative of x^0.5 at x=2 should be {}, got {}",
            expected,
            result
        );
    }

    #[test]
    fn test_polynomial_derivatives() {
        let mut env = Environment::new();

        // d/dx(3x + 2) = 3
        env.set("x", 0.0);
        let result = differentiate_and_evaluate("3*x + 2", "x", &env).unwrap();
        assert_eq!(result, 3.0, "Derivative of 3x + 2 should be 3");

        // d/dx(x^2 + 3x + 2) = 2x + 3, at x=2: 7
        env.set("x", 2.0);
        let result = differentiate_and_evaluate("x^2 + 3*x + 2", "x", &env).unwrap();
        assert_eq!(result, 7.0, "Derivative of x^2 + 3x + 2 at x=2 should be 7");

        // d/dx(2x^3 - 3x^2 + x - 5) = 6x^2 - 6x + 1, at x=2: 24 - 12 + 1 = 13
        env.set("x", 2.0);
        let result = differentiate_and_evaluate("2*x^3 - 3*x^2 + x - 5", "x", &env).unwrap();
        assert!(
            approx_eq(result, 13.0, 1e-10),
            "Derivative of 2x^3 - 3x^2 + x - 5 at x=2 should be 13, got {}",
            result
        );
    }

    #[test]
    fn test_product_rule() {
        let mut env = Environment::new();

        // d/dx(x*(x+1)) = 2x + 1, at x=2: 5
        env.set("x", 2.0);
        let result = differentiate_and_evaluate("x * (x + 1)", "x", &env).unwrap();
        assert_eq!(result, 5.0, "Derivative of x(x+1) at x=2 should be 5");

        // d/dx(x^2*(2x+3)) = 6x^2 + 6x, at x=2: 36
        env.set("x", 2.0);
        let result = differentiate_and_evaluate("x^2 * (2*x + 3)", "x", &env).unwrap();
        assert!(
            approx_eq(result, 36.0, 1e-10),
            "Derivative of x^2(2x+3) at x=2 should be 36, got {}",
            result
        );
    }

    #[test]
    fn test_quotient_rule() {
        let mut env = Environment::new();

        // d/dx(1/x) = -1/x^2, at x=2: -0.25
        env.set("x", 2.0);
        let result = differentiate_and_evaluate("1/x", "x", &env).unwrap();
        assert_eq!(result, -0.25, "Derivative of 1/x at x=2 should be -0.25");

        // d/dx((x^2+1)/(x-1)) = (2x(x-1) - (x^2+1))/(x-1)^2 = (x^2-2x-1)/(x-1)^2
        // at x=3: (9-6-1)/(2^2) = 2/4 = 0.5
        env.set("x", 3.0);
        let result = differentiate_and_evaluate("(x^2 + 1)/(x - 1)", "x", &env).unwrap();
        assert!(
            approx_eq(result, 0.5, 1e-10),
            "Derivative of (x^2+1)/(x-1) at x=3 should be 0.5, got {}",
            result
        );
    }

    #[test]
    fn test_chain_rule() {
        let mut env = Environment::new();

        // d/dx((2x+1)^2) = 4(2x+1), at x=1: 4*3 = 12
        env.set("x", 1.0);
        let result = differentiate_and_evaluate("(2*x + 1)^2", "x", &env).unwrap();
        assert!(
            approx_eq(result, 12.0, 1e-10),
            "Derivative of (2x+1)^2 at x=1 should be 12, got {}",
            result
        );

        // d/dx((x^2+1)^3) = 6x(x^2+1)^2, at x=2: 6*2*25 = 300
        env.set("x", 2.0);
        let result = differentiate_and_evaluate("(x^2 + 1)^3", "x", &env).unwrap();
        assert!(
            approx_eq(result, 300.0, 1e-10),
            "Derivative of (x^2+1)^3 at x=2 should be 300, got {}",
            result
        );
    }

    #[test]
    fn test_multi_variable_derivative() {
        let mut env = Environment::new();
        env.set("x", 3.0);
        env.set("y", 4.0);

        // df/dx of x^2 + y^2 = 2x, at x=3: 6
        let result = differentiate_and_evaluate("x^2 + y^2", "x", &env).unwrap();
        assert_eq!(result, 6.0, "Partial derivative df/dx at (3,4) should be 6");

        // df/dy of x^2 + y^2 = 2y, at y=4: 8
        let result = differentiate_and_evaluate("x^2 + y^2", "y", &env).unwrap();
        assert_eq!(result, 8.0, "Partial derivative df/dy at (3,4) should be 8");
    }

    #[test]
    fn test_sqrt_derivative() {
        let mut env = Environment::new();

        // d/dx(sqrt(x)) = 1/(2*sqrt(x)), at x=4: 1/4 = 0.25
        env.set("x", 4.0);
        let result = differentiate_and_evaluate("\\sqrt{x}", "x", &env).unwrap();
        assert!(
            approx_eq(result, 0.25, 1e-10),
            "Derivative of sqrt(x) at x=4 should be 0.25, got {}",
            result
        );

        // d/dx(x*sqrt(x)) = sqrt(x) + x/(2*sqrt(x)) = 3*sqrt(x)/2, at x=4: 3
        env.set("x", 4.0);
        let result = differentiate_and_evaluate("x * \\sqrt{x}", "x", &env).unwrap();
        assert!(
            approx_eq(result, 3.0, 1e-10),
            "Derivative of x*sqrt(x) at x=4 should be 3, got {}",
            result
        );
    }

    #[test]
    fn test_composition_with_sqrt() {
        let mut env = Environment::new();

        // d/dx(sqrt(2x+1)) = 1/sqrt(2x+1), at x=4: 1/3
        env.set("x", 4.0);
        let result = differentiate_and_evaluate("\\sqrt{2*x + 1}", "x", &env).unwrap();
        assert!(
            approx_eq(result, 1.0 / 3.0, 1e-10),
            "Derivative of sqrt(2x+1) at x=4 should be 1/3, got {}",
            result
        );
    }

    #[test]
    fn test_trig_derivatives() {
        let mut env = Environment::new();
        env.set("x", 0.0);

        // d/dx(sin(x)) = cos(x), at x=0: 1
        let result = differentiate_and_evaluate("\\sin(x)", "x", &env).unwrap();
        assert!(
            approx_eq(result, 1.0, 1e-10),
            "Derivative of sin(x) at x=0 should be 1, got {}",
            result
        );

        // d/dx(cos(x)) = -sin(x), at x=0: 0
        let result = differentiate_and_evaluate("\\cos(x)", "x", &env).unwrap();
        assert!(
            approx_eq(result, 0.0, 1e-10),
            "Derivative of cos(x) at x=0 should be 0, got {}",
            result
        );

        // d/dx(tan(x)) = sec²(x), at x=0: 1
        env.set("x", 0.0);
        let result = differentiate_and_evaluate("\\tan(x)", "x", &env).unwrap();
        assert!(
            approx_eq(result, 1.0, 1e-10),
            "Derivative of tan(x) at x=0 should be 1, got {}",
            result
        );
    }

    #[test]
    fn test_reciprocal_trig_derivatives() {
        let mut env = Environment::new();
        env.set("x", 1.0);

        // d/dx(csc(x)) = -csc(x)·cot(x), at x=1
        let result = differentiate_and_evaluate("\\csc(x)", "x", &env).unwrap();
        let expected = -(1.0 / 1.0_f64.sin()) * (1.0_f64.cos() / 1.0_f64.sin());
        assert!(
            approx_eq(result, expected, 1e-10),
            "d/dx csc(x) at x=1 should be {}, got {}",
            expected,
            result
        );

        // d/dx(sec(x)) = sec(x)·tan(x), at x=1
        let result = differentiate_and_evaluate("\\sec(x)", "x", &env).unwrap();
        let expected = (1.0 / 1.0_f64.cos()) * 1.0_f64.tan();
        assert!(
            approx_eq(result, expected, 1e-10),
            "d/dx sec(x) at x=1 should be {}, got {}",
            expected,
            result
        );

        // d/dx(cot(x)) = -csc²(x), at x=1
        let result = differentiate_and_evaluate("\\cot(x)", "x", &env).unwrap();
        let expected = -(1.0 / 1.0_f64.sin()).powi(2);
        assert!(
            approx_eq(result, expected, 1e-10),
            "d/dx cot(x) at x=1 should be {}, got {}",
            expected,
            result
        );
    }

    #[test]
    fn test_inverse_trig_derivatives() {
        let mut env = Environment::new();
        env.set("x", 0.5);

        // d/dx(arcsin(x)) = 1/√(1-x²), at x=0.5
        let result = differentiate_and_evaluate("\\arcsin(x)", "x", &env).unwrap();
        let expected = 1.0 / (1.0 - 0.25_f64).sqrt();
        assert!(
            approx_eq(result, expected, 1e-10),
            "d/dx arcsin(x) at x=0.5 should be {}, got {}",
            expected,
            result
        );

        // d/dx(arccos(x)) = -1/√(1-x²), at x=0.5
        let result = differentiate_and_evaluate("\\arccos(x)", "x", &env).unwrap();
        assert!(
            approx_eq(result, -expected, 1e-10),
            "d/dx arccos(x) at x=0.5 should be {}, got {}",
            -expected,
            result
        );

        // d/dx(arctan(x)) = 1/(1+x²), at x=0.5
        let result = differentiate_and_evaluate("\\arctan(x)", "x", &env).unwrap();
        let expected = 1.0 / (1.0 + 0.25);
        assert!(
            approx_eq(result, expected, 1e-10),
            "d/dx arctan(x) at x=0.5 should be {}, got {}",
            expected,
            result
        );
    }

    #[test]
    fn test_inverse_reciprocal_trig_derivatives() {
        let mut env = Environment::new();
        env.set("x", 2.0);

        // d/dx(arccsc(x)) = -1/(x·√(x²-1)), at x=2
        let result = differentiate_and_evaluate("\\arccsc(x)", "x", &env).unwrap();
        let expected = -1.0 / (2.0 * (4.0_f64 - 1.0).sqrt());
        assert!(
            approx_eq(result, expected, 1e-10),
            "d/dx arccsc(x) at x=2 should be {}, got {}",
            expected,
            result
        );

        // d/dx(arcsec(x)) = 1/(x·√(x²-1)), at x=2
        let result = differentiate_and_evaluate("\\arcsec(x)", "x", &env).unwrap();
        let expected = 1.0 / (2.0 * (4.0_f64 - 1.0).sqrt());
        assert!(
            approx_eq(result, expected, 1e-10),
            "d/dx arcsec(x) at x=2 should be {}, got {}",
            expected,
            result
        );

        // d/dx(arccot(x)) = -1/(1+x²), at x=2
        let result = differentiate_and_evaluate("\\arccot(x)", "x", &env).unwrap();
        let expected = -1.0 / (1.0 + 4.0);
        assert!(
            approx_eq(result, expected, 1e-10),
            "d/dx arccot(x) at x=2 should be {}, got {}",
            expected,
            result
        );
    }

    #[test]
    fn test_inverse_reciprocal_trig_derivatives_negative_branch() {
        let mut env = Environment::new();
        env.set("x", -2.0);

        // Numeric definitions are asin(1/x) and acos(1/x), so derivatives use |x|.
        let result = differentiate_and_evaluate("\\arccsc(x)", "x", &env).unwrap();
        let expected = -1.0 / (2.0 * (4.0_f64 - 1.0).sqrt());
        assert!(
            approx_eq(result, expected, 1e-10),
            "d/dx arccsc(x) at x=-2 should be {}, got {}",
            expected,
            result
        );

        let result = differentiate_and_evaluate("\\arcsec(x)", "x", &env).unwrap();
        let expected = 1.0 / (2.0 * (4.0_f64 - 1.0).sqrt());
        assert!(
            approx_eq(result, expected, 1e-10),
            "d/dx arcsec(x) at x=-2 should be {}, got {}",
            expected,
            result
        );
    }

    #[test]
    fn test_hyperbolic_derivatives() {
        let mut env = Environment::new();
        env.set("x", 1.0);

        // d/dx(sinh(x)) = cosh(x), at x=1: cosh(1)
        let result = differentiate_and_evaluate("\\sinh(x)", "x", &env).unwrap();
        assert!(
            approx_eq(result, 1.0_f64.cosh(), 1e-10),
            "d/dx sinh(x) at x=1 should be cosh(1)={}, got {}",
            1.0_f64.cosh(),
            result
        );

        // d/dx(cosh(x)) = sinh(x), at x=1: sinh(1)
        let result = differentiate_and_evaluate("\\cosh(x)", "x", &env).unwrap();
        assert!(
            approx_eq(result, 1.0_f64.sinh(), 1e-10),
            "d/dx cosh(x) at x=1 should be sinh(1)={}, got {}",
            1.0_f64.sinh(),
            result
        );

        // d/dx(tanh(x)) = 1 - tanh²(x), at x=1
        let result = differentiate_and_evaluate("\\tanh(x)", "x", &env).unwrap();
        let expected = 1.0 - 1.0_f64.tanh().powi(2);
        assert!(
            approx_eq(result, expected, 1e-10),
            "d/dx tanh(x) at x=1 should be {}, got {}",
            expected,
            result
        );
    }

    #[test]
    fn test_hyperbolic_reciprocal_derivatives() {
        let mut env = Environment::new();
        env.set("x", 0.5);

        // d/dx(csch(x)) = -csch(x)·coth(x), at x=0.5
        let csch_result = differentiate_and_evaluate("\\csch(x)", "x", &env).unwrap();
        let csch_expected = -(1.0 / 0.5_f64.sinh()) * (0.5_f64.cosh() / 0.5_f64.sinh());
        assert!(
            approx_eq(csch_result, csch_expected, 1e-10),
            "d/dx csch(x) at x=0.5 should be {}, got {}",
            csch_expected,
            csch_result
        );

        // d/dx(sech(x)) = -sech(x)·tanh(x), at x=0.5
        let sech_result = differentiate_and_evaluate("\\sech(x)", "x", &env).unwrap();
        let sech_expected = -(1.0 / 0.5_f64.cosh()) * 0.5_f64.tanh();
        assert!(
            approx_eq(sech_result, sech_expected, 1e-10),
            "d/dx sech(x) at x=0.5 should be {}, got {}",
            sech_expected,
            sech_result
        );

        // d/dx(coth(x)) = -csch²(x), at x=0.5
        let coth_result = differentiate_and_evaluate("\\coth(x)", "x", &env).unwrap();
        let coth_expected = -(1.0 / 0.5_f64.sinh()).powi(2);
        assert!(
            approx_eq(coth_result, coth_expected, 1e-10),
            "d/dx coth(x) at x=0.5 should be {}, got {}",
            coth_expected,
            coth_result
        );
    }

    #[test]
    fn test_inverse_hyperbolic_derivatives() {
        let mut env = Environment::new();
        env.set("x", 0.5);

        // d/dx(arcsinh(x)) = 1/√(1+x²), at x=0.5
        let result = differentiate_and_evaluate("\\arcsinh(x)", "x", &env).unwrap();
        let expected = 1.0 / (1.0 + 0.25_f64).sqrt();
        assert!(
            approx_eq(result, expected, 1e-10),
            "d/dx arcsinh(x) at x=0.5 should be {}, got {}",
            expected,
            result
        );

        // d/dx(arccosh(x)) = 1/√(x²-1), at x=2
        env.set("x", 2.0);
        let result = differentiate_and_evaluate("\\arccosh(x)", "x", &env).unwrap();
        let expected = 1.0 / (4.0_f64 - 1.0).sqrt();
        assert!(
            approx_eq(result, expected, 1e-10),
            "d/dx arccosh(x) at x=2 should be {}, got {}",
            expected,
            result
        );

        // d/dx(arctanh(x)) = 1/(1-x²), at x=0.5
        env.set("x", 0.5);
        let result = differentiate_and_evaluate("\\arctanh(x)", "x", &env).unwrap();
        let expected = 1.0 / (1.0 - 0.25_f64);
        assert!(
            approx_eq(result, expected, 1e-10),
            "d/dx arctanh(x) at x=0.5 should be {}, got {}",
            expected,
            result
        );
    }

    #[test]
    fn test_inverse_reciprocal_hyperbolic_derivatives() {
        let mut env = Environment::new();
        env.set("x", 2.0);

        // d/dx(arccsch(x)) = -1/(x·√(x²+1)), at x=2
        let result = differentiate_and_evaluate("\\arccsch(x)", "x", &env).unwrap();
        let expected = -1.0 / (2.0 * (4.0_f64 + 1.0).sqrt());
        assert!(
            approx_eq(result, expected, 1e-10),
            "d/dx arccsch(x) at x=2 should be {}, got {}",
            expected,
            result
        );

        // d/dx(arcsech(x)) = -1/(x·√(1-x²)), at x=0.5
        env.set("x", 0.5);
        let result = differentiate_and_evaluate("\\arcsech(x)", "x", &env).unwrap();
        let expected = -1.0 / (0.5 * (1.0 - 0.25_f64).sqrt());
        assert!(
            approx_eq(result, expected, 1e-10),
            "d/dx arcsech(x) at x=0.5 should be {}, got {}",
            expected,
            result
        );

        // d/dx(arccoth(x)) = 1/(1-x²), at x=2
        env.set("x", 2.0);
        let result = differentiate_and_evaluate("\\arccoth(x)", "x", &env).unwrap();
        let expected = 1.0 / (1.0 - 4.0_f64);
        assert!(
            approx_eq(result, expected, 1e-10),
            "d/dx arccoth(x) at x=2 should be {}, got {}",
            expected,
            result
        );
    }

    #[test]
    fn test_inverse_reciprocal_hyperbolic_derivatives_negative_branch() {
        let mut env = Environment::new();
        env.set("x", -2.0);

        // arccsch(x) = asinh(1/x), so derivative uses |x| in the denominator.
        let result = differentiate_and_evaluate("\\arccsch(x)", "x", &env).unwrap();
        let expected = -1.0 / (2.0 * (4.0_f64 + 1.0).sqrt());
        assert!(
            approx_eq(result, expected, 1e-10),
            "d/dx arccsch(x) at x=-2 should be {}, got {}",
            expected,
            result
        );
    }

    #[test]
    fn test_log_exp_derivatives() {
        let mut env = Environment::new();
        env.set("x", 10.0);

        // d/dx(log(x)) = 1/(x·ln(10)), at x=10
        let result = differentiate_and_evaluate("\\log(x)", "x", &env).unwrap();
        let expected = 1.0 / (10.0 * 10.0_f64.ln());
        assert!(
            approx_eq(result, expected, 1e-10),
            "d/dx log(x) at x=10 should be {}, got {}",
            expected,
            result
        );

        // d/dx(ln(x)) = 1/x, at x=10
        let result = differentiate_and_evaluate("\\ln(x)", "x", &env).unwrap();
        assert!(
            approx_eq(result, 0.1, 1e-10),
            "d/dx ln(x) at x=10 should be 0.1, got {}",
            result
        );

        // d/dx(lg(x)) = 1/(x·ln(2)), at x=10
        let result = differentiate_and_evaluate("\\lg(x)", "x", &env).unwrap();
        let lg_expected = 1.0 / (10.0 * 2.0_f64.ln());
        assert!(
            approx_eq(result, lg_expected, 1e-10),
            "d/dx lg(x) at x=10 should be {}, got {}",
            lg_expected,
            result
        );

        // d/dx(exp(x)) = exp(x), at x=1
        env.set("x", 1.0);
        let result = differentiate_and_evaluate("\\exp(x)", "x", &env).unwrap();
        assert!(
            approx_eq(result, 1.0_f64.exp(), 1e-10),
            "d/dx exp(x) at x=1 should be e, got {}",
            result
        );
    }
}
