use crate::environment::Environment;
use crate::evaluator::Evaluator;
use crate::parser::{build_expression_tree, tokenize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn evaluate_latex_expression_js(latex_expr: &str, env_json: &str) -> Result<String, JsValue> {
    // Deserialize the environment
    let env: Environment = serde_json::from_str(env_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse environment: {}", e)))?;

    // Tokenize and parse the LaTeX expression
    let tokens = tokenize(latex_expr);
    let parsed_expr = build_expression_tree(tokens)
        .map_err(|e| JsValue::from_str(&format!("Error parsing LaTeX: {}", e)))?;

    // Evaluate the expression using the existing evaluator
    let result = Evaluator::evaluate(&parsed_expr, &env)
        .map_err(|e| JsValue::from_str(&format!("Error evaluating expression: {}", e)))?;

    Ok(result.to_string())
}
