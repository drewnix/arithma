use cassy::{build_expression_tree, solve_for_variable, evaluate_expression_tree, Environment};

pub fn eval_equation(equation: &str, expected: f64) {
    let mut env = Environment::new();  // Environment for variable resolution
    let parts: Vec<&str> = equation.split('=').collect();
    let left_expr = parts[0].trim();
    let right_expr = parts[1].trim();

    // Build the expression trees for both sides of the equation
    let left_tree = build_expression_tree(left_expr).expect("Failed to build left expression tree");
    let right_tree = build_expression_tree(right_expr).expect("Failed to build right expression tree");

    // Evaluate the right-hand side tree
    match evaluate_expression_tree(&right_tree, &env) {
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
