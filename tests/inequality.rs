#[cfg(test)]
mod inequality_tests {
    use arithma::{build_expression_tree, Node, Tokenizer};

    fn solve_ineq(input: &str) -> String {
        let mut tokenizer = Tokenizer::new(input);
        let tokens = tokenizer.tokenize();
        let expr = build_expression_tree(tokens).unwrap();
        arithma::solve_inequality(&expr, "x").unwrap()
    }

    // ── Polynomial inequalities ──────────────────────────────

    #[test]
    fn quadratic_strict_greater() {
        assert_eq!(solve_ineq("x^2 - 4 > 0"), "(-∞, -2) ∪ (2, ∞)");
    }

    #[test]
    fn quadratic_less_equal() {
        assert_eq!(solve_ineq("x^2 - 4 <= 0"), "[-2, 2]");
    }

    #[test]
    fn quadratic_greater_equal() {
        assert_eq!(solve_ineq("x^2 - 1 >= 0"), "(-∞, -1] ∪ [1, ∞)");
    }

    #[test]
    fn quadratic_strict_less() {
        assert_eq!(solve_ineq("x^2 - 1 < 0"), "(-1, 1)");
    }

    #[test]
    fn cubic_positive() {
        // x³ - x > 0 at (-1,0) ∪ (1,∞)
        assert_eq!(solve_ineq("x^3 - x > 0"), "(-1, 0) ∪ (1, ∞)");
    }

    #[test]
    fn always_true() {
        // x² + 1 > 0 for all x
        assert_eq!(solve_ineq("x^2 + 1 > 0"), "(-∞, ∞)");
    }

    #[test]
    fn always_false() {
        // x² < 0 has no solution
        assert_eq!(solve_ineq("x^2 < 0"), "∅");
    }

    #[test]
    fn always_nonneg() {
        // x² ≥ 0 for all x
        assert_eq!(solve_ineq("x^2 >= 0"), "(-∞, ∞)");
    }

    // ── Rational inequalities ────────────────────────────────

    #[test]
    fn rational_strict() {
        // (x-1)/(x+2) > 0 → (-∞,-2) ∪ (1,∞)
        assert_eq!(
            solve_ineq("\\frac{x-1}{x+2} > 0"),
            "(-∞, -2) ∪ (1, ∞)"
        );
    }

    #[test]
    fn rational_nonstrict() {
        // (x-1)/(x+2) >= 0 → (-∞,-2) ∪ [1,∞)
        assert_eq!(
            solve_ineq("\\frac{x-1}{x+2} >= 0"),
            "(-∞, -2) ∪ [1, ∞)"
        );
    }

    // ── Linear inequalities ──────────────────────────────────

    #[test]
    fn linear_greater() {
        // 2x - 6 > 0 → x > 3
        assert_eq!(solve_ineq("2x - 6 > 0"), "(3, ∞)");
    }

    #[test]
    fn linear_less_equal() {
        // x - 5 <= 0 → x ≤ 5
        assert_eq!(solve_ineq("x - 5 <= 0"), "(-∞, 5]");
    }

    // ── Inequality with two sides ────────────────────────────

    #[test]
    fn two_sided_inequality() {
        // x^2 > 4 is same as x^2 - 4 > 0
        assert_eq!(solve_ineq("x^2 > 4"), "(-∞, -2) ∪ (2, ∞)");
    }

    // ── MCP path (solve tool handles inequalities) ───────────

    #[test]
    fn cancelled_factor_excludes_pole() {
        // Bug #4: (x²-1)/(x-1) > 0 must exclude x=1 (undefined)
        assert_eq!(
            solve_ineq("\\frac{x^2-1}{x-1} > 0"),
            "(-1, 1) ∪ (1, ∞)"
        );
    }

    #[test]
    fn latex_geq_leq() {
        // Bug #3: \geq and \leq must be recognized
        assert_eq!(solve_ineq("x^2 - 2x + 1 \\geq 0"), "(-∞, ∞)");
        assert_eq!(solve_ineq("x \\leq 5"), "(-∞, 5]");
    }

    #[test]
    fn solve_tool_dispatches_inequality() {
        let mut tokenizer = Tokenizer::new("x^2 - 9 < 0");
        let tokens = tokenizer.tokenize();
        let expr = build_expression_tree(tokens).unwrap();
        assert!(matches!(expr, Node::Less(_, _)));
        let result = arithma::solve_inequality(&expr, "x").unwrap();
        assert_eq!(result, "(-3, 3)");
    }
}
