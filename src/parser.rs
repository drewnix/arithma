use crate::node::Node;
pub fn tokenize(expr: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let mut chars = expr.chars().peekable();

    log::debug!("Tokenizing expression: {}", expr);

    while let Some(c) = chars.next() {
        if c.is_whitespace() {
            continue; // Skip whitespace
        }

        // Handle numbers (digits and decimal point)
        if c.is_digit(10) || c == '.' {
            current_token.push(c);
            // Consume the full number (if it's more than one character long)
            while let Some(&next_char) = chars.peek() {
                if next_char.is_digit(10) || next_char == '.' {
                    current_token.push(next_char);
                    chars.next(); // Move the iterator forward
                } else {
                    break;
                }
            }
            tokens.push(current_token.clone());
            current_token.clear();
        }
        // Handle LaTeX commands like \sin, \log, \frac
        else if c == '\\' {
            current_token.push(c); // Start of a LaTeX command
            while let Some(&next_char) = chars.peek() {
                if next_char.is_alphabetic() {
                    current_token.push(next_char);
                    chars.next(); // Move the iterator forward
                } else {
                    break;
                }
            }
            // Check if it's the LaTeX multiplication command \cdot
            if current_token == "\\cdot" {
                tokens.push("*".to_string()); // Treat \cdot as multiplication
            } else {
                tokens.push(current_token.clone()); // Add other LaTeX commands as is
            }
            current_token.clear();
        }
        // Handle alphabetic variables like x, y, etc.
        else if c.is_alphabetic() {
            current_token.push(c);
            tokens.push(current_token.clone());
            current_token.clear();
        }
        // Handle operators and parentheses
        else if "+-*/^(){}".contains(c) {
            tokens.push(c.to_string());
        }
        // Handle unknown characters
        else {
            log::debug!("Unknown token: {}", c);
            return vec![format!("Unknown token '{}'", c)];
        }
    }

    log::debug!("Tokenized result: {:?}", tokens);
    tokens
}

pub fn shunting_yard(tokens: Vec<String>) -> Result<Vec<String>, String> {
    log::debug!("Starting Shunting Yard with tokens: {:?}", tokens);

    let mut output_queue: Vec<String> = Vec::new();
    let mut operator_stack: Vec<String> = Vec::new();

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
        } else if token.chars().all(|c| c.is_alphabetic()) {
            // Handle variables like 'x', 'y', 't' directly
            log::debug!("Variable detected: {}", token);
            output_queue.push(token);
        } else if "+-*/^".contains(&token) {
            // Handle binary operators with precedence and associativity
            while let Some(top) = operator_stack.last() {
                // Ensure NEG has higher precedence than binary operators
                if get_precedence(top) >= get_precedence(&token) {
                    output_queue.push(operator_stack.pop().unwrap());
                } else {
                    break;
                }
            }
            operator_stack.push(token);
        } else if token == "(" || token == "{" {
            operator_stack.push(token);
        } else if token == ")" || token == "}" {
            // Handle closing parentheses or braces by popping from the operator stack
            while let Some(top) = operator_stack.pop() {
                if top == "(" || top == "{" {
                    break;
                }
                output_queue.push(top);
            }
        } else if token.starts_with("\\") {
            // Handle LaTeX functions by pushing them onto the operator stack
            // LaTeX functions like \log, \sin, \cos are treated as "operators" with higher precedence
            log::debug!("LaTeX function detected: {}", token);
            operator_stack.push(token);
        } else {
            return Err(format!("Unknown token '{}'", token));
        }

        log::debug!("Current output queue: {:?}", output_queue);
        log::debug!("Current operator stack: {:?}", operator_stack);
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
        "NEG" => 4,     // Highest precedence for unary minus
        "^" => 3,       // Exponentiation
        "*" | "/" => 2, // Multiplication and Division
        "+" | "-" => 1, // Addition and Subtraction
        _ => 0,
    }
}

pub fn build_expression_tree(tokens: Vec<String>) -> Result<Node, String> {
    log::debug!("Building expression tree from tokens: {:?}", tokens);

    let rpn = shunting_yard(tokens)?;

    let mut stack: Vec<Node> = Vec::new();

    for token in rpn {
        log::debug!("Processing token: {}", token);

        if let Ok(num) = token.parse::<f64>() {
            // Push numbers directly onto the stack
            log::debug!("Pushing number: {}", num);
            stack.push(Node::Number(num));
        } else if token == "NEG" {
            // Handle unary minus by applying it to the top of the stack
            let operand = stack
                .pop()
                .ok_or_else(|| "Not enough operands for unary minus".to_string())?;
            stack.push(Node::Negate(Box::new(operand)));
        } else if token.chars().all(|c| c.is_alphabetic()) {
            // Handle variables directly (e.g., `x`, `y`)
            log::debug!("Pushing variable: {}", token);
            stack.push(Node::Variable(token));
        } else if token == "\\pi" {
            stack.push(Node::Number(std::f64::consts::PI));
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
        } else if token.starts_with("\\") {
            // Handle LaTeX functions
            match token.as_str() {
                "\\sin" => {
                    let operand = stack
                        .pop()
                        .ok_or_else(|| "Not enough operands for \\sin".to_string())?;
                    stack.push(Node::Function("sin".to_string(), vec![operand]));
                }
                "\\cos" => {
                    let operand = stack
                        .pop()
                        .ok_or_else(|| "Not enough operands for \\cos".to_string())?;
                    stack.push(Node::Function("cos".to_string(), vec![operand]));
                }
                "\\log" => {
                    let operand = stack
                        .pop()
                        .ok_or_else(|| "Not enough operands for \\log".to_string())?;
                    stack.push(Node::Function("log".to_string(), vec![operand]));
                }
                "\\sqrt" => {
                    let operand = stack
                        .pop()
                        .ok_or_else(|| "Not enough operands for \\sqrt".to_string())?;
                    stack.push(Node::Function("sqrt".to_string(), vec![operand]));
                }
                "\\frac" => {
                    let denominator = stack.pop().ok_or_else(|| {
                        "Not enough operands for \\frac (denominator)".to_string()
                    })?;
                    let numerator = stack
                        .pop()
                        .ok_or_else(|| "Not enough operands for \\frac (numerator)".to_string())?;
                    stack.push(Node::Divide(Box::new(numerator), Box::new(denominator)));
                }
                _ => return Err(format!("Unknown LaTeX function '{}'", token)),
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