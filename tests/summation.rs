#[cfg(test)]
mod summation_tests {
    use arithma::simplify::Simplifiable;
    use arithma::{build_expression_tree, Environment, Evaluator, Tokenizer};

    fn simplify_latex(input: &str) -> String {
        let env = Environment::new();
        let mut tokenizer = Tokenizer::new(input);
        let tokens = tokenizer.tokenize();
        let expr = build_expression_tree(tokens).unwrap();
        let simplified = expr.simplify(&env).unwrap();
        format!("{}", simplified)
    }

    fn eval_with(input: &str, var: &str, val: f64) -> f64 {
        let mut env = Environment::new();
        env.set(var, val);
        let mut tokenizer = Tokenizer::new(input);
        let tokens = tokenizer.tokenize();
        let expr = build_expression_tree(tokens).unwrap();
        Evaluator::evaluate(&expr, &env).unwrap()
    }

    // ── Polynomial sums (Faulhaber) ──────────────────────────

    #[test]
    fn sum_of_k() {
        let result = simplify_latex("\\sum_{k=1}^{n} k");
        assert!(
            result.contains("n") && result.contains("n + 1") && result.contains("2"),
            "Expected n(n+1)/2, got: {}",
            result
        );
    }

    #[test]
    fn sum_of_k_squared() {
        let result = simplify_latex("\\sum_{k=1}^{n} k^2");
        assert!(
            result.contains("n") && result.contains("2n + 1"),
            "Expected n(n+1)(2n+1)/6, got: {}",
            result
        );
    }

    #[test]
    fn sum_of_k_cubed() {
        let result = simplify_latex("\\sum_{k=1}^{n} k^3");
        assert!(
            result.contains("n") && result.contains("4"),
            "Expected n²(n+1)²/4, got: {}",
            result
        );
    }

    #[test]
    fn sum_of_k_fourth() {
        let result = simplify_latex("\\sum_{k=1}^{n} k^4");
        assert!(
            result.contains("n") && result.contains("30"),
            "Expected Faulhaber formula /30, got: {}",
            result
        );
    }

    #[test]
    fn sum_of_odd_numbers() {
        // Σ_{k=1}^{n} (2k-1) = n²
        let result = simplify_latex("\\sum_{k=1}^{n} {2k - 1}");
        assert_eq!(result, "n^{2}");
    }

    #[test]
    fn sum_of_polynomial_2k_plus_3() {
        // Σ_{k=1}^{n} (2k+3) = 2·n(n+1)/2 + 3n = n²+n+3n = n²+4n
        // = n(n+4)
        let closed = simplify_latex("\\sum_{k=1}^{n} {2k + 3}");
        // Verify numerically: at n=10, Σ(2k+3) = 2·55 + 30 = 140
        let val = eval_with(&closed, "n", 10.0);
        assert_eq!(val, 140.0, "Closed form {} at n=10 should be 140", closed);
    }

    // ── Constant sums ────────────────────────────────────────

    #[test]
    fn sum_of_constant() {
        let result = simplify_latex("\\sum_{k=1}^{n} 1");
        assert_eq!(result, "n");
    }

    #[test]
    fn sum_of_constant_5() {
        let result = simplify_latex("\\sum_{k=1}^{n} 5");
        assert_eq!(result, "5n");
    }

    // ── Geometric series ─────────────────────────────────────

    #[test]
    fn geometric_sum_base_2() {
        // Σ_{k=0}^{n} 2^k = 2^{n+1} - 1
        let result = simplify_latex("\\sum_{k=0}^{n} 2^k");
        assert_eq!(result, "2^{n + 1} - 1");
    }

    #[test]
    fn geometric_sum_base_3() {
        // Σ_{k=0}^{n} 3^k = (3^{n+1} - 1)/2
        let closed = simplify_latex("\\sum_{k=0}^{n} 3^k");
        // Verify: at n=4, 1+3+9+27+81 = 121 = (243-1)/2
        let val = eval_with(&closed, "n", 4.0);
        assert_eq!(val, 121.0, "Closed form {} at n=4 should be 121", closed);
    }

    #[test]
    fn geometric_sum_with_coefficient() {
        // Σ_{k=0}^{n} 5·2^k = 5·(2^{n+1} - 1)
        let closed = simplify_latex("\\sum_{k=0}^{n} {5 \\cdot 2^k}");
        // At n=3: 5(1+2+4+8) = 75 = 5·(16-1)
        let val = eval_with(&closed, "n", 3.0);
        assert_eq!(val, 75.0, "Closed form {} at n=3 should be 75", closed);
    }

    // ── Telescoping sums ─────────────────────────────────────

    #[test]
    fn telescoping_harmonic_difference() {
        // Σ_{k=1}^{n} (1/k - 1/(k+1)) = 1 - 1/(n+1) = n/(n+1)
        let result = simplify_latex("\\sum_{k=1}^{n} {\\frac{1}{k} - \\frac{1}{k+1}}");
        let val = eval_with(&result, "n", 100.0);
        let expected = 100.0 / 101.0;
        assert!(
            (val - expected).abs() < 1e-10,
            "Expected {}, got {} from: {}",
            expected,
            val,
            result
        );
    }

    // ── Numeric verification of symbolic formulas ────────────

    #[test]
    fn faulhaber_verified_at_n_100() {
        let n = 100.0;

        // Σ_{k=1}^{100} k = 5050
        let val = eval_with(&simplify_latex("\\sum_{k=1}^{n} k"), "n", n);
        assert_eq!(val, 5050.0);

        // Σ_{k=1}^{100} k² = 338350
        let val = eval_with(&simplify_latex("\\sum_{k=1}^{n} k^2"), "n", n);
        assert_eq!(val, 338350.0);

        // Σ_{k=1}^{100} k³ = 25502500
        let val = eval_with(&simplify_latex("\\sum_{k=1}^{n} k^3"), "n", n);
        assert_eq!(val, 25502500.0);
    }

    #[test]
    fn constant_bounds_evaluated() {
        // When both bounds are constant, the symbolic form should evaluate
        assert_eq!(simplify_latex("\\sum_{k=1}^{100} k"), "5050");
    }

    #[test]
    fn large_constant_sum_via_closed_form() {
        // Range > 10 uses closed form, not inline expansion
        let result = simplify_latex("\\sum_{k=1}^{50} k^2");
        assert_eq!(result, "42925");
    }

    #[test]
    fn sum_from_zero() {
        // Σ_{k=0}^{n} k = n(n+1)/2 (k=0 contributes 0)
        let closed = simplify_latex("\\sum_{k=0}^{n} k");
        let val = eval_with(&closed, "n", 10.0);
        assert_eq!(
            val, 55.0,
            "Σ_{{k=0}}^{{10}} k = 55, got {} from: {}",
            val, closed
        );
    }

    #[test]
    fn sum_from_2() {
        // Σ_{k=2}^{n} k = n(n+1)/2 - 1
        let closed = simplify_latex("\\sum_{k=2}^{n} k");
        let val = eval_with(&closed, "n", 10.0);
        // Σ_{k=2}^{10} k = 2+3+...+10 = 55-1 = 54
        assert_eq!(
            val, 54.0,
            "Σ_{{k=2}}^{{10}} k = 54, got {} from: {}",
            val, closed
        );
    }

    // ── Unbraced multi-term bodies (Bug #2 regression) ────────

    #[test]
    fn unbraced_coefficient() {
        // Bug #2: \sum_{k=1}^{n} 3·k² should work without braces
        let closed = simplify_latex("\\sum_{k=1}^{n} 3 \\cdot k^2");
        let val = eval_with(&closed, "n", 10.0);
        // 3·(10·11·21/6) = 3·385 = 1155
        assert_eq!(
            val, 1155.0,
            "3·Σk² at n=10 should be 1155, got {} from: {}",
            val, closed
        );
    }

    #[test]
    fn braced_linear_combination() {
        let closed = simplify_latex("\\sum_{k=1}^{n} {k^2 + k}");
        let val = eval_with(&closed, "n", 5.0);
        // (5·6·11/6) + (5·6/2) = 55 + 15 = 70
        assert_eq!(
            val, 70.0,
            "Σ(k²+k) at n=5 should be 70, got {} from: {}",
            val, closed
        );
    }

    #[test]
    fn small_range_simplifies_to_number() {
        assert_eq!(simplify_latex("\\sum_{k=1}^{5} k"), "15");
        assert_eq!(simplify_latex("\\sum_{k=1}^{3} k^2"), "14");
    }

    // ── Symbolic coefficient summation ─────────────────────────

    #[test]
    fn symbolic_coeff_sum_a_times_k_squared() {
        // Σ_{k=1}^{n} a·k² = a·n(n+1)(2n+1)/6
        let closed = simplify_latex("\\sum_{k=1}^{n} a \\cdot k^2");
        assert!(
            !closed.contains("\\sum"),
            "Should produce closed form, got: {}",
            closed
        );
        // Verify numerically: at n=10, a=3 → 3·385 = 1155
        let mut env = Environment::new();
        env.set("n", 10.0);
        env.set("a", 3.0);
        let mut tokenizer = Tokenizer::new(&closed);
        let expr = build_expression_tree(tokenizer.tokenize()).unwrap();
        let val = Evaluator::evaluate(&expr, &env).unwrap();
        assert_eq!(val, 1155.0, "a·Σk² at a=3,n=10 should be 1155, got {}", val);
    }

    #[test]
    fn symbolic_coeff_sum_a_k2_plus_b_k() {
        // Σ_{k=1}^{n} (a·k² + b·k) = a·n(n+1)(2n+1)/6 + b·n(n+1)/2
        let closed = simplify_latex("\\sum_{k=1}^{n} {a \\cdot k^2 + b \\cdot k}");
        assert!(
            !closed.contains("\\sum"),
            "Should produce closed form, got: {}",
            closed
        );
        // Verify: a=2, b=3, n=5 → 2·55 + 3·15 = 110+45 = 155
        let mut env = Environment::new();
        env.set("n", 5.0);
        env.set("a", 2.0);
        env.set("b", 3.0);
        let mut tokenizer = Tokenizer::new(&closed);
        let expr = build_expression_tree(tokenizer.tokenize()).unwrap();
        let val = Evaluator::evaluate(&expr, &env).unwrap();
        assert_eq!(
            val, 155.0,
            "a·Σk²+b·Σk at a=2,b=3,n=5 should be 155, got {}",
            val
        );
    }

    #[test]
    fn symbolic_coeff_no_regression_pure_numeric() {
        // Pure numeric coefficient sums still work via the polynomial path
        let closed = simplify_latex("\\sum_{k=1}^{n} 3k^2");
        let val = eval_with(&closed, "n", 10.0);
        assert_eq!(val, 1155.0, "3·Σk² at n=10 should be 1155, got {}", val);
    }

    // ── Telescoping via partial fractions ─────────────────────

    #[test]
    fn telescoping_pf_reciprocal_product() {
        // Σ_{k=1}^{n} 1/(k(k+1)) = n/(n+1)
        let closed = simplify_latex("\\sum_{k=1}^{n} \\frac{1}{k(k+1)}");
        assert!(
            !closed.contains("\\sum"),
            "Should produce closed form, got: {}",
            closed
        );
        let val = eval_with(&closed, "n", 100.0);
        let expected = 100.0 / 101.0;
        assert!(
            (val - expected).abs() < 1e-10,
            "1/(k(k+1)) sum at n=100 should be {}, got {} from: {}",
            expected,
            val,
            closed
        );
    }

    #[test]
    fn telescoping_pf_shifted_product() {
        // Σ_{k=1}^{n} 1/(k(k+2)) = 1/2 · (1 + 1/2 - 1/(n+1) - 1/(n+2))
        //   = 1/2 · (3/2 - (2n+3)/((n+1)(n+2)))
        // Partial fractions: 1/(k(k+2)) = 1/2 · (1/k - 1/(k+2))
        // This is a shift-by-2 telescoping, not shift-by-1 — may not be caught by our detector.
        // Testing that PF at least runs without error.
        let result = simplify_latex("\\sum_{k=1}^{n} \\frac{1}{k(k+2)}");
        // If it produces a closed form, verify it; otherwise it returns unevaluated
        if !result.contains("\\sum") {
            let val = eval_with(&result, "n", 10.0);
            // Σ_{k=1}^{10} 1/(k(k+2)) ≈ 0.6590909...
            let expected: f64 = (1..=10).map(|k| 1.0 / (k as f64 * (k as f64 + 2.0))).sum();
            assert!(
                (val - expected).abs() < 1e-10,
                "1/(k(k+2)) sum at n=10 should be {}, got {} from: {}",
                expected,
                val,
                result
            );
        }
    }

    #[test]
    fn telescoping_pf_no_regression_explicit_difference() {
        // The existing telescoping detector still works on explicit differences
        let result = simplify_latex("\\sum_{k=1}^{n} {\\frac{1}{k} - \\frac{1}{k+1}}");
        let val = eval_with(&result, "n", 100.0);
        let expected = 100.0 / 101.0;
        assert!(
            (val - expected).abs() < 1e-10,
            "Explicit telescoping should still work, got {} from: {}",
            val,
            result
        );
    }

    // ── MCP path (simplify tool handles summation) ───────────

    #[test]
    fn simplify_path_handles_summation() {
        let env = Environment::new();
        let mut tokenizer = Tokenizer::new("\\sum_{k=1}^{n} k");
        let tokens = tokenizer.tokenize();
        let expr = build_expression_tree(tokens).unwrap();
        let simplified = expr.simplify(&env).unwrap();
        // The result should not be a Summation node anymore
        assert!(
            !format!("{}", simplified).contains("\\sum"),
            "Should produce closed form, got: {}",
            simplified
        );
    }
}

// ── Composition: Σ/Π as atoms inside larger expressions ─────
// Regression tests for Carl's Session-43 report: any expression
// containing \sum or \prod silently discarded everything around it
// (2·Σk returned 15, 1+Σk² returned 55). A CAS must never return a
// confidently wrong number; these pin the composed values.

#[cfg(test)]
mod indexed_composition_tests {
    use arithma::{build_expression_tree, Environment, Evaluator, Tokenizer};

    fn eval(input: &str) -> f64 {
        let env = Environment::new();
        let mut tokenizer = Tokenizer::new(input);
        let tokens = tokenizer.tokenize();
        let expr = build_expression_tree(tokens).unwrap();
        Evaluator::evaluate(&expr, &env).unwrap()
    }

    #[test]
    fn scalar_times_sum() {
        assert_eq!(eval("2 \\cdot \\sum_{k=1}^{5} k"), 30.0);
    }

    #[test]
    fn constant_plus_sum() {
        assert_eq!(eval("1 + \\sum_{k=1}^{5} k^2"), 56.0);
    }

    #[test]
    fn sum_plus_constant() {
        assert_eq!(eval("\\sum_{k=1}^{5} k^2 + 1"), 56.0);
    }

    #[test]
    fn braced_body_sum_plus_constant() {
        assert_eq!(eval("\\sum_{k=1}^{5}{k^2} + 1"), 56.0);
    }

    #[test]
    fn constant_plus_product() {
        assert_eq!(eval("1 + \\prod_{k=1}^{4} k"), 25.0);
    }

    #[test]
    fn scalar_times_product() {
        assert_eq!(eval("2 \\cdot \\prod_{k=1}^{4} k"), 48.0);
    }

    #[test]
    fn sum_minus_sum() {
        // Σk² − Σk over 1..5 = 55 − 15 = 40
        assert_eq!(eval("\\sum_{k=1}^{5} k^2 - \\sum_{k=1}^{5} k"), 40.0);
    }

    #[test]
    fn sum_divided_by_constant() {
        assert_eq!(eval("\\frac{\\sum_{k=1}^{5} k}{3}"), 5.0);
    }

    #[test]
    fn nested_sum_braced() {
        // Σ_{i=1..3} Σ_{j=1..2} (i·j) = (1+2+3)·(1+2) = 18
        assert_eq!(eval("\\sum_{i=1}^{3}{\\sum_{j=1}^{2}{i \\cdot j}}"), 18.0);
    }

    #[test]
    fn sum_equation_still_parses() {
        // Σ = 15 must still build an Equation node, not error.
        let mut tokenizer = Tokenizer::new("\\sum_{k=1}^{5} k = 15");
        let tokens = tokenizer.tokenize();
        let expr = build_expression_tree(tokens).unwrap();
        assert!(matches!(expr, arithma::Node::Equation(_, _)));
    }

    #[test]
    fn lone_sum_still_works() {
        assert_eq!(eval("\\sum_{k=1}^{5} k"), 15.0);
    }

    #[test]
    fn unbraced_body_still_stops_at_top_level_plus() {
        // The unbraced body convention: Σ_{k=1}^{5} k + 1 sums k, then
        // adds 1 outside. (Braces widen the body: Σ{k+1} would differ.)
        assert_eq!(eval("\\sum_{k=1}^{5} k + 1"), 16.0);
    }
}
