use crate::environment::Environment;
use crate::evaluator::Evaluator;
use crate::node::Node;
use crate::expression::{extract_variable, solve_for_variable};
use crate::parser::{build_expression_tree, mathjson_to_node, tokenize};
use wasm_bindgen::prelude::*;

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

        let right_val = Evaluator::evaluate(&right_tree, &env)?; // Use ? to handle the Result

        // Extract the variable on the left-hand side.
        if let Some(var_name) = extract_variable(parts[0]) {
            // Solve for the variable.
            let result = solve_for_variable(&left_tree, right_val, &var_name)?; // Use ? here as well

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
    }
}
