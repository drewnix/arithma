use crate::node::Node;
use crate::parser::build_expression_tree;
use crate::tokenizer::Tokenizer;
use crate::substitute::substitute_variable;

/// Composes two functions f and g to create f(g(x))
///
/// # Arguments
///
/// * `f` - The outer function expression with a designated variable
/// * `f_var` - The variable in f to be replaced with g(x)
/// * `g` - The inner function expression
///
/// # Returns
///
/// The composed function expression f(g(x))
pub fn compose(f: &Node, f_var: &str, g: &Node) -> Result<Node, String> {
    // Substitute g for the variable in f
    substitute_variable(f, f_var, g)
}

/// Composes two functions represented as LaTeX expressions: f(g(x))
///
/// # Arguments
///
/// * `f_latex` - LaTeX string for the outer function with a designated variable
/// * `f_var` - The variable in f to be replaced with g(x)
/// * `g_latex` - LaTeX string for the inner function
///
/// # Returns
///
/// The composed function expression as a LaTeX string
pub fn compose_latex(f_latex: &str, f_var: &str, g_latex: &str) -> Result<String, String> {
    // Parse the outer function expression
    let mut f_tokenizer = Tokenizer::new(f_latex);
    let f_tokens = f_tokenizer.tokenize();
    let f_expr = build_expression_tree(f_tokens)?;
    
    // Parse the inner function expression
    let mut g_tokenizer = Tokenizer::new(g_latex);
    let g_tokens = g_tokenizer.tokenize();
    let g_expr = build_expression_tree(g_tokens)?;
    
    // Perform the composition
    let result = compose(&f_expr, f_var, &g_expr)?;
    
    // Convert back to LaTeX
    Ok(format!("{}", result))
}

/// Checks if a composition is valid (all referenced variables in g are available)
///
/// # Arguments
///
/// * `f` - The outer function expression
/// * `f_var` - The variable in f to be replaced
/// * `g` - The inner function expression
/// * `available_vars` - The available variables that can be used in the composition
///
/// # Returns
///
/// A Result indicating whether the composition is valid
pub fn validate_composition(f: &Node, f_var: &str, g: &Node, available_vars: &[String]) -> Result<(), String> {
    // Collect variables used in g
    let mut g_vars = Vec::new();
    collect_variables(g, &mut g_vars);
    
    // Check if all variables in g are available
    for var in g_vars {
        if var != f_var && !available_vars.contains(&var) {
            return Err(format!("Variable '{}' used in the inner function is not available", var));
        }
    }
    
    Ok(())
}

/// Helper function to collect variables from an expression
fn collect_variables(node: &Node, vars: &mut Vec<String>) {
    match node {
        Node::Variable(name) => {
            if !vars.contains(name) {
                vars.push(name.clone());
            }
        }
        Node::Add(left, right) | Node::Subtract(left, right) | Node::Multiply(left, right) |
        Node::Divide(left, right) | Node::Power(left, right) | Node::Greater(left, right) |
        Node::Less(left, right) | Node::GreaterEqual(left, right) | Node::LessEqual(left, right) |
        Node::Equal(left, right) | Node::Equation(left, right) => {
            collect_variables(left, vars);
            collect_variables(right, vars);
        }
        Node::Sqrt(operand) | Node::Abs(operand) | Node::Negate(operand) => {
            collect_variables(operand, vars);
        }
        Node::Piecewise(conditions) => {
            for (expr, cond) in conditions {
                collect_variables(expr, vars);
                collect_variables(cond, vars);
            }
        }
        Node::Summation(index, start, end, body) => {
            collect_variables(start, vars);
            collect_variables(end, vars);
            collect_variables(body, vars);
        }
        Node::Function(_, args) => {
            for arg in args {
                collect_variables(arg, vars);
            }
        }
        // Other node types don't contain variables
        _ => {}
    }
}

/// Creates a new function by forming the composition of multiple functions
///
/// # Arguments
///
/// * `functions` - A vector of (function_expr, variable) pairs, where each function
///                 will be composed with the next one.
///
/// # Returns
///
/// The composed function expression
pub fn compose_multiple(functions: &[(Node, String)]) -> Result<Node, String> {
    if functions.is_empty() {
        return Err("Cannot compose empty list of functions".to_string());
    }
    
    // Begin with the innermost function
    let mut result = functions[0].0.clone();
    
    // Apply composition in correct order
    for i in 0..functions.len() - 1 {
        // Get the current outer function and its variable
        let var = &functions[i].1;
        let outer_func = &functions[i + 1].0;
        
        // Compose: outer_func(result) where result is the innermost function composed so far
        result = compose(outer_func, var, &result)?;
    }
    
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluator::Evaluator;
    use crate::Environment;
    
    fn parse_expression(latex: &str) -> Result<Node, String> {
        let mut tokenizer = Tokenizer::new(latex);
        let tokens = tokenizer.tokenize();
        build_expression_tree(tokens)
    }
    
    #[test]
    fn test_basic_composition() {
        // f(x) = x^2
        // g(x) = x + 1
        // f(g(x)) = (x + 1)^2
        let f = parse_expression("x^2").unwrap();
        let g = parse_expression("x + 1").unwrap();
        
        let result = compose(&f, "x", &g).unwrap();
        
        // Create an environment for evaluation
        let mut env = Environment::new();
        env.set("x", 2.0);
        
        // Evaluate f(g(2)) = (2 + 1)^2 = 3^2 = 9
        let eval_result = Evaluator::evaluate(&result, &env).unwrap();
        assert_eq!(eval_result, 9.0);
    }
    
    #[test]
    fn test_function_composition() {
        // f(x) = sin(x)
        // g(x) = pi/2
        // f(g(x)) = sin(pi/2) = 1
        let f = parse_expression("\\sin{x}").unwrap();
        let g = parse_expression("\\pi/2").unwrap();
        
        let result = compose(&f, "x", &g).unwrap();
        
        // Evaluate sin(pi/2) = 1
        let env = Environment::new();
        let eval_result = Evaluator::evaluate(&result, &env).unwrap();
        assert!((eval_result - 1.0).abs() < 1e-10);
    }
    
    #[test]
    fn test_latex_composition() {
        // f(x) = x^2 + 1
        // g(x) = 2x
        // f(g(x)) = (2x)^2 + 1 = 4x^2 + 1
        let result = compose_latex("x^2 + 1", "x", "2*x").unwrap();
        
        // Create an environment for evaluation
        let mut env = Environment::new();
        env.set("x", 3.0);
        
        // Evaluate f(g(3)) = (2*3)^2 + 1 = 6^2 + 1 = 36 + 1 = 37
        let result_expr = parse_expression(&result).unwrap();
        let eval_result = Evaluator::evaluate(&result_expr, &env).unwrap();
        assert_eq!(eval_result, 37.0);
    }
    
    #[test]
    fn test_nested_composition() {
        // f(x) = x^2
        // g(x) = x + 1
        // h(x) = 2x
        // f(g(h(x))) = ((2x) + 1)^2 = (2x + 1)^2
        let f = parse_expression("x^2").unwrap();
        let g = parse_expression("x + 1").unwrap();
        let h = parse_expression("2*x").unwrap();
        
        // Create the compositions
        let g_of_h = compose(&g, "x", &h).unwrap(); // g(h(x)) = (2x) + 1
        let f_of_g_of_h = compose(&f, "x", &g_of_h).unwrap(); // f(g(h(x))) = ((2x) + 1)^2
        
        // Create an environment for evaluation
        let mut env = Environment::new();
        env.set("x", 2.0);
        
        // Evaluate f(g(h(2))) = ((2*2) + 1)^2 = (4 + 1)^2 = 5^2 = 25
        let eval_result = Evaluator::evaluate(&f_of_g_of_h, &env).unwrap();
        assert_eq!(eval_result, 25.0);
    }
    
    #[test]
    fn test_multivariable_composition() {
        // f(x) = a*x + b
        // g(t) = t^2
        // f(g(t)) = a*(t^2) + b
        let f = parse_expression("a*x + b").unwrap();
        let g = parse_expression("t^2").unwrap();
        
        let result = compose(&f, "x", &g).unwrap();
        
        // Create an environment for evaluation
        let mut env = Environment::new();
        env.set("a", 2.0);
        env.set("b", 3.0);
        env.set("t", 4.0);
        
        // Evaluate f(g(4)) where f(x) = 2x + 3 and g(t) = t^2
        // = 2*(4^2) + 3 = 2*16 + 3 = 32 + 3 = 35
        let eval_result = Evaluator::evaluate(&result, &env).unwrap();
        assert_eq!(eval_result, 35.0);
    }
    
    #[test]
    fn test_complex_composition() {
        // f(x) = sqrt(x)
        // g(x) = x^2 + 1
        // f(g(x)) = sqrt(x^2 + 1)
        let f = parse_expression("\\sqrt{x}").unwrap();
        let g = parse_expression("x^2 + 1").unwrap();
        
        let result = compose(&f, "x", &g).unwrap();
        
        // Create an environment for evaluation
        let mut env = Environment::new();
        env.set("x", 3.0);
        
        // Evaluate f(g(3)) = sqrt(3^2 + 1) = sqrt(10) ≈ 3.16227...
        let eval_result = Evaluator::evaluate(&result, &env).unwrap();
        let expected = (3.0_f64.powi(2) + 1.0).sqrt();
        assert!((eval_result - expected).abs() < 1e-10);
    }
    
    #[test]
    fn test_validation() {
        // f(x) = x + y
        // g(t) = t^2
        // Available variables: y, t
        let f = parse_expression("x + y").unwrap();
        let g = parse_expression("t^2").unwrap();
        
        let available_vars = vec!["y".to_string(), "t".to_string()];
        let valid = validate_composition(&f, "x", &g, &available_vars);
        
        assert!(valid.is_ok());
        
        // Now try with a variable that's not available
        let g_invalid = parse_expression("t^2 + z").unwrap();
        let invalid = validate_composition(&f, "x", &g_invalid, &available_vars);
        
        assert!(invalid.is_err());
    }
    
    #[test]
    fn test_multiple_composition() {
        // Chain of functions:
        // f(x) = x^2
        // g(y) = y + 1
        // h(z) = 2*z
        // Compose: f(g(h(w))) = ((2*w) + 1)^2
        
        let f = parse_expression("x^2").unwrap();
        let g = parse_expression("y + 1").unwrap();
        let h = parse_expression("2*z").unwrap();
        
        let functions = vec![
            (h, "z".to_string()),  // First apply h(z)
            (g, "y".to_string()),  // Then apply g(y)
            (f, "x".to_string()),  // Finally apply f(x)
        ];
        
        let result = compose_multiple(&functions).unwrap();
        
        // Create an environment for evaluation
        let mut env = Environment::new();
        env.set("w", 2.0);
        
        // Evaluate f(g(h(2))) = ((2*2) + 1)^2 = (4 + 1)^2 = 5^2 = 25
        let eval_result = Evaluator::evaluate(&result, &env).unwrap();
        assert_eq!(eval_result, 25.0);
    }
}