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
        } else if token == ">"
            || token == "<"
            || token == ">="
            || token == "<="
            || token == "=="
            || token == "="
        {
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
        "=" => -1,                           // Equation has lowest precedence
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
        } else if token == ">"
            || token == "<"
            || token == ">="
            || token == "<="
            || token == "=="
            || token == "="
        {
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
    let sum_pos = tokens
        .iter()
        .position(|t| t == "sum")
        .ok_or("Expected 'sum' token")?;

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
    // Some users might omit braces for single-token values (like \sum_{i=1}^3)
    let has_upper_brace = i < tokens.len() && tokens[i] == "{";
    if has_upper_brace {
        i += 1;
    } else if i >= tokens.len() {
        return Err("Expected upper bound after '^'".to_string());
    }

    // Extract upper bound tokens
    let mut upper_bound_tokens = Vec::new();

    if has_upper_brace {
        // If we have braces, collect all tokens until the closing brace
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
        i += 1; // Move past the closing brace
    } else {
        // If no braces, take just one token as the upper bound
        upper_bound_tokens.push(tokens[i].clone());
        i += 1;
    }

    // Extract the expression to be summed
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
        // Move past the closing brace
        if i < tokens.len() {
            i += 1;
        }
    } else {
        // If there's no brace, extract the summation body with advanced balancing
        parse_unbraced_summation_body(&tokens[i..], &mut body_tokens, &mut i);
    }

    // Parse the start, end, and body expressions with better error handling
    let start_expr = build_expression_tree(lower_bound_tokens)
        .map_err(|e| format!("Error in summation lower bound: {}", e))?;

    let end_expr = build_expression_tree(upper_bound_tokens)
        .map_err(|e| format!("Error in summation upper bound: {}", e))?;

    // Debug logging for body tokens
    log::debug!("Body tokens for summation: {:?}", body_tokens);

    let body_expr = build_expression_tree(body_tokens)
        .map_err(|e| format!("Error in summation body: {}", e))?;

    // The rest of the tokens after the summation need to be parsed
    let mut remaining_tokens = Vec::new();
    remaining_tokens.extend(tokens.iter().skip(i).cloned());

    let summation_node = Node::Summation(
        index_var,
        Box::new(start_expr),
        Box::new(end_expr),
        Box::new(body_expr),
    );

    // If there are no additional tokens, return the summation node
    if remaining_tokens.is_empty() {
        return Ok(summation_node);
    }

    // If there's an equation, we need to parse the right side
    if remaining_tokens.contains(&"=".to_string()) {
        let eq_pos = remaining_tokens.iter().position(|t| t == "=").unwrap();

        let right_tokens = remaining_tokens[eq_pos + 1..].to_vec();
        let right_expr = build_expression_tree(right_tokens)?;

        return Ok(Node::Equation(
            Box::new(summation_node),
            Box::new(right_expr),
        ));
    }

    // If there are remaining tokens, something might be wrong, but we'll try to handle it gracefully
    Ok(summation_node)
}

/// Parse an unbraced summation body, handling cases like i^2, i*j, etc.
fn parse_unbraced_summation_body(
    tokens: &[String],
    body_tokens: &mut Vec<String>,
    pos: &mut usize,
) {
    let mut i = 0;
    let mut paren_depth = 0;
    let mut collecting = true;

    // Handle empty tokens case
    if tokens.is_empty() {
        return;
    }

    // Special case for simple power expressions like i^2
    // This is a common case that needs special handling
    if tokens.len() >= 3
        && tokens[0].chars().all(|c| c.is_alphabetic())
        && tokens[1] == "^"
        && tokens[2].parse::<i64>().is_ok()
    {
        // Add balanced power operation tokens
        body_tokens.push(tokens[0].clone()); // variable
        body_tokens.push("^".to_string()); // power operator
        body_tokens.push(tokens[2].clone()); // exponent

        // Update position and return early
        *pos += 3;
        return;
    }

    // First token (usually a variable)
    body_tokens.push(tokens[i].clone());
    i += 1;

    // Now parse the rest of the expression with balanced operator tracking
    while i < tokens.len() && collecting {
        match tokens[i].as_str() {
            // Opening parenthesis/brace increases depth
            "(" | "{" => {
                paren_depth += 1;
                body_tokens.push(tokens[i].clone());
            }
            // Closing parenthesis/brace decreases depth
            ")" | "}" => {
                if paren_depth > 0 {
                    paren_depth -= 1;
                    body_tokens.push(tokens[i].clone());
                } else {
                    // Unbalanced closing without open - this is not part of our expression
                    collecting = false;
                    continue;
                }
            }
            // Operators: keep collecting as long as we are in depth or have a meaningful continuation
            "+" | "-" | "*" | "/" | "^" => {
                // For operations at the top level (no open parens), collect if they're
                // standard infix operations like i^2, i*3, etc.
                if paren_depth == 0 && i < tokens.len() - 1 {
                    body_tokens.push(tokens[i].clone());
                } else if paren_depth > 0 {
                    // If we're inside parentheses/braces, always collect
                    body_tokens.push(tokens[i].clone());
                } else {
                    // Operator at top level with nothing after it - not our expression
                    collecting = false;
                    continue;
                }
            }
            // Equation sign or another summation - definitely stop collecting
            "=" | "sum" => {
                collecting = false;
                continue;
            }
            // Any other token - include in our expression as long as we're in a balanced state
            _ => {
                body_tokens.push(tokens[i].clone());
            }
        }

        i += 1;

        // If we're at the top level (no open parens) after an operation, check if we should stop
        if paren_depth == 0 && i < tokens.len() {
            // After including a term after an operator, we've completed a binary operation
            // If the next token isn't another operator, we should stop unless in parentheses
            if i < tokens.len()
                && !matches!(tokens[i].as_str(), "+" | "-" | "*" | "/" | "^")
                && body_tokens.len() >= 3
            {
                // We have at least a term-op-term sequence
                collecting = false;
            }
        }
    }

    // Update the position
    *pos += i;

    // Handle the case where we have a power operation that needs to be rewritten for RPN
    if body_tokens.len() >= 2 && body_tokens.contains(&"^".to_string()) {
        // For case like "i^2", ensure we have a valid RPN expression
        // Sometimes, the ^ operator can be problematic in RPN evaluation

        // Add explicit parentheses for power operations to ensure correct precedence
        if body_tokens.len() == 3 && body_tokens[1] == "^" {
            // Format is: var ^ exponent
            let var = body_tokens[0].clone();
            let exponent = body_tokens[2].clone();

            // If it's a numeric exponent, we can special-case it for better handling
            if let Ok(exp_val) = exponent.parse::<i64>() {
                if exp_val == 2 {
                    // For squaring, use explicit multiplication: i*i
                    body_tokens.clear();
                    body_tokens.push(var.clone());
                    body_tokens.push("*".to_string());
                    body_tokens.push(var);
                } else if exp_val > 2 && exp_val <= 5 {
                    // For small powers, use explicit multiplication chain
                    body_tokens.clear();
                    body_tokens.push("(".to_string());

                    // Add the first factor
                    body_tokens.push(var.clone());

                    // Add the remaining factors
                    for _ in 1..exp_val {
                        body_tokens.push("*".to_string());
                        body_tokens.push(var.clone());
                    }

                    body_tokens.push(")".to_string());
                }
                // For higher powers or non-numeric exponents, leave as is
            }
        }
    }
}
