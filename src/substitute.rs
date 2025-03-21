use crate::node::Node;
use crate::parser::build_expression_tree;
use crate::tokenizer::Tokenizer;

/// Substitutes variables in an expression with provided values or expressions
/// 
/// # Arguments
/// 
/// * `expr` - The expression in which to perform substitution
/// * `substitutions` - A mapping of variable names to their replacement expressions
/// 
/// # Returns
/// 
/// The expression with all substitutions applied
pub fn substitute(expr: &Node, substitutions: &[(String, Node)]) -> Result<Node, String> {
    let mut result = expr.clone();
    
    for (var_name, replacement) in substitutions {
        result = substitute_variable(&result, var_name, replacement)?;
    }
    
    Ok(result)
}

/// Substitutes a LaTeX expression with variables replaced by other expressions
///
/// # Arguments
///
/// * `latex_expr` - The LaTeX expression as a string
/// * `substitutions` - A vector of (variable_name, replacement_latex) pairs
///
/// # Returns
///
/// The substituted expression as a string in LaTeX format
pub fn substitute_latex(latex_expr: &str, substitutions: &[(String, String)]) -> Result<String, String> {
    // Parse the input expression
    let mut tokenizer = Tokenizer::new(latex_expr);
    let tokens = tokenizer.tokenize();
    let expr = build_expression_tree(tokens)?;
    
    // Parse each replacement expression
    let mut parsed_substitutions = Vec::new();
    for (var_name, replacement_latex) in substitutions {
        let mut replacement_tokenizer = Tokenizer::new(replacement_latex);
        let replacement_tokens = replacement_tokenizer.tokenize();
        let replacement_expr = build_expression_tree(replacement_tokens)?;
        parsed_substitutions.push((var_name.clone(), replacement_expr));
    }
    
    // Perform the substitution
    let result = substitute(&expr, &parsed_substitutions)?;
    
    // Convert back to LaTeX
    Ok(format!("{}", result))
}

/// Substitute a variable in an expression with the provided value
pub fn substitute_variable(node: &Node, var_name: &str, value: &Node) -> Result<Node, String> {
    match node {
        Node::Number(_) => Ok(node.clone()),
        Node::Variable(name) => {
            if name == var_name {
                Ok(value.clone())
            } else {
                Ok(node.clone())
            }
        },
        Node::Rational(_, _) => Ok(node.clone()),
        Node::ClosingParen | Node::ClosingBrace => Ok(node.clone()),
        
        Node::Add(left, right) => {
            let left_subst = substitute_variable(left, var_name, value)?;
            let right_subst = substitute_variable(right, var_name, value)?;
            Ok(Node::Add(Box::new(left_subst), Box::new(right_subst)))
        },
        Node::Subtract(left, right) => {
            let left_subst = substitute_variable(left, var_name, value)?;
            let right_subst = substitute_variable(right, var_name, value)?;
            Ok(Node::Subtract(Box::new(left_subst), Box::new(right_subst)))
        },
        Node::Multiply(left, right) => {
            let left_subst = substitute_variable(left, var_name, value)?;
            let right_subst = substitute_variable(right, var_name, value)?;
            Ok(Node::Multiply(Box::new(left_subst), Box::new(right_subst)))
        },
        Node::Divide(left, right) => {
            let left_subst = substitute_variable(left, var_name, value)?;
            let right_subst = substitute_variable(right, var_name, value)?;
            Ok(Node::Divide(Box::new(left_subst), Box::new(right_subst)))
        },
        Node::Power(base, exponent) => {
            let base_subst = substitute_variable(base, var_name, value)?;
            let exp_subst = substitute_variable(exponent, var_name, value)?;
            Ok(Node::Power(Box::new(base_subst), Box::new(exp_subst)))
        },
        Node::Sqrt(operand) => {
            let operand_subst = substitute_variable(operand, var_name, value)?;
            Ok(Node::Sqrt(Box::new(operand_subst)))
        },
        Node::Abs(operand) => {
            let operand_subst = substitute_variable(operand, var_name, value)?;
            Ok(Node::Abs(Box::new(operand_subst)))
        },
        Node::Negate(operand) => {
            let operand_subst = substitute_variable(operand, var_name, value)?;
            Ok(Node::Negate(Box::new(operand_subst)))
        },
        
        Node::Greater(left, right) => {
            let left_subst = substitute_variable(left, var_name, value)?;
            let right_subst = substitute_variable(right, var_name, value)?;
            Ok(Node::Greater(Box::new(left_subst), Box::new(right_subst)))
        },
        Node::Less(left, right) => {
            let left_subst = substitute_variable(left, var_name, value)?;
            let right_subst = substitute_variable(right, var_name, value)?;
            Ok(Node::Less(Box::new(left_subst), Box::new(right_subst)))
        },
        Node::GreaterEqual(left, right) => {
            let left_subst = substitute_variable(left, var_name, value)?;
            let right_subst = substitute_variable(right, var_name, value)?;
            Ok(Node::GreaterEqual(Box::new(left_subst), Box::new(right_subst)))
        },
        Node::LessEqual(left, right) => {
            let left_subst = substitute_variable(left, var_name, value)?;
            let right_subst = substitute_variable(right, var_name, value)?;
            Ok(Node::LessEqual(Box::new(left_subst), Box::new(right_subst)))
        },
        Node::Equal(left, right) => {
            let left_subst = substitute_variable(left, var_name, value)?;
            let right_subst = substitute_variable(right, var_name, value)?;
            Ok(Node::Equal(Box::new(left_subst), Box::new(right_subst)))
        },
        Node::Equation(left, right) => {
            let left_subst = substitute_variable(left, var_name, value)?;
            let right_subst = substitute_variable(right, var_name, value)?;
            Ok(Node::Equation(Box::new(left_subst), Box::new(right_subst)))
        },
        
        Node::Piecewise(conditions) => {
            let mut new_conditions = Vec::new();
            for (expr, cond) in conditions {
                let expr_subst = substitute_variable(expr, var_name, value)?;
                let cond_subst = substitute_variable(cond, var_name, value)?;
                new_conditions.push((expr_subst, cond_subst));
            }
            Ok(Node::Piecewise(new_conditions))
        },
        
        Node::Summation(index, start, end, body) => {
            // If the summation uses the same index variable, don't substitute in the body
            // to avoid variable capture issues
            if index == var_name {
                let start_subst = substitute_variable(start, var_name, value)?;
                let end_subst = substitute_variable(end, var_name, value)?;
                Ok(Node::Summation(
                    index.clone(),
                    Box::new(start_subst),
                    Box::new(end_subst),
                    body.clone()
                ))
            } else {
                let start_subst = substitute_variable(start, var_name, value)?;
                let end_subst = substitute_variable(end, var_name, value)?;
                let body_subst = substitute_variable(body, var_name, value)?;
                Ok(Node::Summation(
                    index.clone(),
                    Box::new(start_subst),
                    Box::new(end_subst),
                    Box::new(body_subst)
                ))
            }
        },
        
        Node::Function(name, args) => {
            let mut new_args = Vec::new();
            for arg in args {
                let arg_subst = substitute_variable(arg, var_name, value)?;
                new_args.push(arg_subst);
            }
            Ok(Node::Function(name.clone(), new_args))
        }
    }
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
    fn test_basic_substitution() {
        // Test x + y with x = 3
        let expr = parse_expression("x + y").unwrap();
        let replacement = parse_expression("3").unwrap();
        
        let result = substitute(&expr, &[("x".to_string(), replacement)]).unwrap();
        let expected = parse_expression("3 + y").unwrap();
        
        assert_eq!(format!("{}", result), format!("{}", expected));
    }
    
    #[test]
    fn test_complex_substitution() {
        // Test x^2 + 2*x + 1 with x = y + 1
        let expr = parse_expression("x^2 + 2*x + 1").unwrap();
        let replacement = parse_expression("y + 1").unwrap();
        
        let result = substitute(&expr, &[("x".to_string(), replacement)]).unwrap();
        
        // Should simplify to (y+1)^2 + 2(y+1) + 1 = y^2 + 2y + 1 + 2y + 2 + 1 = y^2 + 4y + 4
        // We won't check for full simplification here, just proper substitution
        let env = Environment::new();
        let eval_at_y2 = |node: &Node| {
            let mut test_env = env.clone();
            test_env.set("y", 2.0);
            Evaluator::evaluate(node, &test_env).unwrap()
        };
        
        // For y = 2: (2+1)^2 + 2(2+1) + 1 = 9 + 6 + 1 = 16
        assert_eq!(eval_at_y2(&result), 16.0);
    }
    
    #[test]
    fn test_multiple_substitutions() {
        // Test a*x + b*y with a = 2, b = 3, y = z^2
        let expr = parse_expression("a*x + b*y").unwrap();
        let a_replacement = parse_expression("2").unwrap();
        let b_replacement = parse_expression("3").unwrap();
        let y_replacement = parse_expression("z^2").unwrap();
        
        let result = substitute(&expr, &[
            ("a".to_string(), a_replacement),
            ("b".to_string(), b_replacement),
            ("y".to_string(), y_replacement)
        ]).unwrap();
        
        let expected = parse_expression("2*x + 3*z^2").unwrap();
        assert_eq!(format!("{}", result), format!("{}", expected));
    }
    
    #[test]
    fn test_substitution_in_function() {
        // Test sin(x + y) with x = π/2
        let expr = parse_expression("\\sin{x + y}").unwrap();
        let replacement = parse_expression("\\pi/2").unwrap();
        
        let result = substitute(&expr, &[("x".to_string(), replacement)]).unwrap();
        let expected = parse_expression("\\sin{\\pi/2 + y}").unwrap();
        
        assert_eq!(format!("{}", result), format!("{}", expected));
        
        // Test with evaluation
        let mut env = Environment::new();
        env.set("y", 0.0);
        
        let eval_result = Evaluator::evaluate(&result, &env).unwrap();
        assert!((eval_result - 1.0).abs() < 1e-10);
    }
    
    #[test]
    fn test_substitution_in_summation() {
        // Test Σ_{i=a}^{b} (i+c) with a = 1, b = n, c = 2
        let expr = parse_expression("\\sum_{i=a}^{b} {i+c}").unwrap();
        let a_replacement = parse_expression("1").unwrap();
        let b_replacement = parse_expression("n").unwrap();
        let c_replacement = parse_expression("2").unwrap();
        
        let result = substitute(&expr, &[
            ("a".to_string(), a_replacement),
            ("b".to_string(), b_replacement),
            ("c".to_string(), c_replacement)
        ]).unwrap();
        
        let expected = parse_expression("\\sum_{i=1}^{n} {i+2}").unwrap();
        assert_eq!(format!("{}", result), format!("{}", expected));
        
        // Test with evaluation
        let mut env = Environment::new();
        env.set("n", 3.0);
        
        // Sum from i=1 to i=3 of (i+2) = (1+2) + (2+2) + (3+2) = 3 + 4 + 5 = 12
        let eval_result = Evaluator::evaluate(&result, &env).unwrap();
        assert_eq!(eval_result, 12.0);
    }
    
    #[test]
    fn test_latex_substitution() {
        // Test "ax^2 + bx + c" with a = 1, b = 2, c = 3
        let result = substitute_latex(
            "a*x^2 + b*x + c", 
            &[
                ("a".to_string(), "1".to_string()),
                ("b".to_string(), "2".to_string()),
                ("c".to_string(), "3".to_string())
            ]
        ).unwrap();
        
        // The result might have different formatting (spaces, etc.)
        // So we parse back and compare with a known expression
        let result_expr = parse_expression(&result).unwrap();
        let mut env = Environment::new();
        env.set("x", 2.0);
        
        let eval_result = Evaluator::evaluate(&result_expr, &env).unwrap();
        // For x = 2: 1(2)^2 + 2(2) + 3 = 4 + 4 + 3 = 11
        assert_eq!(eval_result, 11.0);
    }
}