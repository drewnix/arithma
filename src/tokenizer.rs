use std::iter::Peekable;
use std::str::Chars;

use crate::functions::FUNCTION_REGISTRY;

fn is_decimal_char(c: char) -> bool {
    c.is_ascii_digit() || c == '.'
}

fn is_decimal_literal(token: &str) -> bool {
    !token.is_empty() && token.chars().all(is_decimal_char)
}

fn is_variable_token(token: &str) -> bool {
    !token.is_empty()
        && token.chars().all(|c| c.is_alphabetic())
        && FUNCTION_REGISTRY.get(token).is_none()
        && token != "NEG"
        && token != "sum"
        && !matches!(
            token,
            "int" | "prod" | "oint" | "iint" | "iiint" | "lim" | "nabla" | "infty"
        )
}

/// True when `tokens` ends with `}` closing a `_{…}` or `^{…}` script group.
fn closes_script_bound(tokens: &[String]) -> bool {
    if tokens.last().map(|t| t.as_str()) != Some("}") {
        return false;
    }
    let mut depth = 1i32;
    for i in (0..tokens.len().saturating_sub(1)).rev() {
        match tokens[i].as_str() {
            "}" => depth += 1,
            "{" => {
                depth -= 1;
                if depth == 0 {
                    return i > 0 && matches!(tokens[i - 1].as_str(), "^" | "_");
                }
            }
            _ => {}
        }
    }
    false
}

/// Prior token is an unbraced `^` / `_` script argument (e.g. `\sum^3{…}`).
fn follows_script_operator(tokens: &[String]) -> bool {
    tokens.len() >= 2 && matches!(tokens[tokens.len() - 2].as_str(), "^" | "_")
}

/// Prior token can bind implicitly with a following `{` group.
fn needs_implicit_mul_before_brace(last: &str, tokens: &[String]) -> bool {
    if follows_script_operator(tokens) {
        return false;
    }
    if last == "}" && closes_script_bound(tokens) {
        return false;
    }
    last == ")" || last == "}" || is_decimal_literal(last) || is_variable_token(last)
}

fn greek_letter(name: &str) -> Option<char> {
    match name {
        "pi" => Some('π'),
        "alpha" => Some('α'),
        "beta" => Some('β'),
        "gamma" => Some('γ'),
        "delta" => Some('δ'),
        "epsilon" | "varepsilon" => Some('ε'),
        "zeta" => Some('ζ'),
        "eta" => Some('η'),
        "theta" | "vartheta" => Some('θ'),
        "iota" => Some('ι'),
        "kappa" => Some('κ'),
        "lambda" => Some('λ'),
        "mu" => Some('μ'),
        "nu" => Some('ν'),
        "xi" => Some('ξ'),
        "rho" | "varrho" => Some('ρ'),
        "sigma" | "varsigma" => Some('σ'),
        "tau" => Some('τ'),
        "upsilon" => Some('υ'),
        "phi" | "varphi" => Some('φ'),
        "chi" => Some('χ'),
        "psi" => Some('ψ'),
        "omega" => Some('ω'),
        "Gamma" => Some('Γ'),
        "Delta" => Some('Δ'),
        "Theta" => Some('Θ'),
        "Lambda" => Some('Λ'),
        "Xi" => Some('Ξ'),
        "Sigma" => Some('Σ'),
        "Phi" => Some('Φ'),
        "Psi" => Some('Ψ'),
        "Omega" => Some('Ω'),
        _ => None,
    }
}

fn is_trig_or_hyperbolic(name: &str) -> bool {
    matches!(
        name,
        // Circular trigonometric
        "sin"
        | "cos"
        | "tan"
        // Reciprocal trigonometric
        | "csc"
        | "sec"
        | "cot"
        // Inverse circular trigonometric
        | "arcsin"
        | "arccos"
        | "arctan"
        // Inverse reciprocal trigonometric
        | "arccsc"
        | "arcsec"
        | "arccot"
        // Hyperbolic
        | "sinh"
        | "cosh"
        | "tanh"
        // Reciprocal hyperbolic
        | "csch"
        | "sech"
        | "coth"
        // Inverse hyperbolic
        | "arcsinh"
        | "arccosh"
        | "arctanh"
        // Inverse reciprocal hyperbolic
        | "arccsch"
        | "arcsech"
        | "arccoth"
    )
}

fn is_log_or_exp(name: &str) -> bool {
    matches!(name, "log" | "ln" | "lg" | "exp")
}

pub fn latex_name(c: char) -> Option<&'static str> {
    match c {
        'π' => Some("\\pi"),
        'α' => Some("\\alpha"),
        'β' => Some("\\beta"),
        'γ' => Some("\\gamma"),
        'δ' => Some("\\delta"),
        'ε' => Some("\\epsilon"),
        'ζ' => Some("\\zeta"),
        'η' => Some("\\eta"),
        'θ' => Some("\\theta"),
        'ι' => Some("\\iota"),
        'κ' => Some("\\kappa"),
        'λ' => Some("\\lambda"),
        'μ' => Some("\\mu"),
        'ν' => Some("\\nu"),
        'ξ' => Some("\\xi"),
        'ρ' => Some("\\rho"),
        'σ' => Some("\\sigma"),
        'τ' => Some("\\tau"),
        'υ' => Some("\\upsilon"),
        'φ' => Some("\\phi"),
        'χ' => Some("\\chi"),
        'ψ' => Some("\\psi"),
        'ω' => Some("\\omega"),
        'Γ' => Some("\\Gamma"),
        'Δ' => Some("\\Delta"),
        'Θ' => Some("\\Theta"),
        'Λ' => Some("\\Lambda"),
        'Ξ' => Some("\\Xi"),
        'Σ' => Some("\\Sigma"),
        'Φ' => Some("\\Phi"),
        'Ψ' => Some("\\Psi"),
        'Ω' => Some("\\Omega"),
        _ => None,
    }
}

pub fn normalize_var(name: &str) -> String {
    if let Some(stripped) = name.strip_prefix('\\') {
        if let Some(ch) = greek_letter(stripped) {
            return ch.to_string();
        }
    }
    if let Some(ch) = greek_letter(name) {
        return ch.to_string();
    }
    name.to_string()
}

pub struct Tokenizer<'a> {
    chars: Peekable<Chars<'a>>,
    pub errors: Vec<String>,
}

impl<'a> Tokenizer<'a> {
    /// Create a new instance of Tokenizer with input expression
    pub fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().peekable(),
            errors: Vec::new(),
        }
    }

    /// Tokenize the input string into individual tokens
    pub fn tokenize(&mut self) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current_token = String::new();
        let mut last_token: Option<String> = None;

        while let Some(c) = self.chars.next() {
            if c.is_whitespace() {
                continue; // Skip whitespace
            }

            // Handle numbers
            if is_decimal_char(c) {
                if let Some(last) = last_token.as_ref() {
                    if last == ")" {
                        tokens.push("*".to_string());
                    }
                }
                self.tokenize_numbers(&mut tokens, &mut current_token, c);
            }
            // Handle LaTeX commands
            else if c == '\\' {
                self.tokenize_latex_commands(&mut tokens, &mut current_token);
            }
            // Handle operators and parentheses
            else if "+*/(){}".contains(c) {
                if c == '(' {
                    if let Some(last) = last_token.as_ref() {
                        if last == ")" || is_decimal_literal(last) || is_variable_token(last) {
                            tokens.push("*".to_string());
                        }
                    }
                } else if c == '{' {
                    if let Some(last) = last_token.as_ref() {
                        if needs_implicit_mul_before_brace(last, &tokens) {
                            tokens.push("*".to_string());
                        }
                    }
                }
                self.tokenize_operator_or_paren(&mut tokens, &mut current_token, c);
            }
            // Handle special tokens for summation bounds
            else if c == '_' || c == '^' {
                self.tokenize_special_tokens(&mut tokens, &mut current_token, c);
            }
            // Handle single equals sign for equations
            else if c == '=' {
                self.tokenize_equation(&mut tokens, &mut current_token, c);
            }
            // Handle comparison operators like >, <, >=, <=, and ==
            else if c == '>' || c == '<' {
                self.tokenize_comparisons(&mut tokens, c);
            }
            // Handle matrix cell separator '&' or logical AND '&&'
            else if c == '&' {
                if !current_token.is_empty() {
                    tokens.push(current_token.clone());
                    current_token.clear();
                }
                self.handle_double_ampersand(&mut tokens);
            }
            // Handle alphabetic variables like x, y, etc.
            else if c.is_alphabetic() {
                if let Some(last) = last_token.as_ref() {
                    if is_decimal_literal(last) || last == ")" || is_variable_token(last) {
                        tokens.push("*".to_string());
                    }
                }
                current_token.push(c);
                self.tokenize_variable_or_function(&mut tokens, &mut current_token);
                current_token.clear();
            }
            // Special handling for minus '-'
            else if c == '-' {
                self.tokenize_minus(&mut tokens, &last_token);
            }

            last_token = tokens.last().cloned();
        }

        tokens
    }

    /// Handle numeric tokenization
    fn tokenize_numbers(&mut self, tokens: &mut Vec<String>, current_token: &mut String, c: char) {
        current_token.push(c);
        while let Some(&next_char) = self.chars.peek() {
            if is_decimal_char(next_char) {
                current_token.push(next_char);
                self.chars.next(); // Move the iterator forward
            } else {
                break;
            }
        }
        tokens.push(current_token.clone());
        current_token.clear();
    }

    /// Handle LaTeX commands like \frac, \pi, \mathrm{e}
    fn tokenize_latex_commands(&mut self, tokens: &mut Vec<String>, current_token: &mut String) {
        current_token.push('\\');

        // LaTeX single-character spacing commands: \, \; \! \:
        // These are non-alphabetic, so the loop below would leave stripped_token empty.
        // Consume the character and return early.
        if let Some(&next_char) = self.chars.peek() {
            if matches!(next_char, ',' | ';' | '!' | ':') {
                self.chars.next(); // consume the spacing character
                current_token.clear();
                return;
            }
        }

        while let Some(&next_char) = self.chars.peek() {
            if next_char.is_alphabetic() {
                current_token.push(next_char);
                self.chars.next();
            } else {
                break;
            }
        }

        let stripped_token = current_token.trim_start_matches('\\').to_string();

        // Implicit multiplication: x\sin(x), 2\frac{1}{2}, )\cos(x)
        if let Some(last) = tokens.last() {
            let needs_mul = last == ")"
                || is_decimal_literal(last)
                || (last.len() == 1 && last.chars().next().is_some_and(|c| c.is_alphabetic()));
            let is_value_producing = is_trig_or_hyperbolic(&stripped_token)
                || is_log_or_exp(&stripped_token)
                || matches!(stripped_token.as_str(), "sqrt" | "frac")
                || greek_letter(&stripped_token).is_some();
            if needs_mul && is_value_producing {
                tokens.push("*".to_string());
            }
        }

        // Handle \sin^2(x) → sin(x)^2 pattern for known trig/math functions
        if (is_trig_or_hyperbolic(&stripped_token) || is_log_or_exp(&stripped_token))
            && self.chars.peek() == Some(&'^')
        {
            self.chars.next(); // consume '^'
                               // Read exponent: either {group} or single character
            let power_str = if self.chars.peek() == Some(&'{') {
                self.chars.next(); // consume '{'
                self.consume_brace_group().unwrap_or_default()
            } else if let Some(&c) = self.chars.peek() {
                self.chars.next();
                c.to_string()
            } else {
                String::new()
            };
            // Skip whitespace before argument
            while self.chars.peek().is_some_and(|c| c.is_whitespace()) {
                self.chars.next();
            }
            // If next char is '(', consume the balanced paren group and reorder
            if self.chars.peek() == Some(&'(') {
                self.chars.next(); // consume '('
                let mut depth = 1;
                let mut arg_str = String::new();
                while let Some(&ch) = self.chars.peek() {
                    self.chars.next();
                    if ch == '(' {
                        depth += 1;
                    }
                    if ch == ')' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    arg_str.push(ch);
                }
                // Emit: ( func ( arg_tokens ) ) ^ power
                let arg_tokens = Tokenizer::new(&arg_str).tokenize();
                tokens.push("(".to_string());
                tokens.push(stripped_token.clone());
                tokens.push("(".to_string());
                tokens.extend(arg_tokens);
                tokens.push(")".to_string());
                tokens.push(")".to_string());
                tokens.push("^".to_string());
                tokens.push(power_str);
                current_token.clear();
                return;
            }
        }

        match stripped_token.as_str() {
            "mathrm" => {
                if let Some('{') = self.chars.peek() {
                    self.chars.next();
                    if let Some('e') = self.chars.peek() {
                        tokens.push("e".to_string());
                        self.chars.next();
                        if let Some('}') = self.chars.peek() {
                            self.chars.next();
                        }
                    }
                }
            }
            "cdot" | "times" => {
                tokens.push("*".to_string());
            }
            "div" => {
                tokens.push("/".to_string());
            }
            "geq" | "ge" => {
                tokens.push(">=".to_string());
            }
            "leq" | "le" => {
                tokens.push("<=".to_string());
            }
            "gt" => {
                tokens.push(">".to_string());
            }
            "lt" => {
                tokens.push("<".to_string());
            }
            "frac" => {
                current_token.clear();
                if let Some(&next_char) = self.chars.peek() {
                    if next_char.is_ascii_digit() {
                        self.tokenize_shorthand_fraction(tokens);
                    } else if next_char == '{' {
                        self.chars.next();
                        if let Some(numer_str) = self.consume_brace_group() {
                            while let Some(&c) = self.chars.peek() {
                                if c.is_whitespace() {
                                    self.chars.next();
                                } else {
                                    break;
                                }
                            }
                            if self.chars.peek() == Some(&'{') {
                                self.chars.next();
                                if let Some(denom_str) = self.consume_brace_group() {
                                    let nt = numer_str.trim();
                                    let dt = denom_str.trim();
                                    if nt == "d"
                                        && dt.starts_with('d')
                                        && dt[1..].chars().all(|c| c.is_alphabetic())
                                    {
                                        self.errors.push(format!(
                                            "Leibniz derivative notation \\frac{{d}}{{{}}} is not supported as an expression. Use the 'differentiate' tool instead.",
                                            dt
                                        ));
                                        return;
                                    }
                                    if nt == "\\partial" && dt.starts_with("\\partial") {
                                        self.errors.push(format!(
                                            "Partial derivative notation \\frac{{\\partial}}{{{}}} is not supported as an expression. Use the 'differentiate' tool instead.",
                                            dt
                                        ));
                                        return;
                                    }
                                    let numer_tokens = Tokenizer::new(&numer_str).tokenize();
                                    let denom_tokens = Tokenizer::new(&denom_str).tokenize();
                                    tokens.push("(".to_string());
                                    tokens.extend(numer_tokens);
                                    tokens.push(")".to_string());
                                    tokens.push("/".to_string());
                                    tokens.push("(".to_string());
                                    tokens.extend(denom_tokens);
                                    tokens.push(")".to_string());
                                }
                            }
                        }
                    }
                }
            }
            // Handle absolute value delimiters \left| and \right|
            "left" => {
                if let Some('|') = self.chars.peek() {
                    tokens.push("ABS_START".to_string());
                    self.chars.next(); // Consume the '|'
                }
            }
            "right" => {
                if let Some('|') = self.chars.peek() {
                    tokens.push("ABS_END".to_string());
                    self.chars.next(); // Consume the '|'
                }
            }
            "sum" => {
                tokens.push("sum".to_string());
                // The tokenizer will continue with the _ and ^ tokens handled separately
            }
            "log" => {
                // \log_b(x) or \log_{b}(x) → ln(x)/ln(b)
                // \log(x) → log(x) as before (base-10)
                if self.chars.peek() == Some(&'_') {
                    self.chars.next(); // consume '_'
                                       // Read base: either {group} or single char
                    let base_str = if self.chars.peek() == Some(&'{') {
                        self.chars.next();
                        self.consume_brace_group().unwrap_or_default()
                    } else if let Some(&c) = self.chars.peek() {
                        self.chars.next();
                        c.to_string()
                    } else {
                        String::new()
                    };
                    // Skip whitespace before argument
                    while self.chars.peek().is_some_and(|c| c.is_whitespace()) {
                        self.chars.next();
                    }
                    // Read argument: (group) or {group}
                    let arg_str = if self.chars.peek() == Some(&'(') {
                        self.chars.next();
                        let mut depth = 1;
                        let mut s = String::new();
                        while let Some(&c) = self.chars.peek() {
                            self.chars.next();
                            if c == '(' {
                                depth += 1;
                            } else if c == ')' {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            s.push(c);
                        }
                        s
                    } else if self.chars.peek() == Some(&'{') {
                        self.chars.next();
                        self.consume_brace_group().unwrap_or_default()
                    } else {
                        String::new()
                    };
                    if !base_str.is_empty() && !arg_str.is_empty() {
                        let base_tokens = Tokenizer::new(&base_str).tokenize();
                        let arg_tokens = Tokenizer::new(&arg_str).tokenize();
                        // Emit ln(arg)/ln(base)
                        tokens.push("(".to_string());
                        tokens.push("ln".to_string());
                        tokens.push("(".to_string());
                        tokens.extend(arg_tokens);
                        tokens.push(")".to_string());
                        tokens.push(")".to_string());
                        tokens.push("/".to_string());
                        tokens.push("(".to_string());
                        tokens.push("ln".to_string());
                        tokens.push("(".to_string());
                        tokens.extend(base_tokens);
                        tokens.push(")".to_string());
                        tokens.push(")".to_string());
                    }
                } else {
                    tokens.push("log".to_string());
                }
            }
            "sqrt" => {
                // \sqrt[n]{x} → (x)^(1/(n)), \sqrt{x} → sqrt(x) as before
                if self.chars.peek() == Some(&'[') {
                    self.chars.next(); // consume '['
                    let mut degree_str = String::new();
                    while let Some(&c) = self.chars.peek() {
                        if c == ']' {
                            self.chars.next();
                            break;
                        }
                        degree_str.push(c);
                        self.chars.next();
                    }
                    // consume optional whitespace then the {radicand}
                    while self.chars.peek().is_some_and(|c| c.is_whitespace()) {
                        self.chars.next();
                    }
                    if self.chars.peek() == Some(&'{') {
                        self.chars.next();
                        if let Some(radicand_str) = self.consume_brace_group() {
                            let radicand_tokens = Tokenizer::new(&radicand_str).tokenize();
                            let degree_tokens = Tokenizer::new(&degree_str).tokenize();
                            // Emit (radicand)^(1/(degree))
                            tokens.push("(".to_string());
                            tokens.extend(radicand_tokens);
                            tokens.push(")".to_string());
                            tokens.push("^".to_string());
                            tokens.push("(".to_string());
                            tokens.push("1".to_string());
                            tokens.push("/".to_string());
                            tokens.push("(".to_string());
                            tokens.extend(degree_tokens);
                            tokens.push(")".to_string());
                            tokens.push(")".to_string());
                        }
                    }
                } else {
                    // Plain \sqrt{x} — emit "sqrt" for the parser to handle
                    tokens.push("sqrt".to_string());
                }
            }
            // LaTeX spacing — silently ignore
            "," | ";" | "!" | ":" | "quad" | "qquad" | "enspace" | "thinspace" => {
                current_token.clear();
            }
            _ => {
                if let Some(ch) = greek_letter(&stripped_token) {
                    tokens.push(ch.to_string());
                } else {
                    tokens.push(stripped_token);
                }
            }
        }
        current_token.clear();
    }

    fn consume_brace_group(&mut self) -> Option<String> {
        let mut depth = 1;
        let mut content = String::new();
        for c in self.chars.by_ref() {
            if c == '{' {
                depth += 1;
            } else if c == '}' {
                depth -= 1;
                if depth == 0 {
                    return Some(content);
                }
            }
            content.push(c);
        }
        None
    }

    /// Handle shorthand fraction (like \frac23)
    fn tokenize_shorthand_fraction(&mut self, tokens: &mut Vec<String>) {
        if let Some(numerator_char) = self.chars.next() {
            if numerator_char.is_ascii_digit() {
                tokens.push(numerator_char.to_string());
            } else {
                return;
            }

            if let Some(denominator_char) = self.chars.next() {
                if denominator_char.is_ascii_digit() {
                    tokens.push("/".to_string());
                    tokens.push(denominator_char.to_string());
                }
            }
        }
    }

    /// Handle variables or function names like x, sin, cos
    fn tokenize_variable_or_function(
        &mut self,
        tokens: &mut Vec<String>,
        current_token: &mut String,
    ) {
        while let Some(&next_char) = self.chars.peek() {
            if next_char.is_alphanumeric() {
                current_token.push(next_char);
                self.chars.next();
            } else {
                break;
            }
        }
        tokens.push(current_token.clone());
    }

    /// Handle operators and parentheses
    fn tokenize_operator_or_paren(
        &self,
        tokens: &mut Vec<String>,
        current_token: &mut String,
        c: char,
    ) {
        if !current_token.is_empty() {
            tokens.push(current_token.clone());
            current_token.clear();
        }
        tokens.push(c.to_string());
    }

    /// Handle special tokens like underscore and caret for summation bounds
    fn tokenize_special_tokens(
        &mut self,
        tokens: &mut Vec<String>,
        current_token: &mut String,
        c: char,
    ) {
        if !current_token.is_empty() {
            tokens.push(current_token.clone());
            current_token.clear();
        }
        tokens.push(c.to_string());
    }

    /// Handle comparison operators like >, <, >=, <=, ==
    fn tokenize_comparisons(&mut self, tokens: &mut Vec<String>, c: char) {
        let mut op = c.to_string();
        if let Some(&next_char) = self.chars.peek() {
            if next_char == '=' || (c == '|' && next_char == '|') {
                op.push(next_char);
                self.chars.next();
            }
        }
        tokens.push(op);
    }

    /// Special handler for &&
    fn handle_double_ampersand(&mut self, tokens: &mut Vec<String>) {
        // Check if the next char is also &
        if let Some(&next_char) = self.chars.peek() {
            if next_char == '&' {
                self.chars.next(); // Consume the second &
                tokens.push("&&".to_string());
                return;
            }
        }

        // If not a double ampersand, just push a single &
        tokens.push("&".to_string());
    }

    /// Handle equation with '=' sign
    fn tokenize_equation(&mut self, tokens: &mut Vec<String>, current_token: &mut String, c: char) {
        if !current_token.is_empty() {
            tokens.push(current_token.clone());
            current_token.clear();
        }

        let mut op = c.to_string();
        // Check if it's a double equals (==) for comparison
        if let Some(&next_char) = self.chars.peek() {
            if next_char == '=' {
                op.push(next_char);
                self.chars.next();
            }
        }
        tokens.push(op);
    }

    /// Handle the minus '-' sign, distinguishing between unary and binary usage
    fn tokenize_minus(&mut self, tokens: &mut Vec<String>, last_token: &Option<String>) {
        let is_unary = last_token.is_none()
            || "+-*/^({ABS_START".contains(last_token.as_deref().unwrap_or(""));
        if is_unary {
            tokens.push("NEG".to_string()); // Tokenize unary minus as "NEG"
        } else {
            tokens.push("-".to_string()); // Tokenize binary minus as "-"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{is_variable_token, Tokenizer};

    #[test]
    fn test_tokenize_numbers() {
        let mut tokenizer = Tokenizer::new("123 45.67");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["123", "45.67"]);
    }

    #[test]
    fn test_tokenize_basic_operators() {
        let mut tokenizer = Tokenizer::new("3 + 4 * 10 / 5");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["3", "+", "4", "*", "10", "/", "5"]);
    }

    #[test]
    fn test_tokenize_negative_numbers() {
        let mut tokenizer = Tokenizer::new("-5 + 3 - -2");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["NEG", "5", "+", "3", "-", "NEG", "2"]);
    }

    #[test]
    fn test_tokenize_latex_pi() {
        let mut tokenizer = Tokenizer::new("\\pi * 2");
        let tokens = tokenizer.tokenize();
        assert_eq!(
            tokens,
            vec!["π".to_string(), "*".to_string(), "2".to_string()]
        );
    }

    #[test]
    fn test_tokenize_latex_euler() {
        let mut tokenizer = Tokenizer::new("\\mathrm{e} * 2");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["e", "*", "2"]);
    }

    #[test]
    fn test_tokenize_latex_times() {
        let mut tokenizer = Tokenizer::new(r"4 \times 2");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["4", "*", "2"]);
    }

    #[test]
    fn test_tokenize_latex_cdot() {
        let mut tokenizer = Tokenizer::new(r"4 \cdot 2");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["4", "*", "2"]);
    }

    #[test]
    fn test_tokenize_latex_div() {
        let mut tokenizer = Tokenizer::new(r"10 \div 2");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["10", "/", "2"]);
    }

    #[test]
    fn test_tokenize_latex_fraction() {
        let mut tokenizer = Tokenizer::new("\\frac{3}{4}");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["(", "3", ")", "/", "(", "4", ")"]);
    }

    #[test]
    fn test_tokenize_latex_shorthand_fraction() {
        let mut tokenizer = Tokenizer::new("\\frac34");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["3", "/", "4"]);
    }

    #[test]
    fn test_tokenize_comparison_operators() {
        let mut tokenizer = Tokenizer::new("5 > 3 && 4 <= 10");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["5", ">", "3", "&&", "4", "<=", "10"]);
    }

    #[test]
    fn test_tokenize_absolute_value() {
        let mut tokenizer = Tokenizer::new("\\left|x + 3\\right|");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["ABS_START", "x", "+", "3", "ABS_END"]);
    }

    #[test]
    fn test_tokenize_implicit_multiplication() {
        let mut tokenizer = Tokenizer::new("2x + 3y");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["2", "*", "x", "+", "3", "*", "y"]);
    }

    #[test]
    fn test_tokenize_implicit_multiplication_decimal() {
        let mut tokenizer = Tokenizer::new("0.3x + .4y");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["0.3", "*", "x", "+", ".4", "*", "y"]);
    }

    #[test]
    fn test_tokenize_implicit_mul_frac_var() {
        // \frac{1}{3}x → (1)/(3) * x
        let mut tokenizer = Tokenizer::new("\\frac{1}{3}x");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["(", "1", ")", "/", "(", "3", ")", "*", "x"]);
    }

    #[test]
    fn test_tokenize_implicit_mul_paren_paren() {
        // (x+1)(x-1) → (x+1)*(x-1)
        let mut tokenizer = Tokenizer::new("(x+1)(x-1)");
        let tokens = tokenizer.tokenize();
        assert_eq!(
            tokens,
            vec!["(", "x", "+", "1", ")", "*", "(", "x", "-", "1", ")"]
        );
    }

    #[test]
    fn test_tokenize_implicit_mul_number_paren() {
        // 2(x+1) → 2*(x+1)
        let mut tokenizer = Tokenizer::new("2(x+1)");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["2", "*", "(", "x", "+", "1", ")"]);
    }

    #[test]
    fn test_tokenize_implicit_mul_paren_number() {
        // (x+1)3 → (x+1)*3
        let mut tokenizer = Tokenizer::new("(x+1)3");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["(", "x", "+", "1", ")", "*", "3"]);
    }

    #[test]
    fn test_tokenize_function_no_implicit_mul() {
        // \sin(x) should NOT get implicit multiplication
        let mut tokenizer = Tokenizer::new("\\sin(x)");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["sin", "(", "x", ")"]);
    }

    #[test]
    fn test_tokenize_function_call() {
        let mut tokenizer = Tokenizer::new("\\sin(x)");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["sin", "(", "x", ")"]);
    }

    #[test]
    fn test_tokenize_nested_parentheses() {
        let mut tokenizer = Tokenizer::new("(3 + (2 * (4 / 2)))");
        let tokens = tokenizer.tokenize();
        assert_eq!(
            tokens,
            vec!["(", "3", "+", "(", "2", "*", "(", "4", "/", "2", ")", ")", ")"]
        );
    }

    #[test]
    fn test_tokenize_sin_power_before_arg() {
        // \sin^2(x) should reorder to (sin(x))^2
        let mut tokenizer = Tokenizer::new("\\sin^2(x)");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["(", "sin", "(", "x", ")", ")", "^", "2"]);
    }

    #[test]
    fn test_tokenize_cos_power_brace_before_arg() {
        // \cos^{3}(x) should reorder to (cos(x))^3
        let mut tokenizer = Tokenizer::new("\\cos^{3}(x)");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["(", "cos", "(", "x", ")", ")", "^", "3"]);
    }

    #[test]
    fn test_tokenize_sin_power_after_arg_unchanged() {
        // \sin(x)^2 should remain sin(x)^2 (no reordering needed)
        let mut tokenizer = Tokenizer::new("\\sin(x)^2");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["sin", "(", "x", ")", "^", "2"]);
    }

    #[test]
    fn test_tokenize_sin_power_compound_arg() {
        // \sin^2(x + 1) should reorder to (sin(x + 1))^2
        let mut tokenizer = Tokenizer::new("\\sin^2(x + 1)");
        let tokens = tokenizer.tokenize();
        assert_eq!(
            tokens,
            vec!["(", "sin", "(", "x", "+", "1", ")", ")", "^", "2"]
        );
    }

    #[test]
    fn test_tokenize_greek_alpha() {
        let mut tokenizer = Tokenizer::new("\\alpha + 1");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["α", "+", "1"]);
    }

    #[test]
    fn test_tokenize_greek_implicit_mul() {
        let mut tokenizer = Tokenizer::new("3\\alpha + 4{\\beta}");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["3", "*", "α", "+", "4", "*", "{", "β", "}"]);
    }

    #[test]
    fn test_tokenize_summation_unbraced_upper_braced_body_no_implicit_mul() {
        let mut tokenizer = Tokenizer::new(r"\sum_{i=1}^3{i}");
        let tokens = tokenizer.tokenize();
        assert_eq!(
            tokens,
            vec!["sum", "_", "{", "i", "=", "1", "}", "^", "3", "{", "i", "}"]
        );
    }

    #[test]
    fn test_tokenize_summation_braced_body_no_implicit_mul() {
        let mut tokenizer = Tokenizer::new(r"\sum_{i=a}^{b} {i+c}");
        let tokens = tokenizer.tokenize();
        assert_eq!(
            tokens,
            vec!["sum", "_", "{", "i", "=", "a", "}", "^", "{", "b", "}", "{", "i", "+", "c", "}"]
        );
    }

    #[test]
    fn test_tokenize_greek_squared() {
        let mut tokenizer = Tokenizer::new("\\alpha^2 + \\beta");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["α", "^", "2", "+", "β"]);
    }

    #[test]
    fn test_normalize_var() {
        use super::normalize_var;
        assert_eq!(normalize_var("\\alpha"), "α");
        assert_eq!(normalize_var("alpha"), "α");
        assert_eq!(normalize_var("x"), "x");
        assert_eq!(normalize_var("\\lambda"), "λ");
    }

    // --- Parser hardening: implicit multiplication for variable-paren ---

    #[test]
    fn test_tokenize_var_paren_implicit_mul() {
        // u(3-2u) → u * (3 - 2*u), NOT function call
        let mut tokenizer = Tokenizer::new("u(3-2u)");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["u", "*", "(", "3", "-", "2", "*", "u", ")"]);
    }

    #[test]
    fn test_tokenize_var_paren_chain() {
        // x(x+1)(x-1) → x*(x+1)*(x-1)
        let mut tokenizer = Tokenizer::new("x(x+1)(x-1)");
        let tokens = tokenizer.tokenize();
        assert_eq!(
            tokens,
            vec!["x", "*", "(", "x", "+", "1", ")", "*", "(", "x", "-", "1", ")"]
        );
    }

    #[test]
    fn test_tokenize_greek_paren_implicit_mul() {
        // α(x+1) → α * (x + 1)
        let mut tokenizer = Tokenizer::new("\\alpha(x+1)");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["α", "*", "(", "x", "+", "1", ")"]);
    }

    #[test]
    fn test_tokenize_known_function_no_implicit_mul() {
        // sin(x) must NOT get implicit multiplication
        let mut tokenizer = Tokenizer::new("sin(x)");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["sin", "(", "x", ")"]);
    }

    #[test]
    fn test_tokenize_var_var_implicit_mul() {
        // x y → x * y (space-separated variables)
        let mut tokenizer = Tokenizer::new("x y");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["x", "*", "y"]);
    }

    #[test]
    fn test_tokenize_greek_var_implicit_mul() {
        // α b → α * b
        let mut tokenizer = Tokenizer::new("\\alpha b");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["α", "*", "b"]);
    }

    #[test]
    fn test_tokenize_multichar_var_preserved() {
        // xy stays as single token (multi-char variable name)
        let mut tokenizer = Tokenizer::new("xy");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["xy"]);
    }

    #[test]
    fn test_latex_operators_not_variables() {
        assert!(!is_variable_token("int"));
        assert!(!is_variable_token("prod"));
        assert!(!is_variable_token("oint"));
        assert!(!is_variable_token("lim"));
        // These should still be variables
        assert!(is_variable_token("x"));
        assert!(is_variable_token("a"));
        assert!(is_variable_token("alpha")); // Greek letters before lookup
    }

    #[test]
    fn test_latex_spacing_stripped() {
        // \, should be silently ignored, not produce empty token or error
        let mut t = Tokenizer::new("x \\, y");
        let tokens = t.tokenize();
        assert!(
            !tokens.contains(&String::new()),
            "Empty token from \\,: {:?}",
            tokens
        );
        assert!(
            tokens.contains(&"x".to_string()),
            "Should have x: {:?}",
            tokens
        );
        assert!(
            tokens.contains(&"y".to_string()),
            "Should have y: {:?}",
            tokens
        );

        // \quad should also be stripped
        let mut t2 = Tokenizer::new("x \\quad y");
        let tokens2 = t2.tokenize();
        assert!(
            !tokens2.contains(&String::new()),
            "Empty token from \\quad: {:?}",
            tokens2
        );
        assert!(tokens2.contains(&"x".to_string()));
        assert!(tokens2.contains(&"y".to_string()));

        // \; in a fraction should work
        let mut t3 = Tokenizer::new("\\frac{1}{x \\; + \\; 1}");
        let tokens3 = t3.tokenize();
        assert!(
            !tokens3.contains(&String::new()),
            "Empty token from \\;: {:?}",
            tokens3
        );
    }
}
