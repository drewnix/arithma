use wasm_bindgen::prelude::*;

// Declare the node module
mod node;
pub use crate::node::Node;

// Declare the environment module and make Environment public
mod environment;
pub use crate::environment::Environment;

// Declare the evaluator module
mod evaluator; // Declare evaluator module
pub use crate::evaluator::Evaluator; // Re-export Evaluator so it can be used elsewhere

mod parser; // Add this to lib.rs
pub use crate::parser::{tokenize, build_expression_tree, shunting_yard};

pub fn extract_variable(expr: &str) -> Option<String> {
    let tokens = tokenize(expr);
    for token in tokens {
        if token.chars().all(char::is_alphabetic) {
            return Some(token);
        }
    }
    None
}

#[wasm_bindgen]
pub fn solve_for_variable_js(
    expr_json: &str,
    right_val: f64,
    target_var: &str,
) -> Result<JsValue, JsValue> {
    // Deserialize the JSON input into a Node
    let expr: Node = serde_json::from_str(expr_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse expression: {}", e)))?;

    // Call the original solve_for_variable function
    match solve_for_variable(&expr, right_val, target_var) {
        Ok(result) => Ok(JsValue::from_f64(result)), // Return the result as a JsValue (f64)
        Err(e) => Err(JsValue::from_str(&e)),        // Return the error as a JsValue (String)
    }
}

pub fn mathjson_to_node(mathjson: &serde_json::Value) -> Result<Node, String> {
    match mathjson {
        serde_json::Value::Array(array) => {
            if array.is_empty() {
                return Err("Empty MathJSON array".to_string());
            }

            let operator = array[0].as_str().ok_or("Invalid MathJSON operator")?;

            match operator {
                "Rational" => {
                    let numerator = array
                        .get(1)
                        .and_then(|v| v.as_i64())
                        .ok_or("Invalid numerator")?;
                    let denominator = array
                        .get(2)
                        .and_then(|v| v.as_i64())
                        .ok_or("Invalid denominator")?;
                    Ok(Node::Rational(numerator, denominator))
                }
                "Add" => Ok(Node::Add(
                    Box::new(mathjson_to_node(&array[1])?),
                    Box::new(mathjson_to_node(&array[2])?),
                )),
                "Subtract" => Ok(Node::Subtract(
                    Box::new(mathjson_to_node(&array[1])?),
                    Box::new(mathjson_to_node(&array[2])?),
                )),
                "Multiply" => Ok(Node::Multiply(
                    Box::new(mathjson_to_node(&array[1])?),
                    Box::new(mathjson_to_node(&array[2])?),
                )),
                "Divide" => Ok(Node::Divide(
                    Box::new(mathjson_to_node(&array[1])?),
                    Box::new(mathjson_to_node(&array[2])?),
                )),
                "Power" => Ok(Node::Power(
                    Box::new(mathjson_to_node(&array[1])?),
                    Box::new(mathjson_to_node(&array[2])?),
                )),
                "Sqrt" => Ok(Node::Sqrt(
                    Box::new(mathjson_to_node(&array[1])?)
                )),
                "Abs" => Ok(Node::Abs(
                    Box::new(mathjson_to_node(&array[1])?)
                )),
                _ => Err(format!("Unsupported operator: {}", operator)),
            }
        }
        serde_json::Value::Number(num) => {
            if let Some(n) = num.as_f64() {
                Ok(Node::Number(n))
            } else {
                Err("Invalid number format in MathJSON".to_string())
            }
        }
        serde_json::Value::String(var) => {
            match var.as_str() {
                "ExponentialE" => Ok(Node::Number(std::f64::consts::E)),
                "Pi" => Ok(Node::Number(std::f64::consts::PI)),
                _ => Ok(Node::Variable(var.clone())),
            }
        }
        _ => Err("Invalid MathJSON format".to_string()),
    }
}
#[wasm_bindgen]
pub fn evaluate_expression_js(expr: &str, env_json: &str) -> Result<String, JsValue> {
    // Deserialize the environment
    let env: Environment = serde_json::from_str(env_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse environment: {}", e)))?;

    // Check if the input is MathJSON (you can check based on its format)
    if let Ok(mathjson_value) = serde_json::from_str::<serde_json::Value>(expr) {
        // Handle MathJSON by converting it to a Node structure
        let node = mathjson_to_node(&mathjson_value).map_err(|e| {
            JsValue::from_str(&format!(
                "Error parsing MathJSON: {}, MathJSON: {}",
                e, expr
            ))
        })?;

        // Evaluate the Node
        let result = Evaluator::evaluate(&node, &env).map_err(|e| {
            JsValue::from_str(&format!("Error evaluating MathJSON expression: {}", e))
        })?;

        return Ok(result.to_string()); // Return result as string
    }

    // If expression contains '=' (e.g. "x + 2 = 5"), split into two parts.
    if expr.contains('=') {
        let parts: Vec<&str> = expr.split('=').map(|part| part.trim()).collect();
        if parts.len() != 2 {
            return Err(JsValue::from_str(
                "Invalid equation format. Use 'left = right'.",
            ));
        }

        // Parse the left and right parts of the equation.
        let left_tokens = tokenize(parts[0]);
        let right_tokens = tokenize(parts[1]);

        // Build expression trees for both parts.
        let left_tree = build_expression_tree(left_tokens)
            .map_err(|e| JsValue::from_str(&format!("Error parsing left-hand side: {}", e)))?;
        let right_tree = build_expression_tree(right_tokens)
            .map_err(|e| JsValue::from_str(&format!("Error parsing right-hand side: {}", e)))?;

        let right_val = Evaluator::evaluate(&right_tree, &env)?;  // Use ? to handle the Result

        // Extract the variable on the left-hand side.
        if let Some(var_name) = extract_variable(parts[0]) {
            // Solve for the variable.
            let result = solve_for_variable(&left_tree, right_val, &var_name)?;  // Use ? here as well

            // Return the formatted result as "x = 5".
            return Ok(format!("{} = {}", var_name, result));
        } else {
            return Err(JsValue::from_str(
                "No variable found on the left-hand side to solve for.",
            ));
        }
    }

    // Handle expressions without '=' (standard expression evaluation)
    let tokens = tokenize(expr);
    let tree = build_expression_tree(tokens)
        .map_err(|e| JsValue::from_str(&format!("Error parsing expression: {}", e)))?;
    let result: Result<f64, String> = Evaluator::evaluate(&tree, &env);

    match result {
        Ok(val) => Ok(val.to_string()),
        Err(e) => Err(JsValue::from_str(&e)),
    }}

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
