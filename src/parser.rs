use crate::functions::FUNCTION_REGISTRY;
use crate::node::Node;
use std::iter::Peekable;
use std::str::Chars;

pub fn tokenize(expr: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let mut chars = expr.chars().peekable();
    let mut last_token: Option<String> = None;

    while let Some(c) = chars.next() {
        if c.is_whitespace() {
            continue; // Skip whitespace
        }

        // Handle numbers (digits and decimal point)
        if c.is_digit(10) || c == '.' {
            tokenize_numbers(&mut tokens, &mut current_token, &mut chars, c);
        }
        // Handle LaTeX commands like \sin, \log, \frac
        else if c == '\\' {
            tokenize_latex_commands(&mut tokens, &mut current_token, &mut chars, c);
        } else if c == '>' || c == '<' {
            let mut op = c.to_string();
            if let Some(&next_char) = chars.peek() {
                if next_char == '=' {
                    op.push(next_char); // Combine >= or <=
                    chars.next(); // Move the iterator forward
                }
            }
            tokens.push(op);
        } else if c == '=' && chars.peek() == Some(&'=') {
            tokens.push("==".to_string());
            chars.next();
        }
        // Handle alphabetic variables like x, y, etc.
        else if c.is_alphabetic() {
            // If the previous token was a number, insert an implicit multiplication
            if let Some(last_token) = last_token.as_ref() {
                if last_token.chars().all(char::is_numeric) {
                    tokens.push("*".to_string()); // Insert an implicit multiplication operator
                }
            }

            current_token.push(c);
            tokenize_function_or_variable(&mut tokens, &mut current_token, &mut chars);
            current_token.clear();
        }
        // Handle operators and parentheses
        else if "+*/^(){}".contains(c) {
            tokens.push(c.to_string());
        }
        // Special handling for minus '-' to distinguish unary and binary
        else if c == '-' {
            tokenize_minus(&mut tokens, &last_token);
        }
        last_token = tokens.last().cloned(); // Update last_token for next iteration
    }

    tokens
}

fn tokenize_function_or_variable(
    tokens: &mut Vec<String>,
    current_token: &mut String,
    chars: &mut Peekable<Chars>,
) {
    while let Some(&next_char) = chars.peek() {
        if next_char.is_alphanumeric() {
            current_token.push(next_char);
            chars.next();
        } else {
            break;
        }
    }

    // Check if the token is a registered function name
    if FUNCTION_REGISTRY.get(&current_token).is_some() {
        // If the function is followed by a parenthesis or brace, treat it as a function call
        if chars.peek() == Some(&'(') || chars.peek() == Some(&'{') {
            tokens.push(current_token.clone()); // Push function name
        } else {
            tokens.push(current_token.clone()); // Handle as variable
        }
    } else {
        // Handle as a variable if not a function
        tokens.push(current_token.clone());
    }
    current_token.clear();
}

fn tokenize_minus(tokens: &mut Vec<String>, last_token: &Option<String>) {
    let is_unary =
        last_token.is_none() || "+-*/^({ABS_START".contains(last_token.as_deref().unwrap_or(""));
    if is_unary {
        tokens.push("NEG".to_string()); // Tokenize unary minus as "NEG"
    } else {
        tokens.push("-".to_string()); // Tokenize binary minus as "-"
    }
}

fn tokenize_numbers(
    tokens: &mut Vec<String>,
    current_token: &mut String,
    chars: &mut Peekable<Chars>,
    c: char,
) {
    current_token.push(c);
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

fn tokenize_latex_commands(
    tokens: &mut Vec<String>,
    current_token: &mut String,
    chars: &mut Peekable<Chars>,
    _c: char,
) {
    current_token.push('\\');
    while let Some(&next_char) = chars.peek() {
        if next_char.is_alphabetic() {
            current_token.push(next_char);
            chars.next();
        } else {
            break;
        }
    }

    // Strip backslash for LaTeX commands
    let stripped_token = current_token.trim_start_matches('\\').to_string();

    if stripped_token.starts_with("left") && chars.peek() == Some(&'|') {
        tokens.push("ABS_START".to_string());
        chars.next(); // Consume the '|'
    } else if stripped_token.starts_with("right") && chars.peek() == Some(&'|') {
        tokens.push("ABS_END".to_string());
        chars.next(); // Consume the '|'
    } else if stripped_token == "pi" {
        tokens.push("PI".to_string());
    } else if stripped_token == "mathrm" {
        if chars.peek() == Some(&'{') {
            chars.next(); // Consume the '{'
            current_token.clear();
            while let Some(&next_char) = chars.peek() {
                if next_char == 'e' {
                    current_token.push(next_char);
                    chars.next();
                    if chars.peek() == Some(&'}') {
                        chars.next(); // Consume the closing '}'
                        tokens.push("EULER".to_string()); // Tokenize \mathrm{e} as EULER
                    }
                } else {
                    break;
                }
            }
        }
    } else if stripped_token == "cdot" {
        tokens.push("*".to_string());
    } else if stripped_token == "left" {
        tokens.push("(".to_string()); // Treat \left as (
    } else if stripped_token == "right" {
        tokens.push(")".to_string()); // Treat \right as )
    } else if stripped_token == "frac" {
        if let Some(&next_char) = chars.peek() {
            // Check if next char is a digit, indicating shorthand fraction \frac23
            if next_char.is_digit(10) {
                current_token.clear();
                tokenize_shorthand_fraction(tokens, chars);
            } else {
                tokens.push(stripped_token);
            }
        }
    } else {
        // General LaTeX function case
        tokens.push(stripped_token);
    }

    current_token.clear();
}

fn tokenize_shorthand_fraction(tokens: &mut Vec<String>, chars: &mut Peekable<Chars>) {
    if let Some(numerator_char) = chars.next() {
        if numerator_char.is_digit(10) {
            tokens.push(numerator_char.to_string()); // Push numerator (e.g., '2')
        } else {
            return; // Invalid syntax if no digit found
        }

        if let Some(denominator_char) = chars.next() {
            if denominator_char.is_digit(10) {
                tokens.push("/".to_string()); // Insert division operator
                tokens.push(denominator_char.to_string()); // Push denominator (e.g., '3')
            } else {
                return; // Invalid syntax if no digit found
            }
        }
    }
}
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
        } else if token == ">" || token == "<" || token == ">=" || token == "<=" || token == "==" {
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
        _ => 0,
    }
}

fn is_argument_terminator(arg: &Node) -> bool {
    matches!(arg, Node::ClosingParen | Node::ClosingBrace)
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
        } else if token == ">" || token == "<" || token == ">=" || token == "<=" || token == "==" {
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
                "==" => Node::Equal(Box::new(left), Box::new(right)), // Optional for equality
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
