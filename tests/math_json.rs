#[cfg(test)]
mod tests {
    use cassy::*;
    use serde_json::json;

    // Helper function to evaluate a MathJSON expression and return the result.
    fn evaluate_mathjson(mathjson: serde_json::Value, env: &Environment) -> Result<f64, String> {
        let node = mathjson_to_node(&mathjson)?;
        Evaluator::evaluate(&node, env)
    }

    #[test]
    fn test_sqrt() {
        // Define the MathJSON for square root of 16
        let mathjson = json!(["Sqrt", 16]);

        let env = Environment::new();
        let result = evaluate_mathjson(mathjson, &env);

        assert_eq!(result.unwrap(), 4.0);
    }

    #[test]
    fn test_addition() {
        // Define MathJSON for addition: 5 + 3
        let mathjson = json!(["Add", 5, 3]);

        let env = Environment::new();
        let result = evaluate_mathjson(mathjson, &env);

        assert_eq!(result.unwrap(), 8.0);
    }

    #[test]
    fn test_subtraction_with_variable() {
        // Define MathJSON for subtraction: x - 2
        let mathjson = json!(["Subtract", "x", 2]);

        // Set up the environment with x = 10
        let mut env = Environment::new();
        env.set("x", 10.0);

        let result = evaluate_mathjson(mathjson, &env);

        assert_eq!(result.unwrap(), 8.0);
    }

    #[test]
    fn test_multiplication_rational() {
        // Define MathJSON for multiplication: (3/4) * 4
        let mathjson = json!(["Multiply", ["Rational", 3, 4], 4]);

        let env = Environment::new();
        let result = evaluate_mathjson(mathjson, &env);

        assert_eq!(result.unwrap(), 3.0);
    }

    #[test]
    fn test_division() {
        // Define MathJSON for division: 10 / 2
        let mathjson = json!(["Divide", 10, 2]);

        let env = Environment::new();
        let result = evaluate_mathjson(mathjson, &env);

        assert_eq!(result.unwrap(), 5.0);
    }

    #[test]
    fn test_power() {
        // Define MathJSON for power: 2^3
        let mathjson = json!(["Power", 2, 3]);

        let env = Environment::new();
        let result = evaluate_mathjson(mathjson, &env);

        assert_eq!(result.unwrap(), 8.0);
    }

    #[test]
    fn test_sqrt_negative() {
        // Define MathJSON for square root of -16 (expect error)
        let mathjson = json!(["Sqrt", -16]);

        let env = Environment::new();
        let result = evaluate_mathjson(mathjson, &env);

        assert!(result.is_err()); // Should fail since sqrt of a negative number is not supported
    }
}