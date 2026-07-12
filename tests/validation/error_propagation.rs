// Tests for first-order floating-point error propagation
// (arithma::error_eval). The tracked bound answers one question: how many
// digits of the computed f64 value are trustworthy? Its two consumers are
// the `approximate` result tier (evaluate tool) and the constant-comparison
// path of verify_chain, where it replaces a fixed tolerance.
//
// The bound is a first-order upper estimate: it may OVER-report error
// (harmless — we under-claim digits) but must never meaningfully
// under-report it on the constructs it models. Constructs without a sound
// model return Err rather than an optimistic bound.

use arithma::error_eval::{evaluate_with_error, significant_digits};
use arithma::{parse_latex_raw, Environment};

fn eval(latex: &str, var: Option<(&str, f64)>) -> (f64, f64) {
    let node = parse_latex_raw(latex).expect("parse");
    let mut env = Environment::new();
    if let Some((name, value)) = var {
        env.set(name, value);
    }
    evaluate_with_error(&node, &env).expect("evaluate_with_error")
}

#[test]
fn catastrophic_cancellation_has_zero_significant_digits() {
    // The motivating case: (1 - cos x)/x² at x = 1e-8. True value ~0.5;
    // f64 computes 0 because 1 - cos(1e-8) annihilates every significant
    // digit. The subtraction rule must amplify the bound past the value.
    let (value, bound) = eval("\\frac{1 - \\cos(x)}{x^2}", Some(("x", 1e-8)));
    assert_eq!(
        significant_digits(value, bound),
        0,
        "value={} bound={} — a value with no digits must claim no digits",
        value,
        bound
    );
}

#[test]
fn well_conditioned_evaluation_keeps_its_digits() {
    // Same expression at x = 0.5 is perfectly well-conditioned:
    // (1 - cos 0.5)/0.25 ≈ 0.489669752. Nearly all digits survive.
    let (value, bound) = eval("\\frac{1 - \\cos(x)}{x^2}", Some(("x", 0.5)));
    assert!(
        (value - 0.489_669_752_930_2).abs() < 1e-9,
        "value: {}",
        value
    );
    assert!(
        significant_digits(value, bound) >= 10,
        "value={} bound={} digits={}",
        value,
        bound,
        significant_digits(value, bound)
    );
}

#[test]
fn e_to_minus_50_is_distinguishable_from_zero() {
    // e^{-50} ≈ 1.93e-22 is TINY but well-conditioned (κ = |x| = 50):
    // the bound must stay orders of magnitude below the value, so a
    // comparison against 0 sees a real disagreement, not noise.
    let (value, bound) = eval("e^{-50}", None);
    assert!(value > 0.0, "value: {}", value);
    assert!(
        significant_digits(value, bound) >= 10,
        "value={} bound={} digits={}",
        value,
        bound,
        significant_digits(value, bound)
    );
}

#[test]
fn sin_two_pi_is_indistinguishable_from_zero() {
    // sin(2π) computes ~2.4e-16, but π's own representation error passes
    // through sin with derivative ~1, so the bound (~1e-15) EXCEEDS the
    // value: the result is indistinguishable from 0, and a claim of 0
    // must be accepted rather than refuted.
    let (value, bound) = eval("\\sin(2 \\cdot \\pi)", None);
    assert!(
        bound >= value.abs(),
        "value={} bound={} — sin(2π) must not pretend to nonzero digits",
        value,
        bound
    );
    assert_eq!(significant_digits(value, bound), 0);
}

#[test]
fn integer_arithmetic_carries_full_precision() {
    let (value, bound) = eval("2 + 3", None);
    assert_eq!(value, 5.0);
    assert!(significant_digits(value, bound) >= 15, "bound: {}", bound);
}

#[test]
fn summation_accumulates_error_through_the_fold() {
    let (value, bound) = eval("\\sum_{k=1}^{10} k", None);
    assert_eq!(value, 55.0);
    assert!(significant_digits(value, bound) >= 14, "bound: {}", bound);
}

#[test]
fn undefined_variable_is_an_error() {
    let node = parse_latex_raw("x + 1").expect("parse");
    assert!(evaluate_with_error(&node, &Environment::new()).is_err());
}

#[test]
fn significant_digits_edge_cases() {
    // Zero bound on a nonzero value: full f64 precision, capped at 16.
    assert_eq!(significant_digits(1.0, 0.0), 16);
    // Bound at or above the value: no digits.
    assert_eq!(significant_digits(1.0, 1.0), 0);
    assert_eq!(significant_digits(0.0, 1e-300), 0);
    // Exact zero with zero bound: the value is exactly 0 — full precision.
    assert_eq!(significant_digits(0.0, 0.0), 16);
    // One part in 10^6.
    assert_eq!(significant_digits(1.0, 1e-6), 6);
}
