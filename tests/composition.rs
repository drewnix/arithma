#[cfg(test)]
mod composition_tests {
    use arithma::{build_expression_tree, compose, compose_latex, compose_multiple, Environment, Evaluator, Tokenizer};

    fn parse_expression(latex: &str) -> Result<arithma::Node, String> {
        let mut tokenizer = Tokenizer::new(latex);
        let tokens = tokenizer.tokenize();
        build_expression_tree(tokens)
    }

    fn evaluate_expression(latex: &str, env: &Environment) -> Result<f64, String> {
        let expr = parse_expression(latex)?;
        Evaluator::evaluate(&expr, env)
    }

    #[test]
    fn test_basic_polynomial_composition() {
        // f(x) = 3x + 1, g(x) = x^2
        // f(g(x)) = 3(x^2) + 1 = 3x^2 + 1
        let f_expr = parse_expression("3*x + 1").unwrap();
        let g_expr = parse_expression("x^2").unwrap();
        
        let result = compose(&f_expr, "x", &g_expr).unwrap();
        
        let mut env = Environment::new();
        env.set("x", 2.0);
        
        // At x=2: f(g(2)) = 3(2^2) + 1 = 3*4 + 1 = 13
        let evaluated = Evaluator::evaluate(&result, &env).unwrap();
        assert_eq!(evaluated, 13.0);
    }

    #[test]
    fn test_trigonometric_composition() {
        // f(x) = sin(x), g(x) = 2x
        // f(g(x)) = sin(2x)
        let f_expr = parse_expression("\\sin{x}").unwrap();
        let g_expr = parse_expression("2*x").unwrap();
        
        let result = compose(&f_expr, "x", &g_expr).unwrap();
        
        let mut env = Environment::new();
        env.set("x", std::f64::consts::PI / 4.0); // π/4
        
        // At x=π/4: f(g(π/4)) = sin(2*π/4) = sin(π/2) = 1
        let evaluated = Evaluator::evaluate(&result, &env).unwrap();
        assert!((evaluated - 1.0).abs() < 1e-12);
    }

    #[test]
    fn test_sqrt_composition() {
        // f(x) = sqrt(x), g(x) = x + 4
        // f(g(x)) = sqrt(x + 4)
        let f_expr = parse_expression("\\sqrt{x}").unwrap();
        let g_expr = parse_expression("x + 4").unwrap();
        
        let result = compose(&f_expr, "x", &g_expr).unwrap();
        
        let mut env = Environment::new();
        env.set("x", 5.0);
        
        // At x=5: f(g(5)) = sqrt(5 + 4) = sqrt(9) = 3
        let evaluated = Evaluator::evaluate(&result, &env).unwrap();
        assert_eq!(evaluated, 3.0);
    }

    #[test]
    fn test_complex_composition() {
        // f(x) = x^2 + 2*x + 1, g(x) = 3*x - 2
        // f(g(x)) = (3x-2)^2 + 2(3x-2) + 1 = 9x^2 - 12x + 4 + 6x - 4 + 1 = 9x^2 - 6x + 1
        let result = compose_latex("x^2 + 2*x + 1", "x", "3*x - 2").unwrap();
        
        // Expanded form would be 9x^2 - 6x + 1
        let mut env = Environment::new();
        env.set("x", 1.0);
        
        // At x=1: f(g(1)) = (3*1-2)^2 + 2(3*1-2) + 1 = 1^2 + 2*1 + 1 = 1 + 2 + 1 = 4
        let evaluated = evaluate_expression(&result, &env).unwrap();
        assert_eq!(evaluated, 4.0);
    }

    #[test]
    fn test_multiple_composition() {
        // f(x) = x^2, g(x) = x + 1, h(x) = 2*x
        // f(g(h(x))) = ((2x) + 1)^2 = (2x + 1)^2 = 4x^2 + 4x + 1
        let f_expr = parse_expression("x^2").unwrap();
        let g_expr = parse_expression("x + 1").unwrap();
        let h_expr = parse_expression("2*x").unwrap();
        
        // Define the chain of functions
        let functions = vec![
            (h_expr, "x".to_string()),
            (g_expr, "x".to_string()),
            (f_expr, "x".to_string()),
        ];
        
        let result = compose_multiple(&functions).unwrap();
        
        let mut env = Environment::new();
        env.set("x", 1.5);
        
        // At x=1.5: f(g(h(1.5))) = ((2*1.5) + 1)^2 = (3 + 1)^2 = 4^2 = 16
        let evaluated = Evaluator::evaluate(&result, &env).unwrap();
        assert_eq!(evaluated, 16.0);
    }

    #[test]
    fn test_multi_variable_composition() {
        // f(x, y) = x*y + y, g(t) = t^2
        // f(g(t), y) = (t^2)*y + y = y*t^2 + y = y(t^2 + 1)
        let f_expr = parse_expression("x*y + y").unwrap();
        let g_expr = parse_expression("t^2").unwrap();
        
        let result = compose(&f_expr, "x", &g_expr).unwrap();
        
        let mut env = Environment::new();
        env.set("t", 3.0);
        env.set("y", 2.0);
        
        // At t=3, y=2: f(g(3), 2) = (3^2)*2 + 2 = 9*2 + 2 = 18 + 2 = 20
        let evaluated = Evaluator::evaluate(&result, &env).unwrap();
        assert_eq!(evaluated, 20.0);
    }

    #[test]
    fn test_nested_function_composition() {
        // f(x) = sin(x), g(x) = cos(x), h(x) = x^2
        // f(g(h(x))) = sin(cos(x^2))
        let result = compose_latex("\\sin{x}", "x", "\\cos{x}").unwrap();
        let result = compose_latex(&result, "x", "x^2").unwrap();
        
        // This is a complex composition to verify, but we can test at some specific points
        let mut env = Environment::new();
        env.set("x", 0.0);
        
        // At x=0: f(g(h(0))) = sin(cos(0^2)) = sin(cos(0)) = sin(1) ≈ 0.8415
        let evaluated = evaluate_expression(&result, &env).unwrap();
        assert!((evaluated - f64::sin(1.0)).abs() < 1e-10);
    }

    #[test]
    fn test_composition_with_rational_functions() {
        // f(x) = 1/x, g(x) = x + 2
        // f(g(x)) = 1/(x + 2)
        let f_expr = parse_expression("1/x").unwrap();
        let g_expr = parse_expression("x + 2").unwrap();
        
        let result = compose(&f_expr, "x", &g_expr).unwrap();
        
        let mut env = Environment::new();
        env.set("x", 1.0);
        
        // At x=1: f(g(1)) = 1/(1 + 2) = 1/3
        let evaluated = Evaluator::evaluate(&result, &env).unwrap();
        assert!((evaluated - 1.0/3.0).abs() < 1e-10);
    }
}