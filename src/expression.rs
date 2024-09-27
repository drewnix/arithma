use crate::environment::Environment;
use crate::node::Node;
use crate::parser::tokenize;

pub fn extract_variable(expr: &str) -> Option<String> {
    let tokens = tokenize(expr);
    for token in tokens {
        if token.chars().all(char::is_alphabetic) {
            return Some(token);
        }
    }
    None
}

pub fn solve_for_variable(expr: &Node, right_val: f64, target_var: &str) -> Result<f64, String> {
    let mut coefficient = 0.0; // Coefficient of the target variable
    let mut constant = 0.0; // Constant part to move to the other side

    fn traverse(
        node: &Node,
        target_var: &str,
        coefficient: &mut f64,
        constant: &mut f64,
        multiplier: f64, // Apply multiplier for cases like (x + 2) * 3
    ) -> Result<(), String> {
        match node {
            Node::Number(num) => {
                *constant += multiplier * *num; // Apply multiplier to constants
            }
            Node::Variable(var_name) => {
                if var_name == target_var {
                    *coefficient += multiplier; // Apply multiplier to the variable coefficient
                } else {
                    return Err(format!("Unexpected variable '{}'", var_name));
                }
            }
            Node::Add(left, right) => {
                traverse(left, target_var, coefficient, constant, multiplier)?; // Traverse left side
                traverse(right, target_var, coefficient, constant, multiplier)?;
                // Traverse right side
            }
            Node::Subtract(left, right) => {
                traverse(left, target_var, coefficient, constant, multiplier)?; // Traverse left side
                traverse(right, target_var, coefficient, constant, -multiplier)?;
                // Apply negation to the right
            }
            Node::Multiply(left, right) => {
                if let Node::Number(num) = **left {
                    // If the left node is a number, it's a multiplier for the right side
                    traverse(right, target_var, coefficient, constant, multiplier * num)?;
                } else if let Node::Number(num) = **right {
                    // If the right node is a number, it's a multiplier for the left side
                    traverse(left, target_var, coefficient, constant, multiplier * num)?;
                } else {
                    return Err(
                        "Expected one operand to be a number in multiplication.".to_string()
                    );
                }
            }
            Node::Divide(left, right) => {
                let mut right_constant = 0.0;
                traverse(right, target_var, coefficient, &mut right_constant, 1.0)?;
                if right_constant == 0.0 {
                    return Err("Division by zero".to_string());
                }
                traverse(
                    left,
                    target_var,
                    coefficient,
                    constant,
                    multiplier / right_constant,
                )?;
            }
            _ => return Err("Unexpected node in expression.".to_string()), // Handle any other unexpected node
        }
        Ok(())
    }

    // Start the traversal with a multiplier of 1.0
    traverse(expr, target_var, &mut coefficient, &mut constant, 1.0)?;

    if coefficient == 0.0 {
        return Err(format!(
            "Coefficient of variable '{}' is zero, can't solve.",
            target_var
        ));
    }

    // Solve for the variable: right_val = coefficient * variable + constant
    // Adjust the equation: variable = (right_val - constant) / coefficient
    let result = (right_val - constant) / coefficient;
    Ok(result)
}

pub fn evaluate_rpn(tokens: Vec<String>, env: &Environment) -> Result<f64, String> {
    let mut stack: Vec<f64> = Vec::new();

    for token in tokens {
        if let Ok(num) = token.parse::<f64>() {
            // If the token is a number, push it onto the stack
            stack.push(num);
        } else if let Some(value) = env.get(&token) {
            // If the token is a variable in the environment, retrieve its value
            stack.push(value);
        } else if "+-*/^".contains(&token) {
            // Ensure we have enough operands to apply the operator
            if stack.len() < 2 {
                return Err(format!("Not enough operands for operator '{}'", token));
            }

            let right = stack.pop().unwrap();
            let left = stack.pop().unwrap();

            match token.as_str() {
                "+" => stack.push(left + right),
                "-" => stack.push(left - right),
                "*" => stack.push(left * right),
                "/" => {
                    if right == 0.0 {
                        return Err("Division by zero error".to_string());
                    }
                    stack.push(left / right);
                }
                _ => return Err(format!("Unexpected operator '{}'", token)),
            }
        } else {
            // Assume the token is a variable we're solving for and ignore it for now
            // This lets us skip the variable until it's solved
            continue;
        }
    }

    // After processing all tokens, the stack should contain exactly one value, which is the result
    if stack.len() != 1 {
        return Err("The RPN expression did not resolve to a single value.".to_string());
    }

    Ok(stack.pop().unwrap())
}
