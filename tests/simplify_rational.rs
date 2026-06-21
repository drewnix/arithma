#[cfg(test)]
mod simplify_rational_tests {
    use arithma::simplify::Simplifiable;
    use arithma::{build_expression_tree, Environment, Tokenizer};

    fn simplify_latex(input: &str) -> String {
        let env = Environment::new();
        let mut tokenizer = Tokenizer::new(input);
        let tokens = tokenizer.tokenize();
        let expr = build_expression_tree(tokens).unwrap();
        let simplified = expr.simplify(&env).unwrap();
        format!("{}", simplified)
    }

    // ── Polynomial GCD cancellation ──────────────────────────

    #[test]
    fn cancel_difference_of_squares() {
        assert_eq!(simplify_latex("\\frac{x^2 - 1}{x - 1}"), "x + 1");
    }

    #[test]
    fn cancel_difference_of_cubes() {
        assert_eq!(simplify_latex("\\frac{x^3 - 8}{x - 2}"), "x^{2} + 2x + 4");
    }

    #[test]
    fn cancel_quartic_by_quadratic() {
        assert_eq!(simplify_latex("\\frac{x^4 - 1}{x^2 - 1}"), "x^{2} + 1");
    }

    // ── f64 → exact rational conversion ──────────────────────

    #[test]
    fn rationalize_half() {
        let result = simplify_latex("0.5 \\cdot x");
        assert!(
            result.contains("\\frac{1}{2}"),
            "0.5 should become 1/2, got: {}",
            result
        );
    }

    #[test]
    fn rationalize_third() {
        let result = simplify_latex("0.333333333333 \\cdot x");
        assert!(
            result.contains("\\frac{1}{3}"),
            "0.333... should become 1/3, got: {}",
            result
        );
    }

    #[test]
    fn rationalize_quarter() {
        let result = simplify_latex("0.25 \\cdot x");
        assert!(
            result.contains("\\frac{1}{4}"),
            "0.25 should become 1/4, got: {}",
            result
        );
    }

    #[test]
    fn rationalize_eighth() {
        let result = simplify_latex("0.125 \\cdot x^2");
        assert!(
            result.contains("\\frac{1}{8}"),
            "0.125 should become 1/8, got: {}",
            result
        );
    }

    #[test]
    fn rationalize_sixth() {
        let result = simplify_latex("0.1666666666666 \\cdot x");
        assert!(
            result.contains("\\frac{1}{6}"),
            "0.1666... should become 1/6, got: {}",
            result
        );
    }

    #[test]
    fn rationalize_two_thirds() {
        let result = simplify_latex("0.666666666666 \\cdot x");
        assert!(
            result.contains("\\frac{2}{3}"),
            "0.666... should become 2/3, got: {}",
            result
        );
    }

    // ── Coefficient normalization ────────────────────────────

    #[test]
    fn normalize_fraction_coefficient() {
        let result = simplify_latex("\\frac{2x}{4}");
        assert!(
            result.contains("\\frac{1}{2}"),
            "2x/4 should simplify to (1/2)x, got: {}",
            result
        );
    }

    #[test]
    fn normalize_multivar_fraction() {
        assert_eq!(simplify_latex("\\frac{6x}{3y}"), "\\frac{2x}{y}");
    }

    // ── Equivalence-relevant simplifications ─────────────────

    #[test]
    fn equivalent_forms_simplify_same() {
        let a = simplify_latex("\\frac{x^2 - 4}{x - 2}");
        let b = simplify_latex("x + 2");
        assert_eq!(a, b, "Both should simplify to x+2");
    }

    #[test]
    fn integer_stays_integer() {
        // Make sure rationalization doesn't break integers
        assert_eq!(simplify_latex("3"), "3");
        assert_eq!(simplify_latex("0"), "0");
        assert_eq!(simplify_latex("-5"), "-5");
    }

    #[test]
    fn irrational_stays_float() {
        // pi, sqrt(2), etc. should not be rationalized
        let result = simplify_latex("3.14159265358979");
        assert!(
            !result.contains("\\frac"),
            "pi approx should not become a fraction, got: {}",
            result
        );
    }
}
