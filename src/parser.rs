use crate::exact::ExactNum;
use crate::functions::FUNCTION_REGISTRY;
use crate::node::Node;
use crate::simplify::Simplifiable;

pub fn shunting_yard(tokens: Vec<String>) -> Result<Vec<String>, String> {
    log::debug!("Starting Shunting Yard with tokens: {:?}", tokens);

    let mut output_queue: Vec<String> = Vec::new();
    let mut operator_stack: Vec<String> = Vec::new();

    for token in tokens {
        log::debug!("Processing token: {}", token);

        if token.parse::<f64>().is_ok()
            && token.starts_with(|c: char| c.is_ascii_digit() || c == '.')
        {
            log::debug!("Token is a number: {}", token);
            output_queue.push(token);
        } else if token.starts_with(INDEXED_ATOM_PREFIX) {
            // Pre-parsed \sum/\prod atom: a plain operand.
            output_queue.push(token);
        } else if token == "NEG" {
            log::debug!("Unary minus detected, pushing to operator stack");
            operator_stack.push(token);
        } else if token == "FACT" {
            output_queue.push(token);
        } else if token == "ABS_START" {
            operator_stack.push(token);
        } else if token == "ABS_END" {
            while let Some(op) = operator_stack.pop() {
                if op == "ABS_START" {
                    break;
                }
                output_queue.push(op);
            }
            output_queue.push("ABS".to_string());
        } else if token == "FLOOR_START" {
            operator_stack.push(token);
        } else if token == "FLOOR_END" {
            while let Some(op) = operator_stack.pop() {
                if op == "FLOOR_START" {
                    break;
                }
                output_queue.push(op);
            }
            output_queue.push("FLOOR".to_string());
        } else if token == "CEIL_START" {
            operator_stack.push(token);
        } else if token == "CEIL_END" {
            while let Some(op) = operator_stack.pop() {
                if op == "CEIL_START" {
                    break;
                }
                output_queue.push(op);
            }
            output_queue.push("CEIL".to_string());
        } else if token == ">"
            || token == "<"
            || token == ">="
            || token == "<="
            || token == "=="
            || token == "="
            || "+-*/^".contains(token.as_str())
        {
            while let Some(top) = operator_stack.last() {
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
            while let Some(top) = operator_stack.pop() {
                if top == "(" || top == "{" {
                    break;
                }
                output_queue.push(top);
            }
            if let Some(top) = operator_stack.pop_if(|top| FUNCTION_REGISTRY.get(top).is_some()) {
                output_queue.push(top);
            }
        } else if let Some(_function) = FUNCTION_REGISTRY.get(&token) {
            log::debug!("Function detected: {}", token);
            operator_stack.push(token);
        } else if token.chars().all(|c| c.is_alphabetic()) {
            log::debug!("Variable detected: {}", token);
            output_queue.push(token);
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
        "^" => 5,                            // Exponentiation
        "NEG" => 4,                          // Unary minus (binds tighter than *, looser than ^)
        "*" | "/" => 3,                      // Multiplication and Division
        "+" | "-" => 2,                      // Addition and Subtraction
        ">" | "<" | ">=" | "<=" | "==" => 1, // Inequality operators
        "=" => 0,                            // Equation has lowest precedence
        _ => 0,
    }
}

/// Marks a pre-parsed `\sum`/`\prod` atom in the token stream. A private-use
/// character the tokenizer can never emit, so placeholders cannot collide
/// with user input.
const INDEXED_ATOM_PREFIX: char = '\u{E000}';

pub fn build_expression_tree(tokens: Vec<String>) -> Result<Node, String> {
    let mut indexed_atoms: Vec<Node> = Vec::new();
    build_expression_tree_inner(tokens, &mut indexed_atoms)
}

fn build_expression_tree_inner(
    tokens: Vec<String>,
    indexed_atoms: &mut Vec<Node>,
) -> Result<Node, String> {
    log::debug!("Building expression tree from tokens: {:?}", tokens);

    // \sum and \prod parse as expression ATOMS: each construct is parsed
    // into a Node here and its token span replaced by a placeholder
    // operand, so indexed notation composes with the surrounding grammar
    // (2·Σk, 1 + Σk², Σ − Σ, nested sums). Rightmost-first, so inner
    // constructs resolve before the outer ones whose bodies contain them.
    // (Previously a whole-expression special case silently discarded
    // everything around the Σ/Π — the worst failure class for a CAS.)
    let mut tokens = tokens;
    while let Some(pos) = tokens.iter().rposition(|t| t == "sum" || t == "prod") {
        let kind = if tokens[pos] == "sum" {
            IndexedNotation::Sum
        } else {
            IndexedNotation::Prod
        };
        let (node, end) = parse_indexed_at(&tokens, pos, kind, indexed_atoms)?;
        let placeholder = format!("{}{}", INDEXED_ATOM_PREFIX, indexed_atoms.len());
        indexed_atoms.push(node);
        tokens.splice(pos..end, [placeholder]);
    }

    let rpn = shunting_yard(tokens)?;

    let mut stack: Vec<Node> = Vec::new();

    for token in rpn {
        log::debug!("Processing token: {}", token);

        if let Some(idx_str) = token.strip_prefix(INDEXED_ATOM_PREFIX) {
            let atom = idx_str
                .parse::<usize>()
                .ok()
                .and_then(|idx| indexed_atoms.get(idx).cloned())
                .ok_or_else(|| "Invalid indexed-notation placeholder".to_string())?;
            stack.push(atom);
        } else if token.starts_with(|c: char| c.is_ascii_digit() || c == '.') {
            if let Ok(num) = token.parse::<f64>() {
                log::debug!("Pushing number: {}", num);
                stack.push(Node::Num(ExactNum::from_f64(num)));
            }
        } else if token == "ABS" {
            let operand = stack
                .pop()
                .ok_or_else(|| "Not enough operands for ABS".to_string())?;
            stack.push(Node::Abs(Box::new(operand))); // Handle absolute value
        } else if token == "FLOOR" {
            let operand = stack
                .pop()
                .ok_or_else(|| "Not enough operands for floor".to_string())?;
            stack.push(Node::Floor(Box::new(operand)));
        } else if token == "CEIL" {
            let operand = stack
                .pop()
                .ok_or_else(|| "Not enough operands for ceil".to_string())?;
            stack.push(Node::Ceil(Box::new(operand)));
        } else if token == "NEG" {
            // Handle unary minus by applying it to the top of the stack
            let operand = stack
                .pop()
                .ok_or_else(|| "Not enough operands for unary minus".to_string())?;
            stack.push(Node::Negate(Box::new(operand)));
        } else if token == "FACT" {
            let operand = stack
                .pop()
                .ok_or_else(|| "Not enough operands for factorial".to_string())?;
            stack.push(Node::Factorial(Box::new(operand)));
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
                args.reverse();
                stack.push(Node::Function(token.clone(), args));
            } else {
                // Variable-argument function: collect all remaining stack items as arguments
                let mut args: Vec<Node> = std::mem::take(&mut stack);
                args.reverse();
                stack.push(Node::Function(token.clone(), args));
            }
        } else if token.chars().all(|c| c.is_alphabetic()) {
            // Handle variables directly (e.g., `x`, `y`)
            if token == "e" || token == "EULER" {
                stack.push(Node::Variable("e".to_string()));
            } else if token == "\\pi" || token == "PI" || token == "π" {
                stack.push(Node::Variable("π".to_string()));
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

/// Parse a LaTeX expression string into a Node AST (no simplification)
pub fn parse_latex_raw(latex: &str) -> Result<Node, String> {
    let mut tokenizer = crate::tokenizer::Tokenizer::new(latex);
    let tokens = tokenizer.tokenize();
    if let Some(err) = tokenizer.errors.into_iter().next() {
        return Err(err);
    }
    build_expression_tree(tokens)
}

/// Parse a LaTeX expression string into a Node AST and simplify using the environment.
/// If simplification fails, returns the parsed (unsimplified) expression.
pub fn parse_latex(latex: &str, env: &crate::environment::Environment) -> Result<Node, String> {
    let expr = parse_latex_raw(latex)?;
    Ok(expr.simplify(env).unwrap_or(expr))
}

enum IndexedNotation {
    Sum,
    Prod,
}

/// Parse indexed notation like \sum_{i=1}^{n} i or \prod_{k=1}^{n} k.
/// Parse one `\sum`/`\prod` construct beginning at `op_pos`. Returns the
/// node and the index one past the last token the construct consumed, so
/// the caller can splice the span out and let the surrounding expression
/// grammar see the construct as a single operand. Bounds and body parse
/// through `build_expression_tree_inner` with the shared atom table, so
/// nested constructs (already replaced by placeholders) resolve correctly.
fn parse_indexed_at(
    tokens: &[String],
    op_pos: usize,
    kind: IndexedNotation,
    indexed_atoms: &mut Vec<Node>,
) -> Result<(Node, usize), String> {
    let (op_token, op_label) = match kind {
        IndexedNotation::Sum => ("sum", "summation"),
        IndexedNotation::Prod => ("prod", "product"),
    };
    debug_assert_eq!(tokens[op_pos], op_token);

    // Check for underscore after operator
    if op_pos + 1 >= tokens.len() || tokens[op_pos + 1] != "_" {
        return Err(format!("Expected '_' after '{op_token}'"));
    }

    // Check for opening brace for lower bound
    if op_pos + 2 >= tokens.len() || tokens[op_pos + 2] != "{" {
        return Err("Expected '{' after '_'".to_string());
    }

    // Extract the index variable and starting value
    // Format is typically: i = 1
    let mut lower_bound_tokens = Vec::new();
    let mut i = op_pos + 3;
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

    // Extract the body expression (summand or multiplicand)
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
        // If there's no brace, extract the indexed body with advanced balancing
        parse_unbraced_indexed_body(&tokens[i..], &mut body_tokens, &mut i);
    }

    // Parse the start, end, and body expressions with better error handling
    let start_expr = build_expression_tree_inner(lower_bound_tokens, indexed_atoms)
        .map_err(|e| format!("Error in {op_label} lower bound: {e}"))?;

    let end_expr = build_expression_tree_inner(upper_bound_tokens, indexed_atoms)
        .map_err(|e| format!("Error in {op_label} upper bound: {e}"))?;

    // Debug logging for body tokens
    log::debug!("Body tokens for {op_label}: {:?}", body_tokens);

    let body_expr = build_expression_tree_inner(body_tokens, indexed_atoms)
        .map_err(|e| format!("Error in {op_label} body: {e}"))?;

    let indexed_node = match kind {
        IndexedNotation::Sum => Node::Summation(
            index_var,
            Box::new(start_expr),
            Box::new(end_expr),
            Box::new(body_expr),
        ),
        IndexedNotation::Prod => Node::Product(
            index_var,
            Box::new(start_expr),
            Box::new(end_expr),
            Box::new(body_expr),
        ),
    };

    // Tokens beyond `i` are NOT this construct's concern: the caller
    // splices the consumed span out and the expression grammar handles
    // the rest. (The old code returned the bare node here, silently
    // discarding surrounding context — 2·Σk evaluated to Σk.)
    Ok((indexed_node, i))
}

/// Parse an unbraced indexed-notation body, handling cases like i^2, i*j, etc.
fn parse_unbraced_indexed_body(tokens: &[String], body_tokens: &mut Vec<String>, pos: &mut usize) {
    let mut i = 0;
    let mut paren_depth: i32 = 0;

    while i < tokens.len() {
        match tokens[i].as_str() {
            // Paired delimiters all contribute to depth, so the top-level
            // +/- break below cannot split inside |k−3| or ⌊k/2⌋.
            "(" | "{" | "ABS_START" | "FLOOR_START" | "CEIL_START" => {
                paren_depth += 1;
                body_tokens.push(tokens[i].clone());
            }
            ")" | "}" | "ABS_END" | "FLOOR_END" | "CEIL_END" => {
                if paren_depth > 0 {
                    paren_depth -= 1;
                    body_tokens.push(tokens[i].clone());
                } else {
                    break;
                }
            }
            "=" | "sum" | "prod" | ">" | "<" | ">=" | "<=" => break,
            "+" | "-" if paren_depth == 0 && !body_tokens.is_empty() => break,
            _ => {
                body_tokens.push(tokens[i].clone());
            }
        }
        i += 1;
    }

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

#[cfg(test)]
mod format_simplify_tests {
    use super::{parse_latex, parse_latex_raw};
    use crate::environment::Environment;

    /// Same input as `arithma format` in CLI help.
    const MESSY: &str = r"\frac{2}{2+{\pi}}+.5{\pi}";

    fn format_latex(latex: &str) -> String {
        format!("{}", parse_latex_raw(latex).unwrap())
    }

    fn simplify_latex(latex: &str) -> String {
        let env = Environment::new();
        format!("{}", parse_latex(latex, &env).unwrap())
    }

    #[test]
    fn format_vs_simplify_on_messy_input() {
        assert_eq!(format_latex(MESSY), r"\frac{2}{2 + \pi} + 0.5 \cdot \pi");
        assert_eq!(
            simplify_latex(MESSY),
            r"\frac{\pi^{2} + 2\pi + 4}{2\pi + 4}"
        );
    }

    #[test]
    fn format_vs_simplify_on_literals() {
        assert_eq!(format_latex("1 + 1"), "1 + 1");
        assert_eq!(simplify_latex("1 + 1"), "2");
    }
}
