use cassy::{evaluate_rpn, tokenize, shunting_yard, Environment, solve_for_variable};

pub fn eval_equation(equation: &str, expected: f64) {
    let env = Environment::new();  // Environment doesn't need to be mutable
    let parts: Vec<&str> = equation.split('=').collect();
    let left_expr = parts[0].trim();
    let right_expr = parts[1].trim();

    let left_tokens = tokenize(left_expr);
    let right_tokens = tokenize(right_expr);

    match (shunting_yard(left_tokens), shunting_yard(right_tokens)) {
        (Ok(_left_rpn), Ok(right_rpn)) => {
            match evaluate_rpn(right_rpn, &env) {
                Ok(right_val) => {
                    if let Some(_var_name) = cassy::extract_variable(left_expr) {
                        let result = solve_for_variable(left_expr, right_val, &env).unwrap();
                        assert_eq!(result, expected);
                    }
                }
                _ => panic!("Equation did not evaluate properly"),
            }
        }
        _ => panic!("Error parsing the equation"),
    }
}