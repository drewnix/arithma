use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};

pub struct Environment {
    vars: HashMap<String, f64>,
}

impl Environment {
    pub fn new() -> Self {
        Environment {
            vars: HashMap::new(),
        }
    }

    pub fn get(&self, var: &str) -> Option<f64> {
        self.vars.get(var).cloned()
    }

    pub fn set(&mut self, var: &str, value: f64) {
        self.vars.insert(var.to_string(), value);
    }
}

impl Node {
    pub fn evaluate(&self, env: &Environment) -> Result<f64, String> {
        match self {
            Node::Number(n) => Ok(*n),
            Node::Variable(ref var) => {
                if let Some(val) = env.get(var) {
                    Ok(val)
                } else {
                    Err(format!("Variable '{}' is not defined.", var))
                }
            }
            Node::Add(left, right) => {
                let left_val = left.evaluate(env)?;
                let right_val = right.evaluate(env)?;
                Ok(left_val + right_val)
            }
            Node::Subtract(left, right) => {
                let left_val = left.evaluate(env)?;
                let right_val = right.evaluate(env)?;
                Ok(left_val - right_val)
            }
            Node::Multiply(left, right) => {
                let left_val = left.evaluate(env)?;
                let right_val = right.evaluate(env)?;
                Ok(left_val * right_val)
            }
            Node::Divide(left, right) => {
                let left_val = left.evaluate(env)?;
                let right_val = right.evaluate(env)?;
                if right_val == 0.0 {
                    Err("Division by zero.".to_string())
                } else {
                    Ok(left_val / right_val)
                }
            }
            Node::Power(left, right) => {
                let left_val = left.evaluate(env)?;
                let right_val = right.evaluate(env)?;
                Ok(left_val.powf(right_val))
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Node {
    // Leaf nodes: numbers or variables
    Number(f64),
    Variable(String),

    // Internal nodes: operators with children (operands)
    Add(Box<Node>, Box<Node>),
    Subtract(Box<Node>, Box<Node>),
    Multiply(Box<Node>, Box<Node>),
    Divide(Box<Node>, Box<Node>),
    Power(Box<Node>, Box<Node>),
}


pub fn build_expression_tree(tokens: Vec<String>) -> Result<Node, String> {
    let rpn = shunting_yard(tokens)?;

    let mut stack: Vec<Node> = Vec::new();

    for token in rpn {
        if let Ok(num) = token.parse::<f64>() {
            stack.push(Node::Number(num));
        } else if token.chars().all(char::is_alphabetic) {
            stack.push(Node::Variable(token));
        } else if "+-*/^".contains(&token) {
            let right = stack.pop().ok_or_else(|| format!("Not enough operands for operator '{}'", token))?;
            let left = stack.pop().ok_or_else(|| format!("Not enough operands for operator '{}'", token))?;

            let node = match token.as_str() {
                "+" => Node::Add(Box::new(left), Box::new(right)),
                "-" => Node::Subtract(Box::new(left), Box::new(right)),
                "*" => Node::Multiply(Box::new(left), Box::new(right)),
                "/" => Node::Divide(Box::new(left), Box::new(right)),
                "^" => Node::Power(Box::new(left), Box::new(right)),
                _ => return Err(format!("Unknown operator '{}'", token)),
            };

            stack.push(node);
        } else {
            return Err(format!("Unknown token '{}'", token));
        }
    }

    if stack.len() != 1 {
        return Err("The expression did not resolve into a single tree.".to_string());
    }

    Ok(stack.pop().unwrap())
}


pub fn get_precedence(op: &str) -> i32 {
    match op {
        "+" | "-" => 1,
        "*" | "/" => 2,
        "^" => 3, // Exponentiation
        _ => 0,
    }
}

pub fn is_right_associative(op: &str) -> bool {
    match op {
        "^" => true, // Exponentiation is right-associative
        _ => false,
    }
}

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
pub fn solve_for_variable_js(expr_json: &str, right_val: f64, target_var: &str) -> Result<JsValue, JsValue> {
    // Deserialize the JSON input into a Node
    let expr: Node = serde_json::from_str(expr_json).map_err(|e| JsValue::from_str(&format!("Failed to parse expression: {}", e)))?;

    // Call the original solve_for_variable function
    match solve_for_variable(&expr, right_val, target_var) {
        Ok(result) => Ok(JsValue::from_f64(result)),  // Return the result as a JsValue (f64)
        Err(e) => Err(JsValue::from_str(&e)),  // Return the error as a JsValue (String)
    }
}

pub fn solve_for_variable(expr: &Node, right_val: f64, target_var: &str) -> Result<f64, String> {
    let mut coefficient = 0.0; // Coefficient of the target variable
    let mut constant = 0.0;    // Constant part to move to the other side

    fn traverse(
        node: &Node,
        target_var: &str,
        coefficient: &mut f64,
        constant: &mut f64,
        multiplier: f64,  // Apply multiplier for cases like (x + 2) * 3
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
                traverse(left, target_var, coefficient, constant, multiplier)?;  // Traverse left side
                traverse(right, target_var, coefficient, constant, multiplier)?; // Traverse right side
            }
            Node::Subtract(left, right) => {
                traverse(left, target_var, coefficient, constant, multiplier)?;   // Traverse left side
                traverse(right, target_var, coefficient, constant, -multiplier)?; // Apply negation to the right
            }
            Node::Multiply(left, right) => {
                if let Node::Number(num) = **left {
                    // If the left node is a number, it's a multiplier for the right side
                    traverse(right, target_var, coefficient, constant, multiplier * num)?;
                } else if let Node::Number(num) = **right {
                    // If the right node is a number, it's a multiplier for the left side
                    traverse(left, target_var, coefficient, constant, multiplier * num)?;
                } else {
                    return Err("Expected one operand to be a number in multiplication.".to_string());
                }
            }
            Node::Divide(left, right) => {
                let mut right_constant = 0.0;
                traverse(right, target_var, coefficient, &mut right_constant, 1.0)?;
                if right_constant == 0.0 {
                    return Err("Division by zero".to_string());
                }
                traverse(left, target_var, coefficient, constant, multiplier / right_constant)?;
            }
            _ => return Err("Unexpected node in expression.".to_string()), // Handle any other unexpected node
        }
        Ok(())
    }

    // Start the traversal with a multiplier of 1.0
    traverse(expr, target_var, &mut coefficient, &mut constant, 1.0)?;

    if coefficient == 0.0 {
        return Err(format!("Coefficient of variable '{}' is zero, can't solve.", target_var));
    }

    // Solve for the variable: right_val = coefficient * variable + constant
    // Adjust the equation: variable = (right_val - constant) / coefficient
    let result = (right_val - constant) / coefficient;
    Ok(result)
}

pub fn tokenize(expr: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let mut last_was_operator_or_paren = true;  // Track if last token was an operator or open parenthesis

    for c in expr.chars() {
        if c.is_whitespace() {
            continue; // Skip whitespace
        } else if c.is_digit(10) || c == '.' {
            current_token.push(c); // Build a number token
            last_was_operator_or_paren = false;
        } else if c.is_alphabetic() {
            // Handle variable names
            current_token.push(c);
            last_was_operator_or_paren = false;
        } else {
            if !current_token.is_empty() {
                tokens.push(current_token.clone());
                current_token.clear();
            }

            if "+*/^()=".contains(c) {
                tokens.push(c.to_string()); // Push operator or parentheses
                last_was_operator_or_paren = c == '(' || "+*/^=".contains(c);  // Set flag based on operator or open parenthesis
            } else if c == '-' {
                // If the previous token was an operator or opening parenthesis, treat '-' as unary
                if last_was_operator_or_paren {
                    tokens.push("u-".to_string()); // Unary minus
                } else {
                    tokens.push("-".to_string()); // Binary minus
                }
                last_was_operator_or_paren = true; // After '-' we expect a number or expression
            }
        }
    }

    // Push the last token if any
    if !current_token.is_empty() {
        tokens.push(current_token);
    }

    tokens
}

pub fn shunting_yard(tokens: Vec<String>) -> Result<Vec<String>, String> {
    let mut output_queue: Vec<String> = Vec::new();
    let mut operator_stack: Vec<String> = Vec::new();

    let mut iter = tokens.into_iter().peekable();

    while let Some(token) = iter.next() {
        if token.parse::<f64>().is_ok() || token.chars().all(char::is_alphabetic) {
            // Token is a number or a variable, push to output queue
            output_queue.push(token);
        } else if token == "u-" {
            // Apply unary minus directly to the next number in the token stream
            if let Some(next_token) = iter.peek() {
                if next_token.parse::<f64>().is_ok() {
                    let negated_value = format!("-{}", iter.next().unwrap());
                    output_queue.push(negated_value);
                } else {
                    return Err("Unary minus must be followed by a number.".to_string());
                }
            } else {
                return Err("Unary minus must be followed by a number.".to_string());
            }
        } else if "+-*/^".contains(&token) {
            // Token is a binary operator
            while let Some(op) = operator_stack.last() {
                if "+-*/^".contains(op) &&
                   ((is_right_associative(&token) && get_precedence(op) >= get_precedence(&token)) ||
                   (!is_right_associative(&token) && get_precedence(op) > get_precedence(&token))) {
                    output_queue.push(operator_stack.pop().unwrap());
                } else {
                    break;
                }
            }
            operator_stack.push(token);
        } else if token == "(" {
            // Push the opening parenthesis to the operator stack
            operator_stack.push(token);
        } else if token == ")" {
            // Pop operators from the stack to the output queue until we find an opening parenthesis
            let mut found_left_paren = false;
            while let Some(op) = operator_stack.pop() {
                if op == "(" {
                    found_left_paren = true;
                    break;
                } else {
                    output_queue.push(op);
                }
            }
            if !found_left_paren {
                return Err("Mismatched parentheses: extra closing parenthesis found.".to_string());
            }
        }
    }

    // After processing all tokens, pop any remaining operators to the output queue
    while let Some(op) = operator_stack.pop() {
        if op == "(" {
            return Err("Mismatched parentheses: unclosed opening parenthesis.".to_string());
        }
        output_queue.push(op);
    }

    Ok(output_queue)
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

