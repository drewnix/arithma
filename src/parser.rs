use crate::node::Node;

pub fn tokenize(expr: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let mut last_was_operator_or_paren = true; // Track if the last token was an operator or opening parenthesis

    log::debug!("Tokenizing expression: {}", expr);

    for c in expr.chars() {
        if c.is_whitespace() {
            continue; // Skip whitespace
        } else if c.is_digit(10) || c == '.' {
            current_token.push(c); // Build a number token
            last_was_operator_or_paren = false;
        } else if c.is_alphabetic() {
            // Add support for alphabetic variables like `x`
            if !current_token.is_empty() {
                log::debug!("Token: {}", current_token);
                tokens.push(current_token.clone());
                current_token.clear();
            }
            current_token.push(c); // Capture the variable
            log::debug!("Variable detected: {}", current_token);
            tokens.push(current_token.clone());
            current_token.clear();
            last_was_operator_or_paren = false;
        } else {
            if !current_token.is_empty() {
                log::debug!("Token: {}", current_token);
                tokens.push(current_token.clone());
                current_token.clear();
            }

            if "+*/^()=".contains(c) {
                log::debug!("Operator/Paren: {}", c);
                tokens.push(c.to_string()); // Push operators or parentheses
                last_was_operator_or_paren = c == '(' || "+*/^=".contains(c);
            } else if c == '-' {
                // Treat '-' as unary if the previous token was an operator or '('
                if last_was_operator_or_paren {
                    log::debug!("Unary Minus Detected");
                    tokens.push("NEG".to_string()); // Unary minus
                } else {
                    log::debug!("Binary Minus Detected");
                    tokens.push("-".to_string()); // Binary minus
                }
                last_was_operator_or_paren = true;
            }
        }
    }

    if !current_token.is_empty() {
        log::debug!("Token: {}", current_token);
        tokens.push(current_token);
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
        } else if token == "(" {
            operator_stack.push(token);
        } else if token == ")" {
            while let Some(top) = operator_stack.pop() {
                if top == "(" {
                    break;
                }
                output_queue.push(top);
            }
        } else {
            return Err(format!("Unknown token '{}'", token));
        }

        log::debug!("Current output queue: {:?}", output_queue);
        log::debug!("Current operator stack: {:?}", operator_stack);
    }

    // Pop all remaining operators to the output queue
    while let Some(op) = operator_stack.pop() {
        if op == "(" || op == ")" {
            return Err("Mismatched parentheses".to_string());
        }
        output_queue.push(op);
    }

    log::debug!("Final RPN output: {:?}", output_queue);
    Ok(output_queue)
}



pub fn get_precedence(op: &str) -> i32 {
    match op {
        "NEG" => 4,  // Highest precedence for unary minus
        "^" => 3,    // Exponentiation
        "*" | "/" => 2,    // Multiplication and Division
        "+" | "-" => 1,    // Addition and Subtraction
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
            let operand = stack.pop().ok_or_else(|| "Not enough operands for unary minus".to_string())?;
            stack.push(Node::Negate(Box::new(operand)));
        } else if token.chars().all(|c| c.is_alphabetic()) {
            // Handle variables directly (e.g., `x`, `y`)
            log::debug!("Pushing variable: {}", token);
            stack.push(Node::Variable(token));
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
// pub fn is_right_associative(op: &str) -> bool {
//     match op {
//         "^" => true, // Exponentiation is right-associative
//         _ => false,
//     }
// }

pub fn mathjson_to_node(mathjson: &serde_json::Value) -> Result<Node, String> {
    match mathjson {
        serde_json::Value::Array(array) => {
            if array.is_empty() {
                return Err("Empty MathJSON array".to_string());
            }

            let operator = array[0].as_str().ok_or("Invalid MathJSON operator")?;

            match operator {
                "Rational" => {
                    let numerator = array
                        .get(1)
                        .and_then(|v| v.as_i64())
                        .ok_or("Invalid numerator")?;
                    let denominator = array
                        .get(2)
                        .and_then(|v| v.as_i64())
                        .ok_or("Invalid denominator")?;
                    Ok(Node::Rational(numerator, denominator))
                }
                "Add" => Ok(Node::Add(
                    Box::new(mathjson_to_node(&array[1])?),
                    Box::new(mathjson_to_node(&array[2])?),
                )),
                "Subtract" => Ok(Node::Subtract(
                    Box::new(mathjson_to_node(&array[1])?),
                    Box::new(mathjson_to_node(&array[2])?),
                )),
                "Multiply" => Ok(Node::Multiply(
                    Box::new(mathjson_to_node(&array[1])?),
                    Box::new(mathjson_to_node(&array[2])?),
                )),
                "Divide" => Ok(Node::Divide(
                    Box::new(mathjson_to_node(&array[1])?),
                    Box::new(mathjson_to_node(&array[2])?),
                )),
                "Power" => Ok(Node::Power(
                    Box::new(mathjson_to_node(&array[1])?),
                    Box::new(mathjson_to_node(&array[2])?),
                )),
                "Sqrt" => Ok(Node::Sqrt(Box::new(mathjson_to_node(&array[1])?))),
                "Abs" => Ok(Node::Abs(Box::new(mathjson_to_node(&array[1])?))),
                "Greater" => Ok(Node::Greater(
                    Box::new(mathjson_to_node(&array[1])?),
                    Box::new(mathjson_to_node(&array[2])?),
                )),
                "Less" => Ok(Node::Less(
                    Box::new(mathjson_to_node(&array[1])?),
                    Box::new(mathjson_to_node(&array[2])?),
                )),
                "GreaterEqual" => Ok(Node::GreaterEqual(
                    Box::new(mathjson_to_node(&array[1])?),
                    Box::new(mathjson_to_node(&array[2])?),
                )),
                "LessEqual" => Ok(Node::LessEqual(
                    Box::new(mathjson_to_node(&array[1])?),
                    Box::new(mathjson_to_node(&array[2])?),
                )),
                "Piecewise" => {
                    let conditions = array[1]
                        .as_array()
                        .ok_or("Invalid piecewise format for conditions")?;
                    let mut nodes = Vec::new();
                    for condition in conditions {
                        let expr = mathjson_to_node(&condition[0])?;
                        let cond = mathjson_to_node(&condition[1])?;
                        nodes.push((expr, cond));
                    }
                    Ok(Node::Piecewise(nodes))
                }
                _ => Err(format!("Unsupported operator: {}", operator)),
            }
        }
        serde_json::Value::Number(num) => {
            if let Some(n) = num.as_f64() {
                Ok(Node::Number(n))
            } else {
                Err("Invalid number format in MathJSON".to_string())
            }
        }
        serde_json::Value::String(var) => match var.as_str() {
            "ExponentialE" => Ok(Node::Number(std::f64::consts::E)),
            "Pi" => Ok(Node::Number(std::f64::consts::PI)),
            _ => Ok(Node::Variable(var.clone())),
        },
        _ => Err("Invalid MathJSON format".to_string()),
    }
}
