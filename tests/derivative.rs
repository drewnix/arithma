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
    }
}
