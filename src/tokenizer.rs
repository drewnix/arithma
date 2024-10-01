use std::iter::Peekable;
use std::str::Chars;

pub struct Tokenizer<'a> {
    chars: Peekable<Chars<'a>>,
}

impl<'a> Tokenizer<'a> {
    /// Create a new instance of Tokenizer with input expression
    pub fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().peekable(),
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
            if c.is_digit(10) || c == '.' {
                self.tokenize_numbers(&mut tokens, &mut current_token, c);
            }
            // Handle LaTeX commands
            else if c == '\\' {
                self.tokenize_latex_commands(&mut tokens, &mut current_token);
            }
            // Handle operators and parentheses
            else if "+*/^(){}".contains(c) {
                self.tokenize_operator_or_paren(&mut tokens, &mut current_token, c);
            }
            // Handle comparison operators like >, <, >=, <=, and ==
            else if c == '>' || c == '<' || c == '=' || c == '&' {
                self.tokenize_comparisons(&mut tokens, c);
            }
            // Handle alphabetic variables like x, y, etc.
            else if c.is_alphabetic() {
                if let Some(last) = last_token.as_ref() {
                    if last.chars().all(char::is_numeric) {
                        tokens.push("*".to_string()); // Implicit multiplication before variables
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
            if next_char.is_digit(10) || next_char == '.' {
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
        while let Some(&next_char) = self.chars.peek() {
            if next_char.is_alphabetic() {
                current_token.push(next_char);
                self.chars.next();
            } else {
                break;
            }
        }

        let stripped_token = current_token.trim_start_matches('\\').to_string();

        match stripped_token.as_str() {
            "pi" => tokens.push(std::f64::consts::PI.to_string()),
            "mathrm" => {
                if let Some('{') = self.chars.peek() {
                    self.chars.next(); // Consume the opening brace
                    if let Some('e') = self.chars.peek() {
                        tokens.push(std::f64::consts::E.to_string()); // Euler's number
                        self.chars.next();
                        if let Some('}') = self.chars.peek() {
                            self.chars.next(); // Consume the closing brace
                        }
                    }
                }
            }
            "cdot" => {
                tokens.push("*".to_string());
            }
            "frac" => {
                if let Some(&next_char) = self.chars.peek() {
                    // Check if next char is a digit, indicating shorthand fraction \frac23
                    if next_char.is_digit(10) {
                        current_token.clear();
                        self.tokenize_shorthand_fraction(tokens);
                    } else {
                        self.chars.next(); // Consume '{'
                        let numerator = self.tokenize_group();
                        tokens.push(numerator);
                        tokens.push("/".to_string());

                        if self.chars.peek() == Some(&'{') {
                            self.chars.next(); // Consume '{'
                            let denominator = self.tokenize_group();
                            tokens.push(denominator);
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
            _ => tokens.push(stripped_token),
        }
        current_token.clear();
    }

    /// Handle shorthand fraction (like \frac23)
    fn tokenize_shorthand_fraction(&mut self, tokens: &mut Vec<String>) {
        if let Some(numerator_char) = self.chars.next() {
            if numerator_char.is_digit(10) {
                tokens.push(numerator_char.to_string());
            } else {
                return;
            }

            if let Some(denominator_char) = self.chars.next() {
                if denominator_char.is_digit(10) {
                    tokens.push("/".to_string());
                    tokens.push(denominator_char.to_string());
                } else {
                    return;
                }
            }
        }
    }

    /// Handle variables or function names like x, sin, cos
    fn tokenize_variable_or_function(&mut self, tokens: &mut Vec<String>, current_token: &mut String) {
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
    fn tokenize_operator_or_paren(&self, tokens: &mut Vec<String>, current_token: &mut String, c: char) {
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
            if next_char == '=' || (c == '&' && next_char == '&') || (c == '|' && next_char == '|') {
                op.push(next_char);
                self.chars.next();
            }
        }
        tokens.push(op);
    }

    /// Handle the minus '-' sign, distinguishing between unary and binary usage
    fn tokenize_minus(&mut self, tokens: &mut Vec<String>, last_token: &Option<String>) {
        let is_unary =
            last_token.is_none() || "+-*/^({ABS_START".contains(last_token.as_deref().unwrap_or(""));
        if is_unary {
            tokens.push("NEG".to_string()); // Tokenize unary minus as "NEG"
        } else {
            tokens.push("-".to_string()); // Tokenize binary minus as "-"
        }
    }

    fn tokenize_group(&mut self) -> String {
        let mut group = String::new();
        while let Some(&next_char) = self.chars.peek() {
            if next_char == '}' {
                self.chars.next(); // Consume '}'
                break;
            }
            group.push(next_char);
            self.chars.next();
        }
        group
    }
}

#[cfg(test)]
mod tests {
    use super::Tokenizer;

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
        assert_eq!(tokens, vec![std::f64::consts::PI.to_string(), "*".to_string(), "2".to_string()]);
    }

    #[test]
    fn test_tokenize_latex_euler() {
        let mut tokenizer = Tokenizer::new("\\mathrm{e} * 2");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec![std::f64::consts::E.to_string(), "*".to_string(), "2".to_string()]);
    }

    #[test]
    fn test_tokenize_latex_fraction() {
        let mut tokenizer = Tokenizer::new("\\frac{3}{4}");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["3", "/", "4"]);
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
    fn test_tokenize_function_call() {
        let mut tokenizer = Tokenizer::new("\\sin(x)");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["sin", "(", "x", ")"]);
    }

    #[test]
    fn test_tokenize_nested_parentheses() {
        let mut tokenizer = Tokenizer::new("(3 + (2 * (4 / 2)))");
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec!["(", "3", "+", "(", "2", "*", "(", "4", "/", "2", ")", ")", ")"]);
    }
}
