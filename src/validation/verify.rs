use crate::assumptions::Assumptions;
use crate::environment::Environment;
use crate::evaluator::Evaluator;
use crate::node::Node;
use crate::status::free_variables;
use crate::tokenizer::normalize_var;
use std::collections::HashMap;

const TEST_POINTS: &[f64] = &[
    0.5, -0.5, 1.5, -1.5, 0.3, -0.7, 2.1, 0.1, -2.3, 3.0, 0.8, 4.5,
];

const TOLERANCE: f64 = 1e-8;
pub(crate) const MIN_POINTS_FOR_PASS: usize = 3;

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

    for (i, &base_point) in TEST_POINTS.iter().enumerate() {
        let mut env = Environment::new();
        let mut point_values = Vec::new();
        let mut skip_point = false;

        for (j, var) in normalized.iter().enumerate() {
            let val = match range_constraints.get(var) {
                Some(c) => match (c.min, c.max) {
                    // Anchored integer streams: lo, lo+1, … (or hi, hi−1, …),
                    // offset by variable position so multi-variable points
                    // stay distinct.
                    (Some(lo), _) => lo + (i + j) as f64,
                    (None, Some(hi)) => hi - (i + j) as f64,
                    (None, None) => (base_point + 0.3 * j as f64 + 0.1 * i as f64).round(),
                },
                None => base_point + 0.3 * j as f64 + 0.1 * i as f64,
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
            };
        }

        points_tested += 1;

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
            };
        }
    }

    let insufficient = points_tested < MIN_POINTS_FOR_PASS;
    VerifyResult {
        passed: !insufficient,
        points_tested,
        counterexample: None,
        insufficient_points: insufficient,
        domain_mismatches,
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
            return write!(
                f,
                "Verified: INCONCLUSIVE (only {} point{} tested — need at least {}; check that variable names match the expressions)",
                self.points_tested,
                if self.points_tested == 1 { "" } else { "s" },
                MIN_POINTS_FOR_PASS
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
