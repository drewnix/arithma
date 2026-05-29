#[cfg(test)]
mod parser_hardening_tests {
    use arithma::{parse_latex, Environment, Tokenizer};

    fn parse(latex: &str) -> String {
        let env = Environment::new();
        let node = parse_latex(latex, &env).unwrap();
        format!("{}", node)
    }

    fn parse_ok(latex: &str) -> bool {
        let env = Environment::new();
        parse_latex(latex, &env).is_ok()
    }

    // --- Implicit multiplication: variable followed by parenthesized expression ---

    #[test]
    fn test_var_paren_basic() {
        // u(3-2u) should parse as u*(3-2u) = -2u² + 3u
        assert!(parse_ok("u(3-2u)"));
        let r = parse("u(3-2u)");
        assert!(
            r.contains("u") && !r.contains("Error"),
            "u(3-2u) should parse: {}",
            r
        );
    }

    #[test]
    fn test_var_paren_chained() {
        // x(x+1)(x-1) = x³ - x
        let r = parse("x(x+1)(x-1)");
        assert_eq!(r, "x^{3} - x");
    }

    #[test]
    fn test_var_paren_in_subtraction() {
        // a(b+1) - c(d-1) should parse without error
        assert!(parse_ok("a(b+1) - c(d-1)"));
    }

    #[test]
    fn test_greek_paren() {
        // α(x+1) = αx + α
        assert!(parse_ok("\\alpha(x+1)"));
    }

    // --- Known functions still work as function calls ---

    #[test]
    fn test_sin_not_multiplication() {
        let r = parse("\\sin(x)");
        assert!(r.contains("\\sin"), "sin(x) should remain a function call: {}", r);
    }

    #[test]
    fn test_exp_not_multiplication() {
        // \exp(-x^2) must remain a function call
        assert!(parse_ok("\\exp(-x^2)"));
    }

    #[test]
    fn test_ln_not_multiplication() {
        assert!(parse_ok("\\ln(x+1)"));
    }

    // --- Space-separated variables are multiplication ---

    #[test]
    fn test_space_separated_vars() {
        // x y should parse as x*y
        assert!(parse_ok("x y"));
    }

    // --- Multi-character variable names are preserved ---

    #[test]
    fn test_multichar_variable() {
        // "xy" (no space) stays as single variable
        let mut tok = Tokenizer::new("xy");
        let tokens = tok.tokenize();
        assert_eq!(tokens, vec!["xy"]);
    }

    // --- Complex expressions that agents actually write ---

    #[test]
    fn test_compound_fraction_subtraction() {
        // \frac{a}{b} - \frac{c}{d} should parse
        assert!(parse_ok("\\frac{a}{b} - \\frac{c}{d}"));
    }

    #[test]
    fn test_nested_multivariate_fraction() {
        assert!(parse_ok("\\frac{a - 4\\alpha + 3}{a - 2\\alpha^2 + 1}"));
    }

    // --- Sign normalization in fractions ---

    #[test]
    fn test_sign_normalization_neg_over_neg() {
        // -3/(-2b-1) should simplify to 3/(2b+1)
        let r = parse("\\frac{-3}{-(2b + 1)}");
        assert!(
            !r.contains("--") && !r.contains("\\frac{-"),
            "-3/-(2b+1) should normalize signs: {}",
            r
        );
    }

    #[test]
    fn test_sign_normalization_negate_negate() {
        // -a / -b should simplify to a/b
        let r = parse("\\frac{-a}{-b}");
        assert_eq!(r, "\\frac{a}{b}");
    }

    #[test]
    fn test_sign_normalization_negative_denom_expression() {
        // -3/(-2b-1) should also normalize (the denominator isn't Negate-wrapped after simplification)
        let r = parse("\\frac{-3}{-2b - 1}");
        assert_eq!(r, "\\frac{3}{2b + 1}");
    }

    // --- Rational equation solving ---

    #[test]
    fn test_solve_rational_simple() {
        // 1/x = 2  →  x = 1/2
        let env = Environment::new();
        let expr = parse_latex("\\frac{1}{x} = 2", &env).unwrap();
        let solutions = arithma::solve_for_variable_exact(&expr, "x").unwrap();
        assert!(!solutions.is_empty(), "Should find x = 1/2");
    }

    #[test]
    fn test_solve_rational_linear() {
        // 3/(1+2x) = 2  →  1+2x = 3/2 → 2x = 1/2 → x = 1/4
        let env = Environment::new();
        let expr = parse_latex("\\frac{3}{1 + 2x} = 2", &env).unwrap();
        let solutions = arithma::solve_for_variable_exact(&expr, "x").unwrap();
        assert!(!solutions.is_empty(), "Should find x = 1/4");
    }

    // --- Decimal eigenvalues ---

    #[test]
    fn test_eigenvalues_decimal_3x3() {
        // 3×3 matrix with decimal entries should compute eigenvalues numerically
        let result = mcp_eigenvalues("1 & 0.4349 & 0.4349 \\\\ 0.4349 & 1 & 0.4349 \\\\ 0.4349 & 0.4349 & 1");
        assert!(result.is_ok(), "Decimal 3×3 eigenvalues should work: {:?}", result);
        let eigenvalues = result.unwrap();
        assert_eq!(eigenvalues.len(), 3, "Should find 3 eigenvalues");
    }

    // --- Parametric integration ---

    #[test]
    fn test_integrate_parametric_linear() {
        // ∫1/(x+a) dx = ln|x+a|
        let env = Environment::new();
        let expr = parse_latex("\\frac{1}{x + a}", &env).unwrap();
        let result = arithma::integrate(&expr, "x");
        assert!(result.is_ok(), "Should integrate 1/(x+a): {:?}", result);
        let r = format!("{}", result.unwrap());
        assert!(r.contains("ln"), "Should contain ln: {}", r);
    }

    #[test]
    fn test_integrate_parametric_scaled() {
        // ∫1/(2x+b) dx = (1/2)·ln|2x+b|
        let env = Environment::new();
        let expr = parse_latex("\\frac{1}{2x + b}", &env).unwrap();
        let result = arithma::integrate(&expr, "x");
        assert!(result.is_ok(), "Should integrate 1/(2x+b): {:?}", result);
    }

    fn mcp_eigenvalues(matrix_body: &str) -> Result<Vec<f64>, String> {
        let env = Environment::new();
        let latex = format!("\\begin{{pmatrix}} {} \\end{{pmatrix}}", matrix_body);
        let mat = arithma::parse_latex_matrix(&latex, &env)?;
        let eigs = mat.eigenvalues(&env)?;
        eigs.iter()
            .map(|e| arithma::Evaluator::evaluate(e, &env))
            .collect()
    }
}
