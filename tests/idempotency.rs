#[cfg(test)]
mod idempotency_tests {
    use arithma::simplify::Simplifiable;
    use arithma::{build_expression_tree, parse_latex, Environment, Tokenizer};

    fn parse_raw(latex: &str) -> arithma::Node {
        let mut tokenizer = Tokenizer::new(latex);
        let tokens = tokenizer.tokenize();
        build_expression_tree(tokens).unwrap_or_else(|_| panic!("Failed to parse raw: {}", latex))
    }

    fn assert_idempotent(latex: &str) {
        let env = Environment::new();
        let raw = parse_raw(latex);
        let s1 = raw
            .simplify(&env)
            .unwrap_or_else(|_| panic!("Failed first simplify: {}", latex));
        let s1_str = format!("{}", s1);
        let s2 = s1
            .simplify(&env)
            .unwrap_or_else(|_| panic!("Failed second simplify: {}", latex));
        let s2_str = format!("{}", s2);
        assert_eq!(
            s1_str, s2_str,
            "Idempotency failure for '{}':\n  simplify once:  {}\n  simplify twice: {}",
            latex, s1_str, s2_str
        );
    }

    fn assert_roundtrip_stable(latex: &str) {
        let env = Environment::new();
        let expr = parse_latex(latex, &env).unwrap_or_else(|_| panic!("Failed to parse: {}", latex));
        let s1_str = format!("{}", expr);
        let reparsed = parse_latex(&s1_str, &env).unwrap_or_else(|_| panic!("Failed to reparse: {}", s1_str));
        let s2_str = format!("{}", reparsed);
        assert_eq!(
            s1_str, s2_str,
            "Round-trip instability for '{}':\n  simplify:               {}\n  reparse+simplify again: {}",
            latex, s1_str, s2_str
        );
    }

    // === Arithmetic basics ===

    #[test]
    fn idem_integer() {
        assert_idempotent("42");
    }

    #[test]
    fn idem_fraction() {
        assert_idempotent("\\frac{6}{8}");
    }

    #[test]
    fn idem_add_numbers() {
        assert_idempotent("2 + 3");
    }

    #[test]
    fn idem_nested_arithmetic() {
        assert_idempotent("\\frac{2 + 3}{4 - 1}");
    }

    // === Polynomial expressions ===

    #[test]
    fn idem_polynomial_collect() {
        assert_idempotent("x^{2} + 3x + x^{2} + 2x + 1");
    }

    #[test]
    fn idem_polynomial_multiply() {
        assert_idempotent("(x + 1)(x - 1)");
    }

    #[test]
    fn idem_polynomial_distribute() {
        assert_idempotent("3(x + 2)");
    }

    #[test]
    fn idem_polynomial_subtract() {
        assert_idempotent("2x^{2} - x^{2}");
    }

    #[test]
    fn idem_multivar_collect() {
        assert_idempotent("5x + 3y - 2x");
    }

    // === Power rules ===

    #[test]
    fn idem_power_of_power() {
        assert_idempotent("(x^{2})^{3}");
    }

    #[test]
    fn idem_power_multiply() {
        assert_idempotent("x^{2} \\cdot x^{3}");
    }

    #[test]
    fn idem_power_divide() {
        assert_idempotent("\\frac{x^{5}}{x^{3}}");
    }

    #[test]
    fn idem_x_times_x() {
        assert_idempotent("x \\cdot x");
    }

    #[test]
    fn idem_zero_exponent() {
        assert_idempotent("x^{0}");
    }

    #[test]
    fn idem_one_exponent() {
        assert_idempotent("x^{1}");
    }

    // === Rational expressions / GCD ===

    #[test]
    fn idem_rational_cancel() {
        assert_idempotent("\\frac{x^{2} - 1}{x + 1}");
    }

    #[test]
    fn idem_multivar_gcd() {
        assert_idempotent("\\frac{x \\cdot y + x}{y + 1}");
    }

    #[test]
    fn idem_difference_of_squares_multivar() {
        assert_idempotent("\\frac{x^{2} - y^{2}}{x + y}");
    }

    // === Trig identities ===

    #[test]
    fn idem_pythagorean() {
        assert_idempotent("\\sin(x)^{2} + \\cos(x)^{2}");
    }

    #[test]
    fn idem_one_minus_sin_sq() {
        assert_idempotent("1 - \\sin(x)^{2}");
    }

    #[test]
    fn idem_sin_div_cos() {
        assert_idempotent("\\frac{\\sin(x)}{\\cos(x)}");
    }

    #[test]
    fn idem_one_over_sin() {
        assert_idempotent("\\frac{1}{\\sin(x)}");
    }

    #[test]
    fn idem_sin_neg() {
        assert_idempotent("\\sin(-x)");
    }

    #[test]
    fn idem_cos_neg() {
        assert_idempotent("\\cos(-x)");
    }

    // === Logarithm rules ===

    #[test]
    fn idem_ln_power() {
        assert_idempotent("\\ln(x^{3})");
    }

    #[test]
    fn idem_ln_product() {
        assert_idempotent("\\ln(x \\cdot y)");
    }

    #[test]
    fn idem_ln_quotient() {
        assert_idempotent("\\ln(\\frac{x}{y})");
    }

    #[test]
    fn idem_ln_e_to_x() {
        assert_idempotent("\\ln(e^{x})");
    }

    #[test]
    fn idem_exp_ln() {
        assert_idempotent("\\exp(\\ln(x))");
    }

    // === Composed / cascading rules ===
    // These are the expressions most likely to fail idempotency:
    // a rule fires and produces a form that another rule would simplify further.

    #[test]
    fn idem_ln_of_power_of_product() {
        // ln((xy)^2) → first pass: 2·ln(xy), second pass: 2·(ln(x)+ln(y))
        assert_idempotent("\\ln((x \\cdot y)^{2})");
    }

    #[test]
    fn idem_ln_of_power_of_quotient() {
        // ln((x/y)^3) → 3·ln(x/y) → 3·(ln(x)-ln(y))
        assert_idempotent("\\ln((\\frac{x}{y})^{3})");
    }

    #[test]
    fn idem_ln_of_product_of_powers() {
        // ln(x^2 · y^3) → ln(x^2) + ln(y^3) → 2ln(x) + 3ln(y)
        assert_idempotent("\\ln(x^{2} \\cdot y^{3})");
    }

    #[test]
    fn idem_nested_power_product() {
        // (x·y)^2 through polynomial normalization
        assert_idempotent("(x \\cdot y)^{2}");
    }

    #[test]
    fn idem_sin_of_negated_sum() {
        // sin(-(x+y)) — negate distributes, then odd function rule
        assert_idempotent("\\sin(-(x + y))");
    }

    // === Negate chains ===

    #[test]
    fn idem_double_negate() {
        // Parser doesn't support --x syntax; construct directly
        use arithma::Node;
        let env = Environment::new();
        let expr = Node::Negate(Box::new(Node::Negate(Box::new(Node::Variable(
            "x".to_string(),
        )))));
        let s1 = expr.simplify(&env).unwrap();
        let s2 = s1.simplify(&env).unwrap();
        assert_eq!(
            format!("{}", s1),
            format!("{}", s2),
            "Idempotency failure for double negate"
        );
    }

    #[test]
    fn idem_negate_zero() {
        assert_idempotent("-0");
    }

    // === Abs ===

    #[test]
    fn idem_abs_negate() {
        assert_idempotent("|-x|");
    }

    #[test]
    fn idem_abs_abs() {
        assert_idempotent("||x||");
    }

    // === Sqrt ===

    #[test]
    fn idem_sqrt_of_square() {
        assert_idempotent("\\sqrt{x^{2}}");
    }

    // === Mixed / stress tests ===

    #[test]
    fn idem_complex_rational() {
        assert_idempotent("\\frac{x^{3} - x}{x^{2} - 1}");
    }

    #[test]
    fn idem_trig_with_polynomial() {
        assert_idempotent("x^{2} \\cdot \\sin(x) + x^{2} \\cdot \\sin(x)");
    }

    #[test]
    fn idem_const_folding_in_power() {
        assert_idempotent("2^{3}");
    }

    #[test]
    fn idem_multiply_by_zero() {
        assert_idempotent("0 \\cdot x");
    }

    #[test]
    fn idem_multiply_by_one() {
        assert_idempotent("1 \\cdot x");
    }

    #[test]
    fn idem_divide_by_one() {
        assert_idempotent("\\frac{x}{1}");
    }

    #[test]
    fn idem_divide_self() {
        assert_idempotent("\\frac{x}{x}");
    }

    // === Round-trip stability (parse → simplify → format → parse → simplify → format) ===

    #[test]
    fn idem_ln_of_product_chain() {
        // ln(x·y·z) — chained products
        assert_idempotent("\\ln(x \\cdot y \\cdot z)");
    }

    #[test]
    fn idem_sin_of_negate_var() {
        // sin(-x) → -sin(x) — should be stable
        assert_idempotent("\\sin(-x)");
    }

    #[test]
    fn idem_ln_of_e() {
        assert_idempotent("\\ln(e)");
    }

    #[test]
    fn idem_subtract_self() {
        assert_idempotent("x - x");
    }

    #[test]
    fn idem_nested_fractions() {
        assert_idempotent("\\frac{\\frac{x}{2}}{3}");
    }

    #[test]
    fn idem_power_of_fraction() {
        assert_idempotent("(\\frac{x}{y})^{2}");
    }

    #[test]
    fn idem_sum_with_zero() {
        assert_idempotent("x + 0");
    }

    #[test]
    fn idem_product_with_negative() {
        assert_idempotent("-1 \\cdot x");
    }

    // === Round-trip stability (parse → simplify → format → parse → simplify → format) ===

    #[test]
    fn rt_polynomial() {
        assert_roundtrip_stable("x^{2} + 2x + 1");
    }

    #[test]
    fn rt_fraction() {
        assert_roundtrip_stable("\\frac{x^{2} - 1}{x + 1}");
    }

    #[test]
    fn rt_trig() {
        assert_roundtrip_stable("\\sin(x)^{2} + \\cos(x)^{2}");
    }

    #[test]
    fn rt_ln_power() {
        assert_roundtrip_stable("\\ln(x^{3})");
    }

    #[test]
    fn rt_ln_product() {
        assert_roundtrip_stable("\\ln(x \\cdot y)");
    }

    #[test]
    fn rt_power_rules() {
        assert_roundtrip_stable("x^{2} \\cdot x^{3}");
    }

    #[test]
    fn rt_distribute() {
        assert_roundtrip_stable("3(x + 2)");
    }

    #[test]
    fn rt_complex_expression() {
        assert_roundtrip_stable("\\frac{x^{3} - x}{x^{2} - 1}");
    }
}
