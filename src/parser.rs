use crate::node::Node;  // Node enum should be imported from the node module

pub fn tokenize(expr: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let mut last_was_operator_or_paren = true; // Track if the last token was an operator or open parenthesis

    for c in expr.chars() {
        if c.is_whitespace() {
            continue; // Skip whitespace
        } else if c.is_digit(10) || c == '.' {
            current_token.push(c); // Build a number token
            last_was_operator_or_paren = false;
        } else if c.is_alphabetic() {
            current_token.push(c); // Build a variable token
            last_was_operator_or_paren = false;
        } else {
            if !current_token.is_empty() {
                tokens.push(current_token.clone());
                current_token.clear();
            }

            if "+*/^()=".contains(c) {
                tokens.push(c.to_string()); // Push operators or parentheses
                last_was_operator_or_paren = c == '(' || "+*/^=".contains(c);
            } else if c == '-' {
                // Treat '-' as unary if the previous token was an operator or '('
                if last_was_operator_or_paren {
                    tokens.push("u-".to_string()); // Unary minus
                } else {
                    tokens.push("-".to_string()); // Binary minus
                }
                last_was_operator_or_paren = true;
            }
        }
    }

    if !current_token.is_empty() {
        tokens.push(current_token);
    }

    tokens
}

// pub fn shunting_yard(tokens: Vec<String>) -> Result<Vec<String>, String> {
//     let mut output_queue: Vec<String> = Vec::new();
//     let mut operator_stack: Vec<String> = Vec::new();

//     let mut iter = tokens.into_iter().peekable();
//     let mut last_was_operator_or_paren = true; // Track if last token was an operator or open parenthesis

//     while let Some(token) = iter.next() {
//         if token.parse::<f64>().is_ok() || token.chars().all(char::is_alphabetic) {
//             // Token is a number or a variable, push to output queue
//             output_queue.push(token);
//             last_was_operator_or_paren = false;
//         } else if token == "-" && last_was_operator_or_paren {
//             // If the previous token was an operator or open parenthesis, treat '-' as unary
//             output_queue.push("u-".to_string());
//         } else if "+-*/^".contains(&token) {
//             // Token is a binary operator
//             while let Some(op) = operator_stack.last() {
//                 if "+-*/^".contains(op)
//                     && ((is_right_associative(&token)
//                         && get_precedence(op) >= get_precedence(&token))
//                         || (!is_right_associative(&token)
//                             && get_precedence(op) > get_precedence(&token)))
//                 {
//                     output_queue.push(operator_stack.pop().unwrap());
//                 } else {
//                     break;
//                 }
//             }
//             operator_stack.push(token);
//             last_was_operator_or_paren = true;
//         } else if token == "(" {
//             // Push the opening parenthesis to the operator stack
//             operator_stack.push(token);
//             last_was_operator_or_paren = true;
//         } else if token == ")" {
//             // Pop operators from the stack to the output queue until we find an opening parenthesis
//             let mut found_left_paren = false;
//             while let Some(op) = operator_stack.pop() {
//                 if op == "(" {
//                     found_left_paren = true;
//                     break;
//                 } else {
//                     output_queue.push(op);
//                 }
//             }
//             if !found_left_paren {
//                 return Err("Mismatched parentheses: extra closing parenthesis found.".to_string());
//             }
//             last_was_operator_or_paren = false;
//         }
//     }

//     // After processing all tokens, pop any remaining operators to the output queue
//     while let Some(op) = operator_stack.pop() {
//         if op == "(" {
//             return Err("Mismatched parentheses: unclosed opening parenthesis.".to_string());
//         }
//         output_queue.push(op);
//     }

//     Ok(output_queue)
// }


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
                if "+-*/^".contains(op)
                    && ((is_right_associative(&token)
                        && get_precedence(op) >= get_precedence(&token))
                        || (!is_right_associative(&token)
                            && get_precedence(op) > get_precedence(&token)))
                {
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

// pub fn build_expression_tree(tokens: Vec<String>) -> Result<Node, String> {
//     let rpn = shunting_yard(tokens)?;

//     let mut stack: Vec<Node> = Vec::new();

//     for token in rpn {
//         if let Ok(num) = token.parse::<f64>() {
//             stack.push(Node::Number(num));
//         } else if token == "u-" {
//             // Handle unary minus: pop the next node and negate it
//             if let Some(node) = stack.pop() {
//                 // Negate either a number or a variable
//                 let new_node = match node {
//                     Node::Number(value) => Node::Number(-value),
//                     Node::Variable(_) => Node::Multiply(Box::new(Node::Number(-1.0)), Box::new(node)),
//                     _ => return Err("Unary minus must be followed by a number or variable.".to_string()),
//                 };
//                 stack.push(new_node);
//             } else {
//                 return Err("Unary minus must be followed by a number or variable.".to_string());
//             }
//         } else if token.chars().all(char::is_alphabetic) {
//             stack.push(Node::Variable(token));
//         } else if "+-*/^".contains(&token) {
//             let right = stack
//                 .pop()
//                 .ok_or_else(|| format!("Not enough operands for operator '{}'", token))?;
//             let left = stack
//                 .pop()
//                 .ok_or_else(|| format!("Not enough operands for operator '{}'", token))?;

//             let node = match token.as_str() {
//                 "+" => Node::Add(Box::new(left), Box::new(right)),
//                 "-" => Node::Subtract(Box::new(left), Box::new(right)),
//                 "*" => Node::Multiply(Box::new(left), Box::new(right)),
//                 "/" => Node::Divide(Box::new(left), Box::new(right)),
//                 "^" => Node::Power(Box::new(left), Box::new(right)),
//                 _ => return Err(format!("Unknown operator '{}'", token)),
//             };

//             stack.push(node);
//         } else {
//             return Err(format!("Unknown token '{}'", token));
//         }
//     }

//     if stack.len() != 1 {
//         return Err("The expression did not resolve into a single tree.".to_string());
//     }

//     Ok(stack.pop().unwrap())
// }

// /// Helper functions to handle precedence and associativity.
// pub fn get_precedence(op: &str) -> i32 {
//     match op {
//         "+" | "-" => 1,
//         "*" | "/" => 2,
//         "^" => 3, // Exponentiation
//         _ => 0,
//     }
// }

// pub fn is_right_associative(op: &str) -> bool {
//     match op {
//         "^" => true, // Exponentiation is right-associative
//         _ => false,
//     }
// }


pub fn build_expression_tree(tokens: Vec<String>) -> Result<Node, String> {
    let rpn = shunting_yard(tokens)?;

    let mut stack: Vec<Node> = Vec::new();

    for token in rpn {
        if let Ok(num) = token.parse::<f64>() {
            stack.push(Node::Number(num));
        } else if token.chars().all(char::is_alphabetic) {
            stack.push(Node::Variable(token));
        } else if "+-*/^".contains(&token) {
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