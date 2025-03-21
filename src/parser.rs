use crate::functions::FUNCTION_REGISTRY;
use crate::node::Node;

pub fn shunting_yard(tokens: Vec<String>) -> Result<Vec<String>, String> {
    log::debug!("Starting Shunting Yard with tokens: {:?}", tokens);

    let mut output_queue: Vec<String> = Vec::new();
    let mut operator_stack: Vec<String> = Vec::new();
    let mut function_brace_stack: Vec<String> = Vec::new(); // Stack to track function-specific braces

    for token in tokens {
        log::debug!("Processing token: {}", token);

        if token.parse::<f64>().is_ok() {
            // If token is a number, add it directly to the output queue
            log::debug!("Token is a number: {}", token);
            output_queue.push(token);
        } else if token == "NEG" {
            // Handle unary minus: push it to operator stack with precedence handling
            log::debug!("Unary minus detected, pushing to operator stack");
            operator_stack.push(token); // No need to pop for precedence because it's unary
        } else if token == "ABS_START" {
            operator_stack.push(token); // Treat absolute value as a grouping operation
        } else if token == "ABS_END" {
            while let Some(op) = operator_stack.pop() {
                if op == "ABS_START" {
                    break; // Close the absolute value group
                }
                output_queue.push(op);
            }
            output_queue.push("ABS".to_string()); // Add ABS function to the output
        } else if token == ">" || token == "<" || token == ">=" || token == "<=" || token == "==" || token == "=" {
            while let Some(top) = operator_stack.last() {
                if get_precedence(top) >= get_precedence(&token) {
                    output_queue.push(operator_stack.pop().unwrap());
                } else {
                    break;
                }
            }
            operator_stack.push(token);
        } else if "+-*/^".contains(&token) {
            // Handle binary operators with precedence and associativity
            while let Some(top) = operator_stack.last() {
                if get_precedence(top) >= get_precedence(&token) {
                    output_queue.push(operator_stack.pop().unwrap());
                } else {
                    break;
                }
            }
            operator_stack.push(token);
        } else if token == "(" || token == "{" {
            operator_stack.push(token); // Push opening braces/parentheses onto the stack
            if let Some(function) = operator_stack.last() {
                if FUNCTION_REGISTRY.get(function).is_some() {
                    function_brace_stack.push(function.clone()); // Track function opening
                }
            }
        } else if token == ")" || token == "}" {
            // Handle closing parentheses or braces by popping from the operator stack
            while let Some(top) = operator_stack.pop() {
                if top == "(" || top == "{" {
                    break;
                }
                output_queue.push(top);
            }

            // Check if we're closing a function argument brace
            if let Some(function) = function_brace_stack.last() {
                if FUNCTION_REGISTRY.get(function).is_some() {
                    // Check if we've finished processing both arguments for functions like frac
                    if function_brace_stack.len() == 1 {
                        output_queue.push(function_brace_stack.pop().unwrap()); // Push function to output queue
                    }
                }
            }
        } else if let Some(_function) = FUNCTION_REGISTRY.get(&token) {
            // If it's a function, push to the operator stack
            log::debug!("Function detected: {}", token);
            operator_stack.push(token);
        } else if token.chars().all(|c| c.is_alphabetic()) {
            // Handle variables like 'x', 'y', 't' directly
            log::debug!("Variable detected: {}", token);
            output_queue.push(token);
        } else {
            return Err(format!("Unknown token '{}'", token));
        }

        log::debug!("Current output queue: {:?}", output_queue);
        log::debug!("Current operator stack: {:?}", operator_stack);
        log::debug!("Current function brace stack: {:?}", function_brace_stack);
    }

    // Pop all remaining operators to the output queue
    while let Some(op) = operator_stack.pop() {
        if op == "(" || op == ")" || op == "{" || op == "}" {
            return Err("Mismatched parentheses or braces".to_string());
        }
        output_queue.push(op);
    }

    log::debug!("Final RPN output: {:?}", output_queue);
    Ok(output_queue)
}

pub fn get_precedence(op: &str) -> i32 {
    match op {
        "NEG" => 4,                          // Highest precedence for unary minus
        "^" => 3,                            // Exponentiation
        "*" | "/" => 2,                      // Multiplication and Division
        "+" | "-" => 1,                      // Addition and Subtraction
        ">" | "<" | ">=" | "<=" | "==" => 0, // Inequality operators
        "=" => -1,                          // Equation has lowest precedence
        _ => 0,
    }
}

fn is_argument_terminator(arg: &Node) -> bool {
    matches!(arg, Node::ClosingParen | Node::ClosingBrace)
}

pub fn build_expression_tree(tokens: Vec<String>) -> Result<Node, String> {
    log::debug!("Building expression tree from tokens: {:?}", tokens);

    // Special handling for summation notation: \sum_{i=1}^{n} i
    if tokens.contains(&"sum".to_string()) {
        return parse_summation(&tokens);
    }

    let rpn = shunting_yard(tokens)?;

    let mut stack: Vec<Node> = Vec::new();

    for token in rpn {
        log::debug!("Processing token: {}", token);

        if let Ok(num) = token.parse::<f64>() {
            // Push numbers directly onto the stack
            log::debug!("Pushing number: {}", num);
            stack.push(Node::Number(num));
        } else if token == "ABS" {
            let operand = stack
                .pop()
                .ok_or_else(|| "Not enough operands for ABS".to_string())?;
            stack.push(Node::Abs(Box::new(operand))); // Handle absolute value
        } else if token == "NEG" {
            // Handle unary minus by applying it to the top of the stack
            let operand = stack
                .pop()
                .ok_or_else(|| "Not enough operands for unary minus".to_string())?;
            stack.push(Node::Negate(Box::new(operand)));
        } else if "+-*/^".contains(&token) {
            // Binary operators require two operands
            let right = stack
                .pop()
                .ok_or_else(|| format!("Not enough operands for operator '{}'", token))?;
            let left = stack
                .pop()
                .ok_or_else(|| format!("Not enough operands for operator '{}'", token))?;

            let node = match token.as_str() {
                "+" => Node::Add(Box::new(left), Box::new(right)),
                "-" => Node::Subtract(Box::new(left), Box::new(right)),
                "*" => Node::Multiply(Box::new(left), Box::new(right)),
                "/" => Node::Divide(Box::new(left), Box::new(right)),
                "^" => Node::Power(Box::new(left), Box::new(right)),
                _ => return Err(format!("Unknown operator '{}'", token)),
            };

            log::debug!("Pushing node: {:?}", node);
            stack.push(node);
        } else if token == ">" || token == "<" || token == ">=" || token == "<=" || token == "==" || token == "=" {
            let right = stack
                .pop()
                .ok_or_else(|| format!("Not enough operands for operator '{}'", token))?;
            let left = stack
                .pop()
                .ok_or_else(|| format!("Not enough operands for operator '{}'", token))?;

            let node = match token.as_str() {
                ">" => Node::Greater(Box::new(left), Box::new(right)),
                "<" => Node::Less(Box::new(left), Box::new(right)),
                ">=" => Node::GreaterEqual(Box::new(left), Box::new(right)),
                "<=" => Node::LessEqual(Box::new(left), Box::new(right)),
                "==" => Node::Equal(Box::new(left), Box::new(right)), // For equality comparison
                "=" => Node::Equation(Box::new(left), Box::new(right)), // For equation
                _ => return Err(format!("Unknown operator '{}'", token)),
            };

            stack.push(node);
        } else if let Some(function) = FUNCTION_REGISTRY.get(&token) {
            let arg_count = function.get_arg_count();

            if let Some(count) = arg_count {
                // Fixed-argument function
                let mut args = Vec::new();
                for _ in 0..count {
                    let arg = stack
                        .pop()
                        .ok_or_else(|| format!("Not enough operands for function {}", token))?;
                    args.push(arg);
                }
                args.reverse(); // Reverse to maintain order
                stack.push(Node::Function(token.clone(), args)); // Use the token as function name
            } else {
                // Variable-argument function (pop until we find a closing delimiter or hit an error)
                let mut args = Vec::new();
                while let Some(arg) = stack.pop() {
                    if is_argument_terminator(&arg) {
                        break;
                    }
                    args.push(arg);
                }
                args.reverse();
                stack.push(Node::Function(token.clone(), args)); // Use the token as function name
            }
        } else if token.chars().all(|c| c.is_alphabetic()) {
            // Handle variables directly (e.g., `x`, `y`)
            if token == "e" || token == "EULER" {
                stack.push(Node::Number(std::f64::consts::E)); // Euler's number
            } else if token == "\\pi" || token == "PI" {
                stack.push(Node::Number(std::f64::consts::PI));
            } else {
                log::debug!("Pushing variable: {}", token);
                stack.push(Node::Variable(token));
            }
        } else {
            return Err(format!("Unknown token '{}'", token));
        }

        log::debug!("Current stack state: {:?}", stack);
    }

    // The final expression tree should be a single node on the stack
    if stack.len() != 1 {
        return Err("The expression did not resolve into a single tree.".to_string());
    }

    log::debug!("Final expression tree: {:?}", stack[0]);
    Ok(stack.pop().unwrap())
}

/// Parse a summation expression like \sum_{i=1}^{n} i
fn parse_summation(tokens: &[String]) -> Result<Node, String> {
    // Find the position of the sum token
    let sum_pos = tokens.iter().position(|t| t == "sum").ok_or("Expected 'sum' token")?;
    
    // Check for underscore after sum
    if sum_pos + 1 >= tokens.len() || tokens[sum_pos + 1] != "_" {
        return Err("Expected '_' after 'sum'".to_string());
    }
    
    // Check for opening brace for lower bound
    if sum_pos + 2 >= tokens.len() || tokens[sum_pos + 2] != "{" {
        return Err("Expected '{' after '_'".to_string());
    }
    
    // Extract the index variable and starting value
    // Format is typically: i = 1
    let mut lower_bound_tokens = Vec::new();
    let mut i = sum_pos + 3;
    let mut brace_count = 1;
    
    // Extract the index variable
    let index_var = if i < tokens.len() && tokens[i].chars().all(|c| c.is_alphabetic()) {
        let var = tokens[i].clone();
        i += 1;
        var
    } else {
        return Err("Expected index variable after '{'".to_string());
    };
    
    // Expect equals sign
    if i >= tokens.len() || tokens[i] != "=" {
        return Err("Expected '=' after index variable".to_string());
    }
    i += 1;
    
    // Extract the start value
    while i < tokens.len() && brace_count > 0 {
        if tokens[i] == "{" {
            brace_count += 1;
        } else if tokens[i] == "}" {
            brace_count -= 1;
            if brace_count == 0 {
                break;
            }
        }
        lower_bound_tokens.push(tokens[i].clone());
        i += 1;
    }
    
    if brace_count > 0 {
        return Err("Unclosed lower bound brace".to_string());
    }
    
    // Check for caret after lower bound
    i += 1; // Move past closing brace
    if i >= tokens.len() || tokens[i] != "^" {
        return Err("Expected '^' after lower bound".to_string());
    }
    i += 1;
    
    // Check for opening brace for upper bound
    if i >= tokens.len() || tokens[i] != "{" {
        return Err("Expected '{' after '^'".to_string());
    }
    i += 1;
    
    // Extract upper bound tokens
    let mut upper_bound_tokens = Vec::new();
    brace_count = 1;
    
    while i < tokens.len() && brace_count > 0 {
        if tokens[i] == "{" {
            brace_count += 1;
        } else if tokens[i] == "}" {
            brace_count -= 1;
            if brace_count == 0 {
                break;
            }
        }
        upper_bound_tokens.push(tokens[i].clone());
        i += 1;
    }
    
    if brace_count > 0 {
        return Err("Unclosed upper bound brace".to_string());
    }
    
    // Extract the expression to be summed
    i += 1; // Move past closing brace
    
    // The expression can be a single token or surrounded by braces
    let mut body_tokens = Vec::new();
    
    // If there's a brace, collect all tokens until closing brace
    if i < tokens.len() && tokens[i] == "{" {
        i += 1;
        brace_count = 1;
        
        while i < tokens.len() && brace_count > 0 {
            if tokens[i] == "{" {
                brace_count += 1;
            } else if tokens[i] == "}" {
                brace_count -= 1;
                if brace_count == 0 {
                    break;
                }
            }
            body_tokens.push(tokens[i].clone());
            i += 1;
        }
        
        if brace_count > 0 {
            return Err("Unclosed body brace".to_string());
        }
    } else {
        // If there's no brace, collect all tokens until an equals sign or end of tokens
        while i < tokens.len() && tokens[i] != "=" {
            body_tokens.push(tokens[i].clone());
            i += 1;
        }
    }
    
    // Parse the start, end, and body expressions
    let start_expr = build_expression_tree(lower_bound_tokens)?;
    let end_expr = build_expression_tree(upper_bound_tokens)?;
    let body_expr = build_expression_tree(body_tokens)?;
    
    // The rest of the tokens after the summation need to be parsed
    let mut remaining_tokens = Vec::new();
    for j in i..tokens.len() {
        remaining_tokens.push(tokens[j].clone());
    }
    
    let summation_node = Node::Summation(
        index_var, 
        Box::new(start_expr), 
        Box::new(end_expr), 
        Box::new(body_expr)
    );
    
    // If there are no additional tokens, return the summation node
    if remaining_tokens.is_empty() {
        return Ok(summation_node);
    }
    
    // If there's an equation, we need to parse the right side
    if remaining_tokens.contains(&"=".to_string()) {
        let eq_pos = remaining_tokens.iter().position(|t| t == "=").unwrap();
        
        let right_tokens = remaining_tokens[eq_pos+1..].to_vec();
        let right_expr = build_expression_tree(right_tokens)?;
        
        return Ok(Node::Equation(Box::new(summation_node), Box::new(right_expr)));
    }
    
    // Otherwise, try to parse as a continuation of an expression
    // (this is more complex and would require additional implementation)
    Err("Unsupported tokens after summation".to_string())
}
