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
