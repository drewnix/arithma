//! Reasoning-chain verification: the `verify_chain` engine.
//!
//! A chain is an ordered list of expressions; each step after the first
//! declares a relation to its predecessor (`equals`, `derivative_of`, …).
//! Each relation is checked by the mechanism appropriate to it, and each
//! step reports a machine-readable verdict plus the evidence class that
//! backs it (see `docs/result-status.md`).
//!
//! Design rules inherited from the result-status work:
//! - The chain status is the *minimum* evidence across steps: one numeric
//!   step makes the whole chain numeric. Evidence never upgrades.
//! - The counterexample is the diagnosis: a failing step carries the point
//!   and both values, nothing generative.
//! - A status is earned by the mechanism that ran, never asserted by
//!   optimism. `equals` steps earn `exact` only inside the poly/rational
//!   fragment where canonicalization is a decision procedure; structural
//!   agreement of transcendental forms is corroborated numerically and
//!   reported as `verified`.

use crate::environment::Environment;
use crate::node::Node;
use crate::simplify::Simplifiable;
pub use crate::status::Verdict;
use crate::status::{free_variables, is_algebraic_exact, ResultStatus, StatusReport};
use crate::verify::verify_identity;

/// The relation a step declares to its predecessor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    /// This step is equivalent to the previous one.
    Equals,
    /// This step is d/d(variable) of the previous one.
    DerivativeOf,
    /// This step is an antiderivative of the previous one (checked by the
    /// differentiation round-trip, which is algebraic — this relation can
    /// earn `exact`).
    IntegralOf,
    /// This step is the previous one with `variable := value` substituted.
    Substitution,
    /// The previous step (an equation) implies this one. Capped at
    /// `verified`: implication is checked at the antecedent's solutions,
    /// which is evidence, not proof.
    Implies,
    /// This step (`var = value`) is a solution of the previous step (an
    /// equation). Checking membership is exact arithmetic; completeness of
    /// the solution set is not claimed.
    SolutionOf,
    /// This step is a factored form of the previous one.
    FactoredFormOf,
}

impl Relation {
    pub fn parse(s: &str) -> Result<Relation, String> {
        match s {
            "equals" => Ok(Relation::Equals),
            "derivative_of" => Ok(Relation::DerivativeOf),
            "integral_of" => Ok(Relation::IntegralOf),
            "substitution" => Ok(Relation::Substitution),
            "implies" => Ok(Relation::Implies),
            "solution_of" => Ok(Relation::SolutionOf),
            "factored_form_of" => Ok(Relation::FactoredFormOf),
            other => Err(format!(
                "Unknown relation '{}'. Use: equals, derivative_of, integral_of, substitution, implies, solution_of, factored_form_of",
                other
            )),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Relation::Equals => "equals",
            Relation::DerivativeOf => "derivative_of",
            Relation::IntegralOf => "integral_of",
            Relation::Substitution => "substitution",
            Relation::Implies => "implies",
            Relation::SolutionOf => "solution_of",
            Relation::FactoredFormOf => "factored_form_of",
        }
    }
}

/// One step of a chain, as supplied by the caller. `relation` is ignored
/// for the first (anchor) step. `variable` names the variable for
/// derivative/integral/substitution/solution relations; `value` is the
/// LaTeX expression substituted by a `substitution` step.
#[derive(Debug, Clone)]
pub struct ChainStepInput {
    pub label: Option<String>,
    pub expr: String,
    pub relation: Relation,
    pub variable: Option<String>,
    pub value: Option<String>,
}

/// The outcome of checking one step against its predecessor.
#[derive(Debug, Clone)]
pub struct StepResult {
    pub label: String,
    /// `None` for the anchor step, which makes no claim.
    pub relation: Option<Relation>,
    pub verdict: Verdict,
    pub status: StatusReport,
    /// The mechanism that actually ran, named so over-claims are auditable,
    /// e.g. `canonical_form_Q`, `numeric_sample`.
    pub mechanism: String,
}

/// The outcome of verifying a whole chain.
#[derive(Debug, Clone)]
pub struct ChainResult {
    pub steps: Vec<StepResult>,
    pub verdict: Verdict,
    /// Minimum evidence across steps; carries the weakest step's caveats
    /// and counterexample.
    pub status: StatusReport,
    /// Index of the relation step with the weakest evidence — the chain's
    /// evidence floor. On PASS/INCONCLUSIVE chains this is also the step
    /// whose report becomes the chain status; on FAIL chains the status
    /// routes from `first_failure` instead (the diagnosis is never
    /// outranked), so the two indices can differ. `None` for a chain with
    /// no relation steps.
    pub weakest_step: Option<usize>,
    /// Index of the first failing step, if any.
    pub first_failure: Option<usize>,
}

/// Evidence rank for the min-across-steps rule. Higher is stronger.
fn rank(status: &ResultStatus) -> u8 {
    match status {
        ResultStatus::Exact => 4,
        ResultStatus::Verified { .. } => 3,
        ResultStatus::Heuristic => 2,
        ResultStatus::UnableToCompute { .. } => 1,
        ResultStatus::ProvablyImpossible { .. } => 0,
    }
}

/// Verify a chain of reasoning steps. Each step after the first is checked
/// against its predecessor by the mechanism its declared relation calls
/// for. Errors are reserved for malformed input (empty chain, unparseable
/// expressions, missing parameters); a step that *checks* and fails is a
/// `Fail` verdict, not an error.
pub fn verify_chain(steps: &[ChainStepInput], env: &Environment) -> Result<ChainResult, String> {
    if steps.is_empty() {
        return Err("chain must contain at least one step".to_string());
    }

    // Parse every expression up front so a malformed step is reported by
    // index before any checking runs.
    let mut nodes: Vec<Node> = Vec::with_capacity(steps.len());
    for (i, step) in steps.iter().enumerate() {
        let node = crate::parser::parse_latex_raw(&step.expr)
            .map_err(|e| format!("step {} ({}): parse error: {}", i, step_label(step, i), e))?;
        nodes.push(node);
    }

    let mut results: Vec<StepResult> = Vec::with_capacity(steps.len());
    results.push(StepResult {
        label: step_label(&steps[0], 0),
        relation: None,
        verdict: Verdict::Pass,
        status: StatusReport::exact(),
        mechanism: "anchor".to_string(),
    });

    for i in 1..steps.len() {
        let checked = check_step(&nodes[i - 1], &nodes[i], &steps[i], env)
            .map_err(|e| format!("step {} ({}): {}", i, step_label(&steps[i], i), e))?;
        results.push(StepResult {
            label: step_label(&steps[i], i),
            relation: Some(steps[i].relation),
            verdict: checked.verdict,
            status: checked.status,
            mechanism: checked.mechanism,
        });
    }

    // Chain verdict: fail at the first failing step; otherwise inconclusive
    // if any step could not be checked; otherwise pass.
    let first_failure = results.iter().position(|r| r.verdict == Verdict::Fail);
    let verdict = if first_failure.is_some() {
        Verdict::Fail
    } else if results.iter().any(|r| r.verdict == Verdict::Inconclusive) {
        Verdict::Inconclusive
    } else {
        Verdict::Pass
    };

    // Chain status: on FAIL, the chain-level report is the first failing
    // step's report — the diagnosis must never be outranked by a passing
    // step. Otherwise it is the weakest evidence among relation
    // steps (the anchor makes no claim); a one-step chain verifies nothing
    // and says so.
    let weakest_step = results
        .iter()
        .enumerate()
        .skip(1)
        .min_by_key(|(_, r)| rank(&r.status.status))
        .map(|(i, _)| i);
    let status = match (first_failure, weakest_step) {
        (Some(i), _) => results[i].status.clone(),
        (None, Some(i)) => results[i].status.clone(),
        (None, None) => StatusReport::exact()
            .with_caveat("anchor only; a one-step chain contains no relations to verify"),
    };

    Ok(ChainResult {
        steps: results,
        verdict,
        status,
        weakest_step,
        first_failure,
    })
}

fn step_label(step: &ChainStepInput, index: usize) -> String {
    step.label
        .clone()
        .unwrap_or_else(|| format!("step {}", index))
}

struct CheckedStep {
    verdict: Verdict,
    status: StatusReport,
    mechanism: String,
}

fn check_step(
    prev: &Node,
    current: &Node,
    step: &ChainStepInput,
    env: &Environment,
) -> Result<CheckedStep, String> {
    match step.relation {
        Relation::Equals | Relation::FactoredFormOf => {
            // Equation-shaped steps get equation semantics: comparing
            // residuals pointwise refutes valid algebra (dividing both
            // sides by 2 changes the residual but not the solutions), and
            // letting equations fall through to the expression ladder
            // would bypass the exact fragment entirely. Mixing shapes is
            // ambiguous and refused.
            match (as_equation(prev).is_some(), as_equation(current).is_some()) {
                (true, true) => check_equation_equals(prev, current, step.variable.as_deref(), env),
                (false, false) => Ok(check_equals(prev, current, env)),
                _ => Err(
                    "cannot compare an equation with an expression via 'equals'; \
                     state both steps as equations or both as expressions"
                        .to_string(),
                ),
            }
        }
        Relation::DerivativeOf => {
            let var = infer_variable(step.variable.as_deref(), prev, "derivative_of")?;
            check_derivative_of(prev, current, &var, env)
        }
        Relation::IntegralOf => {
            let var = infer_variable(step.variable.as_deref(), current, "integral_of")?;
            check_integral_of(prev, current, &var, env)
        }
        Relation::Substitution => {
            let value = step
                .value
                .as_deref()
                .ok_or("substitution requires a 'value' parameter")?;
            let var = infer_variable(step.variable.as_deref(), prev, "substitution")?;
            check_substitution(prev, current, &var, value, env)
        }
        Relation::SolutionOf => check_solution_of(prev, current, env),
        Relation::Implies => check_implies(prev, current, step.variable.as_deref(), env),
    }
}

/// One rule for every relation: use the declared variable, or
/// infer it when the relevant expression has exactly one free variable.
/// Defaulting silently to `x` refutes true steps in other variables
/// (d/dx t² = 0). A constant expression accepts any variable.
fn infer_variable(declared: Option<&str>, node: &Node, relation: &str) -> Result<String, String> {
    if let Some(v) = declared {
        return Ok(crate::tokenizer::normalize_var(v));
    }
    let vars = free_variables(&[node]);
    match vars.as_slice() {
        [] => Ok("x".to_string()),
        [v] => Ok(v.clone()),
        _ => Err(format!(
            "{} needs a 'variable' parameter: the expression has {} free variables ({})",
            relation,
            vars.len(),
            vars.join(", ")
        )),
    }
}

/// `current = d/d(var) prev`. The derivative rules are complete and sound,
/// so the computed derivative is exact; what remains is comparing it with
/// the claimed form, which reuses the `equals` evidence ladder.
fn check_derivative_of(
    prev: &Node,
    current: &Node,
    var: &str,
    env: &Environment,
) -> Result<CheckedStep, String> {
    let derivative = crate::derivative::differentiate(prev, var)
        .map_err(|e| format!("could not differentiate the previous step: {}", e))?;
    let mut checked = compare_constructed_derivative(&derivative, current, env);
    checked.mechanism = format!("derivative_rules+{}", checked.mechanism);
    Ok(checked)
}

/// Compare a derivative the checker itself constructed against a claimed
/// expression. The raw derivative is tried first — the exact tiers of the
/// equals ladder must not depend on the simplifier. If (and only if) the
/// raw comparison is inconclusive, the constructed side is simplified and
/// retried: the product rule leaves exact-zero terms that can still mention
/// a special function (erf, Ei, li) which refuses numeric evaluation, and
/// folding them away is what makes the derivative sampleable at all.
///
/// The retry can *pass* (sampling evidence, with `simplify+` named in the
/// mechanism so the assist is auditable) but never *refute*: a disagreement
/// reached only through a believed-sound-but-unverified transform might
/// refute the transform rather than the step, and a false refutation is the
/// worst report a verifier can make. Such a disagreement stays inconclusive,
/// with the witness preserved as a caveat.
fn compare_constructed_derivative(
    derivative: &Node,
    claimed: &Node,
    env: &Environment,
) -> CheckedStep {
    let raw = check_equals(derivative, claimed, env);
    if raw.verdict != Verdict::Inconclusive {
        return raw;
    }
    let simplified = match derivative.simplify(env) {
        Ok(s) if &s != derivative => s,
        _ => return raw,
    };
    let retried = check_equals(&simplified, claimed, env);
    match retried.verdict {
        Verdict::Pass => CheckedStep {
            mechanism: format!("simplify+{}", retried.mechanism),
            ..retried
        },
        Verdict::Fail => {
            let witness = retried
                .status
                .counterexample_json()
                .map(|cx| cx.to_string())
                .unwrap_or_else(|| "no witness recorded".to_string());
            CheckedStep {
                verdict: Verdict::Inconclusive,
                status: raw.status.with_caveat(&format!(
                    "the simplified derivative disagreed with the claimed step at {}; \
                     a refutation through an unverified simplification is not certified",
                    witness
                )),
                mechanism: format!("simplify+{}", retried.mechanism),
            }
        }
        // The retry ran and was also inconclusive: report the retried
        // result with the assist named, so an auditor can distinguish
        // "no retry possible" from "retry ran, also inconclusive".
        Verdict::Inconclusive => CheckedStep {
            mechanism: format!("simplify+{}", retried.mechanism),
            ..retried
        },
    }
}

/// `current` is an antiderivative of `prev`: differentiate the current step
/// and compare with the previous one. The round-trip is algebraic, which is
/// why this relation can earn `exact` where `implies` never can. Constants
/// of integration vanish under d/d(var) and so cannot cause a false fail.
fn check_integral_of(
    prev: &Node,
    current: &Node,
    var: &str,
    env: &Environment,
) -> Result<CheckedStep, String> {
    let derivative = crate::derivative::differentiate(current, var).map_err(|e| {
        format!(
            "could not differentiate this step for the round-trip: {}",
            e
        )
    })?;
    let mut checked = compare_constructed_derivative(&derivative, prev, env);
    checked.mechanism = format!("differentiation_roundtrip+{}", checked.mechanism);
    Ok(checked)
}

/// `current = prev[var := value]`. Capture-avoiding substitution is
/// algebraic; the comparison of the substituted form with the claimed form
/// reuses the `equals` ladder (which follows the changed variable set).
fn check_substitution(
    prev: &Node,
    current: &Node,
    var: &str,
    value: &str,
    env: &Environment,
) -> Result<CheckedStep, String> {
    let value_node = crate::parser::parse_latex_raw(value)
        .map_err(|e| format!("could not parse substitution value '{}': {}", value, e))?;
    let substituted = match capture_aware_substitute(prev, var, &value_node, "substitute")? {
        SubstOutcome::Node(s) => s,
        SubstOutcome::Refused(step) => return Ok(step),
    };
    let mut checked = check_equals(&substituted, current, env);
    checked.mechanism = format!("substitute+{}", checked.mechanism);
    Ok(checked)
}

enum SubstOutcome {
    Node(Node),
    Refused(CheckedStep),
}

/// Substitution with the capture-refusal-as-step-outcome policy applied
/// uniformly: a refusal is a step-level `inconclusive` naming the capture —
/// a ten-step chain with a capture at step 7 must still report steps 1–6
/// (audit trail over abort). It
/// covers every relation that substitutes: `substitution`, `solution_of`,
/// and `implies`. Genuine failures remain chain-aborting protocol errors.
fn capture_aware_substitute(
    expr: &Node,
    var: &str,
    value: &Node,
    mechanism: &str,
) -> Result<SubstOutcome, String> {
    match crate::substitute::substitute_variable(expr, var, value) {
        Ok(n) => Ok(SubstOutcome::Node(n)),
        Err(e) if e.contains("capture") => Ok(SubstOutcome::Refused(CheckedStep {
            verdict: Verdict::Inconclusive,
            status: StatusReport::unable_to_compute(&e),
            mechanism: mechanism.to_string(),
        })),
        Err(e) => Err(format!("substitution failed: {}", e)),
    }
}

/// Split an equation node into (lhs, rhs).
fn as_equation(node: &Node) -> Option<(&Node, &Node)> {
    match node {
        Node::Equation(l, r) | Node::Equal(l, r) => Some((l, r)),
        _ => None,
    }
}

/// `equals` between two equations means SAME SOLUTION SET — the semantics
/// under which dividing both sides by 2 is an identity step. Both
/// equations are solved and the solution sets compared exactly. Capped at
/// `verified`: the comparison inherits the solver's completeness, which is
/// not proven. A solution of one equation missing from the other refutes
/// the step and is the witness.
fn check_equation_equals(
    prev: &Node,
    current: &Node,
    variable: Option<&str>,
    env: &Environment,
) -> Result<CheckedStep, String> {
    use std::collections::BTreeMap;

    let mechanism = "solution_set_comparison".to_string();
    let var = match variable {
        Some(v) => crate::tokenizer::normalize_var(v),
        None => {
            let vars = free_variables(&[prev, current]);
            match vars.as_slice() {
                [v] => v.clone(),
                [] => {
                    return Err(
                        "equation comparison requires at least one free variable".to_string()
                    )
                }
                _ => {
                    return Err(format!(
                        "equation comparison needs a 'variable' parameter: the equations have {} free variables ({})",
                        vars.len(),
                        vars.join(", ")
                    ))
                }
            }
        }
    };

    let (sa, sb) = match (
        crate::expression::solve_full(prev, &var),
        crate::expression::solve_full(current, &var),
    ) {
        (Ok(a), Ok(b)) => (a, b),
        _ => {
            return Ok(CheckedStep {
                verdict: Verdict::Inconclusive,
                status: StatusReport::unable_to_compute(&format!(
                    "could not solve both equations for {} to compare solution sets; use 'implies' for one-directional checks or 'solution_of' for a specific root",
                    var
                )),
                mechanism,
            })
        }
    };

    let canon = |n: &Node| -> String {
        let s = n.simplify(env).unwrap_or_else(|_| n.clone());
        format!("{}", s)
    };
    let set_a: BTreeMap<String, Node> =
        sa.solutions.iter().map(|n| (canon(n), n.clone())).collect();
    let set_b: BTreeMap<String, Node> =
        sb.solutions.iter().map(|n| (canon(n), n.clone())).collect();

    if set_a.keys().eq(set_b.keys()) {
        let mut status = StatusReport::verified(set_a.len().max(1)).with_caveat(
            "equations compared by solution set as computed by the solver — capped at verified (solver completeness is not proven)",
        );
        if set_a.is_empty() {
            status = status.with_caveat(
                "both equations have empty solution sets as far as the solver can find",
            );
        }
        if sa.complex_omitted != sb.complex_omitted {
            status = status.with_caveat(&format!(
                "complex solutions omitted from the comparison: {} vs {}",
                sa.complex_omitted, sb.complex_omitted
            ));
        }
        return Ok(CheckedStep {
            verdict: Verdict::Pass,
            status,
            mechanism,
        });
    }

    // A witness from the symmetric difference: a solution of one equation
    // that the other lacks, checked against the equation lacking it.
    let (witness_str, witness, lacking) = set_a
        .iter()
        .find(|(k, _)| !set_b.contains_key(*k))
        .map(|(k, n)| (k.clone(), n.clone(), current))
        .or_else(|| {
            set_b
                .iter()
                .find(|(k, _)| !set_a.contains_key(*k))
                .map(|(k, n)| (k.clone(), n.clone(), prev))
        })
        .expect("sets differ, symmetric difference is nonempty");

    let mut status =
        StatusReport::verified(set_a.len().max(set_b.len()).max(1)).with_caveat(&format!(
            "the equations have different solution sets: {} = {} satisfies one but not the other",
            var, witness_str
        ));
    if let Some((l, r)) = as_equation(lacking) {
        let lhs_val = crate::substitute::substitute_variable(l, &var, &witness)
            .map(|n| node_to_f64(&n, env))
            .unwrap_or(f64::NAN);
        let rhs_val = crate::substitute::substitute_variable(r, &var, &witness)
            .map(|n| node_to_f64(&n, env))
            .unwrap_or(f64::NAN);
        let point_val = node_to_f64(&witness, env);
        if point_val.is_finite() && lhs_val.is_finite() && rhs_val.is_finite() {
            status = status.with_counterexample(&crate::verify::Counterexample {
                point: vec![(var.clone(), point_val)],
                lhs_value: lhs_val,
                rhs_value: rhs_val,
            });
        }
    }
    Ok(CheckedStep {
        verdict: Verdict::Fail,
        status,
        mechanism,
    })
}

/// `current` (var = value) is a solution of `prev` (an equation).
/// Substitute the value and compare the two sides — a checker, not a
/// finder, so exact arithmetic decides it. Membership only: the solution
/// set's completeness is explicitly not claimed.
fn check_solution_of(
    prev: &Node,
    current: &Node,
    env: &Environment,
) -> Result<CheckedStep, String> {
    let (eq_lhs, eq_rhs) =
        as_equation(prev).ok_or("solution_of requires the previous step to be an equation")?;
    let (sol_var, sol_value) = match as_equation(current) {
        Some((Node::Variable(v), value)) => (v.clone(), value.clone()),
        _ => {
            return Err(
                "solution_of requires this step to have the form 'variable = value'".to_string(),
            )
        }
    };

    let lhs_sub =
        match capture_aware_substitute(eq_lhs, &sol_var, &sol_value, "solution_substitution")? {
            SubstOutcome::Node(n) => n,
            SubstOutcome::Refused(step) => return Ok(step),
        };
    let rhs_sub =
        match capture_aware_substitute(eq_rhs, &sol_var, &sol_value, "solution_substitution")? {
            SubstOutcome::Node(n) => n,
            SubstOutcome::Refused(step) => return Ok(step),
        };

    let mut checked = check_equals(&lhs_sub, &rhs_sub, env);
    checked.mechanism = format!("solution_substitution+{}", checked.mechanism);
    if checked.verdict == Verdict::Pass {
        // The membership sentence is earned only by exact arithmetic. A
        // float value agreeing within f64 tolerance is a near-root, and a
        // near-root is a provable NON-root — the caveat must say
        // approximate, never "verified".
        checked.status = if matches!(checked.status.status, ResultStatus::Exact) {
            checked.status.with_caveat(
                "solution membership verified; completeness of the solution set is not claimed",
            )
        } else {
            checked.status.with_caveat(
                "agreement within floating-point tolerance only — approximate membership, not verified as an exact root; supply an exact value (fraction or radical) for exact membership verification",
            )
        };
    }
    Ok(checked)
}

/// `prev ⇒ current`, both equations. Solve the antecedent and check every
/// solution against the consequent. A solution that violates the consequent
/// refutes the implication (and is the counterexample). All solutions
/// passing is evidence capped at `verified`: the solver's solution set is
/// what was checked, and finitely many checked points do not prove an
/// implication.
fn check_implies(
    prev: &Node,
    current: &Node,
    variable: Option<&str>,
    env: &Environment,
) -> Result<CheckedStep, String> {
    if as_equation(prev).is_none() || as_equation(current).is_none() {
        return Err("implies requires both steps to be equations".to_string());
    }

    // The variable to solve for: declared, or the antecedent's single free
    // variable.
    let var = match variable {
        Some(v) => crate::tokenizer::normalize_var(v),
        None => {
            let vars = free_variables(&[prev]);
            match vars.as_slice() {
                [v] => v.clone(),
                _ => {
                    return Err(format!(
                        "implies needs a 'variable' parameter when the antecedent has {} free variables",
                        vars.len()
                    ))
                }
            }
        }
    };

    let mechanism = "antecedent_solutions".to_string();
    let solved = match crate::expression::solve_full(prev, &var) {
        Ok(s) => s,
        Err(e) => {
            return Ok(CheckedStep {
                verdict: Verdict::Inconclusive,
                status: StatusReport::unable_to_compute(&format!(
                    "could not solve the antecedent for {}: {}",
                    var, e
                )),
                mechanism,
            })
        }
    };
    if solved.solutions.is_empty() {
        // A vacuously true implication is still only evidence: the solver
        // found no solutions, which is not a proof that none exist.
        return Ok(CheckedStep {
            verdict: Verdict::Inconclusive,
            status: StatusReport::unable_to_compute(
                "the antecedent has no solutions the solver can find; implication is vacuous as far as checked",
            ),
            mechanism,
        });
    }

    let (con_lhs, con_rhs) = as_equation(current).expect("checked above");
    let mut checked_count = 0usize;
    for sol in &solved.solutions {
        let lhs_sub = match capture_aware_substitute(con_lhs, &var, sol, &mechanism)? {
            SubstOutcome::Node(n) => n,
            SubstOutcome::Refused(step) => return Ok(step),
        };
        let rhs_sub = match capture_aware_substitute(con_rhs, &var, sol, &mechanism)? {
            SubstOutcome::Node(n) => n,
            SubstOutcome::Refused(step) => return Ok(step),
        };
        let step = check_equals(&lhs_sub, &rhs_sub, env);
        match step.verdict {
            Verdict::Pass => checked_count += 1,
            Verdict::Fail => {
                // This solution of the antecedent violates the consequent:
                // the implication is refuted, and the solution is the
                // diagnosis.
                let mut status = StatusReport::verified(checked_count + 1).with_caveat(&format!(
                    "the antecedent solution {} = {} does not satisfy the consequent",
                    var, sol
                ));
                // Prefer the inner check's counterexample — for parametric
                // solutions it carries actual numbers where a direct f64
                // rendering of the symbolic solution would serialize as
                // null.
                if let Some(inner_cx) = step.status.counterexample_json() {
                    status = status.with_counterexample_value(inner_cx.clone());
                } else {
                    let point_val = node_to_f64(sol, env);
                    let lhs_val = node_to_f64(&lhs_sub, env);
                    let rhs_val = node_to_f64(&rhs_sub, env);
                    if point_val.is_finite() && lhs_val.is_finite() && rhs_val.is_finite() {
                        status = status.with_counterexample(&crate::verify::Counterexample {
                            point: vec![(var.clone(), point_val)],
                            lhs_value: lhs_val,
                            rhs_value: rhs_val,
                        });
                    }
                    // Otherwise the symbolic witness lives in the caveat —
                    // no null-stuffed counterexample field.
                }
                return Ok(CheckedStep {
                    verdict: Verdict::Fail,
                    status,
                    mechanism,
                });
            }
            Verdict::Inconclusive => {
                return Ok(CheckedStep {
                    verdict: Verdict::Inconclusive,
                    status: StatusReport::unable_to_compute(&format!(
                        "could not evaluate the consequent at the antecedent solution {} = {}",
                        var, sol
                    )),
                    mechanism,
                })
            }
        }
    }

    let mut status = StatusReport::verified(checked_count).with_caveat(
        "implication checked at the antecedent's solutions — evidence, not proof; capped at verified by design",
    );
    if solved.complex_omitted > 0 {
        status = status.with_caveat(&format!(
            "{} complex solution{} of the antecedent omitted from the check",
            solved.complex_omitted,
            if solved.complex_omitted == 1 { "" } else { "s" }
        ));
    }
    Ok(CheckedStep {
        verdict: Verdict::Pass,
        status,
        mechanism,
    })
}

/// Best-effort f64 rendering of a node for counterexample reporting.
fn node_to_f64(node: &Node, env: &Environment) -> f64 {
    crate::evaluator::Evaluator::evaluate(node, env).unwrap_or(f64::NAN)
}

/// The `equals` mechanism, in decreasing order of evidence:
/// 1. Both sides in the poly/rational fragment and canonical forms agree
///    (directly or as difference-simplifies-to-zero) → `exact`.
///    Canonicalization there is a decision procedure over ℚ(x₁,…,xₙ).
/// 2. Outside the fragment, structural agreement is only as trustworthy as
///    the simplifier's rewrite rules — which is precisely what a chain
///    verifier must not assume — so agreement is corroborated numerically:
///    pass → `verified`, disagreement → `Fail` with the counterexample,
///    too few valid points → `Inconclusive`.
fn check_equals(prev: &Node, current: &Node, env: &Environment) -> CheckedStep {
    // Syntactic identity needs no simplifier and no fragment restriction —
    // but it must be structural equality of the trees, not a Display-string
    // coincidence (printer normalization makes distinct trees
    // print alike).
    if prev == current {
        return CheckedStep {
            verdict: Verdict::Pass,
            status: StatusReport::exact(),
            mechanism: "syntactic_identity".to_string(),
        };
    }
    // Unit-law normalization (u·1 → u, u+0 → u, u^1 → u, −(−u) → u) is
    // sound in every interpretation — the laws hold pointwise even where u
    // is undefined — so structural equality after it still carries
    // decision-procedure weight. This is NOT the general simplifier; only
    // rewrites valid without side conditions are admitted here.
    if unit_normal_form(prev) == unit_normal_form(current) {
        return CheckedStep {
            verdict: Verdict::Pass,
            status: StatusReport::exact(),
            mechanism: "unit_normal_form".to_string(),
        };
    }
    if !(is_algebraic_exact(prev) && is_algebraic_exact(current)) {
        return numeric_check(prev, current, env);
    }

    // In the fragment, equality is decidable and f64 tolerance never gets
    // a vote. Degree bounds of the difference (as a rational function)
    // drive everything: they tell us how many sample points constitute a
    // PROOF (polynomial identity theorem) and whether the simplifier can
    // be afforded at all — large-degree inputs go straight to bounded
    // exact evaluation instead of hanging inside polynomial expansion.
    //
    // A REFUSED bound (None: exponent beyond the cap) must propagate as
    // "bounds unavailable → bounded sampling, verified". A default here
    // would read the refusal as "degree zero, one point suffices" and
    // certify a false identity whose payload vanishes at that point. The
    // refusal also skips the simplifier: the size guard cannot be trusted
    // when the size computation itself declined.
    let diff_raw = Node::Subtract(Box::new(prev.clone()), Box::new(current.clone()));
    let bounds = match numerator_degree_bounds(&diff_raw) {
        Some(b) => b,
        None => return bounded_exact_sample(prev, current, env, &free_variables(&[prev, current])),
    };
    let total_degree: u64 = bounds.values().fold(0u64, |acc, d| acc.saturating_add(*d));

    const MAX_SIMPLIFY_DEGREE: u64 = 32;
    if total_degree <= MAX_SIMPLIFY_DEGREE {
        let prev_s = prev.simplify(env).unwrap_or_else(|_| prev.clone());
        let current_s = current.simplify(env).unwrap_or_else(|_| current.clone());
        if prev_s == current_s {
            return CheckedStep {
                verdict: Verdict::Pass,
                status: StatusReport::exact(),
                mechanism: "canonical_form_Q".to_string(),
            };
        }
        let diff = Node::Subtract(Box::new(prev_s.clone()), Box::new(current_s.clone()));
        if let Ok(d) = diff.simplify(env) {
            if matches!(&d, Node::Num(n) if n.is_zero()) {
                return CheckedStep {
                    verdict: Verdict::Pass,
                    status: StatusReport::exact(),
                    mechanism: "difference_zero_Q".to_string(),
                };
            }
        }
        return exact_rational_check(&prev_s, &current_s, env, &bounds);
    }
    exact_rational_check(prev, current, env, &bounds)
}

/// Per-variable degree bounds of a fragment expression's numerator when
/// the expression is viewed as a single rational function (numerator,
/// denominator) — structural bounds, no expansion, saturating arithmetic.
/// Sound but not tight: cancellation only lowers true degrees. `None`
/// outside the fragment or when an exponent is unreasonably large.
fn numerator_degree_bounds(node: &Node) -> Option<std::collections::BTreeMap<String, u64>> {
    fn pair_bounds(node: &Node) -> Option<std::collections::BTreeMap<String, (u64, u64)>> {
        use std::collections::BTreeMap;
        let combine = |l: BTreeMap<String, (u64, u64)>,
                       r: BTreeMap<String, (u64, u64)>,
                       f: &dyn Fn((u64, u64), (u64, u64)) -> (u64, u64)|
         -> BTreeMap<String, (u64, u64)> {
            let mut out = BTreeMap::new();
            for key in l.keys().chain(r.keys()) {
                let a = l.get(key).copied().unwrap_or((0, 0));
                let b = r.get(key).copied().unwrap_or((0, 0));
                out.insert(key.clone(), f(a, b));
            }
            out
        };
        match node {
            Node::Num(crate::exact::ExactNum::Rational(_)) => Some(BTreeMap::new()),
            Node::Variable(v) if !crate::status::is_builtin_constant(v) => {
                let mut m = BTreeMap::new();
                m.insert(v.clone(), (1, 0));
                Some(m)
            }
            Node::Add(l, r) | Node::Subtract(l, r) => {
                let (l, r) = (pair_bounds(l)?, pair_bounds(r)?);
                // a/b ± c/d = (ad ± cb)/(bd)
                Some(combine(l, r, &|(an, ad), (bn, bd)| {
                    (
                        an.saturating_add(bd).max(bn.saturating_add(ad)),
                        ad.saturating_add(bd),
                    )
                }))
            }
            Node::Multiply(l, r) => {
                let (l, r) = (pair_bounds(l)?, pair_bounds(r)?);
                Some(combine(l, r, &|(an, ad), (bn, bd)| {
                    (an.saturating_add(bn), ad.saturating_add(bd))
                }))
            }
            Node::Divide(l, r) => {
                let (l, r) = (pair_bounds(l)?, pair_bounds(r)?);
                Some(combine(l, r, &|(an, ad), (bn, bd)| {
                    (an.saturating_add(bd), ad.saturating_add(bn))
                }))
            }
            Node::Negate(inner) => pair_bounds(inner),
            Node::Power(base, exp) => {
                let b = pair_bounds(base)?;
                let k = integer_exponent_value(exp)?;
                if k.unsigned_abs() > 10_000 {
                    return None;
                }
                let mag = k.unsigned_abs();
                Some(
                    b.into_iter()
                        .map(|(v, (n, d))| {
                            let scaled = if k >= 0 {
                                (n.saturating_mul(mag), d.saturating_mul(mag))
                            } else {
                                (d.saturating_mul(mag), n.saturating_mul(mag))
                            };
                            (v, scaled)
                        })
                        .collect(),
                )
            }
            _ => None,
        }
    }
    pair_bounds(node).map(|m| m.into_iter().map(|(v, (n, _))| (v, n)).collect())
}

fn integer_exponent_value(node: &Node) -> Option<i64> {
    match node {
        Node::Num(crate::exact::ExactNum::Rational(r)) => {
            use num_traits::One;
            if r.denom().is_one() {
                r.numer().try_into().ok()
            } else {
                None
            }
        }
        Node::Negate(inner) => integer_exponent_value(inner).map(|k| -k),
        _ => None,
    }
}

/// Deterministic sample-value stream: 1/2, −1/2, 1, −1, 3/2, −3/2, 2, −2, …
/// Distinct by construction; mixes halves and integers so integer-assumed
/// variables are not starved.
fn sample_value(k: usize) -> crate::exact::ExactNum {
    use crate::exact::ExactNum;
    let m = (k / 4) as i64;
    match k % 4 {
        0 => ExactNum::rational(2 * m + 1, 2),
        1 => ExactNum::rational(-(2 * m + 1), 2),
        2 => ExactNum::integer(m + 1),
        _ => ExactNum::integer(-(m + 1)),
    }
}

/// Compare two in-fragment expressions by exact rational evaluation.
/// `bounds` gives per-variable degree bounds of the difference's
/// numerator: agreement at deg+1 points per variable (a full grid in the
/// multivariate case) makes the numerator identically zero — the
/// polynomial identity theorem — so within budget this is a DECISION
/// PROCEDURE earning `exact`, not sampling. A fixed-count sample would be
/// defeatable by a polynomial interpolated through the sample points.
/// Any disagreement is a certificate: f(p) ≠ g(p) in exact arithmetic, no
/// tolerance anywhere. Over budget (or starved of valid points), agreement
/// downgrades honestly to `verified` with a caveat naming the shortfall.
fn exact_rational_check(
    lhs: &Node,
    rhs: &Node,
    env: &Environment,
    bounds: &std::collections::BTreeMap<String, u64>,
) -> CheckedStep {
    use crate::evaluator::Evaluator;
    use crate::exact::ExactNum;

    const MAX_EVALS: u64 = 2048;
    let vars = free_variables(&[lhs, rhs]);

    // Variable-free comparison: one exact evaluation DECIDES it — exact
    // rational arithmetic on constants is a decision procedure, and
    // repeating the same point would be evidence inflation.
    if vars.is_empty() {
        let env_pt = Environment::new();
        if let (Ok(ExactNum::Rational(a)), Ok(ExactNum::Rational(b))) = (
            Evaluator::evaluate_exact(lhs, &env_pt),
            Evaluator::evaluate_exact(rhs, &env_pt),
        ) {
            let mechanism = "exact_constant_eval".to_string();
            if a == b {
                return CheckedStep {
                    verdict: Verdict::Pass,
                    status: StatusReport::exact(),
                    mechanism,
                };
            }
            return CheckedStep {
                verdict: Verdict::Fail,
                status: StatusReport::exact()
                    .with_caveat("constants differ in exact rational arithmetic")
                    .with_counterexample_value(exact_counterexample_json(&[], &a, &b)),
                mechanism,
            };
        }
        return numeric_check(lhs, rhs, env);
    }

    // Points needed per variable for the identity theorem: deg + 1.
    // Every free variable must have a bound — a missing entry would be
    // another refusal-becomes-default hole ("no bound" read as "degree
    // zero"). It should be structurally impossible (the bounds are
    // computed on the difference of these very expressions), but if it
    // happens, downgrade to bounded sampling rather than mint a proof.
    let needed: Vec<(String, u64)> = match vars
        .iter()
        .map(|v| bounds.get(v).map(|d| (v.clone(), d + 1)))
        .collect::<Option<Vec<_>>>()
    {
        Some(n) => n,
        None => return bounded_exact_sample(lhs, rhs, env, &vars),
    };
    let total_evals = needed
        .iter()
        .fold(1u64, |acc, (_, n)| acc.saturating_mul(*n));

    if total_evals <= MAX_EVALS {
        // Assemble per-variable value lists: deg+1 assumption-valid values
        // each. If assumptions starve a list, fall through to bounded
        // sampling — evidence, not proof.
        let mut value_lists: Vec<Vec<ExactNum>> = Vec::with_capacity(needed.len());
        let mut starved = false;
        for (var, n) in &needed {
            let mut list = Vec::with_capacity(*n as usize);
            let mut k = 0usize;
            while list.len() < *n as usize && k < (*n as usize) * 2 + 64 {
                let val = sample_value(k);
                k += 1;
                if crate::verify::point_satisfies_assumptions(var, val.to_f64(), env.assumptions())
                {
                    list.push(val);
                }
            }
            if list.len() < *n as usize {
                starved = true;
                break;
            }
            value_lists.push(list);
        }

        if !starved {
            // Iterate the full grid. Every point must evaluate exactly on
            // both sides for the identity theorem to apply; a pole on the
            // grid drops us to bounded sampling instead.
            let mut indices = vec![0usize; needed.len()];
            let mut grid_complete = true;
            'grid: loop {
                let mut env_pt = Environment::with_assumptions(env.assumptions().clone());
                let mut point_values: Vec<(String, f64)> = Vec::new();
                for (slot, (var, _)) in needed.iter().enumerate() {
                    let val = value_lists[slot][indices[slot]].clone();
                    point_values.push((var.clone(), val.to_f64()));
                    env_pt.set_exact(var, val);
                }
                match (
                    Evaluator::evaluate_exact(lhs, &env_pt),
                    Evaluator::evaluate_exact(rhs, &env_pt),
                ) {
                    (Ok(ExactNum::Rational(a)), Ok(ExactNum::Rational(b))) => {
                        if a != b {
                            return CheckedStep {
                                verdict: Verdict::Fail,
                                status: StatusReport::exact()
                                    .with_caveat(
                                        "disagreement established in exact rational arithmetic — a disproof, not a tolerance judgement",
                                    )
                                    .with_counterexample_value(exact_counterexample_json(
                                        &point_values,
                                        &a,
                                        &b,
                                    )),
                                mechanism: "exact_rational_sample".to_string(),
                            };
                        }
                    }
                    _ => {
                        grid_complete = false;
                        break 'grid;
                    }
                }
                // Odometer increment.
                let mut slot = 0;
                loop {
                    if slot == indices.len() {
                        break 'grid;
                    }
                    indices[slot] += 1;
                    if indices[slot] < value_lists[slot].len() {
                        break;
                    }
                    indices[slot] = 0;
                    slot += 1;
                }
            }
            if grid_complete {
                // Agreement on a (deg_v + 1)-per-variable grid: the
                // difference's numerator is identically zero. A proof of
                // equality in ℚ(x₁,…,xₙ), not a sample.
                return CheckedStep {
                    verdict: Verdict::Pass,
                    status: StatusReport::exact().with_caveat(
                        "equality in the rational function field, decided by exact evaluation on a grid exceeding the degree bound of the difference (polynomial identity theorem)",
                    ),
                    mechanism: "interpolation_identity_Q".to_string(),
                };
            }
        }
    }

    // Over budget, starved, or a pole on the grid: bounded exact sampling.
    // Zero tolerance still — but agreement below the degree bound is
    // evidence, not proof.
    bounded_exact_sample(lhs, rhs, env, &vars)
}

/// Fixed-count exact sampling: the honest fallback when the degree bound
/// is out of reach. Disagreement remains a certificate; agreement is
/// `verified` with the shortfall named.
fn bounded_exact_sample(lhs: &Node, rhs: &Node, env: &Environment, vars: &[String]) -> CheckedStep {
    use crate::evaluator::Evaluator;
    use crate::exact::ExactNum;

    let mechanism = "exact_rational_sample".to_string();
    let mut tested = 0usize;
    for i in 0..24 {
        let mut env_pt = Environment::with_assumptions(env.assumptions().clone());
        let mut point_values: Vec<(String, f64)> = Vec::new();
        let mut skip = false;
        for (j, var) in vars.iter().enumerate() {
            // Spread variables apart so same-named coordinates differ.
            let val = &sample_value(i) + &ExactNum::rational(3 * j as i64, 10);
            if !crate::verify::point_satisfies_assumptions(var, val.to_f64(), env.assumptions()) {
                skip = true;
                break;
            }
            point_values.push((var.clone(), val.to_f64()));
            env_pt.set_exact(var, val);
        }
        if skip {
            continue;
        }
        let (a, b) = match (
            Evaluator::evaluate_exact(lhs, &env_pt),
            Evaluator::evaluate_exact(rhs, &env_pt),
        ) {
            (Ok(ExactNum::Rational(a)), Ok(ExactNum::Rational(b))) => (a, b),
            _ => continue,
        };
        tested += 1;
        if a != b {
            return CheckedStep {
                verdict: Verdict::Fail,
                status: StatusReport::exact()
                    .with_caveat(
                        "disagreement established in exact rational arithmetic — a disproof, not a tolerance judgement",
                    )
                    .with_counterexample_value(exact_counterexample_json(&point_values, &a, &b)),
                mechanism,
            };
        }
        if tested >= 12 {
            break;
        }
    }

    if tested < crate::verify::MIN_POINTS_FOR_PASS {
        return CheckedStep {
            verdict: Verdict::Inconclusive,
            status: StatusReport::unable_to_compute(&format!(
                "only {} valid exact test point{} in the assumed domain (need at least {})",
                tested,
                if tested == 1 { "" } else { "s" },
                crate::verify::MIN_POINTS_FOR_PASS
            )),
            mechanism,
        };
    }
    CheckedStep {
        verdict: Verdict::Pass,
        status: StatusReport::verified(tested).with_caveat(
            "agreement established by exact rational evaluation (no floating-point tolerance), but at fewer points than the degree bound of the difference — evidence, not proof",
        ),
        mechanism,
    }
}

/// Counterexample JSON for exact-arithmetic disagreements: carries the
/// f64 renderings for uniformity AND the exact values as strings, because
/// two distinct rationals can share an f64 image (1/2 vs 1/2 + 10⁻³⁰) and
/// the witness must never contradict its own claim.
fn exact_counterexample_json(
    point: &[(String, f64)],
    lhs: &num_rational::BigRational,
    rhs: &num_rational::BigRational,
) -> serde_json::Value {
    use crate::exact::ExactNum;
    let point_map: serde_json::Map<String, serde_json::Value> = point
        .iter()
        .map(|(var, val)| (var.clone(), serde_json::json!(val)))
        .collect();
    // Exact witnesses can be enormous ((1/2)^10001 has thousands of
    // digits); cap the rendering — the magnitude of the disagreement is
    // the message, not the digits.
    let render_exact = |v: &num_rational::BigRational| -> String {
        let s = format!("{}", Node::Num(ExactNum::Rational(v.clone())));
        if s.len() > 64 {
            format!("{}… ({} more characters)", &s[..64], s.len() - 64)
        } else {
            s
        }
    };
    serde_json::json!({
        "point": point_map,
        "lhs": ExactNum::Rational(lhs.clone()).to_f64(),
        "rhs": ExactNum::Rational(rhs.clone()).to_f64(),
        "lhs_exact": render_exact(lhs),
        "rhs_exact": render_exact(rhs),
    })
}

/// Apply only rewrites that are identities in every interpretation,
/// including at points of undefinedness: multiplicative and additive unit
/// laws, exponent one, and double negation. Deliberately NOT the general
/// simplifier — nothing here has a side condition. (u·0 → 0 is excluded:
/// it fails where u is undefined.)
fn unit_normal_form(node: &Node) -> Node {
    let is_one = |n: &Node| matches!(n, Node::Num(crate::exact::ExactNum::Rational(r)) if r.numer() == &1.into() && r.denom() == &1.into());
    let is_zero = |n: &Node| matches!(n, Node::Num(crate::exact::ExactNum::Rational(r)) if r.numer() == &0.into());
    match node {
        Node::Multiply(l, r) => {
            let l = unit_normal_form(l);
            let r = unit_normal_form(r);
            if is_one(&l) {
                r
            } else if is_one(&r) {
                l
            } else {
                Node::Multiply(Box::new(l), Box::new(r))
            }
        }
        Node::Add(l, r) => {
            let l = unit_normal_form(l);
            let r = unit_normal_form(r);
            if is_zero(&l) {
                r
            } else if is_zero(&r) {
                l
            } else {
                Node::Add(Box::new(l), Box::new(r))
            }
        }
        Node::Subtract(l, r) => {
            let l = unit_normal_form(l);
            let r = unit_normal_form(r);
            if is_zero(&r) {
                l
            } else {
                Node::Subtract(Box::new(l), Box::new(r))
            }
        }
        Node::Power(b, e) => {
            let b = unit_normal_form(b);
            let e = unit_normal_form(e);
            if is_one(&e) {
                b
            } else {
                Node::Power(Box::new(b), Box::new(e))
            }
        }
        Node::Negate(inner) => {
            let inner = unit_normal_form(inner);
            if let Node::Negate(twice) = inner {
                *twice
            } else {
                Node::Negate(Box::new(inner))
            }
        }
        Node::Divide(l, r) => {
            Node::Divide(Box::new(unit_normal_form(l)), Box::new(unit_normal_form(r)))
        }
        Node::Function(name, args) => {
            Node::Function(name.clone(), args.iter().map(unit_normal_form).collect())
        }
        Node::Sqrt(inner) => Node::Sqrt(Box::new(unit_normal_form(inner))),
        Node::Abs(inner) => Node::Abs(Box::new(unit_normal_form(inner))),
        other => other.clone(),
    }
}

/// Assumption-aware numeric comparison, phrased as a step outcome.
fn numeric_check(lhs: &Node, rhs: &Node, env: &Environment) -> CheckedStep {
    let vars = free_variables(&[lhs, rhs]);

    // Variable-free comparison: there is exactly one point to test, so the
    // evidence is one evaluation — running the sampler would report the
    // same point twelve times as twelve (evidence inflation).
    if vars.is_empty() {
        let env_pt = Environment::new();
        let mechanism = "numeric_constant_eval".to_string();
        let (a, b) = match (
            crate::evaluator::Evaluator::evaluate(lhs, &env_pt),
            crate::evaluator::Evaluator::evaluate(rhs, &env_pt),
        ) {
            (Ok(a), Ok(b)) if !a.is_nan() && !b.is_nan() => (a, b),
            _ => {
                return CheckedStep {
                    verdict: Verdict::Inconclusive,
                    status: StatusReport::unable_to_compute(
                        "variable-free comparison could not be evaluated numerically",
                    ),
                    mechanism,
                }
            }
        };
        if crate::verify::values_match(a, b) {
            return CheckedStep {
                verdict: Verdict::Pass,
                status: StatusReport::verified(1).with_caveat(
                    "variable-free comparison: a single floating-point evaluation is the evidence",
                ),
                mechanism,
            };
        }
        let cx = crate::verify::Counterexample {
            point: Vec::new(),
            lhs_value: a,
            rhs_value: b,
        };
        return CheckedStep {
            verdict: Verdict::Fail,
            status: StatusReport::verified(1).with_counterexample(&cx),
            mechanism,
        };
    }

    let result = verify_identity(lhs, rhs, &vars, env.assumptions());

    // One-sided undefinedness witnesses a domain difference: excluded
    // from the numeric evidence, but never silently: skipping such
    // points is correct, hiding them is not.
    let domain_caveat = |report: StatusReport| {
        if result.domain_mismatches > 0 {
            report.with_caveat(&format!(
                "the expressions differ in domain at {} sample point{} (one side undefined); values compared only where both sides are defined",
                result.domain_mismatches,
                if result.domain_mismatches == 1 { "" } else { "s" }
            ))
        } else {
            report
        }
    };

    if let Some(ref cx) = result.counterexample {
        return CheckedStep {
            verdict: Verdict::Fail,
            status: domain_caveat(
                StatusReport::verified(result.points_tested).with_counterexample(cx),
            ),
            mechanism: "numeric_sample".to_string(),
        };
    }
    if result.insufficient_points {
        return CheckedStep {
            verdict: Verdict::Inconclusive,
            status: StatusReport::unable_to_compute(&format!(
                "only {} valid test point{} in the assumed domain (need at least 3)",
                result.points_tested,
                if result.points_tested == 1 { "" } else { "s" }
            )),
            mechanism: "numeric_sample".to_string(),
        };
    }
    CheckedStep {
        verdict: Verdict::Pass,
        status: domain_caveat(StatusReport::verified(result.points_tested)),
        mechanism: "numeric_sample".to_string(),
    }
}
