//! Result-status taxonomy: machine-readable evidence classification for
//! every tool response.
//!
//! The design and the per-tool classification rules live in
//! `docs/result-status.md`. The one-sentence version: a status states what
//! *kind of evidence* backs a result — an algebraic decision procedure
//! (`exact`), numeric agreement at n points (`verified`), an unchecked but
//! believed-sound transformation (`heuristic`), an honest failure
//! (`unable_to_compute`), or a proof that no answer exists in the requested
//! class (`provably_impossible`).
//!
//! Three invariants:
//! 1. Numeric evidence never masquerades as proof — statuses may be
//!    downgraded along a pipeline, never upgraded.
//! 2. The counterexample is the diagnosis — failing checks carry the point
//!    and both values, nothing generative.
//! 3. No certificate, no exact — the tool boundary grants `exact` only
//!    after a certificate proves it. An empty certificate slot cannot be
//!    defaulted into anything.

use std::collections::BTreeSet;

use serde_json::{json, Value};

use crate::derivative::differentiate;
use crate::environment::Environment;
use crate::exact::ExactNum;
use crate::node::Node;
use crate::simplify::Simplifiable;
use crate::verify::verify_identity;
use num_traits::One;

/// A checkable receipt proving that a result is exact. The tool boundary
/// grants `exact` only after verifying that a certificate exists and has
/// been checked. Finding is hard, checking is easy — every checker is
/// asymptotically cheaper than its finder.
#[derive(Debug, Clone, PartialEq)]
pub struct Certificate {
    /// What kind of check was performed (e.g. "factor_multiply_back",
    /// "differentiation_round_trip", "decision_procedure").
    pub kind: String,
    /// Human-readable summary of what was verified (e.g. "product of
    /// factors equals input polynomial").
    pub witness: String,
    /// Whether the check passed. Must be `true` at the tool boundary.
    pub checked: bool,
}

impl Certificate {
    /// The algorithm is a decision procedure or provably complete and
    /// sound — the computation IS the proof, no separate replay needed.
    pub fn by_construction(algorithm: &str) -> Self {
        Certificate {
            kind: "decision_procedure".to_string(),
            witness: algorithm.to_string(),
            checked: true,
        }
    }

    /// Result verified by replaying a cheap check in exact arithmetic.
    pub fn replay(kind: &str, witness: &str) -> Self {
        Certificate {
            kind: kind.to_string(),
            witness: witness.to_string(),
            checked: true,
        }
    }

    /// JSON shape for the MCP payload.
    pub fn to_json(&self) -> Value {
        json!({
            "kind": self.kind,
            "witness": self.witness,
            "checked": self.checked,
        })
    }
}

/// A structured proof that some mathematical object does not exist in a
/// requested class. The impossibility is a theorem, not a failure — the
/// certificate names the method, the formal reason, and a plain-language
/// explanation. Analogous to `Certificate` for exact results, but with
/// negative polarity: instead of "here is the answer, proved," it says
/// "no answer exists, proved."
#[derive(Debug, Clone, PartialEq)]
pub struct ProofCertificate {
    /// The proof method, kebab-case (e.g. "risch-de", "rational-root-theorem",
    /// "negative-discriminant", "abel-ruffini").
    pub method: String,
    /// Formal mathematical reason (e.g. "The differential equation q' + f·q = g
    /// has no rational solution").
    pub reason: String,
    /// Plain-language explanation for non-specialists (e.g. "This integral has
    /// no formula using elementary functions. This is a theorem, not a
    /// limitation of the tool.").
    pub explanation: String,
}

impl ProofCertificate {
    pub fn new(method: &str, reason: &str, explanation: &str) -> Self {
        ProofCertificate {
            method: method.to_string(),
            reason: reason.to_string(),
            explanation: explanation.to_string(),
        }
    }

    pub fn to_json(&self) -> Value {
        json!({
            "method": self.method,
            "reason": self.reason,
            "explanation": self.explanation,
        })
    }
}

/// The five evidence classes. See `docs/result-status.md` for the earning
/// rules — a status must be earned by the mechanism that justifies it.
#[derive(Debug, Clone, PartialEq)]
pub enum ResultStatus {
    /// Result of a decision procedure or complete, sound algebraic algorithm.
    /// Requires a checked `Certificate` at the tool boundary.
    Exact,
    /// Independently checked numerically at `points_tested` points. Evidence,
    /// not proof.
    Verified { points_tested: usize },
    /// Transformation believed sound but not independently verified.
    Heuristic,
    /// The request was understood but no answer could be produced.
    UnableToCompute { reason: String },
    /// A proof that no answer exists in the requested class. A theorem, not
    /// a failure. Carries a structured `ProofCertificate` naming the method,
    /// the formal reason, and a plain-language explanation.
    ProvablyImpossible { proof: ProofCertificate },
}

/// Machine-readable verdict for tools whose result *is* a yes/no claim
/// (verify, equivalent, verify_chain). Orthogonal to the evidence class:
/// "not equal, counterexample attached" is a `fail` verdict carried by
/// well-earned `verified` evidence. Uniform vocabulary across tools so a
/// consumer switches on one enum, never parses prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    Fail,
    Inconclusive,
}

impl Verdict {
    pub fn as_str(&self) -> &'static str {
        match self {
            Verdict::Pass => "pass",
            Verdict::Fail => "fail",
            Verdict::Inconclusive => "inconclusive",
        }
    }
}

/// A status plus caveats. Caveats are orthogonal to the evidence class:
/// domain restrictions, truncation orders, precision notes.
#[derive(Debug, Clone, PartialEq)]
pub struct StatusReport {
    pub status: ResultStatus,
    pub caveats: Vec<String>,
    /// Present for verdict-shaped tools; `None` for tools whose result is
    /// an expression rather than a claim.
    pub verdict: Option<Verdict>,
    /// Present when a `verified` status carries a *negative* verdict: the
    /// specific point where the expressions disagree, as JSON
    /// `{point: {var: value, …}, lhs: …, rhs: …}`. The counterexample is
    /// the diagnosis — nothing generative.
    counterexample: Option<Value>,
    /// Present on `provably_impossible` integration results whose
    /// antiderivative was recognized as a named special function:
    /// `(name, LaTeX form)`, e.g. `("erf", "(√π/2)·erf(x)")`. A strictly
    /// additive refinement of the impossibility — the theorem stands.
    special_form: Option<(String, String)>,
    /// Present when status is `Exact`. The certificate proves the result
    /// by replaying a cheap check. At the tool boundary, exact without a
    /// checked certificate is downgraded to heuristic.
    certificate: Option<Certificate>,
}

impl StatusReport {
    /// Exact result backed by a checked certificate. The certificate
    /// proves the result by naming the check and recording that it passed.
    pub fn exact(certificate: Certificate) -> Self {
        let mut r = Self::new(ResultStatus::Exact);
        r.certificate = Some(certificate);
        r
    }

    pub fn verified(points_tested: usize) -> Self {
        Self::new(ResultStatus::Verified { points_tested })
    }

    pub fn heuristic() -> Self {
        Self::new(ResultStatus::Heuristic)
    }

    pub fn unable_to_compute(reason: &str) -> Self {
        Self::new(ResultStatus::UnableToCompute {
            reason: reason.to_string(),
        })
    }

    pub fn provably_impossible(proof: ProofCertificate) -> Self {
        Self::new(ResultStatus::ProvablyImpossible { proof })
    }

    fn new(status: ResultStatus) -> Self {
        StatusReport {
            status,
            caveats: Vec::new(),
            verdict: None,
            counterexample: None,
            special_form: None,
            certificate: None,
        }
    }

    /// Access the certificate, if present.
    pub fn certificate(&self) -> Option<&Certificate> {
        self.certificate.as_ref()
    }

    /// Attach a recognized special-function antiderivative to a
    /// `provably_impossible` result: the impossibility concerns the
    /// *elementary* class; the named form is the answer beyond it.
    pub fn with_special_form(mut self, function: &str, latex_form: &str) -> Self {
        self.special_form = Some((function.to_string(), latex_form.to_string()));
        self
    }

    pub fn with_caveat(mut self, caveat: &str) -> Self {
        self.caveats.push(caveat.to_string());
        self
    }

    pub fn with_verdict(mut self, verdict: Verdict) -> Self {
        self.verdict = Some(verdict);
        self
    }

    pub fn with_counterexample(mut self, cx: &crate::verify::Counterexample) -> Self {
        // NaN would serialize as a bare null; an undefined side is a
        // meaningful verdict (domain violation) and says so explicitly.
        let render = |v: f64| -> Value {
            if v.is_nan() {
                json!("undefined")
            } else {
                json!(v)
            }
        };
        let point: serde_json::Map<String, Value> = cx
            .point
            .iter()
            .map(|(var, val)| (var.clone(), render(*val)))
            .collect();
        self.counterexample = Some(json!({
            "point": point,
            "lhs": render(cx.lhs_value),
            "rhs": render(cx.rhs_value),
        }));
        self
    }

    pub fn counterexample_json(&self) -> Option<&Value> {
        self.counterexample.as_ref()
    }

    /// Attach an already-serialized counterexample (used when propagating a
    /// counterexample from an inner check, e.g. implies re-reporting the
    /// consequent check's witness).
    pub fn with_counterexample_value(mut self, cx: Value) -> Self {
        self.counterexample = Some(cx);
        self
    }

    /// The JSON shape consumed by MCP clients. Contract: consumers switch on
    /// the `status` string and ignore unknown fields; evidence fields appear
    /// only for the status that earns them.
    ///
    /// **Gate (invariant 3):** exact without a checked certificate is
    /// downgraded to heuristic at this boundary — the tool may have
    /// computed correctly, but it cannot prove it did.
    pub fn to_json(&self) -> Value {
        let mut obj = json!({});
        match &self.status {
            ResultStatus::Exact => match &self.certificate {
                Some(cert) if cert.checked => {
                    obj["status"] = json!("exact");
                    obj["certificate"] = cert.to_json();
                }
                _ => {
                    obj["status"] = json!("heuristic");
                    obj["caveats"] = json!(["uncertified exact result — no certificate"]);
                    return obj;
                }
            },
            ResultStatus::Verified { points_tested } => {
                obj["status"] = json!("verified");
                obj["points_tested"] = json!(points_tested);
            }
            ResultStatus::Heuristic => {
                obj["status"] = json!("heuristic");
            }
            ResultStatus::UnableToCompute { reason } => {
                obj["status"] = json!("unable_to_compute");
                obj["reason"] = json!(reason);
            }
            ResultStatus::ProvablyImpossible { proof } => {
                obj["status"] = json!("provably_impossible");
                obj["proof_certificate"] = proof.to_json();
            }
        }
        if let Some(v) = &self.verdict {
            obj["verdict"] = json!(v.as_str());
        }
        if !self.caveats.is_empty() {
            obj["caveats"] = json!(self.caveats);
        }
        if let Some(cx) = &self.counterexample {
            obj["counterexample"] = cx.clone();
        }
        if let Some((function, form)) = &self.special_form {
            obj["special_function"] = json!(function);
            obj["special_form"] = json!(form);
        }
        obj
    }

    /// Text marker for non-exact statuses. `exact` is quiet (unmarked
    /// means proof). Everything else is loud — the marker tells the
    /// consumer what kind of evidence backs the result. This is the
    /// delivery surface: most MCP hosts strip the result_status sidecar,
    /// so the marker is the only tier signal that reaches the agent.
    pub fn marker(&self) -> Option<String> {
        match &self.status {
            ResultStatus::Exact => None,
            ResultStatus::Verified { points_tested } => {
                // Verdict-shaped tools (verify, equivalent, verify_chain)
                // already carry their tier in-band in the sentence text —
                // suppress the generic marker to avoid double-marking.
                if self.verdict.is_some() {
                    return None;
                }
                let detail = if self.caveats.is_empty() {
                    format!("numeric evidence, {} points — not proof", points_tested)
                } else {
                    format!("{} points — {}", points_tested, self.caveats.join("; "))
                };
                Some(format!("[verified] {}", detail))
            }
            ResultStatus::Heuristic => {
                let detail = if self.caveats.is_empty() {
                    "result not independently verified".to_string()
                } else {
                    self.caveats.join("; ")
                };
                Some(format!("[heuristic] {}", detail))
            }
            ResultStatus::UnableToCompute { reason } => {
                // Caveats can carry the diagnosis (e.g. the witness from a
                // simplify-assisted retry) — attached evidence must reach
                // the wire, not just the data structure.
                if self.caveats.is_empty() {
                    Some(format!("[unable to compute] {}", reason))
                } else {
                    Some(format!(
                        "[unable to compute] {} — {}",
                        reason,
                        self.caveats.join("; ")
                    ))
                }
            }
            ResultStatus::ProvablyImpossible { proof } => match &self.special_form {
                Some((_, form)) => Some(format!(
                    "[provably impossible] {} — antiderivative in special functions: {}",
                    proof.explanation, form
                )),
                None => Some(format!("[provably impossible] {}", proof.explanation)),
            },
        }
    }
}

/// Does this expression lie in the fragment where canonicalization is a
/// decision procedure: rational constants, variables, field operations, and
/// integer powers? Within that fragment, equal canonical forms *prove*
/// equivalence over ℚ(x₁,…,xₙ). Anything outside it (transcendental
/// functions, radicals, float literals, variable exponents) disqualifies —
/// conservatively: the classifier may under-claim, never over-claim.
/// Names the evaluator treats as built-in transcendental constants, not
/// free variables. They must be excluded from sampling (binding e := 0.5
/// shadows Euler's constant and manufactures false counterexamples)
/// and from the ℚ-exact fragment (they are not rational atoms).
pub fn is_builtin_constant(name: &str) -> bool {
    matches!(name, "e" | "π")
}

pub fn is_algebraic_exact(node: &Node) -> bool {
    match node {
        Node::Num(ExactNum::Rational(_)) => true,
        Node::Num(ExactNum::Float(_)) => false,
        Node::Variable(v) => !is_builtin_constant(v),
        Node::Add(l, r) | Node::Subtract(l, r) | Node::Multiply(l, r) | Node::Divide(l, r) => {
            is_algebraic_exact(l) && is_algebraic_exact(r)
        }
        Node::Negate(inner) => is_algebraic_exact(inner),
        Node::Power(base, exp) => is_algebraic_exact(base) && is_integer_exponent(exp),
        _ => false,
    }
}

/// Integer exponents keep us inside the rational-function field. The parser
/// produces `x^{-2}` as `Power(x, Negate(2))`, so unwrap negation.
fn is_integer_exponent(node: &Node) -> bool {
    match node {
        Node::Num(ExactNum::Rational(r)) => r.denom().is_one(),
        Node::Negate(inner) => is_integer_exponent(inner),
        _ => false,
    }
}

/// `bound` is the stack of binder-scoped names currently in force. Scoping
/// must be tracked on the way DOWN, not undone on the way up: removing a
/// binder's name from the shared accumulator after recursion also erases
/// same-named FREE occurrences collected from sibling subtrees
/// (`y + Σ_{y=1}^{3} y` has a free y), which turns variable inference —
/// and everything downstream of it — silently wrong.
fn collect_variables(node: &Node, vars: &mut BTreeSet<String>, bound: &mut Vec<String>) {
    match node {
        Node::Variable(v) => {
            if !is_builtin_constant(v) && !bound.iter().any(|b| b == v) {
                vars.insert(v.clone());
            }
        }
        Node::Num(_) => {}
        Node::Add(l, r)
        | Node::Subtract(l, r)
        | Node::Multiply(l, r)
        | Node::Divide(l, r)
        | Node::Power(l, r)
        | Node::Greater(l, r)
        | Node::Less(l, r)
        | Node::GreaterEqual(l, r)
        | Node::LessEqual(l, r)
        | Node::Equal(l, r)
        | Node::Equation(l, r) => {
            collect_variables(l, vars, bound);
            collect_variables(r, vars, bound);
        }
        Node::Sqrt(inner)
        | Node::Abs(inner)
        | Node::Floor(inner)
        | Node::Ceil(inner)
        | Node::Round(inner)
        | Node::Trunc(inner)
        | Node::Negate(inner)
        | Node::Factorial(inner) => collect_variables(inner, vars, bound),
        Node::Piecewise(arms) => {
            for (expr, cond) in arms {
                collect_variables(expr, vars, bound);
                collect_variables(cond, vars, bound);
            }
        }
        Node::Summation(idx, start, end, body) | Node::Product(idx, start, end, body) => {
            // The index is bound in the body only; the bounds are outside
            // the binder's scope.
            collect_variables(start, vars, bound);
            collect_variables(end, vars, bound);
            bound.push(idx.clone());
            collect_variables(body, vars, bound);
            bound.pop();
        }
        Node::Function(_, args) => {
            for a in args {
                collect_variables(a, vars, bound);
            }
        }
    }
}

/// Free variables across a set of expressions, sorted. Summation and
/// product index variables are bound in their bodies, not free.
pub fn free_variables(nodes: &[&Node]) -> Vec<String> {
    let mut vars = BTreeSet::new();
    let mut bound = Vec::new();
    for n in nodes {
        collect_variables(n, &mut vars, &mut bound);
    }
    vars.into_iter().collect()
}

/// Certify the numeric equivalence of two expressions and phrase the result
/// as a status. `context` names the check in caveats ("self-check",
/// "round-trip") so a failure identifies its own mechanism.
fn numeric_equivalence_status(
    lhs: &Node,
    rhs: &Node,
    env: &Environment,
    context: &str,
) -> StatusReport {
    let vars = free_variables(&[lhs, rhs]);
    if vars.is_empty() {
        // Constant expressions: a single evaluation decides.
        let result = verify_identity(lhs, rhs, &["x".to_string()], env.assumptions());
        // verify_identity treats absent variables as trivially consistent;
        // fall through to the same handling below.
        return status_from_verify(result, context);
    }
    let result = verify_identity(lhs, rhs, &vars, env.assumptions());
    status_from_verify(result, context)
}

fn status_from_verify(result: crate::verify::VerifyResult, context: &str) -> StatusReport {
    if let Some(cx) = result.counterexample {
        let point: Vec<String> = cx
            .point
            .iter()
            .map(|(v, val)| format!("{}={}", v, val))
            .collect();
        return StatusReport::heuristic().with_caveat(&format!(
            "numeric {} FAILED at {}: lhs={}, rhs={} — treat result with suspicion",
            context,
            point.join(", "),
            cx.lhs_value,
            cx.rhs_value
        ));
    }
    if result.insufficient_points {
        return StatusReport::heuristic().with_caveat(&format!(
            "numeric {} inconclusive (only {} valid test points)",
            context, result.points_tested
        ));
    }
    StatusReport::verified(result.points_tested)
}

/// Classify the verify tool's own verdict. Unlike the classifiers above,
/// a FAIL here is not a degraded result: "not equal, witness attached" is a
/// well-evidenced verdict, so both PASS and FAIL map to `verified`. Only
/// insufficient sampling is an honest `unable_to_compute`. Never `exact` —
/// this tool is numeric by definition.
pub fn classify_verify(result: &crate::verify::VerifyResult) -> StatusReport {
    if result.insufficient_points {
        return StatusReport::unable_to_compute(&format!(
            "only {} valid test point{} in the assumed domain (need at least 3)",
            result.points_tested,
            if result.points_tested == 1 { "" } else { "s" }
        ))
        .with_verdict(Verdict::Inconclusive);
    }
    let mut report = StatusReport::verified(result.points_tested);
    if result.domain_mismatches > 0 {
        report = report.with_caveat(&format!(
            "the expressions differ in domain at {} sample point{} (one side undefined); values compared only where both sides are defined",
            result.domain_mismatches,
            if result.domain_mismatches == 1 { "" } else { "s" }
        ));
    }
    match &result.counterexample {
        Some(cx) => report.with_counterexample(cx).with_verdict(Verdict::Fail),
        None => report.with_verdict(Verdict::Pass),
    }
}

/// Classify a simplification `input → output`.
///
/// Poly/rational fragment → `exact` (canonicalization is a decision
/// procedure). Identity transformation → `exact` (trivial claim). Otherwise
/// the rewrite involves transcendental structure we cannot certify
/// algebraically without rule-level provenance, so it is checked numerically:
/// `verified` on agreement, loud `heuristic` on failure — which doubles as an
/// in-production bug detector for the simplifier itself.
pub fn classify_simplify(input: &Node, output: &Node, env: &Environment) -> StatusReport {
    if is_algebraic_exact(input) && is_algebraic_exact(output) {
        return StatusReport::exact(Certificate::by_construction(
            "canonical_form_Q — polynomial/rational canonicalization is a decision procedure",
        ));
    }
    if format!("{}", input) == format!("{}", output) {
        return StatusReport::exact(Certificate::by_construction(
            "identity — input and output are structurally identical",
        ));
    }
    numeric_equivalence_status(input, output, env, "self-check")
}

/// Classify a limit result by numeric corroboration along the approach
/// path. The symbolic limit engine mixes exact series arithmetic with
/// dominant-term analysis, and without rule-level provenance we cannot
/// certify which path produced the answer — so a numerically checkable
/// claim is corroborated by sampling toward the point: `verified` when the
/// samples converge to the claim, loud `heuristic` when they contradict it,
/// quiet `heuristic` when corroboration is unavailable (symbolic claims).
pub fn classify_limit(
    expr: &Node,
    var: &str,
    point_str: &str,
    claimed_latex: &str,
    env: &Environment,
) -> StatusReport {
    use crate::limits::{parse_limit_point, LimitDirection, LimitPoint};

    let not_corroborated = || {
        StatusReport::heuristic()
            .with_caveat("symbolic limit; not numerically corroborated along the approach path")
    };

    let (point, direction) = match parse_limit_point(point_str) {
        Ok(p) => p,
        Err(_) => return not_corroborated(),
    };

    // Parse the claimed value: ±∞ by string form, otherwise a constant.
    let trimmed = claimed_latex.trim().trim_start_matches('+');
    let claimed = if trimmed == "\\infty" || trimmed == "inf" {
        LimitPoint::PosInfinity
    } else if trimmed == "-\\infty" || trimmed == "-inf" {
        LimitPoint::NegInfinity
    } else {
        let node = match crate::parser::parse_latex(claimed_latex, env) {
            Ok(n) => n,
            Err(_) => return not_corroborated(),
        };
        if !free_variables(&[&node]).is_empty() {
            return not_corroborated();
        }
        match crate::evaluator::Evaluator::evaluate(&node, &Environment::new()) {
            Ok(v) if v.is_finite() => LimitPoint::Finite(ExactNum::from_f64(v)),
            _ => return not_corroborated(),
        }
    };

    // Sample points approaching the limit point, tagged with their approach
    // level (higher level = closer). Levels let the trend check compare
    // "coarse" against "fine" without confusing the two sides of a
    // two-sided approach.
    let approach: Vec<(usize, f64)> = match &point {
        LimitPoint::PosInfinity => [1e2, 1e3, 1e4, 1e5]
            .iter()
            .enumerate()
            .map(|(k, t)| (k, *t))
            .collect(),
        LimitPoint::NegInfinity => [-1e2, -1e3, -1e4, -1e5]
            .iter()
            .enumerate()
            .map(|(k, t)| (k, *t))
            .collect(),
        LimitPoint::Finite(a) => {
            let a = a.to_f64();
            let offsets = [1e-2, 1e-3, 1e-4, 1e-5];
            match direction {
                LimitDirection::Right => offsets
                    .iter()
                    .enumerate()
                    .map(|(k, h)| (k, a + h))
                    .collect(),
                LimitDirection::Left => offsets
                    .iter()
                    .enumerate()
                    .map(|(k, h)| (k, a - h))
                    .collect(),
                LimitDirection::Both => offsets
                    .iter()
                    .enumerate()
                    .flat_map(|(k, h)| [(k, a + h), (k, a - h)])
                    .collect(),
            }
        }
    };

    let mut samples: Vec<(usize, f64)> = Vec::new(); // (level, f(t))
    for (level, t) in approach {
        let mut sample_env = Environment::new();
        sample_env.set(var, t);
        if let Ok(v) = crate::evaluator::Evaluator::evaluate(expr, &sample_env) {
            if !v.is_nan() {
                samples.push((level, v));
            }
        }
    }
    if samples.len() < 2 {
        return not_corroborated();
    }
    let n = samples.len();
    let coarsest = samples.iter().map(|(k, _)| *k).min().unwrap();
    let finest = samples.iter().map(|(k, _)| *k).max().unwrap();

    // Three-way outcome. "Slow" matters: a correct limit like 1/ln(x) → 0
    // is nowhere near tolerance at x = 10^5, and reporting a correct answer
    // as FAILED is a false alarm agents will learn to ignore. The
    // discriminator between "wrong" and "right but slow" is whether the
    // error CONTRACTS along the approach (at least halves from the coarsest
    // to the finest level).
    enum Corroboration {
        Confirmed,
        SlowButConsistent,
        Contradicted,
    }

    let outcome = match &claimed {
        LimitPoint::Finite(l) => {
            let l = l.to_f64();
            let min_err = samples
                .iter()
                .map(|(_, v)| (v - l).abs())
                .fold(f64::INFINITY, f64::min);
            if min_err <= 0.01 * l.abs().max(1.0) {
                Corroboration::Confirmed
            } else if coarsest < finest {
                // Worst error per level, compared coarse vs fine.
                let level_err = |k: usize| {
                    samples
                        .iter()
                        .filter(|(kk, _)| *kk == k)
                        .map(|(_, v)| (v - l).abs())
                        .fold(0.0, f64::max)
                };
                if level_err(finest) <= 0.5 * level_err(coarsest) {
                    Corroboration::SlowButConsistent
                } else {
                    Corroboration::Contradicted
                }
            } else {
                Corroboration::Contradicted
            }
        }
        LimitPoint::PosInfinity | LimitPoint::NegInfinity => {
            let sign_ok = match &claimed {
                LimitPoint::PosInfinity => samples.iter().all(|(_, v)| *v > 0.0),
                _ => samples.iter().all(|(_, v)| *v < 0.0),
            };
            let max_mag = samples.iter().map(|(_, v)| v.abs()).fold(0.0, f64::max);
            let level_min_mag = |k: usize| {
                samples
                    .iter()
                    .filter(|(kk, _)| *kk == k)
                    .map(|(_, v)| v.abs())
                    .fold(f64::INFINITY, f64::min)
            };
            if !sign_ok {
                Corroboration::Contradicted
            } else if max_mag > 1e4 {
                Corroboration::Confirmed
            } else if coarsest < finest && level_min_mag(finest) >= 2.0 * level_min_mag(coarsest) {
                Corroboration::SlowButConsistent
            } else {
                Corroboration::Contradicted
            }
        }
    };

    match outcome {
        Corroboration::Confirmed => StatusReport::verified(n)
            .with_caveat("corroborated numerically along the approach path"),
        Corroboration::SlowButConsistent => StatusReport::heuristic().with_caveat(
            "samples move toward the claimed limit but too slowly to corroborate within tolerance",
        ),
        Corroboration::Contradicted => StatusReport::heuristic().with_caveat(
            "numeric corroboration FAILED: samples along the approach path do not converge to the claimed limit — treat result with suspicion",
        ),
    }
}

/// Classify an indefinite integration result by the differentiation
/// round-trip: differentiate the antiderivative and compare to the
/// integrand. A structural match after simplification is an algebraic
/// certificate (`exact`) — this is why `integral_of` can reach `exact` where
/// `implies` never can. Numeric-only agreement is `verified`; disagreement
/// is a loud `heuristic`.
pub fn classify_integral(
    integrand: &Node,
    antiderivative: &Node,
    var: &str,
    env: &Environment,
) -> StatusReport {
    let derivative = match differentiate(antiderivative, var) {
        Ok(d) => d,
        Err(e) => {
            return StatusReport::heuristic()
                .with_caveat(&format!("round-trip check unavailable: {}", e))
        }
    };
    let derivative = derivative.simplify(env).unwrap_or(derivative);
    let integrand_s = integrand
        .simplify(env)
        .unwrap_or_else(|_| integrand.clone());

    if format!("{}", derivative) == format!("{}", integrand_s) {
        return StatusReport::exact(Certificate::replay(
            "differentiation_round_trip",
            "d/dx of antiderivative matches integrand structurally",
        ));
    }

    // Structural forms differ — try the difference.
    let diff = Node::Subtract(Box::new(derivative.clone()), Box::new(integrand_s.clone()));
    if let Ok(d) = diff.simplify(env) {
        if format!("{}", d) == "0" {
            return StatusReport::exact(Certificate::replay(
                "differentiation_round_trip",
                "d/dx of antiderivative minus integrand simplifies to zero",
            ));
        }
    }

    numeric_equivalence_status(&derivative, &integrand_s, env, "round-trip")
}
