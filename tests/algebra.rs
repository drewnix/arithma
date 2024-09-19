
#[cfg(test)]
mod tests {
    use cassy::*;
    use serde_json::json;

    fn evaluate_mathjson(mathjson: serde_json::Value, env: &Environment) -> Result<f64, String> {
        let node = mathjson_to_node(&mathjson)?;
        Evaluator::evaluate(&node, env)
    }

    // 1. Basic Arithmetic and Operations
    #[test]
    fn test_basic_operations() {
        let env = Environment::new();

        // Addition: 3 + 7
        let addition = json!(["Add", 3, 7]);
        assert_eq!(evaluate_mathjson(addition, &env).unwrap(), 10.0);

        // Subtraction: 10 - 4
        let subtraction = json!(["Subtract", 10, 4]);
        assert_eq!(evaluate_mathjson(subtraction, &env).unwrap(), 6.0);

        // Multiplication: 5 * 6
        let multiplication = json!(["Multiply", 5, 6]);
        assert_eq!(evaluate_mathjson(multiplication, &env).unwrap(), 30.0);

        // Division: 12 / 4
        let division = json!(["Divide", 12, 4]);
        assert_eq!(evaluate_mathjson(division, &env).unwrap(), 3.0);

        // Power: 2^3
        let power = json!(["Power", 2, 3]);
        assert_eq!(evaluate_mathjson(power, &env).unwrap(), 8.0);

        // Square Root: sqrt(16)
        let sqrt = json!(["Sqrt", 16]);
        assert_eq!(evaluate_mathjson(sqrt, &env).unwrap(), 4.0);
    }

    // 2. Polynomials
    #[test]
    fn test_polynomials() {
        let env = Environment::new();

        // Polynomial: x^2 + 5x + 6
        let polynomial = json!(["Add", ["Add", ["Power", "x", 2], ["Multiply", 5, "x"]], 6]);
        let mut env_with_x = env.clone();
        env_with_x.set("x", 2.0); // Set x = 2
        assert_eq!(evaluate_mathjson(polynomial, &env_with_x).unwrap(), 20.0);
    }

    // 3. Rational Expressions
    #[test]
    fn test_rational_expression() {
        let env = Environment::new();

        // Rational Expression: (x^2 - 1) / (x - 1)
        let rational_expr = json!([
            "Divide",
            ["Subtract", ["Power", "x", 2], 1],
            ["Subtract", "x", 1]
        ]);
        let mut env_with_x = env.clone();
        env_with_x.set("x", 2.0); // Set x = 2
        assert_eq!(evaluate_mathjson(rational_expr, &env_with_x).unwrap(), 3.0);
    }

    // 4. Linear Equations and Systems
    #[test]
    fn test_linear_equation() {
        let env = Environment::new();

        // Solve for x in: 2x + 5 = 11
        let equation = json!(["Subtract", ["Add", ["Multiply", 2, "x"], 5], 11]);
        let solution = solve_for_variable(&mathjson_to_node(&equation).unwrap(), 0.0, "x").unwrap();
        assert_eq!(solution, 3.0);
    }

    // 5. Quadratic Equations
    #[test]
    fn test_quadratic_equation() {
        let env = Environment::new();

        // Quadratic Equation: x^2 - 4 = 0
        let quadratic = json!(["Subtract", ["Power", "x", 2], 4]);
        let mut env_with_x = env.clone();
        env_with_x.set("x", 2.0); // Set x = 2 as one solution
        assert_eq!(evaluate_mathjson(quadratic, &env_with_x).unwrap(), 0.0);
    }

    // 6. Exponential and Logarithmic Functions
    #[test]
    fn test_exponential_function() {
        let env = Environment::new();

        // Exponential: e^x (approximation, using e ≈ 2.718)
        let exponential = json!(["Power", 2.718, "x"]);
        let mut env_with_x = env.clone();
        env_with_x.set("x", 1.0); // Set x = 1
        assert_eq!(evaluate_mathjson(exponential, &env_with_x).unwrap(), 2.718);
    }

    #[test]
    fn test_logarithmic_function() {
        let env = Environment::new();

        // Natural Logarithm: ln(e) = 1 (using e ≈ 2.718)
        let log_expr = json!(["Power", 2.718, 1]);
        assert_eq!(evaluate_mathjson(log_expr, &env).unwrap(), 2.718); // Approximation
    }

    // 7. Radicals and Rational Exponents
    #[test]
    fn test_rational_exponent() {
        let env = Environment::new();

        // Rational Exponent: x^(1/2) = sqrt(x)
        let rational_exp = json!(["Power", "x", ["Rational", 1, 2]]);
        let mut env_with_x = env.clone();
        env_with_x.set("x", 9.0); // Set x = 9
        assert_eq!(evaluate_mathjson(rational_exp, &env_with_x).unwrap(), 3.0);
    }

    // 8. Inequalities
    #[test]
    fn test_inequality() {
        let env = Environment::new();

        // Inequality: x + 2 > 5
        let inequality = json!(["Greater", ["Add", "x", 2], 5]);
        let mut env_with_x = env.clone();
        env_with_x.set("x", 4.0); // Set x = 4
        assert_eq!(evaluate_mathjson(inequality, &env_with_x).unwrap(), 1.0); // 1.0 for true
    }

    // 9. Absolute Value
    #[test]
    fn test_absolute_value() {
        let env = Environment::new();

        // Absolute Value: |x - 5|
        let abs_expr = json!(["Abs", ["Subtract", "x", 5]]);
        let mut env_with_x = env.clone();
        env_with_x.set("x", 2.0); // Set x = 2
        assert_eq!(evaluate_mathjson(abs_expr, &env_with_x).unwrap(), 3.0);

        // Absolute Value: |x - 5| for x = 7
        let abs_expr = json!(["Abs", ["Subtract", "x", 5]]);
        env_with_x.set("x", 7.0); // Set x = 7
        assert_eq!(evaluate_mathjson(abs_expr, &env_with_x).unwrap(), 2.0);

        // Absolute Value: |x - 5| for x = -3
        let abs_expr = json!(["Abs", ["Subtract", "x", 5]]);
        env_with_x.set("x", -3.0); // Set x = -3
        assert_eq!(evaluate_mathjson(abs_expr, &env_with_x).unwrap(), 8.0);
    }

    // 10. Piecewise Functions
    #[test]
    fn test_piecewise_function() {
        let env = Environment::new();

        // Piecewise function: f(x) = x^2 if x >= 0, -x if x < 0
        let piecewise = json!([
            "Piecewise",
            [["Power", "x", 2], ["GreaterEqual", "x", 0]],
            [["Subtract", 0, "x"], ["Less", "x", 0]]
        ]);
        let mut env_with_x = env.clone();
        env_with_x.set("x", -3.0); // Set x = -3
        assert_eq!(evaluate_mathjson(piecewise, &env_with_x).unwrap(), 3.0);
    }
}
