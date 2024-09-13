use std::collections::HashMap;

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

pub fn solve_for_variable(left_expr: &str, right_val: f64, env: &Environment) -> Result<f64, String> {
    // Tokenize and parse the left-hand side of the equation
    let left_tokens = tokenize(left_expr);
    let left_rpn = shunting_yard(left_tokens)?;

    // Stack to simulate solving for the variable
    let mut stack: Vec<f64> = Vec::new();
    let mut pending_operator: Option<String> = None;
    
    for token in left_rpn {
        if let Ok(num) = token.parse::<f64>() {
            // Push constant numbers directly to the stack
            stack.push(num);
        } else if env.get(&token).is_some() {
            // Ignore the variable for now, solve for it later
            continue;
        } else if "+-*/".contains(&token) {
            // Handle operators for solving the equation
            pending_operator = Some(token);
        }
    }

    if let Some(operator) = pending_operator {
        let value = stack.pop().ok_or_else(|| "Missing value to solve for".to_string())?;
        match operator.as_str() {
            "+" => return Ok(right_val - value),
            "-" => return Ok(right_val + value),
            "*" => return Ok(right_val / value),
            "/" => return Ok(right_val * value),
            _ => return Err("Unsupported operator".to_string()),
        }
    }

    Err("Unable to solve for variable".to_string())
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
            // Push number to stack
            stack.push(num);
        } else if let Some(value) = env.get(&token) {
            // If token is a variable, resolve it and push its value to the stack
            stack.push(value);
        } else if "+-*/^".contains(&token) {
            // Handle binary operators
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
                        return Err("Division by zero error.".to_string());
                    }
                    stack.push(left / right);
                }
                "^" => stack.push(left.powf(right)),
                _ => return Err(format!("Unexpected operator '{}'", token)),
            }
        } else {
            // If the token is neither a number, operator, nor a known variable, return an error
            return Err(format!("Unexpected token '{}'", token));
        }
    }

    // Check if exactly one value is left on the stack, which is the result
    if stack.len() == 1 {
        Ok(stack.pop().unwrap())
    } else {
        Err("Malformed expression: too many operands.".to_string())
    }
}
