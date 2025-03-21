#[cfg(test)]
mod integration_tests {
    use arithma::{build_expression_tree, integrate, integrate_latex, definite_integral, definite_integral_latex, Environment, Evaluator, Tokenizer};

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
        // Remove the "+ C" constant of integration for evaluation
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
        assert_eq!(result, 15.0, "Integration of 5 with respect to x at x=3 should be 15");
    }

    #[test]
    fn test_variable_integration() {
        // ∫x dx = x²/2
        let mut env = Environment::new();
        env.set("x", 4.0);
        let result = evaluate_integral("x", "x", &env).unwrap();
        assert_eq!(result, 8.0, "Integration of x with respect to x at x=4 should be 8");
    }

    #[test]
    fn test_power_rule() {
        // ∫x^n dx = x^(n+1)/(n+1)
        
        // Test x^2
        let mut env = Environment::new();
        env.set("x", 2.0);
        let result = evaluate_integral("x^2", "x", &env).unwrap();
        assert!(approx_eq(result, 2.67, 0.01), "Integration of x^2 with respect to x at x=2 should be 2.67 ≈ 8/3");
        
        // Test x^3
        env.set("x", 2.0);
        let result = evaluate_integral("x^3", "x", &env).unwrap();
        assert_eq!(result, 4.0, "Integration of x^3 with respect to x at x=2 should be 4");
        
        // Test x^(-2)
        env.set("x", 2.0);
        let result = evaluate_integral("x^(-2)", "x", &env).unwrap();
        assert_eq!(result, -0.5, "Integration of x^(-2) with respect to x at x=2 should be -0.5");
    }

    #[test]
    fn test_logarithmic_integration() {
        // ∫(1/x) dx = ln|x|
        let mut env = Environment::new();
        env.set("x", 2.0);
        let result = evaluate_integral("1/x", "x", &env).unwrap();
        assert!(approx_eq(result, 2.0_f64.ln(), 1e-10), 
                "Integration of 1/x with respect to x at x=2 should be ln(2)");
    }

    #[test]
    fn test_sum_integration() {
        // ∫(x^2 + 2x + 1) dx = x³/3 + x² + x
        let mut env = Environment::new();
        env.set("x", 2.0);
        let result = evaluate_integral("x^2 + 2*x + 1", "x", &env).unwrap();
        
        // At x=2: 2³/3 + 2² + 2 = 8/3 + 4 + 2 = 2.67 + 6 = 8.67
        assert!(approx_eq(result, 8.67, 0.01), 
                "Integration of x^2 + 2x + 1 with respect to x at x=2 should be approximately 8.67");
    }

    #[test]
    fn test_definite_integrals() {
        // ∫₁² x² dx = [x³/3]₁² = 8/3 - 1/3 = 7/3 ≈ 2.33
        let result = definite_integral_latex("x^2", "x", 1.0, 2.0).unwrap();
        let value = result.parse::<f64>().unwrap_or(0.0);
        assert!(approx_eq(value, 7.0/3.0, 0.01), 
                "Definite integral of x^2 from 1 to 2 should be approximately 2.33");
        
        // ∫₀¹ (2x + 1) dx = [x² + x]₀¹ = (1 + 1) - (0 + 0) = 2
        let result = definite_integral_latex("2*x + 1", "x", 0.0, 1.0).unwrap();
        let value = result.parse::<f64>().unwrap_or(0.0);
        assert!(approx_eq(value, 2.0, 0.01), 
                "Definite integral of 2x + 1 from 0 to 1 should be 2");
    }

    #[test]
    fn test_polynomial_integration() {
        // ∫(3x⁴ - 2x² + 4) dx = (3x⁵/5) - (2x³/3) + 4x
        let mut env = Environment::new();
        env.set("x", 2.0);
        let result = evaluate_integral("3*x^4 - 2*x^2 + 4", "x", &env).unwrap();
        
        // At x=2: (3*2⁵/5) - (2*2³/3) + 4*2 = (3*32/5) - (2*8/3) + 8 = 19.2 - 5.33 + 8 = 21.87
        assert!(approx_eq(result, 21.87, 0.01), 
                "Integration of 3x⁴ - 2x² + 4 with respect to x at x=2 should be approximately 21.87");
    }

    #[test]
    fn test_composite_terms() {
        // Test integration with coefficient and power: ∫(2x³) dx = 2∫x³ dx = 2(x⁴/4) = x⁴/2
        let mut env = Environment::new();
        env.set("x", 2.0);
        let result = evaluate_integral("2*x^3", "x", &env).unwrap();
        
        // At x=2: 2⁴/2 = 16/2 = 8
        assert_eq!(result, 8.0, "Integration of 2x³ with respect to x at x=2 should be 8");
    }

    #[test]
    fn test_complex_integrals() {
        // ∫(x³ + x² - 2x + 1) dx = x⁴/4 + x³/3 - x² + x
        let result = integrate_latex("x^3 + x^2 - 2*x + 1", "x").unwrap();
        
        // Check that the result contains the expected terms
        assert!(result.contains("x^4") && result.contains("x^3") && result.contains("x^2") && result.contains("+ x")
               && result.contains("+ C"), 
               "Integration result should have the correct form");
        
        // Verify with a definite integral
        let def_result = definite_integral_latex("x^3 + x^2 - 2*x + 1", "x", 0.0, 1.0).unwrap();
        let value = def_result.parse::<f64>().unwrap_or(0.0);
        
        // [x⁴/4 + x³/3 - x² + x]₀¹ = (1/4 + 1/3 - 1 + 1) - 0 = 0.583
        assert!(approx_eq(value, 0.583, 0.01), 
                "Definite integral of x³ + x² - 2x + 1 from 0 to 1 should be approximately 0.583");
    }
}