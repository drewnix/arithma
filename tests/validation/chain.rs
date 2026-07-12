//! Tests for verify_chain: reasoning-chain verification.
//!
//! A chain is an ordered list of steps; each step after the first declares
//! a relation to its predecessor. The chain verdict is fail at the first
//! failing step; the chain status is the minimum evidence across steps
//! (exact > verified > heuristic > unable_to_compute) — numeric evidence
//! never upgrades, per docs/result-status.md.

use arithma::chain::{verify_chain, ChainStepInput, Relation, Verdict};
use arithma::status::ResultStatus;
use arithma::Environment;

/// An `equals` step with a label.
fn eq_step(label: &str, expr: &str) -> ChainStepInput {
    ChainStepInput {
        label: Some(label.to_string()),
        expr: expr.to_string(),
        relation: Relation::Equals,
        variable: None,
        value: None,
    }
}

#[test]
fn all_pass_polynomial_chain_is_exact() {
    // (x+1)^2 = x^2+2x+1 = x(x+2)+1 — all canonical-form equalities over Q.
    let steps = vec![
        eq_step("start", "(x+1)^2"),
        eq_step("expand", "x^2 + 2x + 1"),
        eq_step("regroup", "x(x + 2) + 1"),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();

    assert_eq!(result.verdict, Verdict::Pass);
    assert_eq!(result.status.status, ResultStatus::Exact);
    assert_eq!(result.steps.len(), 3);
    // Anchor step makes no claim and passes vacuously.
    assert_eq!(result.steps[0].verdict, Verdict::Pass);
    assert_eq!(result.steps[1].verdict, Verdict::Pass);
    assert_eq!(result.steps[2].verdict, Verdict::Pass);
    assert_eq!(result.first_failure, None);
}

#[test]
fn mid_chain_failure_reports_first_failure_with_counterexample() {
    // Step 2 is wrong: (x+1)^2 ≠ x^2+1. Step 3 is consistent with step 2,
    // so only step 2 fails.
    let steps = vec![
        eq_step("start", "(x+1)^2"),
        eq_step("bad-expand", "x^2 + 1"),
        eq_step("regroup", "x^2 + 1"),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();

    assert_eq!(result.verdict, Verdict::Fail);
    assert_eq!(result.first_failure, Some(1));
    assert_eq!(result.steps[1].verdict, Verdict::Fail);
    // The counterexample is the diagnosis: the failing step carries it.
    assert!(result.steps[1].status.counterexample_json().is_some());
    // The step after the failure is still checked (against its predecessor).
    assert_eq!(result.steps[2].verdict, Verdict::Pass);
}

#[test]
fn single_step_chain_passes_vacuously() {
    let steps = vec![eq_step("only", "x^2 + 1")];
    let result = verify_chain(&steps, &Environment::new()).unwrap();

    assert_eq!(result.verdict, Verdict::Pass);
    assert_eq!(result.status.status, ResultStatus::Exact);
    assert_eq!(result.steps.len(), 1);
    // A one-step chain verifies nothing; the status says so.
    assert!(result
        .status
        .caveats
        .iter()
        .any(|c| c.contains("anchor only")));
}

#[test]
fn empty_chain_is_an_error() {
    let result = verify_chain(&[], &Environment::new());
    assert!(result.is_err());
}

#[test]
fn transcendental_equality_is_verified_not_exact() {
    // sin(x)+sin(x) = 2 sin(x): true, but outside the algebraic-exact
    // fragment — numeric evidence only, so the chain caps at verified.
    let steps = vec![
        eq_step("start", "\\sin(x) + \\sin(x)"),
        eq_step("collect", "2\\sin(x)"),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();

    assert_eq!(result.verdict, Verdict::Pass);
    match result.status.status {
        ResultStatus::Verified { points_tested } => assert!(points_tested >= 3),
        ref other => panic!("expected Verified, got {:?}", other),
    }
    assert_eq!(result.weakest_step, Some(1));
}

/// A step with an explicit relation and optional variable/value.
fn rel_step(
    label: &str,
    expr: &str,
    relation: Relation,
    variable: Option<&str>,
    value: Option<&str>,
) -> ChainStepInput {
    ChainStepInput {
        label: Some(label.to_string()),
        expr: expr.to_string(),
        relation,
        variable: variable.map(str::to_string),
        value: value.map(str::to_string),
    }
}

#[test]
fn derivative_of_polynomial_is_exact() {
    let steps = vec![
        eq_step("f", "x^3 + 2x"),
        rel_step("f'", "3x^2 + 2", Relation::DerivativeOf, Some("x"), None),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Pass);
    assert_eq!(result.status.status, ResultStatus::Exact);
}

#[test]
fn derivative_of_wrong_claim_fails() {
    let steps = vec![
        eq_step("f", "x^3"),
        rel_step("f'", "x^2", Relation::DerivativeOf, Some("x"), None),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Fail);
    assert_eq!(result.first_failure, Some(1));
}

#[test]
fn derivative_of_defaults_to_variable_x() {
    let steps = vec![
        eq_step("f", "\\sin(x)"),
        rel_step("f'", "\\cos(x)", Relation::DerivativeOf, None, None),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Pass);
    // d/dx sin = cos comes straight from the (sound, complete) rule table
    // and the outputs agree syntactically — no simplifier trust involved.
    assert_eq!(result.status.status, ResultStatus::Exact);
}

#[test]
fn integral_of_earns_exact_via_roundtrip() {
    // x^3 + 5 is an antiderivative of 3x^2 — the constant must not matter.
    let steps = vec![
        eq_step("integrand", "3x^2"),
        rel_step(
            "antiderivative",
            "x^3 + 5",
            Relation::IntegralOf,
            Some("x"),
            None,
        ),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Pass);
    assert_eq!(result.status.status, ResultStatus::Exact);
}

#[test]
fn integral_of_wrong_antiderivative_fails() {
    let steps = vec![
        eq_step("integrand", "x^2"),
        rel_step("wrong", "x^3", Relation::IntegralOf, Some("x"), None),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Fail);
}

#[test]
fn substitution_step_handles_variable_set_change() {
    // x := u + 1 replaces x by a fresh variable — the free-variable set
    // changes between steps and the check must follow it.
    let steps = vec![
        eq_step("start", "x^2 + y"),
        rel_step(
            "sub",
            "(u+1)^2 + y",
            Relation::Substitution,
            Some("x"),
            Some("u + 1"),
        ),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Pass);
    assert_eq!(result.status.status, ResultStatus::Exact);
}

#[test]
fn substitution_with_wrong_result_fails() {
    let steps = vec![
        eq_step("start", "x^2"),
        rel_step(
            "sub",
            "u^2",
            Relation::Substitution,
            Some("x"),
            Some("u + 1"),
        ),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Fail);
}

#[test]
fn substitution_without_value_is_an_error() {
    let steps = vec![
        eq_step("start", "x^2"),
        rel_step("sub", "4", Relation::Substitution, Some("x"), None),
    ];
    assert!(verify_chain(&steps, &Environment::new()).is_err());
}

#[test]
fn solution_of_verifies_membership_exactly() {
    let steps = vec![
        eq_step("equation", "x^2 - 4 = 0"),
        rel_step("root", "x = 2", Relation::SolutionOf, None, None),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Pass);
    // Checking a root is exact rational arithmetic — a checker, not a
    // finder — but membership is not completeness.
    assert_eq!(result.status.status, ResultStatus::Exact);
    assert!(result
        .status
        .caveats
        .iter()
        .any(|c| c.contains("completeness")));
}

#[test]
fn solution_of_rejects_a_non_solution() {
    let steps = vec![
        eq_step("equation", "x^2 - 4 = 0"),
        rel_step("root", "x = 3", Relation::SolutionOf, None, None),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Fail);
    assert!(result.steps[1].status.counterexample_json().is_some());
}

#[test]
fn solution_of_requires_equations() {
    let steps = vec![
        eq_step("not-an-equation", "x^2 - 4"),
        rel_step("root", "x = 2", Relation::SolutionOf, None, None),
    ];
    assert!(verify_chain(&steps, &Environment::new()).is_err());
}

#[test]
fn implies_pass_is_capped_at_verified() {
    // x = 2 ⇒ x^2 = 4. The check substitutes the antecedent's solutions
    // into the consequent; even when every evaluation is exact, finitely
    // many checked solutions are evidence, not proof of implication.
    let steps = vec![
        eq_step("antecedent", "x = 2"),
        rel_step("consequent", "x^2 = 4", Relation::Implies, Some("x"), None),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Pass);
    assert!(matches!(
        result.status.status,
        ResultStatus::Verified { .. }
    ));
}

#[test]
fn implies_fails_when_a_solution_violates_the_consequent() {
    // x^2 = 4 does NOT imply x = 2: the solution x = -2 is the witness.
    let steps = vec![
        eq_step("antecedent", "x^2 = 4"),
        rel_step("consequent", "x = 2", Relation::Implies, Some("x"), None),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Fail);
    assert!(result.steps[1].status.counterexample_json().is_some());
}

#[test]
fn factored_form_of_is_exact_over_q() {
    let steps = vec![
        eq_step("expanded", "x^2 - 1"),
        rel_step(
            "factored",
            "(x-1)(x+1)",
            Relation::FactoredFormOf,
            None,
            None,
        ),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Pass);
    assert_eq!(result.status.status, ResultStatus::Exact);
}

#[test]
fn factored_form_of_wrong_factors_fails() {
    let steps = vec![
        eq_step("expanded", "x^2 - 1"),
        rel_step(
            "factored",
            "(x-1)(x+2)",
            Relation::FactoredFormOf,
            None,
            None,
        ),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Fail);
}

#[test]
fn unknown_relation_string_is_rejected() {
    assert!(Relation::parse("proves").is_err());
    assert_eq!(Relation::parse("equals").unwrap(), Relation::Equals);
}

#[test]
fn chain_status_is_minimum_across_steps() {
    // Step 1: exact polynomial identity. Step 2: transcendental (verified).
    // The chain must report the weaker evidence.
    let steps = vec![
        eq_step("start", "(x+1)^2 - x^2 - 2x - 1 + \\sin(x)"),
        eq_step("cancel", "\\sin(x)"),
        eq_step("rewrite", "2\\sin(\\frac{x}{2})\\cos(\\frac{x}{2})"),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();

    assert_eq!(result.verdict, Verdict::Pass);
    assert!(matches!(
        result.status.status,
        ResultStatus::Verified { .. }
    ));
}

// ---------------------------------------------------------------------------
// Adversarial-review regression suite (PR #67, round 1).
// Each test names the finding it pins down.
// ---------------------------------------------------------------------------

use arithma::assumptions::Assumptions;

fn env_with(assumptions: serde_json::Value) -> Environment {
    Environment::with_assumptions(Assumptions::from_json(&assumptions).unwrap())
}

#[test]
fn r1_constant_offset_below_tolerance_is_refuted_exactly() {
    // x vs x + 10^-15: provably false in the fragment. The f64 tolerance
    // must never get a vote — the simplified difference is a nonzero
    // rational constant, which is a disproof certificate.
    let steps = vec![
        eq_step("start", "x"),
        eq_step("offset", "x + \\frac{1}{1000000000000000}"),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Fail);
    assert_eq!(result.steps[1].status.status, ResultStatus::Exact);
}

#[test]
fn r1_relative_offset_below_tolerance_is_refuted_exactly() {
    // x vs x + x/10^13: non-constant in-fragment difference — exact
    // rational sampling finds a nonzero exact value; no tolerance anywhere.
    let steps = vec![
        eq_step("start", "x"),
        eq_step("offset", "x + \\frac{x}{10000000000000}"),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Fail);
    assert_eq!(result.steps[1].status.status, ResultStatus::Exact);
    assert!(result.steps[1].status.counterexample_json().is_some());
}

#[test]
fn r2_euler_constant_is_not_a_free_variable() {
    // ln(e^x) = x is true; sampling e := 0.5 refuted it and handed the
    // user a "counterexample" that redefines Euler's constant.
    let steps = vec![eq_step("start", "\\ln(e^{x})"), eq_step("simplified", "x")];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Pass);
}

#[test]
fn r2_pi_is_not_a_free_variable() {
    let steps = vec![
        eq_step("start", "\\sin(2 \\cdot \\pi)"),
        eq_step("zero", "0"),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Pass);
}

#[test]
fn r2_regression_e_to_minus_50_documented_limitation() {
    // e^{-50} ≠ 0, but with e bound to its constant the difference is
    // ~2e-22 and f64 tolerance accepts it. This is the documented limit of
    // numeric evidence until the `approximate` tier lands (ar-schema-v2).
    // This test pins the CURRENT behavior so the eventual fix flips it
    // deliberately, not accidentally.
    let steps = vec![eq_step("start", "e^{-50}"), eq_step("zero", "0")];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Pass); // known false pass — see above
    assert!(matches!(
        result.status.status,
        ResultStatus::Verified { .. }
    ));
}

#[test]
fn r3_substitution_refuses_binder_capture() {
    // Σ_{k=1}^{3} k·x with x := k would capture the bound index and
    // silently produce Σ k² = 14. Refusal with an explicit capture error;
    // silence is not acceptable in either direction.
    let steps = vec![
        eq_step("start", "\\sum_{k=1}^{3} k \\cdot x"),
        rel_step(
            "sub",
            "6 \\cdot k",
            Relation::Substitution,
            Some("x"),
            Some("k"),
        ),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.steps[1].verdict, Verdict::Inconclusive);
    match &result.steps[1].status.status {
        ResultStatus::UnableToCompute { reason } => {
            assert!(reason.contains("capture"), "got: {}", reason)
        }
        other => panic!("expected UnableToCompute, got {:?}", other),
    }
}

#[test]
fn r3_capture_artifact_cannot_be_confirmed() {
    // The dual probe: the capture artifact (14) must not PASS either.
    let steps = vec![
        eq_step("start", "\\sum_{k=1}^{3} k \\cdot x"),
        rel_step("sub", "14", Relation::Substitution, Some("x"), Some("k")),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_ne!(result.steps[1].verdict, Verdict::Pass);
}

#[test]
fn r4_solution_of_accepts_negative_roots() {
    // "x = -2" must parse: the tool's own counterexample for x²=4 ⇒ x=2
    // is a step its parser previously rejected.
    let steps = vec![
        eq_step("equation", "x^2 - 4 = 0"),
        rel_step("root", "x = -2", Relation::SolutionOf, None, None),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Pass);
    assert_eq!(result.status.status, ResultStatus::Exact);
}

#[test]
fn r5a_failing_chain_status_carries_the_counterexample() {
    // A passing verified step must not outrank the failing step in the
    // chain-level report: on FAIL, the chain status is the failing step's
    // report, diagnosis included.
    let steps = vec![
        eq_step("start", "\\sin(x) + \\sin(x)"),
        eq_step("collect", "2\\sin(x)"),
        eq_step("broken", "2\\sin(x) + 1"),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Fail);
    assert!(
        result.status.counterexample_json().is_some(),
        "chain-level FAIL status must carry the failing step's counterexample"
    );
}

#[test]
fn r5b_parametric_implies_counterexample_has_values() {
    // x² = a² does not imply x = a (witness: x = −a). The machine-readable
    // counterexample must carry actual numbers, not nulls.
    let steps = vec![
        eq_step("antecedent", "x^2 = a^2"),
        rel_step("consequent", "x = a", Relation::Implies, Some("x"), None),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Fail);
    let cx = result.steps[1]
        .status
        .counterexample_json()
        .expect("counterexample present");
    let point = cx["point"].as_object().expect("point object");
    assert!(
        point.values().all(|v| v.is_number()),
        "counterexample point must be numeric, got: {}",
        cx
    );
}

#[test]
fn r5c_shared_undefined_domain_is_inconclusive_not_verified() {
    // Under x < 0 both sides of ln(x) = ln(x)·(2/2) are undefined at every
    // sample point. Points of shared undefinedness test domain agreement,
    // not values — they must not count as evidence.
    let steps = vec![
        eq_step("start", "\\ln(x)"),
        eq_step("times-one", "\\ln(x) \\cdot \\frac{2}{2}"),
    ];
    let env = env_with(serde_json::json!({"x": ["negative"]}));
    let result = verify_chain(&steps, &env).unwrap();
    assert_eq!(result.verdict, Verdict::Inconclusive);
}

#[test]
fn r6b_derivative_variable_is_inferred_when_unambiguous() {
    // d/dt t² = 2t: with a single free variable, defaulting to x would
    // refute a true step (d/dx t² = 0).
    let steps = vec![
        eq_step("f", "t^2"),
        rel_step("f'", "2t", Relation::DerivativeOf, None, None),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Pass);
    assert_eq!(result.status.status, ResultStatus::Exact);
}

#[test]
fn r6b_ambiguous_derivative_variable_is_an_error() {
    let steps = vec![
        eq_step("f", "x \\cdot y"),
        rel_step("f'", "y", Relation::DerivativeOf, None, None),
    ];
    let err = verify_chain(&steps, &Environment::new()).unwrap_err();
    assert!(err.contains("variable"), "got: {}", err);
}

#[test]
fn r6c_display_coincidence_is_not_syntactic_identity() {
    // √x·1 prints identically to √x after normalization, but they are not
    // the same tree — the mechanism must not claim syntactic identity it
    // did not establish. It IS however a unit-law identity (u·1 = u holds
    // in every interpretation, undefined points included), so the honest
    // outcome is exact via the named unit_normal_form mechanism.
    let steps = vec![
        eq_step("start", "\\sqrt{x} \\cdot 1"),
        eq_step("dropped-one", "\\sqrt{x}"),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Pass);
    assert_eq!(result.steps[1].mechanism, "unit_normal_form");
    assert_eq!(result.steps[1].status.status, ResultStatus::Exact);
}

#[test]
fn p3_zero_over_nonzero_simplifies_so_fragment_identities_stay_exact() {
    // An in-fragment identity (independently machine-checked in a proof
    // assistant) fell to numeric
    // sampling because the simplifier lacked 0/u → 0 and the difference
    // stalled at \frac{0}{...}. The evidence ladder must decide such
    // identities exactly — by canonical form or by the difference rule.
    let steps = vec![
        eq_step(
            "ratio",
            "\\frac{(1-b)(1+3a)}{(1-a)(1+3b)} - \\frac{(1-b)(1+3a)}{(1-a)(1+3b)} + \\frac{0}{x^2+1}",
        ),
        eq_step("zero", "0"),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Pass);
    assert_eq!(result.status.status, ResultStatus::Exact);
}

// ---------------------------------------------------------------------------
// Adversarial-review residuals (PR #67, round 2) — none blocking, all
// fixed before merge anyway.
// ---------------------------------------------------------------------------

#[test]
fn residual2_exact_counterexample_survives_f64_collapse() {
    // 1/3 + 1/6 ≠ 1/2 + 10⁻³⁰, but both render to 0.5 in f64. The
    // counterexample must carry the exact values so it never asserts a
    // disagreement its own numbers fail to exhibit. The 30-digit literal
    // requires bigint parsing (ar-bigint-literals) to stay exact.
    let steps = vec![
        eq_step("a", "\\frac{1}{3} + \\frac{1}{6}"),
        eq_step(
            "b",
            "\\frac{1}{2} + \\frac{1}{1000000000000000000000000000000}",
        ),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Fail);
    let cx = result.steps[1]
        .status
        .counterexample_json()
        .expect("counterexample");
    let lhs = cx["lhs_exact"].as_str().expect("lhs_exact string");
    let rhs = cx["rhs_exact"].as_str().expect("rhs_exact string");
    assert_ne!(lhs, rhs, "exact witnesses must differ: {}", cx);
}

#[test]
fn residual3_variable_free_transcendental_comparison_is_one_evaluation() {
    // The golden ratio solves x² = x + 1. The comparison after substitution
    // is variable-free and outside the ℚ fragment: one f64 evaluation is
    // the evidence — reporting verified(12) for the same point twelve
    // times is evidence inflation.
    let steps = vec![
        eq_step("equation", "x^2 = x + 1"),
        rel_step(
            "root",
            "x = \\frac{1 + \\sqrt{5}}{2}",
            Relation::SolutionOf,
            None,
            None,
        ),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Pass);
    match result.steps[1].status.status {
        ResultStatus::Verified { points_tested } => assert_eq!(points_tested, 1),
        ref other => panic!("expected Verified, got {:?}", other),
    }
}

#[test]
fn design_capture_refusal_is_step_level_and_preserves_the_chain() {
    // A capture at step 2 must not amputate the report for step 1: the
    // refusal is a step-level inconclusive, and the audit trail survives.
    let steps = vec![
        eq_step("start", "\\sum_{k=1}^{3} k \\cdot x"),
        eq_step("same", "\\sum_{k=1}^{3} k \\cdot x"),
        rel_step(
            "capture",
            "6 \\cdot k",
            Relation::Substitution,
            Some("x"),
            Some("k"),
        ),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Inconclusive);
    assert_eq!(result.steps[1].verdict, Verdict::Pass);
    assert_eq!(result.steps[2].verdict, Verdict::Inconclusive);
    match &result.steps[2].status.status {
        ResultStatus::UnableToCompute { reason } => {
            assert!(reason.contains("capture"), "got: {}", reason)
        }
        other => panic!("expected UnableToCompute, got {:?}", other),
    }
}

#[test]
fn p3_zero_over_fraction_node_simplifies_to_zero() {
    // Minimal case: the zero-numerator reduction fired only for
    // polynomial denominators; \frac{0}{\frac{x}{y}} stalled.
    use arithma::simplify::Simplifiable;
    let node = arithma::parse_latex_raw("\\frac{0}{\\frac{x}{y}}").unwrap();
    let simplified = node.simplify(&Environment::new()).unwrap();
    assert_eq!(format!("{}", simplified), "0");
}

#[test]
fn p3_correction_ratio_chain_lands_exact() {
    // A rational-function identity from a real research workflow,
    // independently proved in Lean; the chain verifier must agree exactly.
    let steps = vec![
        eq_step(
            "excess-ratio",
            "\\frac{\\frac{3}{1+2 \\cdot \\beta} - 1}{\\frac{3}{1+2 \\cdot \\alpha} - 1}",
        ),
        eq_step(
            "closed-form",
            "\\frac{(1-\\beta) \\cdot (1+2 \\cdot \\alpha)}{(1-\\alpha) \\cdot (1+2 \\cdot \\beta)}",
        ),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Pass);
    assert_eq!(result.status.status, ResultStatus::Exact);
}

// ---------------------------------------------------------------------------
// Probe-harness findings (PR #67, round 3): gaps in the round-2 fixes
// themselves, surfaced by adversarial probes against the fixed branch.
// ---------------------------------------------------------------------------

#[test]
fn probe_zero_over_hidden_zero_is_not_simplified_to_zero() {
    // sin²x + cos²x − 1 is identically zero but does not reduce to the
    // literal 0. 0/(that) is 0/0 everywhere — undefined, not 0. The
    // 0/u → 0 rule must fire only where u is certified nonzero, i.e.
    // inside the Q fragment.
    use arithma::simplify::Simplifiable;
    let node = arithma::parse_latex_raw("\\frac{0}{\\sin(x)^2 + \\cos(x)^2 - 1}").unwrap();
    let simplified = node.simplify(&Environment::new()).unwrap();
    assert_ne!(format!("{}", simplified), "0");
}

#[test]
fn probe_solution_of_capture_is_step_level() {
    // The capture-refusal-as-step-outcome decision must cover the
    // substitutions inside solution_of, not just the substitution relation.
    let steps = vec![
        eq_step("eq", "\\sum_{k=1}^{3} k \\cdot x = 0"),
        rel_step("root", "x = k", Relation::SolutionOf, None, None),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.steps[1].verdict, Verdict::Inconclusive);
    match &result.steps[1].status.status {
        ResultStatus::UnableToCompute { reason } => {
            assert!(reason.contains("capture"), "got: {}", reason)
        }
        other => panic!("expected UnableToCompute, got {:?}", other),
    }
}

#[test]
fn probe_implies_capture_is_step_level() {
    let steps = vec![
        eq_step("antecedent", "x = k"),
        rel_step(
            "consequent",
            "\\sum_{k=1}^{3} k \\cdot x = \\sum_{k=1}^{3} k^2",
            Relation::Implies,
            Some("x"),
            None,
        ),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.steps[1].verdict, Verdict::Inconclusive);
}

#[test]
fn probe_one_sided_undefinedness_is_a_domain_refutation() {
    // √(x²) and (√x)² differ on the entire negative axis: one side is
    // defined there, the other is not. One-sided undefinedness is a
    // DOMAIN counterexample — a refutation, not a skippable point.
    // (Skipping points where BOTH sides are undefined remains correct.)
    let steps = vec![eq_step("a", "\\sqrt{x^2}"), eq_step("b", "(\\sqrt{x})^2")];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Fail);
    assert!(
        result.steps[1]
            .status
            .caveats
            .iter()
            .any(|c| c.contains("domain")),
        "expected a domain caveat, got: {:?}",
        result.steps[1].status.caveats
    );
}

// ---------------------------------------------------------------------------
// Code-review findings (PR #67 review, round 4). Findings 1-3 were
// merge-blockers: a certified false refutation from variable-scope
// erasure, and two constructible false-PASS classes inside the fragment.
// ---------------------------------------------------------------------------

#[test]
fn finding1_shadowed_binder_does_not_erase_free_variable() {
    // d/dy (y + Σ_{y=1}^{3} y) = 1: the outer y is free, the Σ index y is
    // bound. Post-hoc removal of the index from a shared accumulator
    // erased the FREE y, inference fell back to x, and a true step was
    // refuted with certificate-grade language.
    use arithma::status::free_variables;
    let node = arithma::parse_latex_raw("y + \\sum_{y=1}^{3} y").unwrap();
    assert_eq!(free_variables(&[&node]), vec!["y".to_string()]);

    let steps = vec![
        eq_step("f", "y + \\sum_{y=1}^{3} y"),
        rel_step("f'", "1", Relation::DerivativeOf, None, None),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Pass);
}

#[test]
fn finding2_equation_shaped_equals_cannot_escape_to_tolerance() {
    // x = 2 vs x = 2 + 10⁻¹⁵: equation-shaped steps must not slip out of
    // the exact fragment into f64 tolerance — that is the R1 false-PASS
    // class wrapped in one node constructor.
    let steps = vec![
        eq_step("a", "x = 2"),
        eq_step("b", "x = 2 + \\frac{1}{1000000000000000}"),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Fail);
}

#[test]
fn finding3_interpolation_through_the_sample_grid_is_caught() {
    // The 12 sample points are deterministic; a polynomial vanishing at
    // exactly those points defeats fixed-count sampling. Degree-aware
    // sampling (deg+1 points is the polynomial identity theorem) closes
    // the hole.
    let spoof = "x + (x-\\frac{1}{2})(x+\\frac{2}{5})(x-\\frac{17}{10})(x+\\frac{6}{5})(x-\\frac{7}{10})(x+\\frac{1}{5})(x-\\frac{27}{10})(x-\\frac{4}{5})(x+\\frac{3}{2})(x-\\frac{39}{10})(x-\\frac{9}{5})(x-\\frac{28}{5})";
    let steps = vec![eq_step("honest", "x"), eq_step("spoof", spoof)];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Fail);
}

#[test]
fn finding3_degree_aware_agreement_earns_exact_for_univariate_identities() {
    // With degree counting, agreement at deg+1 points IS the polynomial
    // identity theorem: in-fragment univariate identities that canonical
    // forms fail to decide are now decided, not sampled.
    let steps = vec![
        eq_step("a", "(x + 1)^3 - x^3 - 3x^2 - 3x"),
        eq_step("b", "1"),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Pass);
    assert_eq!(result.status.status, ResultStatus::Exact);
}

#[test]
fn finding5_equation_equals_means_same_solution_set() {
    // Dividing both sides by 2 is valid algebra: 2x = 4 and x = 2 have
    // the same solution set. Residual (pointwise) comparison refuted it.
    let steps = vec![
        eq_step("scaled", "2 \\cdot x = 4"),
        eq_step("solved", "x = 2"),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Pass);

    // Additive rearrangement passes likewise.
    let steps = vec![eq_step("a", "x + 3 = 7"), eq_step("b", "x = 4")];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Pass);

    // And a genuine solution-set difference is refuted: x² = 4 has the
    // extra solution x = −2.
    let steps = vec![eq_step("a", "x^2 = 4"), eq_step("b", "x = 2")];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Fail);
}

#[test]
fn finding6_large_degree_comparison_returns_within_budget() {
    // Degree 120 is a Tuesday, not an adversary: the exact path must
    // answer (or honestly downgrade), never hang.
    let big = "\\frac{x^{120} - 1}{x - 1}";
    let steps = vec![
        eq_step("a", big),
        eq_step(
            "b",
            "\\frac{x^{120} - 1}{x - 1} + \\frac{1}{1000000000000000}",
        ),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Fail);
}

#[test]
fn finding7_float_valued_solution_of_does_not_claim_membership() {
    // 1.4142135623 is provably not a root of x² = 2. Numeric agreement
    // within tolerance may pass as evidence, but the membership sentence
    // is reserved for exact verification.
    let steps = vec![
        eq_step("eq", "x^2 = 2"),
        rel_step(
            "near-root",
            "x = 1.4142135623",
            Relation::SolutionOf,
            None,
            None,
        ),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    let caveats = &result.steps[1].status.caveats;
    assert!(
        !caveats.iter().any(|c| c.contains("membership verified")),
        "float near-root must not earn the membership sentence: {:?}",
        caveats
    );
    assert!(
        caveats
            .iter()
            .any(|c| c.contains("approximate") || c.contains("floating-point")),
        "expected an approximate-membership caveat, got: {:?}",
        caveats
    );
}

// ---------------------------------------------------------------------------
// Round-5 finding: a REFUSED degree bound must never become a zero bound.
// (The refusal-becomes-default family: the mathematics is sound, the hole
// is where the mechanism declines and a default swallows the refusal.)
// ---------------------------------------------------------------------------

#[test]
fn refused_degree_bound_is_not_a_zero_bound() {
    // Exponents above the bound-computation cap make the degree bounds
    // unavailable. Unavailable must route to bounded exact sampling —
    // reading it as "degree zero, one point suffices" certifies a false
    // identity whose payload vanishes at the single sample point.
    let steps = vec![
        eq_step("a", "x^{10001}"),
        eq_step("b", "x^{10001} + 2 \\cdot x - 1"),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Fail);

    // Control: a payload that does not vanish at the first sample point.
    let steps = vec![eq_step("a", "x^{10001}"), eq_step("b", "x^{10001} + x")];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Fail);

    // The zero-bound collapse also re-admitted the simplifier for these
    // inputs (total degree 0 passes the size guard) — pin the bypass.
    let steps = vec![
        eq_step("a", "(x+1)^{10001}"),
        eq_step("b", "(x+1)^{10001} + 2 \\cdot x - 1"),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Fail);
}

#[test]
fn refused_degree_bound_agreement_is_verified_not_exact() {
    // When the bounds are unavailable, agreement is evidence from bounded
    // sampling — never the polynomial identity theorem.
    let steps = vec![eq_step("a", "x^{10001} + x"), eq_step("b", "x + x^{10001}")];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Pass);
    assert!(
        matches!(result.status.status, ResultStatus::Verified { .. }),
        "unavailable bounds must cap at verified, got {:?}",
        result.status.status
    );
}

// --- Special functions (erf, Ei, li) in chain steps ---
//
// A recognized non-elementary antiderivative (e.g. (√π/2)·erf(x) for
// e^{−x²}) must be checkable as an integral_of step: differentiation
// eliminates the special function, so the comparison is between elementary
// expressions. The raw derivative, however, carries exact-zero terms that
// still mention erf — which (deliberately) refuses numeric evaluation — so
// the checker must fold those away before sampling rather than reporting
// a starved, inconclusive step.

fn step(label: &str, expr: &str, relation: Relation) -> ChainStepInput {
    ChainStepInput {
        label: Some(label.to_string()),
        expr: expr.to_string(),
        relation,
        variable: None,
        value: None,
    }
}

#[test]
fn integral_of_step_with_recognized_erf_form_passes() {
    let steps = vec![
        step("integrand", "\\exp(-x^2)", Relation::Equals),
        step(
            "antiderivative",
            "\\frac{\\sqrt{\\pi}}{2} \\cdot \\erf(x)",
            Relation::IntegralOf,
        ),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(
        result.verdict,
        Verdict::Pass,
        "erf integral_of step should pass, got step reports: {:?}",
        result
            .steps
            .iter()
            .map(|s| (&s.mechanism, s.verdict))
            .collect::<Vec<_>>()
    );
    // √π is outside the ℚ fragment: agreement is sampling evidence.
    // (Originally this also asserted the simplify+ mechanism — with
    // d(c·f) = c·f' built correctly, the raw path handles this family and
    // the assist is reserved for the residue; see
    // true_scaled_erf_claim_passes_through_the_raw_path.)
    assert!(
        matches!(result.status.status, ResultStatus::Verified { .. }),
        "transcendental agreement caps at verified, got {:?}",
        result.status.status
    );
}

#[test]
fn derivative_of_step_producing_erf_free_form_passes() {
    let steps = vec![
        step(
            "antiderivative",
            "\\frac{\\sqrt{\\pi}}{2} \\cdot \\erf(x)",
            Relation::Equals,
        ),
        step("derivative", "\\exp(-x^2)", Relation::DerivativeOf),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(
        result.verdict,
        Verdict::Pass,
        "d/dx[(√π/2)·erf(x)] = e^{{-x²}} should pass, got: {:?}",
        result
            .steps
            .iter()
            .map(|s| (&s.mechanism, s.verdict))
            .collect::<Vec<_>>()
    );
}

#[test]
fn integral_of_step_with_wrong_erf_constant_is_not_certified() {
    // erf(x) alone (missing the √π/2 factor) is NOT an antiderivative of
    // e^{−x²}. The simplify-assisted retry must not certify a refutation
    // through an unverified transform: the honest report is a non-pass.
    let steps = vec![
        step("integrand", "\\exp(-x^2)", Relation::Equals),
        step("wrong", "\\erf(x)", Relation::IntegralOf),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_ne!(
        result.verdict,
        Verdict::Pass,
        "a wrong constant must never pass"
    );
}

// Refutability must not depend on a constant
// factor. Bare wrong claims (erf(x) for ∫e^{−x²}) were refutable because
// their raw derivative mentions no special function; c·erf(x) claims were
// unrefutable because the product rule emitted erf(x)·d(c) with d(c) an
// exact-zero TREE — erf refuses evaluation and every sample point starved.
// With d(c·f) = c·f' built correctly (constant factors never generate a
// sibling term), wrong c·SF claims are refuted through the RAW path with
// counterexamples, and true ones pass raw without the simplify assist.

#[test]
fn wrong_sign_erf_constant_is_refuted() {
    let steps = vec![
        step("integrand", "\\exp(-x^2)", Relation::Equals),
        step(
            "wrong-sign",
            "-\\frac{\\sqrt{\\pi}}{2} \\cdot \\erf(x)",
            Relation::IntegralOf,
        ),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(
        result.verdict,
        Verdict::Fail,
        "a wrong sign must be refuted, not starved into inconclusive; got: {:?}",
        result
            .steps
            .iter()
            .map(|s| (&s.mechanism, s.verdict))
            .collect::<Vec<_>>()
    );
}

#[test]
fn wrong_multiple_ei_is_refuted() {
    let steps = vec![
        step("integrand", "\\frac{\\exp(2x)}{x}", Relation::Equals),
        step("wrong-multiple", "5 \\cdot \\Ei(2x)", Relation::IntegralOf),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Fail);
}

#[test]
fn wrong_argument_scaled_erf_is_refuted() {
    let steps = vec![
        step("integrand", "\\exp(-x^2)", Relation::Equals),
        step(
            "wrong-arg",
            "\\frac{\\sqrt{\\pi}}{2} \\cdot \\erf(2x)",
            Relation::IntegralOf,
        ),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Fail);
}

#[test]
fn true_scaled_erf_claim_passes_through_the_raw_path() {
    // With constant factors handled by d(c·f) = c·f', the raw derivative of
    // (√π/2)·erf(x) mentions no special function — the true claim passes
    // WITHOUT the simplify assist, which is the stronger audit trail.
    let steps = vec![
        step("integrand", "\\exp(-x^2)", Relation::Equals),
        step(
            "antiderivative",
            "\\frac{\\sqrt{\\pi}}{2} \\cdot \\erf(x)",
            Relation::IntegralOf,
        ),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Pass);
    assert!(
        !result.steps[1].mechanism.contains("simplify"),
        "true c·erf claims should pass raw now; the simplify assist should be \
         reserved for the residue that genuinely needs it. Got mechanism: {}",
        result.steps[1].mechanism
    );
}

#[test]
fn retry_that_also_starves_is_audited_as_simplify_assisted() {
    // When the raw comparison is inconclusive AND
    // the simplify retry also comes back inconclusive, the mechanism must
    // say the retry ran (simplify+ prefix) — an auditor must be able to
    // distinguish "no retry possible" from "retry ran, also inconclusive".
    // erf(x)² keeps erf in its derivative even after simplification, so
    // both passes starve.
    let steps = vec![
        step(
            "integrand",
            "2 \\cdot \\erf(x) \\cdot \\frac{2}{\\sqrt{\\pi}} \\cdot \\exp(-x^2)",
            Relation::Equals,
        ),
        step("claim", "\\erf(x)^2", Relation::IntegralOf),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.steps[1].verdict, Verdict::Inconclusive);
    assert!(
        result.steps[1].mechanism.contains("simplify"),
        "the attempted retry must be auditable, got mechanism: {}",
        result.steps[1].mechanism
    );
}

#[test]
fn summation_closure_earns_verified_never_exact() {
    // A chain whose spine crosses a symbolic summation is a value check:
    // no summation relation exists, and canonical_form_Q has no Σ object.
    // With integer-sampled bounds the closure PASSES on numeric evidence —
    // and it must never reach exact, because sampling is not a proof.
    let steps = vec![
        eq_step(
            "telescoped",
            "\\sum_{k=1}^{n} {\\frac{1}{k} - \\frac{1}{k+1}}",
        ),
        eq_step("closed form", "1 - \\frac{1}{n+1}"),
    ];
    let result = verify_chain(&steps, &Environment::new()).unwrap();
    assert_eq!(result.verdict, Verdict::Pass);
    assert!(
        matches!(result.status.status, ResultStatus::Verified { .. }),
        "summation closure must land verified — never exact, never spurious FAIL; got {:?}",
        result.status.status
    );
}
