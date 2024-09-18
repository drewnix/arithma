use cassy::{build_expression_tree, solve_for_variable, tokenize, Environment, Evaluator};

pub fn eval_equation(equation: &str, expected: f64) {
    let env = Environment::new();  // Environment for variable resolution
    let parts: Vec<&str> = equation.split('=').collect();
    let left_expr = parts[0].trim();
    let right_expr = parts[1].trim();

    // Tokenize the expressions
    let left_tokens = tokenize(left_expr);
    let right_tokens = tokenize(right_expr);

    // Build the expression trees for both sides of the equation
    let left_tree = build_expression_tree(left_tokens).expect("Failed to build left expression tree");
    let right_tree = build_expression_tree(right_tokens).expect("Failed to build right expression tree");

    // Evaluate the right-hand side tree using the Evaluator
    match Evaluator::evaluate(&right_tree, &env) {
        Ok(right_val) => {
            // Find the variable to solve for
            if let Some(variable) = cassy::extract_variable(left_expr) {
                let result = solve_for_variable(&left_tree, right_val, &variable).expect("Failed to solve for variable");
                assert_eq!(result, expected);
            } else {
                panic!("Could not find a variable to solve for.");
            }
        }
        Err(e) => panic!("Equation did not evaluate properly: {}", e),
    }
}