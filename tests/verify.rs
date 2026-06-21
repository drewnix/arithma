#[cfg(test)]
mod verify_tests {
    use arithma::{build_expression_tree, Tokenizer};

    fn parse(input: &str) -> arithma::Node {
        let mut tokenizer = Tokenizer::new(input);
        let tokens = tokenizer.tokenize();
        build_expression_tree(tokens).unwrap()
    }

    fn verify(a: &str, b: &str, vars: &[&str]) -> arithma::verify::VerifyResult {
        let a_expr = parse(a);
        let b_expr = parse(b);
        let variables: Vec<String> = vars.iter().map(|s| s.to_string()).collect();
        arithma::verify_identity(&a_expr, &b_expr, &variables)
    }

    // ── Passing verifications ────────────────────────────────

    #[test]
    fn verify_difference_of_squares() {
        let result = verify("x^2 - 1", "(x-1)(x+1)", &["x"]);
        assert!(result.passed, "x²-1 = (x-1)(x+1) should pass");
        assert!(result.points_tested >= 5);
    }

    #[test]
    fn verify_derivative() {
        let result = verify("3x^2", "3x^2", &["x"]);
        assert!(result.passed, "d/dx[x³] = 3x² should pass");
    }

    #[test]
    fn verify_trig_identity() {
        let result = verify("\\sin(x)^2 + \\cos(x)^2", "1", &["x"]);
        assert!(result.passed, "sin²+cos²=1 should pass");
    }

    #[test]
    fn verify_multivar() {
        let result = verify("(x+y)^2", "x^2 + 2 \\cdot x \\cdot y + y^2", &["x", "y"]);
        assert!(result.passed, "(x+y)² = x²+2xy+y² should pass");
    }

    // ── Failing verifications ────────────────────────────────

    #[test]
    fn verify_not_equal() {
        let result = verify("x^2", "x^3", &["x"]);
        assert!(!result.passed, "x² ≠ x³");
        assert!(result.counterexample.is_some());
        let cx = result.counterexample.unwrap();
        assert!((cx.lhs_value - cx.rhs_value).abs() > 1e-8);
    }

    #[test]
    fn verify_close_but_wrong() {
        let result = verify("x^2 + 1", "x^2 + 2", &["x"]);
        assert!(!result.passed, "x²+1 ≠ x²+2");
    }

    // ── Display format ───────────────────────────────────────

    #[test]
    fn display_pass() {
        let result = verify("x", "x", &["x"]);
        let s = format!("{}", result);
        assert!(s.contains("PASS"), "Should say PASS, got: {}", s);
        assert!(s.contains("points"), "Should mention points, got: {}", s);
    }

    #[test]
    fn display_fail() {
        let result = verify("x", "2x", &["x"]);
        let s = format!("{}", result);
        assert!(s.contains("FAIL"), "Should say FAIL, got: {}", s);
        assert!(s.contains("LHS"), "Should show LHS value, got: {}", s);
        assert!(s.contains("RHS"), "Should show RHS value, got: {}", s);
    }

    // ── Bug #1 fixes ────────────────────────────────────────

    #[test]
    fn greek_variable_names_normalized() {
        let result = verify("\\frac{1}{1+\\alpha}", "\\frac{1}{1+\\alpha}", &["\\alpha"]);
        assert!(
            result.passed,
            "Greek var should be normalized, got: {}",
            result
        );
        assert!(result.points_tested >= 5, "Should test at multiple points");
    }

    #[test]
    fn zero_points_is_not_pass() {
        // Use a variable name that doesn't exist in the expression
        let result = verify("x", "x", &["nonexistent_var_zzzz"]);
        assert!(!result.passed, "0 points tested should not be PASS");
        assert!(result.insufficient_points);
        let s = format!("{}", result);
        assert!(
            s.contains("INCONCLUSIVE"),
            "Should say INCONCLUSIVE, got: {}",
            s
        );
    }
}
