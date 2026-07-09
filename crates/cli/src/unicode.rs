/// Convert LaTeX math output to human-readable Unicode for terminal display.
pub fn latex_to_unicode(input: &str) -> String {
    let b = input.as_bytes();
    let len = b.len();
    let mut out = String::with_capacity(len);
    let mut i = 0;

    while i < len {
        if b[i] == b'\\' {
            let rest = &input[i..];

            // \frac{num}{den}
            if rest.starts_with("\\frac") || rest.starts_with("\\dfrac") {
                let cmd_len = if rest.starts_with("\\dfrac") { 6 } else { 5 };
                if let Some((end, text)) = convert_frac(input, i + cmd_len) {
                    out.push_str(&text);
                    i = end;
                    continue;
                }
            }

            // \sqrt[n]{expr} or \sqrt{expr}
            if rest.starts_with("\\sqrt") {
                if let Some((end, text)) = convert_sqrt(input, i + 5) {
                    out.push_str(&text);
                    i = end;
                    continue;
                }
            }

            // \mathbb{X}
            if rest.starts_with("\\mathbb{") {
                if let Some((close, content)) = extract_brace_group(input, i + 7) {
                    out.push_str(match content.as_str() {
                        "Q" => "ℚ",
                        "R" => "ℝ",
                        "Z" => "ℤ",
                        "N" => "ℕ",
                        "C" => "ℂ",
                        _ => &content,
                    });
                    i = close + 1;
                    continue;
                }
            }

            // \text{...} and \operatorname{...}
            if rest.starts_with("\\text{") {
                if let Some((close, content)) = extract_brace_group(input, i + 5) {
                    out.push_str(&content);
                    i = close + 1;
                    continue;
                }
            }
            if rest.starts_with("\\operatorname{") {
                if let Some((close, content)) = extract_brace_group(input, i + 14) {
                    out.push_str(&content);
                    i = close + 1;
                    continue;
                }
            }

            // \left / \right — drop the command, keep the delimiter
            if rest.starts_with("\\left") && !rest.starts_with("\\leftarrow") {
                i += 5;
                continue;
            }
            if rest.starts_with("\\right") && !rest.starts_with("\\rightarrow") {
                i += 6;
                continue;
            }

            // Simple word replacements (longest-first to avoid prefix conflicts)
            if let Some((advance, replacement)) = match_simple_command(rest) {
                out.push_str(replacement);
                i += advance;
                continue;
            }

            // Unknown command — pass through
            out.push('\\');
            i += 1;
            continue;
        }

        // ^{...} or ^c
        if b[i] == b'^' && i + 1 < len {
            if b[i + 1] == b'{' {
                if let Some((close, content)) = extract_brace_group(input, i + 1) {
                    let content_u = latex_to_unicode(&content);
                    out.push_str(&to_superscript(&content_u));
                    i = close + 1;
                    continue;
                }
            } else {
                let c = b[i + 1] as char;
                out.push_str(&to_superscript(&c.to_string()));
                i += 2;
                continue;
            }
        }

        // _{...} or _c
        if b[i] == b'_' && i + 1 < len {
            if b[i + 1] == b'{' {
                if let Some((close, content)) = extract_brace_group(input, i + 1) {
                    let content_u = latex_to_unicode(&content);
                    out.push_str(&to_subscript(&content_u));
                    i = close + 1;
                    continue;
                }
            } else if b[i + 1].is_ascii_alphanumeric() {
                let c = b[i + 1] as char;
                out.push_str(&to_subscript(&c.to_string()));
                i += 2;
                continue;
            }
        }

        // Drop bare grouping braces
        if b[i] == b'{' || b[i] == b'}' {
            i += 1;
            continue;
        }

        out.push(b[i] as char);
        i += 1;
    }

    out
}

fn extract_brace_group(s: &str, open: usize) -> Option<(usize, String)> {
    let b = s.as_bytes();
    if open >= b.len() || b[open] != b'{' {
        return None;
    }
    let mut depth = 1;
    let mut i = open + 1;
    while i < b.len() {
        match b[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((i, s[open + 1..i].to_string()));
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn convert_frac(input: &str, after_cmd: usize) -> Option<(usize, String)> {
    let b = input.as_bytes();
    let mut j = after_cmd;
    while j < b.len() && b[j] == b' ' {
        j += 1;
    }
    let (close1, num) = extract_brace_group(input, j)?;
    let mut k = close1 + 1;
    while k < b.len() && b[k] == b' ' {
        k += 1;
    }
    let (close2, den) = extract_brace_group(input, k)?;

    let num_u = latex_to_unicode(&num);
    let den_u = latex_to_unicode(&den);
    let num_s = if needs_parens(&num_u) {
        format!("({num_u})")
    } else {
        num_u
    };
    let den_s = if needs_parens(&den_u) {
        format!("({den_u})")
    } else {
        den_u
    };
    Some((close2 + 1, format!("{num_s}/{den_s}")))
}

fn convert_sqrt(input: &str, after_cmd: usize) -> Option<(usize, String)> {
    let b = input.as_bytes();
    let mut j = after_cmd;
    let mut prefix = "√".to_string();

    // Optional [n] for nth root
    if j < b.len() && b[j] == b'[' {
        let bracket_start = j;
        while j < b.len() && b[j] != b']' {
            j += 1;
        }
        if j < b.len() {
            let n = &input[bracket_start + 1..j];
            prefix = match n {
                "3" => "∛".to_string(),
                "4" => "∜".to_string(),
                _ => format!("{}√", to_superscript(n)),
            };
            j += 1; // skip ]
        }
    }

    let (close, content) = extract_brace_group(input, j)?;
    let content_u = latex_to_unicode(&content);
    let text = if is_simple_term(&content_u) {
        format!("{prefix}{content_u}")
    } else {
        format!("{prefix}({content_u})")
    };
    Some((close + 1, text))
}

fn is_simple_term(s: &str) -> bool {
    !s.contains('+') && !s.contains('-') && !s.contains(' ') && s.len() <= 4
}

fn needs_parens(s: &str) -> bool {
    let b = s.as_bytes();
    let mut depth = 0i32;
    for (i, &byte) in b.iter().enumerate() {
        match byte {
            b'(' => depth += 1,
            b')' => depth -= 1,
            b'+' if depth == 0 && i > 0 => return true,
            b'-' if depth == 0 && i > 0 => return true,
            _ => {}
        }
    }
    false
}

fn match_simple_command(s: &str) -> Option<(usize, &'static str)> {
    // Sorted longest-first within each prefix group to avoid prefix conflicts
    let commands: &[(&str, &str)] = &[
        ("\\operatorname", ""),
        ("\\arccosh", "arccosh"),
        ("\\arcsinh", "arcsinh"),
        ("\\arctanh", "arctanh"),
        ("\\arctan", "arctan"),
        ("\\arcsin", "arcsin"),
        ("\\arccos", "arccos"),
        ("\\sinh", "sinh"),
        ("\\cosh", "cosh"),
        ("\\tanh", "tanh"),
        ("\\sin", "sin"),
        ("\\cos", "cos"),
        ("\\tan", "tan"),
        ("\\sec", "sec"),
        ("\\csc", "csc"),
        ("\\cot", "cot"),
        ("\\ln", "ln"),
        ("\\log", "log"),
        ("\\exp", "exp"),
        ("\\det", "det"),
        ("\\lim", "lim"),
        ("\\max", "max"),
        ("\\min", "min"),
        ("\\erf", "erf"),
        ("\\Ei", "Ei"),
        ("\\li", "li"),
        ("\\varepsilon", "ε"),
        ("\\epsilon", "ε"),
        ("\\varphi", "φ"),
        ("\\lambda", "λ"),
        ("\\alpha", "α"),
        ("\\infty", "∞"),
        ("\\times", "×"),
        ("\\equiv", "≡"),
        ("\\theta", "θ"),
        ("\\sigma", "σ"),
        ("\\omega", "ω"),
        ("\\gamma", "γ"),
        ("\\delta", "δ"),
        ("\\kappa", "κ"),
        ("\\beta", "β"),
        ("\\cdot", "·"),
        ("\\zeta", "ζ"),
        ("\\phi", "φ"),
        ("\\psi", "ψ"),
        ("\\tau", "τ"),
        ("\\eta", "η"),
        ("\\rho", "ρ"),
        ("\\leq", "≤"),
        ("\\geq", "≥"),
        ("\\neq", "≠"),
        ("\\pi", "π"),
        ("\\mu", "μ"),
        ("\\nu", "ν"),
        ("\\xi", "ξ"),
        ("\\pm", "±"),
        ("\\mp", "∓"),
        ("\\to", "→"),
        ("\\in", "∈"),
        ("\\ ", " "),
    ];

    for &(pattern, replacement) in commands {
        if s.starts_with(pattern) {
            let plen = pattern.len();
            if replacement.is_empty() {
                continue;
            }
            let at_end = plen >= s.len();
            let next_ok = at_end || !s.as_bytes()[plen].is_ascii_alphabetic();
            if next_ok {
                return Some((plen, replacement));
            }
        }
    }
    None
}

fn to_superscript(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '0' => out.push('⁰'),
            '1' => out.push('¹'),
            '2' => out.push('²'),
            '3' => out.push('³'),
            '4' => out.push('⁴'),
            '5' => out.push('⁵'),
            '6' => out.push('⁶'),
            '7' => out.push('⁷'),
            '8' => out.push('⁸'),
            '9' => out.push('⁹'),
            '+' => out.push('⁺'),
            '-' => out.push('⁻'),
            '=' => out.push('⁼'),
            '(' => out.push('⁽'),
            ')' => out.push('⁾'),
            'n' => out.push('ⁿ'),
            'i' => out.push('ⁱ'),
            _ => return format!("^({s})"),
        }
    }
    out
}

fn to_subscript(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '0' => out.push('₀'),
            '1' => out.push('₁'),
            '2' => out.push('₂'),
            '3' => out.push('₃'),
            '4' => out.push('₄'),
            '5' => out.push('₅'),
            '6' => out.push('₆'),
            '7' => out.push('₇'),
            '8' => out.push('₈'),
            '9' => out.push('₉'),
            '+' => out.push('₊'),
            '-' => out.push('₋'),
            'a' => out.push('ₐ'),
            'e' => out.push('ₑ'),
            'i' => out.push('ᵢ'),
            'j' => out.push('ⱼ'),
            'k' => out.push('ₖ'),
            'n' => out.push('ₙ'),
            'o' => out.push('ₒ'),
            'r' => out.push('ᵣ'),
            's' => out.push('ₛ'),
            't' => out.push('ₜ'),
            'x' => out.push('ₓ'),
            _ => return format!("_{{{s}}}"),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fractions() {
        assert_eq!(latex_to_unicode("\\frac{7}{12}"), "7/12");
        assert_eq!(latex_to_unicode("\\frac{1}{3}"), "1/3");
        assert_eq!(latex_to_unicode("\\frac{x+1}{x-1}"), "(x+1)/(x-1)");
        assert_eq!(latex_to_unicode("\\frac{-x}{2}"), "-x/2");
    }

    #[test]
    fn superscripts() {
        assert_eq!(latex_to_unicode("x^{2}"), "x²");
        assert_eq!(latex_to_unicode("x^{10}"), "x¹⁰");
        assert_eq!(latex_to_unicode("x^{-2}"), "x⁻²");
        assert_eq!(latex_to_unicode("x^{n+1}"), "xⁿ⁺¹");
        assert_eq!(latex_to_unicode("x^{abc}"), "x^(abc)");
    }

    #[test]
    fn subscripts() {
        assert_eq!(latex_to_unicode("C_{1}"), "C₁");
        assert_eq!(latex_to_unicode("x_{0}"), "x₀");
    }

    #[test]
    fn greek_and_symbols() {
        assert_eq!(latex_to_unicode("\\pi"), "π");
        assert_eq!(latex_to_unicode("\\infty"), "∞");
        assert_eq!(latex_to_unicode("\\cdot"), "·");
        assert_eq!(latex_to_unicode("2\\pi"), "2π");
    }

    #[test]
    fn functions() {
        assert_eq!(latex_to_unicode("\\sin(x)"), "sin(x)");
        assert_eq!(latex_to_unicode("\\cos(x)"), "cos(x)");
        assert_eq!(latex_to_unicode("\\arctan(x)"), "arctan(x)");
        assert_eq!(latex_to_unicode("\\ln(x)"), "ln(x)");
    }

    #[test]
    fn sqrt() {
        assert_eq!(latex_to_unicode("\\sqrt{2}"), "√2");
        assert_eq!(latex_to_unicode("\\sqrt{x+1}"), "√(x+1)");
        assert_eq!(latex_to_unicode("\\frac{\\sqrt{2}}{2}"), "√2/2");
    }

    #[test]
    fn complex_expressions() {
        assert_eq!(
            latex_to_unicode("\\frac{1}{3} \\cdot x^{3} + C"),
            "1/3 · x³ + C"
        );
        assert_eq!(latex_to_unicode("x^{2} + 2x + 1"), "x² + 2x + 1");
        assert_eq!(
            latex_to_unicode("C_{1} \\cdot \\cos(x) + C_{2} \\cdot \\sin(x)"),
            "C₁ · cos(x) + C₂ · sin(x)"
        );
        assert_eq!(
            latex_to_unicode("2^{4} \\cdot 3^{2} \\cdot 5"),
            "2⁴ · 3² · 5"
        );
    }

    #[test]
    fn passthrough() {
        assert_eq!(latex_to_unicode("5"), "5");
        assert_eq!(latex_to_unicode("x + 1"), "x + 1");
        assert_eq!(latex_to_unicode("-1"), "-1");
    }

    #[test]
    fn blackboard_bold() {
        assert_eq!(
            latex_to_unicode("(irreducible over \\mathbb{Q})"),
            "(irreducible over ℚ)"
        );
    }
}
