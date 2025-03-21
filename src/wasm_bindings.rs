use crate::environment::Environment;
use crate::evaluator::Evaluator;
use crate::expression::extract_variable;
use crate::expression::solve_for_variable;
use crate::node::Node;
use crate::parser::build_expression_tree;
use crate::simplify::Simplifiable;
use crate::tokenizer::Tokenizer;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn evaluate_latex_expression_js(latex_expr: &str, env_json: &str) -> Result<String, JsValue> {
    // Deserialize the environment
    let env: Environment = serde_json::from_str(env_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse environment: {}", e)))?;

    // Create an instance of the Tokenizer
    let mut tokenizer = Tokenizer::new(latex_expr); // Pass input as a reference

    // Tokenize and parse the input
    let tokens = tokenizer.tokenize(); // Call the instance method on tokenizer
    let parsed_expr = build_expression_tree(tokens)
        .map_err(|e| JsValue::from_str(&format!("Error parsing LaTeX: {}", e)))?;

    // Check if it's an equation that we need to solve
    if let Node::Equation(_, _) = &parsed_expr {
        // Try to find a variable to solve for
        if let Some(var_name) = extract_variable(latex_expr) {
            match solve_for_variable(&parsed_expr, &var_name) {
                Ok(solution) => return Ok(format!("{} = {}", var_name, solution)),
                Err(e) => return Err(JsValue::from_str(&format!("Error solving equation: {}", e))),
            }
        }
    }

    // Always simplify the expression first
    let simplified_expr = parsed_expr
        .simplify(&env)
        .map_err(|e| JsValue::from_str(&format!("Error simplifying expression: {}", e)))?;

    // Try to evaluate the simplified expression
    match Evaluator::evaluate(&simplified_expr, &env) {
        Ok(result) => Ok(result.to_string()), // Return fully evaluated result if possible
        Err(_) => Ok(simplified_expr.to_string()), // If evaluation fails, return the simplified expression
    }
}
