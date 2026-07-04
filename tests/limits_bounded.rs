// Bounded-oscillation limits (Carl's C1, Session 43).
// lim x·sin(1/x) at 0 and lim sin(x) at ∞ previously HUNG (unbounded
// series-by-differentiation of sin(1/x)); after budgeting, the fallback
// cascade answered +∞ for lim sin(x) — a wrong value for a limit that
// does not exist. Fixed by squeeze and continuity rules for the bounded
// atoms sin/cos, plus a derivative-growth budget in Taylor expansion.

use arithma::limits::limit_latex_str;

#[test]
fn squeeze_x_sin_one_over_x_at_zero() {
    assert_eq!(
        limit_latex_str("x \\cdot \\sin(\\frac{1}{x})", "x", "0").unwrap(),
        "0"
    );
}

#[test]
fn squeeze_x_squared_cos_one_over_x_at_zero() {
    assert_eq!(
        limit_latex_str("x^2 \\cdot \\cos(\\frac{1}{x})", "x", "0").unwrap(),
        "0"
    );
}

#[test]
fn sin_at_infinity_does_not_exist() {
    let err = limit_latex_str("\\sin(x)", "x", "inf").unwrap_err();
    assert!(err.contains("does not exist"), "got: {}", err);
}

#[test]
fn cos_at_infinity_does_not_exist() {
    let err = limit_latex_str("\\cos(x)", "x", "inf").unwrap_err();
    assert!(err.contains("does not exist"), "got: {}", err);
}

#[test]
fn sin_of_vanishing_argument_at_infinity_is_zero() {
    // Continuity: sin(1/x) → sin(0) = 0. The old cascade said +∞.
    assert_eq!(
        limit_latex_str("\\sin(\\frac{1}{x})", "x", "inf").unwrap(),
        "0"
    );
}

#[test]
fn cos_of_vanishing_argument_at_infinity_is_one() {
    assert_eq!(
        limit_latex_str("\\cos(\\frac{1}{x})", "x", "inf").unwrap(),
        "1"
    );
}

#[test]
fn sin_x_over_x_still_works_both_directions() {
    assert_eq!(
        limit_latex_str("\\frac{\\sin(x)}{x}", "x", "0").unwrap(),
        "1"
    );
    assert_eq!(
        limit_latex_str("\\frac{\\sin(x)}{x}", "x", "inf").unwrap(),
        "0"
    );
}
