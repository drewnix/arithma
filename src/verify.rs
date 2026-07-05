use crate::assumptions::Assumptions;
use crate::environment::Environment;
use crate::evaluator::Evaluator;
use crate::node::Node;
use crate::tokenizer::normalize_var;

const TEST_POINTS: &[f64] = &[
    0.5, -0.5, 1.5, -1.5, 0.3, -0.7, 2.1, 0.1, -2.3, 3.0, 0.8, 4.5,
];

const TOLERANCE: f64 = 1e-8;
const MIN_POINTS_FOR_PASS: usize = 3;

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
    let mut domain_mismatches = 0;

    for (i, &base_point) in TEST_POINTS.iter().enumerate() {
        let mut env = Environment::new();
        let mut point_values = Vec::new();
        let mut skip_point = false;

        for (j, var) in normalized.iter().enumerate() {
            let val = base_point + 0.3 * j as f64 + 0.1 * i as f64;
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

        // NaN means the point tests domain membership, not values (Carl
        // R5c): shared undefinedness is not numeric agreement (skip,
        // uncounted); one-sided undefinedness witnesses a DOMAIN
        // difference — excluded from the numeric evidence but counted, so
        // callers can caveat it instead of hiding it.
        if lhs_val.is_nan() && rhs_val.is_nan() {
            continue;
        }
        if lhs_val.is_nan() || rhs_val.is_nan() {
            domain_mismatches += 1;
            continue;
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
