use crate::composition::compose_latex;
use crate::derivative::differentiate_latex;
use crate::environment::Environment;
use crate::evaluator::Evaluator;
use crate::exact::ExactNum;
use crate::expression::extract_variable;
use crate::integration::{definite_integral_latex, integrate_latex};
use crate::limits::limit_latex;
use crate::matrix::parse_latex_matrix;
use crate::node::Node;
use crate::ode::solve_ode_latex;
use crate::parser::build_expression_tree;
use crate::series::taylor_series_latex;
use crate::simplify::Simplifiable;
use crate::substitute::substitute_latex;
use crate::tokenizer::Tokenizer;
use wasm_bindgen::prelude::*;

#[allow(unexpected_cfgs)]
#[wasm_bindgen]
pub fn simplify_latex_js(latex_expr: &str) -> Result<String, JsValue> {
    let mut tokenizer = Tokenizer::new(latex_expr);
    let tokens = tokenizer.tokenize();
    let expr = build_expression_tree(tokens)
        .map_err(|e| JsValue::from_str(&format!("Error parsing LaTeX: {}", e)))?;
    let env = crate::environment::Environment::new();
    let simplified = expr
        .simplify(&env)
        .map_err(|e| JsValue::from_str(&format!("Error simplifying: {}", e)))?;
    Ok(format!("{}", simplified))
}

#[allow(unexpected_cfgs)]
#[wasm_bindgen]
pub fn polynomial_factor_js(latex_expr: &str, var_name: &str) -> Result<String, JsValue> {
    let mut tokenizer = Tokenizer::new(latex_expr);
    let tokens = tokenizer.tokenize();
    let expr = build_expression_tree(tokens)
        .map_err(|e| JsValue::from_str(&format!("Error parsing LaTeX: {}", e)))?;
    let poly = crate::polynomial::Polynomial::from_node(&expr, var_name)
        .map_err(|e| JsValue::from_str(&format!("Not a polynomial: {}", e)))?;
    let factors = poly.square_free_decomposition();
    let parts: Vec<String> = factors
        .iter()
        .map(|(f, m)| {
            if *m == 1 {
                format!("({})", f)
            } else {
                format!("({})^{{{}}}", f, m)
            }
        })
        .collect();
    Ok(parts.join(" \\cdot "))
}

#[allow(unexpected_cfgs)]
#[wasm_bindgen]
pub fn compose_functions_js(f_latex: &str, f_var: &str, g_latex: &str) -> Result<String, JsValue> {
    // Compose the functions
    match compose_latex(f_latex, f_var, g_latex) {
        Ok(result) => Ok(result),
        Err(e) => Err(JsValue::from_str(&format!(
            "Error in function composition: {}",
            e
        ))),
    }
}

#[allow(unexpected_cfgs)]
#[wasm_bindgen]
pub fn integrate_expression_js(latex_expr: &str, var_name: &str) -> Result<String, JsValue> {
    // Calculate the indefinite integral
    match integrate_latex(latex_expr, var_name) {
        Ok(result) => Ok(result),
        Err(e) => Err(JsValue::from_str(&format!("Error in integration: {}", e))),
    }
}

#[allow(unexpected_cfgs)]
#[wasm_bindgen]
pub fn definite_integral_js(
    latex_expr: &str,
    var_name: &str,
    lower: f64,
    upper: f64,
) -> Result<String, JsValue> {
    // Calculate the definite integral
    match definite_integral_latex(latex_expr, var_name, lower, upper) {
        Ok(result) => Ok(result),
        Err(e) => Err(JsValue::from_str(&format!(
            "Error in definite integration: {}",
            e
        ))),
    }
}

#[allow(unexpected_cfgs)]
#[wasm_bindgen]
pub fn taylor_series_js(
    latex_expr: &str,
    var_name: &str,
    center: f64,
    order: usize,
) -> Result<String, JsValue> {
    match taylor_series_latex(latex_expr, var_name, center, order) {
        Ok(result) => Ok(result),
        Err(e) => Err(JsValue::from_str(&format!(
            "Error computing Taylor series: {}",
            e
        ))),
    }
}

#[allow(unexpected_cfgs)]
#[wasm_bindgen]
pub fn limit_js(latex_expr: &str, var_name: &str, point: f64) -> Result<String, JsValue> {
    match limit_latex(latex_expr, var_name, point) {
        Ok(result) => Ok(result),
        Err(e) => Err(JsValue::from_str(&format!("Error computing limit: {}", e))),
    }
}

#[allow(unexpected_cfgs)]
#[wasm_bindgen]
pub fn evaluate_latex_expression_js(latex_expr: &str, env_json: &str) -> Result<String, JsValue> {
    // Deserialize the environment
    let env: Environment = serde_json::from_str(env_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse environment: {}", e)))?;

    // Special case for matrix multiplication with \cdot
    if latex_expr.contains("\\begin{pmatrix}")
        && latex_expr.contains("\\cdot")
        && latex_expr.contains("\\end{pmatrix}")
    {
        // Try to split the expression by \cdot
        let parts: Vec<&str> = latex_expr.split("\\cdot").collect();
        if parts.len() == 2 {
            let matrix_a = parts[0].trim();
            let matrix_b = parts[1].trim();

            // Try to perform matrix multiplication
            return matrix_multiply_js(matrix_a, matrix_b, env_json);
        }
    }

    // Special case for common summation notations in the frontend that might cause parsing issues
    if latex_expr.contains("\\sum_") && latex_expr.contains("^") {
        // Check for common patterns like \sum_{i=1}^3i^2 - unbraced with exponent
        if let Some(captures) = regex::Regex::new(r"\\sum_\{([a-z])=(\d+)\}\^(\d+)([a-z])\^(\d+)")
            .ok()
            .and_then(|re| re.captures(latex_expr))
        {
            // Extract the parts of the summation
            let var = captures.get(1).unwrap().as_str();
            let start = captures
                .get(2)
                .unwrap()
                .as_str()
                .parse::<i64>()
                .unwrap_or(1);
            let end = captures
                .get(3)
                .unwrap()
                .as_str()
                .parse::<i64>()
                .unwrap_or(10);
            let body_var = captures.get(4).unwrap().as_str();
            let exponent = captures
                .get(5)
                .unwrap()
                .as_str()
                .parse::<i64>()
                .unwrap_or(1);

            // Verify that the variable in the summation body matches the index variable
            if var == body_var {
                let mut sum = 0;
                for i in start..=end {
                    sum += i.pow(exponent as u32);
                }
                return Ok(sum.to_string());
            }
        }
    }

    if latex_expr.contains("\\prod_") && latex_expr.contains("^") {
        if let Some(captures) = regex::Regex::new(r"\\prod_\{([a-z])=(\d+)\}\^(\d+)([a-z])\^(\d+)")
            .ok()
            .and_then(|re| re.captures(latex_expr))
        {
            let var = captures.get(1).unwrap().as_str();
            let start = captures
                .get(2)
                .unwrap()
                .as_str()
                .parse::<i64>()
                .unwrap_or(1);
            let end = captures
                .get(3)
                .unwrap()
                .as_str()
                .parse::<i64>()
                .unwrap_or(10);
            let body_var = captures.get(4).unwrap().as_str();
            let exponent = captures
                .get(5)
                .unwrap()
                .as_str()
                .parse::<i64>()
                .unwrap_or(1);

            if var == body_var {
                let mut product = 1_i64;
                for i in start..=end {
                    product *= i.pow(exponent as u32);
                }
                return Ok(product.to_string());
            }
        }
    }

    // Create an instance of the Tokenizer
    let mut tokenizer = Tokenizer::new(latex_expr); // Pass input as a reference

    // Tokenize and parse the input
    let tokens = tokenizer.tokenize(); // Call the instance method on tokenizer
    let parsed_expr = build_expression_tree(tokens)
        .map_err(|e| JsValue::from_str(&format!("Error parsing LaTeX: {}", e)))?;

    // Check if it's an equation that we need to solve
    if let Node::Equation(left, right) = &parsed_expr {
        // First try to evaluate both sides
        let env_clone = env.clone();
        match (
            Evaluator::evaluate(left, &env_clone),
            Evaluator::evaluate(right, &env_clone),
        ) {
            (Ok(left_val), Ok(right_val)) => {
                if (left_val - right_val).abs() < 1e-9 {
                    return Ok(format!("Equation is true: {} = {}", left_val, right_val));
                } else {
                    return Ok(format!("Equation is false: {} ≠ {}", left_val, right_val));
                }
            }
            _ => {
                if let Some(var_name) = extract_variable(latex_expr) {
                    match crate::expression::solve_for_variable_exact(&parsed_expr, &var_name) {
                        Ok(solutions) => {
                            let parts: Vec<String> = solutions
                                .iter()
                                .map(|s| format!("{} = {}", var_name, s))
                                .collect();
                            return Ok(parts.join(", "));
                        }
                        Err(e) => {
                            if e.contains("summation") || e.contains("function") {
                                return Ok(format!("{}", parsed_expr));
                            }
                            return Err(JsValue::from_str(&format!(
                                "Error solving equation: {}",
                                e
                            )));
                        }
                    }
                }
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

#[allow(unexpected_cfgs)]
#[wasm_bindgen]
pub fn parse_matrix_js(latex_expr: &str, env_json: &str) -> Result<String, JsValue> {
    // Deserialize the environment
    let env: Environment = serde_json::from_str(env_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse environment: {}", e)))?;

    // Parse the matrix
    match parse_latex_matrix(latex_expr, &env) {
        Ok(matrix) => Ok(matrix.to_latex()),
        Err(e) => Err(JsValue::from_str(&format!("Error parsing matrix: {}", e))),
    }
}

#[allow(unexpected_cfgs)]
#[wasm_bindgen]
pub fn matrix_determinant_js(latex_expr: &str, env_json: &str) -> Result<String, JsValue> {
    // Deserialize the environment
    let env: Environment = serde_json::from_str(env_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse environment: {}", e)))?;

    // Parse the matrix
    let matrix = parse_latex_matrix(latex_expr, &env)
        .map_err(|e| JsValue::from_str(&format!("Error parsing matrix: {}", e)))?;

    // Calculate the determinant
    match matrix.determinant(&env) {
        Ok(det) => Ok(det.to_string()),
        Err(e) => Err(JsValue::from_str(&format!(
            "Error calculating determinant: {}",
            e
        ))),
    }
}

#[allow(unexpected_cfgs)]
#[wasm_bindgen]
pub fn matrix_inverse_js(latex_expr: &str, env_json: &str) -> Result<String, JsValue> {
    // Deserialize the environment
    let env: Environment = serde_json::from_str(env_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse environment: {}", e)))?;

    // Parse the matrix
    let matrix = parse_latex_matrix(latex_expr, &env)
        .map_err(|e| JsValue::from_str(&format!("Error parsing matrix: {}", e)))?;

    // Calculate the inverse
    match matrix.inverse(&env) {
        Ok(inv) => Ok(inv.to_latex()),
        Err(e) => Err(JsValue::from_str(&format!(
            "Error calculating inverse: {}",
            e
        ))),
    }
}

#[allow(unexpected_cfgs)]
#[wasm_bindgen]
pub fn matrix_multiply_js(
    matrix_a: &str,
    matrix_b: &str,
    env_json: &str,
) -> Result<String, JsValue> {
    // Deserialize the environment
    let env: Environment = serde_json::from_str(env_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse environment: {}", e)))?;

    // Parse the matrices
    let matrix_a = parse_latex_matrix(matrix_a, &env)
        .map_err(|e| JsValue::from_str(&format!("Error parsing matrix A: {}", e)))?;

    let matrix_b = parse_latex_matrix(matrix_b, &env)
        .map_err(|e| JsValue::from_str(&format!("Error parsing matrix B: {}", e)))?;

    // Multiply the matrices
    match matrix_a.multiply(&matrix_b, &env) {
        Ok(result) => Ok(result.to_latex()),
        Err(e) => Err(JsValue::from_str(&format!(
            "Error multiplying matrices: {}",
            e
        ))),
    }
}

#[allow(unexpected_cfgs)]
#[wasm_bindgen]
pub fn matrix_rank_js(latex_expr: &str, env_json: &str) -> Result<usize, JsValue> {
    // Deserialize the environment
    let env: Environment = serde_json::from_str(env_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse environment: {}", e)))?;

    // Parse the matrix
    let matrix = parse_latex_matrix(latex_expr, &env)
        .map_err(|e| JsValue::from_str(&format!("Error parsing matrix: {}", e)))?;

    // Calculate the rank
    match matrix.rank(&env) {
        Ok(rank) => Ok(rank),
        Err(e) => Err(JsValue::from_str(&format!("Error calculating rank: {}", e))),
    }
}

#[allow(unexpected_cfgs)]
#[wasm_bindgen]
pub fn matrix_eigenvalues_js(latex_expr: &str, env_json: &str) -> Result<String, JsValue> {
    // Deserialize the environment
    let env: Environment = serde_json::from_str(env_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse environment: {}", e)))?;

    // Parse the matrix
    let matrix = parse_latex_matrix(latex_expr, &env)
        .map_err(|e| JsValue::from_str(&format!("Error parsing matrix: {}", e)))?;

    // Calculate eigenvalues
    match matrix.eigenvalues(&env) {
        Ok(values) => {
            // Format the eigenvalues as a LaTeX array
            let values_str: Vec<String> = values.iter().map(|v| v.to_string()).collect();
            Ok(format!("\\lambda = \\{{{}}}", values_str.join(", ")))
        }
        Err(e) => Err(JsValue::from_str(&format!(
            "Error calculating eigenvalues: {}",
            e
        ))),
    }
}

#[allow(unexpected_cfgs)]
#[wasm_bindgen]
pub fn solve_linear_system_js(
    matrix_a: &str,
    vector_b: &str,
    env_json: &str,
) -> Result<String, JsValue> {
    // Deserialize the environment
    let env: Environment = serde_json::from_str(env_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse environment: {}", e)))?;

    // Parse the matrix and vector
    let matrix_a = parse_latex_matrix(matrix_a, &env)
        .map_err(|e| JsValue::from_str(&format!("Error parsing coefficient matrix: {}", e)))?;

    let vector_b = parse_latex_matrix(vector_b, &env)
        .map_err(|e| JsValue::from_str(&format!("Error parsing right-hand side vector: {}", e)))?;

    // Check if vector_b is actually a column vector
    if vector_b.cols != 1 {
        return Err(JsValue::from_str("Right-hand side must be a column vector"));
    }

    // Solve the system
    match matrix_a.solve(&vector_b, &env) {
        Ok(solution) => Ok(solution.to_latex()),
        Err(e) => Err(JsValue::from_str(&format!(
            "Error solving linear system: {}",
            e
        ))),
    }
}

#[allow(unexpected_cfgs)]
#[wasm_bindgen]
pub fn differentiate_js(latex_expr: &str, var_name: &str) -> Result<String, JsValue> {
    match differentiate_latex(latex_expr, var_name) {
        Ok(result) => Ok(result),
        Err(e) => Err(JsValue::from_str(&format!(
            "Error in differentiation: {}",
            e
        ))),
    }
}

#[allow(unexpected_cfgs)]
#[wasm_bindgen]
pub fn substitute_js(
    latex_expr: &str,
    var_name: &str,
    value_latex: &str,
) -> Result<String, JsValue> {
    let substitutions = vec![(var_name.to_string(), value_latex.to_string())];
    match substitute_latex(latex_expr, &substitutions) {
        Ok(result) => Ok(result),
        Err(e) => Err(JsValue::from_str(&format!("Error in substitution: {}", e))),
    }
}

#[allow(unexpected_cfgs)]
#[wasm_bindgen]
pub fn solve_js(latex_equation: &str, var_name: &str) -> Result<String, JsValue> {
    let mut tokenizer = Tokenizer::new(latex_equation);
    let tokens = tokenizer.tokenize();
    let expr = build_expression_tree(tokens)
        .map_err(|e| JsValue::from_str(&format!("Error parsing: {}", e)))?;
    match crate::expression::solve_for_variable_exact(&expr, var_name) {
        Ok(solutions) => {
            let parts: Vec<String> = solutions.iter().map(|s| format!("{}", s)).collect();
            Ok(parts.join(", "))
        }
        Err(e) => Err(JsValue::from_str(&format!("Error solving: {}", e))),
    }
}

#[allow(unexpected_cfgs)]
#[wasm_bindgen]
pub fn partial_fractions_js(latex_expr: &str, var_name: &str) -> Result<String, JsValue> {
    // Parse the expression and extract numerator/denominator from a Divide node
    let mut tokenizer = Tokenizer::new(latex_expr);
    let tokens = tokenizer.tokenize();
    let expr = build_expression_tree(tokens)
        .map_err(|e| JsValue::from_str(&format!("Error parsing: {}", e)))?;
    match expr {
        Node::Divide(num, den) => {
            let num_str = format!("{}", num);
            let den_str = format!("{}", den);
            match crate::partial_fractions_latex(&num_str, &den_str, var_name) {
                Ok(result) => Ok(result),
                Err(e) => Err(JsValue::from_str(&format!(
                    "Error in partial fractions: {}",
                    e
                ))),
            }
        }
        _ => Err(JsValue::from_str(
            "Expression must be a fraction (numerator/denominator)",
        )),
    }
}

#[allow(unexpected_cfgs)]
#[wasm_bindgen]
pub fn equivalent_js(expr1: &str, expr2: &str) -> Result<String, JsValue> {
    let env = Environment::new();
    let mut t1 = Tokenizer::new(expr1);
    let mut t2 = Tokenizer::new(expr2);
    let e1 = build_expression_tree(t1.tokenize())
        .map_err(|e| JsValue::from_str(&format!("Error parsing expr1: {}", e)))?;
    let e2 = build_expression_tree(t2.tokenize())
        .map_err(|e| JsValue::from_str(&format!("Error parsing expr2: {}", e)))?;
    let s1 = e1
        .simplify(&env)
        .map_err(|e| JsValue::from_str(&format!("Error simplifying expr1: {}", e)))?;
    let s2 = e2
        .simplify(&env)
        .map_err(|e| JsValue::from_str(&format!("Error simplifying expr2: {}", e)))?;
    let diff = Node::Subtract(Box::new(s1), Box::new(s2));
    let diff_simplified = diff
        .simplify(&env)
        .map_err(|e| JsValue::from_str(&format!("Error: {}", e)))?;
    if diff_simplified == Node::Num(ExactNum::zero()) {
        Ok("true".to_string())
    } else {
        Ok("false".to_string())
    }
}

#[allow(unexpected_cfgs)]
#[wasm_bindgen]
pub fn solve_ode_js(rhs_latex: &str, indep_var: &str, dep_var: &str) -> Result<String, JsValue> {
    match solve_ode_latex(rhs_latex, indep_var, dep_var) {
        Ok(result) => Ok(result),
        Err(e) => Err(JsValue::from_str(&format!("Error solving ODE: {}", e))),
    }
}
