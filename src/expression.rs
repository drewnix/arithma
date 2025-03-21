use crate::node::Node;
use crate::Tokenizer;

pub fn extract_variable(expr: &str) -> Option<String> {
    // Create an instance of the Tokenizer
    let mut tokenizer = Tokenizer::new(expr); // Pass input as a reference

    // Tokenize and parse the input
    let tokens = tokenizer.tokenize(); // Call the instance method on tokenizer
    for token in tokens {
        if token.chars().all(char::is_alphabetic) {
            return Some(token);
        }
    }
    None
}

pub fn solve_for_variable(expr: &Node, target_var: &str) -> Result<f64, String> {
    // Check if the expression is an equation
    if let Node::Equation(left, right) = expr {
        // Move everything to the left side of the equation: left - right = 0
        let equation_expr = Node::Subtract(left.clone(), right.clone());
        return solve_equation(&equation_expr, target_var);
    } else {
        // If not an equation, assume we're setting it to zero
        return solve_equation(expr, target_var);
    }
}

fn solve_equation(expr: &Node, target_var: &str) -> Result<f64, String> {
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

    // Solve for the variable: 0 = coefficient * variable + constant
    // Adjust the equation: variable = -constant / coefficient
    let result = -constant / coefficient;
    Ok(result)
}
