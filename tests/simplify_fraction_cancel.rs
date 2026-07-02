#[cfg(test)]
mod simplify_fraction_cancel_tests {
    use arithma::{Environment, Evaluator, Node};

    fn simplify_latex(input: &str) -> Node {
        let env = Environment::new();
        let expr = arithma::parse_latex(input, &env).unwrap();
        Evaluator::simplify(&expr, &env).unwrap()
    }

    fn assert_simplify_latex(input: &str, expected_latex: &str) {
        let env = Environment::new();
        let result = simplify_latex(input);
        let expected = arithma::parse_latex(expected_latex, &env).unwrap();
        assert_eq!(result, expected, "input: {input}");
    }

    fn assert_simplify_display_contains(input: &str, needle: &str) {
        let result = simplify_latex(input);
        let display = format!("{result}");
        assert!(
            display.contains(needle),
            "expected {needle:?} in {display:?} for input: {input}"
        );
    }

    // ── x factor position in numerator ───────────────────────────────

    #[test]
    fn cancel_x_factor_numerator_coeff_first() {
        assert_simplify_latex(r"\frac{3 \cdot x}{x}", "3");
    }

    #[test]
    fn cancel_x_factor_numerator_coeff_last() {
        assert_simplify_latex(r"\frac{x \cdot 3}{x}", "3");
    }

    #[test]
    fn cancel_x_factor_numerator_coeff_middle() {
        assert_simplify_latex(r"\frac{x \cdot 3 \cdot 2}{x}", "6");
    }

    #[test]
    fn cancel_x_factor_numerator_x_middle() {
        assert_simplify_latex(r"\frac{2 \cdot x \cdot 5}{x}", "10");
    }

    #[test]
    fn cancel_x_factor_numerator_x_first_of_three() {
        assert_simplify_latex(r"\frac{3 \cdot x \cdot 5}{x}", "15");
    }

    #[test]
    fn cancel_x_factor_numerator_x_between_numeric() {
        assert_simplify_latex(r"\frac{x \cdot 2 \cdot 5}{x}", "10");
    }

    #[test]
    fn cancel_x_factor_numerator_permutation_a() {
        assert_simplify_latex(r"\frac{x \cdot 5 \cdot 2}{x}", "10");
    }

    #[test]
    fn cancel_x_factor_numerator_permutation_b() {
        assert_simplify_latex(r"\frac{5 \cdot x \cdot 2}{x}", "10");
    }

    // ── product denominator ────────────────────────────────────────

    #[test]
    fn cancel_x_factor_denominator_one_times_x() {
        assert_simplify_latex(r"\frac{3 \cdot x}{1 \cdot x}", "3");
    }

    #[test]
    fn cancel_x_factor_denominator_x_times_one() {
        assert_simplify_latex(r"\frac{x \cdot 3}{x \cdot 1}", "3");
    }

    #[test]
    fn cancel_x_factor_denominator_one_times_x_chained_numeric() {
        assert_simplify_latex(r"\frac{2 \cdot 5 \cdot x}{1 \cdot x}", "10");
    }

    // ── shared ln sum ──────────────────────────────────────────────

    #[test]
    fn cancel_ln_sum_factor_after_numeric() {
        assert_simplify_latex(r"\frac{2 \cdot (\ln(2) + \ln(3))}{\ln(2) + \ln(3)}", "2");
    }

    #[test]
    fn cancel_ln_sum_factor_before_numeric() {
        assert_simplify_latex(r"\frac{(\ln(2) + \ln(3)) \cdot 2}{\ln(2) + \ln(3)}", "2");
    }

    #[test]
    fn cancel_ln_sum_factor_between_numeric_and_x() {
        assert_simplify_latex(
            r"\frac{2 \cdot (\ln(2) + \ln(3)) \cdot x}{(\ln(2) + \ln(3))}",
            "2x",
        );
    }

    #[test]
    fn cancel_ln_sum_factor_after_x() {
        assert_simplify_latex(
            r"\frac{x \cdot 2 \cdot (\ln(2) + \ln(3))}{\ln(2) + \ln(3)}",
            "2x",
        );
    }

    // ── multiple variables ─────────────────────────────────────────

    #[test]
    fn cancel_x_leaving_y_and_numeric_a() {
        assert_simplify_latex(r"\frac{y \cdot x \cdot 3}{x}", "3y");
    }

    #[test]
    fn cancel_x_leaving_y_and_numeric_b() {
        assert_simplify_latex(r"\frac{3 \cdot y \cdot x}{x}", "3y");
    }

    #[test]
    fn cancel_y_leaving_x_and_numeric() {
        assert_simplify_latex(r"\frac{x \cdot y \cdot 3}{y}", "3x");
    }

    // ── negated numerator ──────────────────────────────────────────

    #[test]
    fn cancel_x_factor_negated_numerator_coeff_first() {
        assert_simplify_latex(r"\frac{-(3 \cdot x)}{x}", "-3");
    }

    #[test]
    fn cancel_x_factor_negated_numerator_coeff_last() {
        assert_simplify_latex(r"\frac{-(x \cdot 3)}{x}", "-3");
    }

    fn assert_simplify_is_nan(input: &str) {
        let result = simplify_latex(input);
        let Node::Num(n) = result else {
            panic!("expected numeric NaN for {input}, got: {result:?}");
        };
        assert!(n.to_f64().is_nan(), "expected NaN for {input}, got: {n:?}");
    }

    // ── zero denominator (must not cancel) ───────────────────────────

    #[test]
    fn zero_denominator_does_not_cancel_x() {
        assert_simplify_display_contains(r"\frac{3 \cdot x}{0}", r"\frac");
        assert_simplify_display_contains(r"\frac{3 \cdot x}{0}", "0");
    }

    #[test]
    fn zero_denominator_does_not_cancel_shared_zero_factor() {
        // Cancel is skipped; 3·0 / 0 folds to 0/0 → NaN (not 3).
        assert_simplify_is_nan(r"\frac{3 \cdot 0}{0}");
    }

    #[test]
    fn zero_denominator_does_not_cancel_x_times_zero() {
        assert_simplify_is_nan(r"\frac{x \cdot 0}{0}");
    }

    #[test]
    fn zero_denominator_does_not_cancel_zero_times_x() {
        assert_simplify_is_nan(r"\frac{0 \cdot x}{0}");
    }

    #[test]
    fn zero_numerator_over_nonzero_denominator() {
        assert_simplify_latex(r"\frac{0 \cdot x}{x}", "0");
    }
}
