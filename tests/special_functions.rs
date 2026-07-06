// Special function recognition from DE structure (ar-special-functions).
//
// When Risch proves an integrand has no elementary antiderivative, the
// integral may still equal a *named* special function — erf, Ei, li — whose
// defining identity (DLMF 7.2.1, 6.2.5, 6.2.8) is exactly the integrand
// shape Risch rejected. Naming the function is strictly more information
// than "non-elementary", and the name must be earned: a structural match
// against a definition is a proof; anything less attaches no name.

#[cfg(test)]
mod special_function_derivative_tests {
    use arithma::{differentiate_and_evaluate, Environment};

    fn approx_eq(a: f64, b: f64, epsilon: f64) -> bool {
        (a - b).abs() < epsilon
    }

    #[test]
    fn test_erf_derivative() {
        let mut env = Environment::new();

        // d/dx erf(x) = (2/√π)·e^{-x²}  (DLMF 7.2.1), at x=1
        env.set("x", 1.0);
        let result = differentiate_and_evaluate("\\erf(x)", "x", &env).unwrap();
        let expected = 2.0 / std::f64::consts::PI.sqrt() * (-1.0f64).exp();
        assert!(
            approx_eq(result, expected, 1e-12),
            "d/dx erf(x) at x=1: expected {}, got {}",
            expected,
            result
        );

        // Chain rule: d/dx erf(x²) = (2/√π)·e^{-x⁴}·2x, at x=1
        let result = differentiate_and_evaluate("\\erf(x^2)", "x", &env).unwrap();
        let expected = 2.0 / std::f64::consts::PI.sqrt() * (-1.0f64).exp() * 2.0;
        assert!(
            approx_eq(result, expected, 1e-12),
            "d/dx erf(x²) at x=1: expected {}, got {}",
            expected,
            result
        );
    }

    #[test]
    fn test_ei_derivative() {
        let mut env = Environment::new();

        // d/dx Ei(x) = e^x/x  (DLMF 6.2.5), at x=2
        env.set("x", 2.0);
        let result = differentiate_and_evaluate("\\Ei(x)", "x", &env).unwrap();
        let expected = 2.0f64.exp() / 2.0;
        assert!(
            approx_eq(result, expected, 1e-12),
            "d/dx Ei(x) at x=2: expected {}, got {}",
            expected,
            result
        );

        // Chain rule: d/dx Ei(2x) = e^{2x}/(2x)·2 = e^{2x}/x, at x=2
        let result = differentiate_and_evaluate("\\Ei(2x)", "x", &env).unwrap();
        let expected = 4.0f64.exp() / 2.0;
        assert!(
            approx_eq(result, expected, 1e-12),
            "d/dx Ei(2x) at x=2: expected {}, got {}",
            expected,
            result
        );
    }

    #[test]
    fn test_li_derivative() {
        let mut env = Environment::new();

        // d/dx li(x) = 1/ln(x)  (DLMF 6.2.8), at x=e²: 1/2
        env.set("x", std::f64::consts::E * std::f64::consts::E);
        let result = differentiate_and_evaluate("\\li(x)", "x", &env).unwrap();
        assert!(
            approx_eq(result, 0.5, 1e-12),
            "d/dx li(x) at x=e²: expected 0.5, got {}",
            result
        );
    }
}

#[cfg(test)]
mod special_recognition_tests {
    use arithma::integration::{integrate_outcome, IntegralOutcome};
    use arithma::{build_expression_tree, derivative, Environment, Evaluator, Tokenizer};

    fn parse_expression(latex: &str) -> arithma::Node {
        let mut tokenizer = Tokenizer::new(latex);
        build_expression_tree(tokenizer.tokenize()).unwrap()
    }

    /// The correctness assertion for a recognized form: d/dx(form) must agree
    /// with the integrand wherever both evaluate. The derivative of any
    /// recognized form is elementary (the special function disappears under
    /// d/dx), so both sides are numerically evaluable.
    fn assert_roundtrip(integrand_latex: &str, form: &arithma::Node, points: &[f64]) {
        use arithma::simplify::Simplifiable;
        let integrand = parse_expression(integrand_latex);
        let form_derivative = derivative::differentiate(form, "x").unwrap();
        // The product rule leaves exact-zero terms that still mention the
        // special function (which refuses numeric evaluation); simplifying
        // folds them away and leaves an elementary, evaluable derivative.
        let env = Environment::new();
        let form_derivative = form_derivative.simplify(&env).unwrap();
        for &p in points {
            let mut env = Environment::new();
            env.set("x", p);
            let lhs = Evaluator::evaluate(&form_derivative, &env).unwrap();
            let rhs = Evaluator::evaluate(&integrand, &env).unwrap();
            assert!(
                (lhs - rhs).abs() <= 1e-9 * rhs.abs().max(1.0),
                "round-trip failed for {} at x={}: d/dx(form)={}, integrand={}",
                integrand_latex,
                p,
                lhs,
                rhs
            );
        }
    }

    /// Integrate and expect a recognized non-elementary result.
    fn expect_special(integrand_latex: &str, function_name: &str, points: &[f64]) {
        let expr = parse_expression(integrand_latex);
        match integrate_outcome(&expr, "x").unwrap() {
            IntegralOutcome::NonElementary {
                certificate,
                special: Some(special),
            } => {
                assert!(!certificate.is_empty(), "certificate must be preserved");
                assert_eq!(
                    special.function, function_name,
                    "wrong special function for {}",
                    integrand_latex
                );
                assert_roundtrip(integrand_latex, &special.form, points);
            }
            other => panic!(
                "expected recognized non-elementary for {}, got {:?}",
                integrand_latex, other
            ),
        }
    }

    #[test]
    fn test_gaussian_recognized_as_erf() {
        // ∫e^{-x²} dx = (√π/2)·erf(x)
        expect_special("\\exp(-x^2)", "erf", &[0.3, 1.1, 2.7]);
    }

    #[test]
    fn test_scaled_gaussian_recognized_as_erf() {
        // Constant multiples fold into the form: ∫3e^{-x²} dx = 3·(√π/2)·erf(x)
        expect_special("3\\exp(-x^2)", "erf", &[0.3, 1.1, 2.7]);
    }

    #[test]
    fn test_standard_normal_kernel_recognized_as_erf() {
        // ∫e^{-x²/2} dx = √(π/2)·erf(x/√2) — the Gaussian an agent actually meets
        expect_special("\\exp(-\\frac{x^2}{2})", "erf", &[0.3, 1.1, 2.7]);
    }

    #[test]
    fn test_exp_over_x_recognized_as_ei() {
        // ∫e^x/x dx = Ei(x)
        expect_special("\\frac{\\exp(x)}{x}", "Ei", &[0.5, 1.3, 2.9]);
    }

    // The stated coverage is c·e^{bx}/x and
    // c/ln(x), but the constant-peeler had no arm for a free factor in the
    // numerator of a Divide whose denominator carries the variable — and
    // simplify normalizes every alternative spelling into exactly that
    // shape, so no spelling could dodge the missing arm.

    #[test]
    fn test_scaled_reciprocal_log_recognized_as_li() {
        // ∫3/ln(x) dx = 3·li(x)
        expect_special("\\frac{3}{\\ln(x)}", "li", &[2.0, 3.5, 7.0]);
    }

    #[test]
    fn test_negative_scaled_li_via_product_spelling() {
        // -3·(1/ln(x)) — simplify routes this into the Divide shape too
        expect_special("-3 \\cdot \\frac{1}{\\ln(x)}", "li", &[2.0, 3.5, 7.0]);
    }

    #[test]
    fn test_scaled_ei_with_constant_in_numerator() {
        // ∫3e^{2x}/x dx = 3·Ei(2x)
        expect_special("\\frac{3\\exp(2x)}{x}", "Ei", &[0.5, 1.3, 2.9]);
    }

    #[test]
    fn test_negative_scaled_ei() {
        // ∫(-2e^x)/x dx = -2·Ei(x)
        expect_special("\\frac{-2\\exp(x)}{x}", "Ei", &[0.5, 1.3, 2.9]);
    }

    #[test]
    fn test_rational_scaled_ei_product_spelling() {
        // ∫(22/7)·e^x/x dx = (22/7)·Ei(x)
        expect_special(
            "\\frac{22}{7} \\cdot \\frac{\\exp(x)}{x}",
            "Ei",
            &[0.5, 1.3, 2.9],
        );
    }

    #[test]
    fn test_scaled_exp_over_x_recognized_as_ei() {
        // ∫e^{2x}/x dx = Ei(2x)
        expect_special("\\frac{\\exp(2x)}{x}", "Ei", &[0.5, 1.3, 2.9]);
    }

    #[test]
    fn test_reciprocal_log_recognized_as_li() {
        // ∫dx/ln(x) = li(x)
        expect_special("\\frac{1}{\\ln(x)}", "li", &[2.0, 3.5, 7.0]);
    }

    #[test]
    fn test_unrecognized_non_elementary_stays_unnamed() {
        // ∫e^{x³} dx is non-elementary and matches no table row: the honest
        // answer remains a bare impossibility proof. No guessed names, ever.
        let expr = parse_expression("\\exp(x^3)");
        match integrate_outcome(&expr, "x").unwrap() {
            IntegralOutcome::NonElementary {
                certificate,
                special: None,
            } => {
                assert!(!certificate.is_empty());
            }
            other => panic!("expected unnamed non-elementary, got {:?}", other),
        }
    }

    #[test]
    fn test_positive_exponent_gaussian_not_recognized() {
        // ∫e^{x²} dx needs erfi, which is not in the v1 table. It must stay
        // unnamed rather than be mislabeled erf.
        let expr = parse_expression("\\exp(x^2)");
        match integrate_outcome(&expr, "x").unwrap() {
            IntegralOutcome::NonElementary { special: None, .. } => {}
            other => panic!("expected unnamed non-elementary, got {:?}", other),
        }
    }

    #[test]
    fn test_latex_level_recognition_for_tool_boundaries() {
        // The one-call helper the CLI and MCP server use to enrich a
        // provably_impossible status: LaTeX in, (name, LaTeX form) out.
        let (name, form) =
            arithma::special_functions::recognize_special_form_latex("\\exp(-x^2)", "x").unwrap();
        assert_eq!(name, "erf");
        assert!(
            form.contains("erf"),
            "form should mention erf, got: {}",
            form
        );

        assert!(
            arithma::special_functions::recognize_special_form_latex("\\exp(x^3)", "x").is_none(),
            "unrecognized integrands must yield no form"
        );
    }

    #[test]
    fn test_emitted_forms_are_valid_reparseable_latex() {
        // The emitted LaTeX must round-trip through our own parser — the two
        // spellings of a radical (\sqrt{u} vs \sqrt(u)) must not diverge, and
        // an agent must be able to feed the form back into any tool.
        for integrand in [
            "\\exp(-x^2)",
            "\\exp(-\\frac{x^2}{2})",
            "\\frac{\\exp(x)}{x}",
            "\\frac{1}{\\ln(x)}",
        ] {
            let (_, form) =
                arithma::special_functions::recognize_special_form_latex(integrand, "x")
                    .unwrap_or_else(|| panic!("recognition failed for {}", integrand));
            assert!(
                !form.contains("\\sqrt("),
                "parenthesized \\sqrt( is not valid LaTeX, got: {}",
                form
            );
            parse_expression(&form); // panics if the form does not re-parse
        }
    }

    #[test]
    fn test_elementary_integral_stays_elementary() {
        // ∫x·e^{-x²} dx is elementary; the outcome API must not disturb it.
        let expr = parse_expression("x \\cdot \\exp(-x^2)");
        match integrate_outcome(&expr, "x").unwrap() {
            IntegralOutcome::Elementary(_) => {}
            other => panic!("expected elementary, got {:?}", other),
        }
    }
}
