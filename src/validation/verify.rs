use crate::assumptions::Assumptions;
use crate::environment::Environment;
use crate::evaluator::Evaluator;
use crate::exact::ExactNum;
use crate::node::Node;
use crate::simplify::Simplifiable;
use crate::status::free_variables;
use crate::tokenizer::normalize_var;
use std::collections::{HashMap, HashSet};

const TEST_POINTS: &[f64] = &[
    0.5, -0.5, 1.5, -1.5, 0.3, -0.7, 2.1, 0.1, -2.3, 3.0, 0.8, 4.5,
];

const TOLERANCE: f64 = 1e-8;
pub(crate) const MIN_POINTS_FOR_PASS: usize = 3;
/// A Σ/Π whose range length varies must realize at least this many
/// distinct lengths before a PASS is granted — otherwise the sampler is
/// certifying an identity over a slice of its parameter space.
const MIN_DISTINCT_RANGE_LENGTHS: usize = 3;

pub struct VerifyResult {
    pub passed: bool,
    pub points_tested: usize,
    pub counterexample: Option<Counterexample>,
    pub insufficient_points: bool,
    /// Sample points where exactly one side was undefined (NaN). Such
    /// points are excluded from the numeric evidence — they witness a
    /// *domain* difference, which callers surface as a caveat rather than
    /// as a numeric counterexample with a null in it.
    pub domain_mismatches: usize,
    /// When a PASS was withheld because some Σ/Π range realized too few
    /// distinct lengths, the minimum realized count — so renderers can
    /// state the actual reason instead of misattributing the refusal to
    /// the point count. None when length coverage was adequate or not owed.
    pub starved_range_lengths: Option<usize>,
}

impl VerifyResult {
    /// The reason a PASS was withheld. Meaningful when
    /// `insufficient_points` is true; every renderer of an inconclusive
    /// verdict must use this rather than assuming the point count was
    /// the cause.
    pub fn insufficiency_reason(&self) -> String {
        match self.starved_range_lengths {
            Some(k) => format!(
                "a Σ/Π range realized only {k} distinct length{} across the sampled points \
                 (need at least {MIN_DISTINCT_RANGE_LENGTHS} — the identity is parameterized \
                 by its range length)",
                if k == 1 { "" } else { "s" }
            ),
            None => format!(
                "only {} valid test point{} in the assumed domain (need at least {MIN_POINTS_FOR_PASS})",
                self.points_tested,
                if self.points_tested == 1 { "" } else { "s" }
            ),
        }
    }
}

pub struct Counterexample {
    pub point: Vec<(String, f64)>,
    pub lhs_value: f64,
    pub rhs_value: f64,
}

pub fn verify_identity(
    lhs: &Node,
    rhs: &Node,
    variables: &[String],
    assumptions: &Assumptions,
) -> VerifyResult {
    let normalized: Vec<String> = variables.iter().map(|v| normalize_var(v)).collect();
    let mut points_tested = 0;
    let domain_mismatches = 0;

    let range_constraints = {
        let mut m = HashMap::new();
        collect_range_bound_constraints(lhs, &mut m);
        collect_range_bound_constraints(rhs, &mut m);
        m
    };

    // A Σ/Π whose bounds are both unanchored variables is parameterized
    // by its range length L = end − start + 1, and the sampler owes the
    // identity coverage in L: independent per-variable streams round to
    // the same integer, so every sampled range has L = 1 — and any claim
    // true of single-term ranges (Σ_{k=m}^{n} f(k) = f(m), for every f)
    // would earn a false PASS. Each such pair samples its lower bound
    // from an integer stream and derives the upper as lo + L − 1.
    let bound_pairs = {
        let mut pairs = Vec::new();
        collect_symbolic_bound_pairs(lhs, &mut pairs);
        collect_symbolic_bound_pairs(rhs, &mut pairs);
        // Anchored variables already walk an integer stream away from
        // their constant opposite bound, which varies L on its own.
        let unanchored = |v: &String| {
            range_constraints
                .get(v)
                .is_none_or(|c| c.min.is_none() && c.max.is_none())
        };
        pairs.retain(|(lo, hi)| unanchored(lo) && unanchored(hi));
        pairs
    };

    // The empty range (L = 0) is legitimate and sharply discriminating:
    // most false closed forms are not 0 there.
    const RANGE_LENGTH_CYCLE: &[f64] = &[0.0, 1.0, 2.0, 3.0, 5.0, 7.0];

    // Coverage assertion — the backstop behind the pair constructor
    // above. A Σ/Π whose range length L = end − start + 1 is not
    // structurally constant is an identity parameterized by L, and a
    // PASS must be backed by evidence across lengths: any sampling
    // strategy that collapses to one length (a bound the constructor
    // doesn't recognize falls back to the rounding stream, which does)
    // silently verifies every claim true of that one length. The
    // constructor is a strategy for achieving coverage; this makes the
    // strategy's failure loud. Realized lengths are recorded at each
    // counted point and checked before a PASS is granted.
    let mut length_ranges: Vec<((Node, Node), HashSet<i64>)> = {
        let mut ranges = Vec::new();
        collect_variable_length_ranges(lhs, &normalized, &mut ranges);
        collect_variable_length_ranges(rhs, &normalized, &mut ranges);
        ranges.into_iter().map(|r| (r, HashSet::new())).collect()
    };

    let mut seen_points = HashSet::new();

    for (i, &base_point) in TEST_POINTS.iter().enumerate() {
        let mut env = Environment::new();
        let mut point_values = Vec::new();
        let mut skip_point = false;

        let mut pair_values: HashMap<String, f64> = HashMap::new();
        for (lo_var, hi_var) in &bound_pairs {
            let lo = match pair_values.get(lo_var) {
                Some(&v) => v,
                None => (base_point + 0.1 * i as f64).round(),
            };
            let len = RANGE_LENGTH_CYCLE[i % RANGE_LENGTH_CYCLE.len()];
            pair_values.entry(lo_var.clone()).or_insert(lo);
            pair_values.entry(hi_var.clone()).or_insert(lo + len - 1.0);
        }

        for (j, var) in normalized.iter().enumerate() {
            let val = if let Some(&paired) = pair_values.get(var) {
                paired
            } else {
                match range_constraints.get(var) {
                    Some(c) => match (c.min, c.max) {
                        // Anchored integer streams: lo, lo+1, … (or hi, hi−1, …),
                        // offset by variable position so multi-variable points
                        // stay distinct.
                        (Some(lo), _) => lo + (i + j) as f64,
                        (None, Some(hi)) => hi - (i + j) as f64,
                        (None, None) => (base_point + 0.3 * j as f64 + 0.1 * i as f64).round(),
                    },
                    None => base_point + 0.3 * j as f64 + 0.1 * i as f64,
                }
            };
            if let Some(c) = range_constraints.get(var) {
                // A var constrained from both sides (appears in a lower and
                // an upper bound) samples from its min; skip past the max.
                if c.max.is_some_and(|hi| val > hi) || c.min.is_some_and(|lo| val < lo) {
                    skip_point = true;
                    break;
                }
            }
            if !point_satisfies_assumptions(var, val, assumptions) {
                skip_point = true;
                break;
            }
            env.set(var, val);
            point_values.push((var.clone(), val));
        }

        if skip_point {
            continue;
        }

        // Rounding and derived bounds can collide onto an already-tested
        // assignment; a duplicate is not new evidence and must not inflate
        // points_tested. (−0.0 normalizes to +0.0 — they are the same
        // sample point even though their bit patterns differ.)
        let point_key: Vec<u64> = point_values
            .iter()
            .map(|(_, v)| if *v == 0.0 { 0.0f64 } else { *v }.to_bits())
            .collect();
        if !seen_points.insert(point_key) {
            continue;
        }

        let lhs_val = match Evaluator::evaluate(lhs, &env) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let rhs_val = match Evaluator::evaluate(rhs, &env) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Shared undefinedness is not numeric agreement: a point where
        // BOTH sides are NaN tests domain membership, not values — skip,
        // uncounted. One-sided undefinedness is a DOMAIN VIOLATION: one
        // expression exists where the other does not, which refutes the
        // identity as stated. That is a counterexample (serialized with an
        // explicit "undefined", never a bare null), not a skippable point.
        if lhs_val.is_nan() && rhs_val.is_nan() {
            continue;
        }
        if lhs_val.is_nan() || rhs_val.is_nan() {
            return VerifyResult {
                passed: false,
                points_tested,
                counterexample: Some(Counterexample {
                    point: point_values,
                    lhs_value: lhs_val,
                    rhs_value: rhs_val,
                }),
                insufficient_points: false,
                domain_mismatches: domain_mismatches + 1,
                starved_range_lengths: None,
            };
        }

        points_tested += 1;

        for ((start, end), realized) in length_ranges.iter_mut() {
            if let (Ok(s), Ok(e)) = (
                Evaluator::evaluate(start, &env),
                Evaluator::evaluate(end, &env),
            ) {
                realized.insert((e - s + 1.0).round() as i64);
            }
        }

        if !values_match(lhs_val, rhs_val) {
            return VerifyResult {
                passed: false,
                points_tested,
                counterexample: Some(Counterexample {
                    point: point_values,
                    lhs_value: lhs_val,
                    rhs_value: rhs_val,
                }),
                insufficient_points: false,
                domain_mismatches,
                starved_range_lengths: None,
            };
        }
    }

    let starved_range_lengths = length_ranges
        .iter()
        .map(|(_, realized)| realized.len())
        .min()
        .filter(|&k| k < MIN_DISTINCT_RANGE_LENGTHS);
    let insufficient = points_tested < MIN_POINTS_FOR_PASS || starved_range_lengths.is_some();
    VerifyResult {
        passed: !insufficient,
        points_tested,
        counterexample: None,
        insufficient_points: insufficient,
        domain_mismatches,
        starved_range_lengths,
    }
}

pub(crate) fn point_satisfies_assumptions(var: &str, val: f64, assumptions: &Assumptions) -> bool {
    if assumptions.is_positive(var) && val <= 0.0 {
        return false;
    }
    if assumptions.is_nonneg(var) && val < 0.0 {
        return false;
    }
    if assumptions.is_negative(var) && val >= 0.0 {
        return false;
    }
    if assumptions.is_nonzero(var) && val == 0.0 {
        return false;
    }
    if assumptions.is_integer(var) && val.fract() != 0.0 {
        return false;
    }
    true
}

/// Sampling constraint for a variable that appears in a Σ/Π range bound.
/// Such a variable only means anything at integer values, and when the
/// opposite bound is a constant, only on the meaningful side of it: a sum
/// written Σ_{k=1}^{n} presupposes n ≥ 1, and sampling n = 0.5 — where the
/// sum has no value and a truncated evaluation invents one — tests nothing
/// the author wrote. (Evaluation of a non-integer bound is an error; this
/// constraint is what keeps the sampler from wasting its points on errors.)
#[derive(Default, Clone, Copy)]
struct RangeBoundConstraint {
    min: Option<f64>,
    max: Option<f64>,
}

/// Σ/Π ranges whose length L = end − start + 1 is NOT structurally
/// constant — the ranges for which `verify_identity` owes realized
/// length coverage. Restricted to ranges whose bound variables are all
/// sampled: an inner range depending on an enclosing Σ/Π index cannot
/// be evaluated standalone and is not asserted here.
fn collect_variable_length_ranges(node: &Node, sampled: &[String], out: &mut Vec<(Node, Node)>) {
    match node {
        Node::Variable(_) | Node::Num(_) => {}
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
            collect_variable_length_ranges(l, sampled, out);
            collect_variable_length_ranges(r, sampled, out);
        }
        Node::Sqrt(inner)
        | Node::Abs(inner)
        | Node::Floor(inner)
        | Node::Ceil(inner)
        | Node::Round(inner)
        | Node::Trunc(inner)
        | Node::Negate(inner)
        | Node::Factorial(inner) => collect_variable_length_ranges(inner, sampled, out),
        Node::Piecewise(arms) => {
            for (expr, cond) in arms {
                collect_variable_length_ranges(expr, sampled, out);
                collect_variable_length_ranges(cond, sampled, out);
            }
        }
        Node::Function(_, args) => {
            for a in args {
                collect_variable_length_ranges(a, sampled, out);
            }
        }
        Node::Summation(_, start, end, body) | Node::Product(_, start, end, body) => {
            let bound_vars = free_variables(&[start, end]);
            if !bound_vars.is_empty() && bound_vars.iter().all(|v| sampled.contains(v)) {
                let length_expr = Node::Add(
                    Box::new(Node::Subtract(end.clone(), start.clone())),
                    Box::new(Node::Num(ExactNum::integer(1))),
                );
                let structurally_constant =
                    matches!(length_expr.simplify(&Environment::new()), Ok(Node::Num(_)));
                let already = out
                    .iter()
                    .any(|(s, e)| s == start.as_ref() && e == end.as_ref());
                if !structurally_constant && !already {
                    out.push((start.as_ref().clone(), end.as_ref().clone()));
                }
            }
            collect_variable_length_ranges(start, sampled, out);
            collect_variable_length_ranges(end, sampled, out);
            collect_variable_length_ranges(body, sampled, out);
        }
    }
}

/// Σ/Π ranges whose start and end are both plain, distinct variables —
/// the pairs whose length coverage `verify_identity` must guarantee.
fn collect_symbolic_bound_pairs(node: &Node, out: &mut Vec<(String, String)>) {
    match node {
        Node::Variable(_) | Node::Num(_) => {}
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
            collect_symbolic_bound_pairs(l, out);
            collect_symbolic_bound_pairs(r, out);
        }
        Node::Sqrt(inner)
        | Node::Abs(inner)
        | Node::Floor(inner)
        | Node::Ceil(inner)
        | Node::Round(inner)
        | Node::Trunc(inner)
        | Node::Negate(inner)
        | Node::Factorial(inner) => collect_symbolic_bound_pairs(inner, out),
        Node::Piecewise(arms) => {
            for (expr, cond) in arms {
                collect_symbolic_bound_pairs(expr, out);
                collect_symbolic_bound_pairs(cond, out);
            }
        }
        Node::Function(_, args) => {
            for a in args {
                collect_symbolic_bound_pairs(a, out);
            }
        }
        Node::Summation(_, start, end, body) | Node::Product(_, start, end, body) => {
            if let (Node::Variable(lo), Node::Variable(hi)) = (start.as_ref(), end.as_ref()) {
                if lo != hi && !out.iter().any(|(a, b)| a == lo && b == hi) {
                    out.push((lo.clone(), hi.clone()));
                }
            }
            collect_symbolic_bound_pairs(start, out);
            collect_symbolic_bound_pairs(end, out);
            collect_symbolic_bound_pairs(body, out);
        }
    }
}

fn collect_range_bound_constraints(node: &Node, out: &mut HashMap<String, RangeBoundConstraint>) {
    match node {
        Node::Variable(_) | Node::Num(_) => {}
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
            collect_range_bound_constraints(l, out);
            collect_range_bound_constraints(r, out);
        }
        Node::Sqrt(inner)
        | Node::Abs(inner)
        | Node::Floor(inner)
        | Node::Ceil(inner)
        | Node::Round(inner)
        | Node::Trunc(inner)
        | Node::Negate(inner)
        | Node::Factorial(inner) => collect_range_bound_constraints(inner, out),
        Node::Piecewise(arms) => {
            for (expr, cond) in arms {
                collect_range_bound_constraints(expr, out);
                collect_range_bound_constraints(cond, out);
            }
        }
        Node::Function(_, args) => {
            for a in args {
                collect_range_bound_constraints(a, out);
            }
        }
        Node::Summation(_, start, end, body) | Node::Product(_, start, end, body) => {
            let constant_of = |bound: &Node| Evaluator::evaluate(bound, &Environment::new()).ok();
            // Variables in the upper bound are bounded below by a constant
            // lower bound (and vice versa). Multiple sums over the same
            // variable merge to the tightest constraint.
            let lo = constant_of(start);
            for v in free_variables(&[end]) {
                let entry = out.entry(v).or_default();
                if let Some(lo) = lo {
                    let lo = lo.ceil();
                    entry.min = Some(entry.min.map_or(lo, |m: f64| m.max(lo)));
                }
            }
            let hi = constant_of(end);
            for v in free_variables(&[start]) {
                let entry = out.entry(v).or_default();
                if let Some(hi) = hi {
                    let hi = hi.floor();
                    entry.max = Some(entry.max.map_or(hi, |m: f64| m.min(hi)));
                }
            }
            collect_range_bound_constraints(start, out);
            collect_range_bound_constraints(end, out);
            collect_range_bound_constraints(body, out);
        }
    }
}

pub(crate) fn values_match(a: f64, b: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    if a.is_infinite() && b.is_infinite() {
        return a.signum() == b.signum();
    }
    let diff = (a - b).abs();
    let scale = a.abs().max(b.abs()).max(1.0);
    diff / scale < TOLERANCE
}

impl std::fmt::Display for VerifyResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.insufficient_points {
            let hint = if self.starved_range_lengths.is_none() {
                "; check that variable names match the expressions"
            } else {
                ""
            };
            return write!(
                f,
                "Verified: INCONCLUSIVE ({}{})",
                self.insufficiency_reason(),
                hint
            );
        }
        if self.passed {
            write!(
                f,
                "Verified: PASS (tested at {} points)",
                self.points_tested
            )
        } else if let Some(ref cx) = self.counterexample {
            let point_str: Vec<String> = cx
                .point
                .iter()
                .map(|(var, val)| format!("{}={}", var, val))
                .collect();
            write!(
                f,
                "Verified: FAIL at {}: LHS={}, RHS={}",
                point_str.join(", "),
                cx.lhs_value,
                cx.rhs_value
            )
        } else {
            write!(f, "Verified: FAIL")
        }
    }
}
