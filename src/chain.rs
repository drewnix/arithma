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
    /// The mechanism that actually ran, named so over-claims are auditable
    /// (Carl F2): e.g. `canonical_form_Q`, `numeric_sample`.
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
    /// Index of the step whose evidence determined the chain status
    /// (`None` for a chain with no relation steps).
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
    // step (Carl R5a). Otherwise it is the weakest evidence among relation
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
        Relation::Equals | Relation::FactoredFormOf => Ok(check_equals(prev, current, env)),
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

/// One rule for every relation (Carl R6b): use the declared variable, or
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
    let mut checked = check_equals(&derivative, current, env);
    checked.mechanism = format!("derivative_rules+{}", checked.mechanism);
    Ok(checked)
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
    let mut checked = check_equals(&derivative, prev, env);
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
    let substituted = crate::substitute::substitute_variable(prev, var, &value_node)
        .map_err(|e| format!("substitution failed: {}", e))?;
    let mut checked = check_equals(&substituted, current, env);
    checked.mechanism = format!("substitute+{}", checked.mechanism);
    Ok(checked)
}

/// Split an equation node into (lhs, rhs).
fn as_equation(node: &Node) -> Option<(&Node, &Node)> {
    match node {
        Node::Equation(l, r) | Node::Equal(l, r) => Some((l, r)),
        _ => None,
    }
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

    let lhs_sub = crate::substitute::substitute_variable(eq_lhs, &sol_var, &sol_value)
        .map_err(|e| format!("substitution failed: {}", e))?;
    let rhs_sub = crate::substitute::substitute_variable(eq_rhs, &sol_var, &sol_value)
        .map_err(|e| format!("substitution failed: {}", e))?;

    let mut checked = check_equals(&lhs_sub, &rhs_sub, env);
    checked.mechanism = format!("solution_substitution+{}", checked.mechanism);
    if checked.verdict == Verdict::Pass {
        checked.status = checked.status.with_caveat(
            "solution membership verified; completeness of the solution set is not claimed",
        );
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
        let lhs_sub = crate::substitute::substitute_variable(con_lhs, &var, sol)
            .map_err(|e| format!("substitution failed: {}", e))?;
        let rhs_sub = crate::substitute::substitute_variable(con_rhs, &var, sol)
            .map_err(|e| format!("substitution failed: {}", e))?;
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
                // null (Carl R5b).
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
    // coincidence (Carl R6c: printer normalization makes distinct trees
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
    let prev_s = prev.simplify(env).unwrap_or_else(|_| prev.clone());
    let current_s = current.simplify(env).unwrap_or_else(|_| current.clone());
    let in_fragment = is_algebraic_exact(prev) && is_algebraic_exact(current);

    if in_fragment {
        if format!("{}", prev_s) == format!("{}", current_s) {
            return CheckedStep {
                verdict: Verdict::Pass,
                status: StatusReport::exact(),
                mechanism: "canonical_form_Q".to_string(),
            };
        }
        let diff = Node::Subtract(Box::new(prev_s.clone()), Box::new(current_s.clone()));
        if let Ok(d) = diff.simplify(env) {
            if format!("{}", d) == "0" {
                return CheckedStep {
                    verdict: Verdict::Pass,
                    status: StatusReport::exact(),
                    mechanism: "difference_zero_Q".to_string(),
                };
            }
        }
        // Canonical forms did not decide it. In the fragment the honest
        // fallback is exact rational evaluation — never f64 tolerance,
        // which accepts provably false steps like x = x + 10⁻¹⁵ (Carl R1).
        return exact_rational_check(&prev_s, &current_s, env);
    }

    numeric_check(prev, current, env)
}

/// Compare two in-fragment expressions by exact rational evaluation at
/// assumption-respecting sample points. Any disagreement is a certificate:
/// f(p) ≠ g(p) in exact arithmetic, no tolerance anywhere. Agreement at n
/// points is `verified` evidence (canonicalization should have decided it;
/// that it did not is a simplifier completeness gap, not a licence to
/// upgrade).
fn exact_rational_check(lhs: &Node, rhs: &Node, env: &Environment) -> CheckedStep {
    use crate::evaluator::Evaluator;
    use crate::exact::ExactNum;

    const POINTS: &[(i64, i64)] = &[
        (1, 2),
        (-1, 2),
        (3, 2),
        (-3, 2),
        (3, 10),
        (-7, 10),
        (21, 10),
        (1, 10),
        (-23, 10),
        (3, 1),
        (4, 5),
        (9, 2),
    ];
    let mechanism = "exact_rational_sample".to_string();
    let vars = free_variables(&[lhs, rhs]);

    // Variable-free comparison: one exact evaluation DECIDES it — exact
    // rational arithmetic on constants is a decision procedure, and
    // repeating the same point 12 times would be evidence inflation.
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
            let cx = crate::verify::Counterexample {
                point: Vec::new(),
                lhs_value: ExactNum::Rational(a).to_f64(),
                rhs_value: ExactNum::Rational(b).to_f64(),
            };
            return CheckedStep {
                verdict: Verdict::Fail,
                status: StatusReport::exact()
                    .with_caveat("constants differ in exact rational arithmetic")
                    .with_counterexample(&cx),
                mechanism,
            };
        }
        return numeric_check(lhs, rhs, env);
    }

    let mut tested = 0usize;

    for (i, &(pn, pd)) in POINTS.iter().enumerate() {
        let mut env_pt = Environment::with_assumptions(env.assumptions().clone());
        let mut point_values: Vec<(String, f64)> = Vec::new();
        let mut skip = false;
        for (j, var) in vars.iter().enumerate() {
            // Same spread as the f64 sampler: base + 3j/10 + i/10, exactly.
            let val = &(&ExactNum::rational(pn, pd) + &ExactNum::rational(3 * j as i64, 10))
                + &ExactNum::rational(i as i64, 10);
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
        // Only exact rational results count; an error (pole) or a float
        // leak means the point decides nothing.
        let (a, b) = match (
            Evaluator::evaluate_exact(lhs, &env_pt),
            Evaluator::evaluate_exact(rhs, &env_pt),
        ) {
            (Ok(ExactNum::Rational(a)), Ok(ExactNum::Rational(b))) => (a, b),
            _ => continue,
        };
        tested += 1;
        if a != b {
            let cx = crate::verify::Counterexample {
                point: point_values,
                lhs_value: ExactNum::Rational(a).to_f64(),
                rhs_value: ExactNum::Rational(b).to_f64(),
            };
            return CheckedStep {
                verdict: Verdict::Fail,
                status: StatusReport::exact()
                    .with_caveat(
                        "disagreement established in exact rational arithmetic — a disproof, not a tolerance judgement",
                    )
                    .with_counterexample(&cx),
                mechanism,
            };
        }
    }

    if tested < 3 {
        return CheckedStep {
            verdict: Verdict::Inconclusive,
            status: StatusReport::unable_to_compute(&format!(
                "only {} valid exact test point{} in the assumed domain (need at least 3)",
                tested,
                if tested == 1 { "" } else { "s" }
            )),
            mechanism,
        };
    }
    CheckedStep {
        verdict: Verdict::Pass,
        status: StatusReport::verified(tested).with_caveat(
            "canonical forms did not decide; agreement established by exact rational evaluation (no floating-point tolerance)",
        ),
        mechanism,
    }
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
    let mut vars = free_variables(&[lhs, rhs]);
    if vars.is_empty() {
        vars.push("x".to_string());
    }
    let result = verify_identity(lhs, rhs, &vars, env.assumptions());

    if let Some(ref cx) = result.counterexample {
        return CheckedStep {
            verdict: Verdict::Fail,
            status: StatusReport::verified(result.points_tested).with_counterexample(cx),
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
        status: StatusReport::verified(result.points_tested),
        mechanism: "numeric_sample".to_string(),
    }
}
