//! Literal simplification rules used by [`crate::simplify`]: transcendental functions
//! at special arguments and canonical π-multiple display.
//!
//! # Supported exact families
//!
//! - **Circular trig** (`sin`, `cos`, `tan` + reciprocals): arguments `k·π` where the
//!   reduced reference angle in `[0, π/2]` is constructible — denominators
//!   `{1,3,4,5,6,8,10,12}` (e.g. `π/6`, `π/5`, `π/12`). All four quadrants via
//!   `reduce_sin_to_principal`. Non-constructible angles (e.g. `π/7`) stay symbolic.
//! - **Inverse circular** (`arcsin`, `arccos`, `arctan` + reciprocals): selected
//!   rationals and surds (`±1/2`, `±√2/2`, `±√3/2`, `±1/√3`, `±√3`, `±√2`, …).
//! - **Hyperbolic** (`sinh`, `cosh`, `tanh` + reciprocals): `±ln(a)` for integer
//!   `a > 1`; selected inverse values (`arcsinh(±1)`, `arccosh(2|3)`, `arctanh(±1/2|±1/3|±1/√3)`, …).
//! - **Log / exp**: `log(10^n)`, `lg(2^n)` for arbitrary positive integer `n`
//!   ([`integer_log_power`]); `ln(e)`, `exp(ln x)` (since `exp` = e^x),
//!   `exp(k·ln a)` for integer `a > 1` with `|k| ≤ i32::MAX` (see TODO on
//!   [`rational_int_pow`]). Prime factorization of `log`/`lg`/`ln`
//!   arguments is handled in [`crate::simplify`] (`factor_log_integer`; see TODO there
//!   for BigInt factorization beyond `u64::MAX`).
//!
//! # Integration with simplify
//!
//! [`try_exact_function_value`] is invoked **before** parity rules and algebraic
//! log laws in `simplify`, so literal angles like `sin(-π/6)` fold to `-1/2` directly.

use crate::exact::ExactNum;
use crate::function_meta::canonical_function_name;
use crate::node::Node;
use num_bigint::BigInt;
use num_integer::Integer;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};

fn pi_node() -> Node {
    Node::Variable("π".to_string())
}

fn as_pi_multiple(node: &Node) -> Option<BigRational> {
    match node {
        Node::Variable(v) if v == "π" => Some(BigRational::one()),
        Node::Negate(inner) => as_pi_multiple(inner).map(|r| -r),
        Node::Divide(numer, denom) => {
            if let Node::Num(ExactNum::Rational(d)) = denom.as_ref() {
                if let Some(n_coeff) = as_pi_multiple(numer) {
                    return Some(n_coeff / d);
                }
            }
            None
        }
        Node::Multiply(left, right) => {
            if let Node::Num(ExactNum::Rational(n)) = left.as_ref() {
                if let Node::Variable(v) = right.as_ref() {
                    if v == "π" {
                        return Some(n.clone());
                    }
                }
            }
            if let Node::Num(ExactNum::Rational(n)) = right.as_ref() {
                if let Node::Variable(v) = left.as_ref() {
                    if v == "π" {
                        return Some(n.clone());
                    }
                }
            }
            if let Node::Num(n) = left.as_ref() {
                if let Some(c) = n.to_rational() {
                    if let Some(k) = as_pi_multiple(right) {
                        return Some(c * k);
                    }
                }
            }
            if let Node::Num(n) = right.as_ref() {
                if let Some(c) = n.to_rational() {
                    if let Some(k) = as_pi_multiple(left) {
                        return Some(c * k);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

/// Canonical display for multiples of π: `2π/8` → `π/4`, `1/4·π` → `π/4`.
fn pi_rational_to_node(k: &BigRational) -> Node {
    if k.is_zero() {
        return Node::Num(ExactNum::zero());
    }
    let neg = k.is_negative();
    let k = k.abs();
    let numer = k.numer();
    let denom = k.denom();

    let pi_part = if numer.is_one() {
        pi_node()
    } else {
        Node::Multiply(
            Box::new(Node::Num(ExactNum::Rational(BigRational::from_integer(
                numer.clone(),
            )))),
            Box::new(pi_node()),
        )
    };

    let result = if denom.is_one() {
        pi_part
    } else {
        Node::Divide(
            Box::new(pi_part),
            Box::new(Node::Num(ExactNum::Rational(BigRational::from_integer(
                denom.clone(),
            )))),
        )
    };

    if neg {
        Node::Negate(Box::new(result))
    } else {
        result
    }
}

pub fn try_normalize_pi_multiple(node: &Node) -> Option<Node> {
    let k = as_pi_multiple(node)?;
    Some(pi_rational_to_node(&k))
}

/// If `n = base^exp` for integer `exp ≥ 0`, return `exp`.
fn integer_log_power(n: &BigInt, base: i64) -> Option<BigInt> {
    let base = BigInt::from(base);
    if n <= &BigInt::zero() || base <= BigInt::one() {
        return None;
    }
    let mut v = n.clone();
    let mut exp = BigInt::zero();
    while v > BigInt::one() {
        let (q, r) = v.div_mod_floor(&base);
        if !r.is_zero() {
            return None;
        }
        v = q;
        exp += 1;
    }
    Some(exp)
}

fn bigint_to_node(n: BigInt) -> Node {
    Node::Num(ExactNum::Rational(BigRational::from_integer(n)))
}

fn rational_int_pow(base: &BigRational, exp: &BigInt) -> Option<BigRational> {
    if !base.is_integer() {
        return None;
    }
    // TODO: BigInt exponent when |exp| > i32::MAX (exp(k·ln a) stays symbolic until then).
    if exp.is_negative() {
        let pos = (-exp.clone()).to_i32()?;
        return Some(BigRational::one() / base.pow(pos));
    }
    Some(base.pow(exp.to_i32()?))
}

pub fn try_exact_function_value(name: &str, args: &[Node]) -> Option<Node> {
    if args.len() != 1 {
        return None;
    }
    let name = canonical_function_name(name);
    let arg = &args[0];

    // Exact values at 0 for transcendental functions (kept symbolic otherwise).
    if let Node::Num(n) = arg {
        if n.to_rational().is_some_and(|r| r.is_zero()) {
            match name {
                // Circular / inverse circular / hyperbolic / inverse hyperbolic → 0
                "sin" | "tan" | "arcsin" | "arctan" | "sinh" | "tanh" | "arcsinh" | "arctanh" => {
                    return Some(Node::Num(ExactNum::integer(0)));
                }
                // cos, cosh, sec, sech, exp → 1
                "cos" | "cosh" | "sec" | "sech" | "exp" => {
                    return Some(Node::Num(ExactNum::integer(1)));
                }
                _ => {}
            }
        }
    }

    match name {
        // --- Circular trigonometric ---
        "sin" => try_exact_sin(arg),
        "cos" => try_exact_cos(arg),
        "tan" => try_exact_tan(arg),
        // --- Reciprocal trigonometric ---
        "csc" => reciprocal_exact("sin", arg),
        "sec" => reciprocal_exact("cos", arg),
        "cot" => reciprocal_exact("tan", arg),
        // --- Inverse circular trigonometric ---
        "arcsin" => try_exact_arcsin(arg),
        "arccos" => try_exact_arccos(arg),
        "arctan" => try_exact_arctan(arg),
        // --- Inverse reciprocal trigonometric ---
        "arccsc" => try_exact_arccsc(arg),
        "arcsec" => try_exact_arcsec(arg),
        "arccot" => try_exact_arccot(arg),
        // --- Hyperbolic ---
        "sinh" => try_exact_sinh(arg),
        "cosh" => try_exact_cosh(arg),
        "tanh" => try_exact_tanh(arg),
        // --- Reciprocal hyperbolic ---
        "csch" => reciprocal_exact("sinh", arg),
        "sech" => reciprocal_exact("cosh", arg),
        "coth" => reciprocal_exact("tanh", arg),
        // --- Inverse hyperbolic ---
        "arcsinh" => try_exact_arcsinh(arg),
        "arccosh" => try_exact_arccosh(arg),
        "arctanh" => try_exact_arctanh(arg),
        // --- Inverse reciprocal hyperbolic ---
        "arccsch" => inverse_reciprocal_exact("arcsinh", arg),
        "arcsech" => inverse_reciprocal_exact("arccosh", arg),
        "arccoth" => inverse_reciprocal_exact("arctanh", arg),
        // --- Logarithmic and exponential ---
        "exp" => try_exact_exp(arg),
        "log" | "ln" | "lg" => {
            if let Node::Num(n) = arg {
                if n.is_one() {
                    return Some(Node::Num(ExactNum::integer(0)));
                }
                if let Some(r) = n.to_rational() {
                    if r.is_integer() && r.is_positive() {
                        let v = r.numer();
                        match name {
                            "log" => {
                                if let Some(exp) = integer_log_power(v, 10) {
                                    return Some(bigint_to_node(exp));
                                }
                            }
                            "lg" => {
                                if let Some(exp) = integer_log_power(v, 2) {
                                    return Some(bigint_to_node(exp));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            if name == "ln" {
                if let Node::Variable(v) = arg {
                    if v == "e" {
                        return Some(Node::Num(ExactNum::integer(1)));
                    }
                }
            }
            None
        }
        _ => None,
    }
}

fn is_zero_node(node: &Node) -> bool {
    match node {
        Node::Num(n) => n.is_zero(),
        Node::Negate(inner) => is_zero_node(inner),
        _ => false,
    }
}

fn inverse_reciprocal_exact(base: &str, arg: &Node) -> Option<Node> {
    if is_zero_node(arg) {
        return None;
    }
    let reciprocal = Node::Divide(
        Box::new(Node::Num(ExactNum::integer(1))),
        Box::new(arg.clone()),
    );
    try_exact_function_value(base, std::slice::from_ref(&reciprocal))
}

fn as_num(node: &Node) -> Option<ExactNum> {
    match node {
        Node::Num(n) => Some(n.clone()),
        _ => None,
    }
}

/// Reciprocal of an exact numeric node (`Num`, rational `Divide`, or leading `Negate`).
fn reciprocal_of_node(node: &Node) -> Option<Node> {
    match node {
        Node::Num(n) => Some(Node::Num(ExactNum::one() / n.clone())),
        Node::Negate(inner) => {
            let rec = reciprocal_of_node(inner)?;
            match rec {
                Node::Num(n) => Some(Node::Num(-n)),
                other => Some(Node::Negate(Box::new(other))),
            }
        }
        Node::Divide(numer, denom) => {
            let na = as_num(numer)?;
            let nb = as_num(denom)?;
            Some(Node::Num(nb / na))
        }
        _ => None,
    }
}

fn reciprocal_exact(base: &str, arg: &Node) -> Option<Node> {
    let inner = try_exact_function_value(base, std::slice::from_ref(arg))?;
    if is_zero_node(&inner) {
        return None;
    }
    if let Node::Num(n) = &inner {
        if n.is_one() {
            return Some(Node::Num(ExactNum::integer(1)));
        }
        if n.is_negative() && (-n.clone()).is_one() {
            return Some(Node::Num(ExactNum::integer(-1)));
        }
        return Some(Node::Num(ExactNum::one() / n.clone()));
    }
    if let Some(rec) = reciprocal_of_node(&inner) {
        return Some(rec);
    }
    Some(Node::Divide(
        Box::new(Node::Num(ExactNum::integer(1))),
        Box::new(inner),
    ))
}

fn as_exact_rational(node: &Node) -> Option<BigRational> {
    match node {
        Node::Num(n) => n.to_rational(),
        Node::Negate(inner) => as_exact_rational(inner).map(|r| -r),
        Node::Divide(numer, denom) => Some(as_exact_rational(numer)? / as_exact_rational(denom)?),
        _ => None,
    }
}

fn exact_num(r: BigRational) -> Node {
    Node::Num(ExactNum::Rational(r))
}

fn ln_of(arg: Node) -> Node {
    Node::Function("ln".to_string(), vec![arg])
}

/// `(1/2)·ln(ratio)` for `ratio > 0`.
fn half_ln(ratio: BigRational) -> Node {
    Node::Multiply(
        Box::new(exact_num(BigRational::new(
            BigInt::from(1),
            BigInt::from(2),
        ))),
        Box::new(ln_of(exact_num(ratio))),
    )
}

/// If `arg = ±ln(a)` with integer `a > 1`, return `(a, ln_is_negated)`.
fn as_ln_positive_integer(arg: &Node) -> Option<(BigRational, bool)> {
    match arg {
        Node::Function(name, args) if name == "ln" && args.len() == 1 => {
            let a = as_exact_rational(&args[0])?;
            if a.is_integer() && a > BigRational::one() {
                Some((a, false))
            } else {
                None
            }
        }
        Node::Negate(inner) => {
            let (a, _) = as_ln_positive_integer(inner)?;
            Some((a, true))
        }
        _ => None,
    }
}

fn hyperbolic_at_ln(a: &BigRational, neg_ln: bool, kind: HyperbolicLnKind) -> Option<Node> {
    let two = BigRational::from_integer(BigInt::from(2));
    let a2 = a * a;
    let value = match kind {
        HyperbolicLnKind::Sinh => (a2.clone() - BigRational::one()) / (two.clone() * a),
        HyperbolicLnKind::Cosh => (a2.clone() + BigRational::one()) / (two.clone() * a),
        HyperbolicLnKind::Tanh => (a2.clone() - BigRational::one()) / (a2 + BigRational::one()),
    };
    let mut node = exact_num(value);
    if neg_ln && matches!(kind, HyperbolicLnKind::Sinh | HyperbolicLnKind::Tanh) {
        node = Node::Negate(Box::new(node));
    }
    Some(node)
}

enum HyperbolicLnKind {
    Sinh,
    Cosh,
    Tanh,
}

fn exact_half() -> Node {
    Node::Divide(
        Box::new(Node::Num(ExactNum::integer(1))),
        Box::new(Node::Num(ExactNum::integer(2))),
    )
}

fn exact_sqrt2_over_2() -> Node {
    Node::Divide(
        Box::new(Node::Sqrt(Box::new(Node::Num(ExactNum::integer(2))))),
        Box::new(Node::Num(ExactNum::integer(2))),
    )
}

fn exact_sqrt3_over_2() -> Node {
    Node::Divide(
        Box::new(Node::Sqrt(Box::new(Node::Num(ExactNum::integer(3))))),
        Box::new(Node::Num(ExactNum::integer(2))),
    )
}

fn exact_sqrt6_plus_sqrt2_over_4() -> Node {
    Node::Divide(
        Box::new(Node::Add(
            Box::new(Node::Sqrt(Box::new(Node::Num(ExactNum::integer(6))))),
            Box::new(Node::Sqrt(Box::new(Node::Num(ExactNum::integer(2))))),
        )),
        Box::new(Node::Num(ExactNum::integer(4))),
    )
}

fn exact_sqrt6_minus_sqrt2_over_4() -> Node {
    Node::Divide(
        Box::new(Node::Subtract(
            Box::new(Node::Sqrt(Box::new(Node::Num(ExactNum::integer(6))))),
            Box::new(Node::Sqrt(Box::new(Node::Num(ExactNum::integer(2))))),
        )),
        Box::new(Node::Num(ExactNum::integer(4))),
    )
}

fn exact_sin_pi_8() -> Node {
    Node::Divide(
        Box::new(Node::Sqrt(Box::new(Node::Subtract(
            Box::new(Node::Num(ExactNum::integer(2))),
            Box::new(Node::Sqrt(Box::new(Node::Num(ExactNum::integer(2))))),
        )))),
        Box::new(Node::Num(ExactNum::integer(2))),
    )
}

fn exact_cos_pi_8() -> Node {
    Node::Divide(
        Box::new(Node::Sqrt(Box::new(Node::Add(
            Box::new(Node::Num(ExactNum::integer(2))),
            Box::new(Node::Sqrt(Box::new(Node::Num(ExactNum::integer(2))))),
        )))),
        Box::new(Node::Num(ExactNum::integer(2))),
    )
}

fn exact_sqrt5() -> Node {
    Node::Sqrt(Box::new(Node::Num(ExactNum::integer(5))))
}

fn exact_sin_pi_10() -> Node {
    // sin(π/10) = (√5 − 1) / 4
    Node::Divide(
        Box::new(Node::Subtract(
            Box::new(exact_sqrt5()),
            Box::new(Node::Num(ExactNum::integer(1))),
        )),
        Box::new(Node::Num(ExactNum::integer(4))),
    )
}

fn exact_sin_3pi_10() -> Node {
    // sin(3π/10) = (√5 + 1) / 4  (= cos(π/5))
    Node::Divide(
        Box::new(Node::Add(
            Box::new(exact_sqrt5()),
            Box::new(Node::Num(ExactNum::integer(1))),
        )),
        Box::new(Node::Num(ExactNum::integer(4))),
    )
}

fn exact_sin_pi_5() -> Node {
    // sin(π/5) = √(10 − 2√5) / 4
    Node::Divide(
        Box::new(Node::Sqrt(Box::new(Node::Subtract(
            Box::new(Node::Num(ExactNum::integer(10))),
            Box::new(Node::Multiply(
                Box::new(Node::Num(ExactNum::integer(2))),
                Box::new(exact_sqrt5()),
            )),
        )))),
        Box::new(Node::Num(ExactNum::integer(4))),
    )
}

fn exact_sin_2pi_5() -> Node {
    // sin(2π/5) = √(10 + 2√5) / 4
    Node::Divide(
        Box::new(Node::Sqrt(Box::new(Node::Add(
            Box::new(Node::Num(ExactNum::integer(10))),
            Box::new(Node::Multiply(
                Box::new(Node::Num(ExactNum::integer(2))),
                Box::new(exact_sqrt5()),
            )),
        )))),
        Box::new(Node::Num(ExactNum::integer(4))),
    )
}

fn exact_tan_pi_5() -> Node {
    // tan(π/5) = √(5 − 2√5)
    Node::Sqrt(Box::new(Node::Subtract(
        Box::new(Node::Num(ExactNum::integer(5))),
        Box::new(Node::Multiply(
            Box::new(Node::Num(ExactNum::integer(2))),
            Box::new(exact_sqrt5()),
        )),
    )))
}

fn exact_one_over_sqrt3() -> Node {
    Node::Divide(
        Box::new(Node::Num(ExactNum::integer(1))),
        Box::new(Node::Sqrt(Box::new(Node::Num(ExactNum::integer(3))))),
    )
}

fn exact_tan_pi_12() -> Node {
    // tan(π/12) = 2 − √3
    Node::Subtract(
        Box::new(Node::Num(ExactNum::integer(2))),
        Box::new(Node::Sqrt(Box::new(Node::Num(ExactNum::integer(3))))),
    )
}

fn exact_tan_5pi_12() -> Node {
    // tan(5π/12) = 2 + √3
    Node::Add(
        Box::new(Node::Num(ExactNum::integer(2))),
        Box::new(Node::Sqrt(Box::new(Node::Num(ExactNum::integer(3))))),
    )
}

fn exact_cos_pi_10() -> Node {
    // cos(π/10) = sin(2π/5)
    exact_sin_2pi_5()
}

fn exact_tan_pi_10() -> Node {
    Node::Divide(Box::new(exact_sin_pi_10()), Box::new(exact_cos_pi_10()))
}

fn exact_tan_2pi_5() -> Node {
    Node::Divide(Box::new(exact_sin_2pi_5()), Box::new(exact_sin_pi_10()))
}

fn exact_tan_3pi_10() -> Node {
    Node::Divide(Box::new(exact_sin_3pi_10()), Box::new(exact_sin_pi_5()))
}

fn pi_over(n: i64) -> Node {
    Node::Divide(
        Box::new(pi_node()),
        Box::new(Node::Num(ExactNum::integer(n))),
    )
}

fn neg_pi_over(n: i64) -> Node {
    Node::Negate(Box::new(pi_over(n)))
}

fn k_pi_over(k: i64, n: i64) -> Node {
    Node::Divide(
        Box::new(Node::Multiply(
            Box::new(Node::Num(ExactNum::integer(k))),
            Box::new(pi_node()),
        )),
        Box::new(Node::Num(ExactNum::integer(n))),
    )
}

fn is_num_integer(node: &Node, value: i64) -> bool {
    matches!(node, Node::Num(n) if n.to_rational() == Some(BigRational::from_integer(BigInt::from(value))))
}

fn is_sqrt_of(node: &Node, radicand: i64) -> bool {
    matches!(node, Node::Sqrt(inner) if is_num_integer(inner, radicand))
}

/// Recognize standard surd forms for inverse trig (±√n, ±√n/d, ±1/√n).
enum StandardSurd {
    Sqrt2,
    Sqrt3,
    Sqrt2Over2,
    Sqrt3Over2,
    InvSqrt2,
    InvSqrt3,
}

fn as_signed_standard_surd(node: &Node) -> Option<(bool, StandardSurd)> {
    let (neg, inner) = match node {
        Node::Negate(n) => (true, n.as_ref()),
        other => (false, other),
    };
    let surd = if is_sqrt_of(inner, 2) {
        StandardSurd::Sqrt2
    } else if is_sqrt_of(inner, 3) {
        StandardSurd::Sqrt3
    } else if let Node::Divide(numer, denom) = inner {
        if is_sqrt_of(numer, 2) && is_num_integer(denom, 2) {
            StandardSurd::Sqrt2Over2
        } else if is_sqrt_of(numer, 3) && is_num_integer(denom, 2) {
            StandardSurd::Sqrt3Over2
        } else if is_num_integer(numer, 1) && is_sqrt_of(denom, 2) {
            StandardSurd::InvSqrt2
        } else if (is_num_integer(numer, 1) && is_sqrt_of(denom, 3))
            || (is_sqrt_of(numer, 3) && is_num_integer(denom, 3))
        {
            StandardSurd::InvSqrt3
        } else {
            return None;
        }
    } else {
        return None;
    };
    Some((neg, surd))
}

/// Map `(inverse fn, sign, surd)` to an exact π-angle result.
fn inverse_trig_from_surd(name: &str, neg: bool, surd: StandardSurd) -> Option<Node> {
    use StandardSurd::*;
    match (name, surd) {
        ("arcsin", Sqrt2Over2) | ("arcsin", InvSqrt2) => {
            Some(if neg { neg_pi_over(4) } else { pi_over(4) })
        }
        ("arcsin", Sqrt3Over2) => Some(if neg { neg_pi_over(3) } else { pi_over(3) }),
        ("arccos", Sqrt2Over2) | ("arccos", InvSqrt2) => {
            Some(if neg { k_pi_over(3, 4) } else { pi_over(4) })
        }
        ("arccos", Sqrt3Over2) => Some(if neg { k_pi_over(5, 6) } else { pi_over(6) }),
        ("arctan", InvSqrt3) => Some(if neg { neg_pi_over(6) } else { pi_over(6) }),
        ("arctan", Sqrt3) => Some(if neg { neg_pi_over(3) } else { pi_over(3) }),
        ("arccsc", Sqrt2) => Some(if neg { neg_pi_over(4) } else { pi_over(4) }),
        ("arcsec", Sqrt2) => Some(if neg { k_pi_over(3, 4) } else { pi_over(4) }),
        ("arccot", Sqrt3) => Some(if neg { neg_pi_over(6) } else { pi_over(6) }),
        ("arccot", InvSqrt3) => Some(if neg { neg_pi_over(3) } else { pi_over(3) }),
        _ => None,
    }
}

fn try_exact_inverse_surd(name: &str, arg: &Node) -> Option<Node> {
    let (neg, surd) = as_signed_standard_surd(arg)?;
    inverse_trig_from_surd(name, neg, surd)
}

/// True when `reference = num/den` in lowest terms (compared via `BigRational`).
fn ref_is(reference: &BigRational, num: i64, den: i64) -> bool {
    reference == &BigRational::new(BigInt::from(num), BigInt::from(den))
}

/// Exact value of `sin(reference·π)` for a reference angle in `[0, 1/2]`.
/// (`cos` reuses this via `cos(θ) = sin(π/2 − θ)`.)
fn principal_sin_value(reference: &BigRational) -> Option<Node> {
    if ref_is(reference, 0, 1) {
        return Some(Node::Num(ExactNum::integer(0)));
    }
    if ref_is(reference, 1, 12) {
        return Some(exact_sqrt6_minus_sqrt2_over_4());
    }
    if ref_is(reference, 1, 10) {
        return Some(exact_sin_pi_10());
    }
    if ref_is(reference, 1, 8) {
        return Some(exact_sin_pi_8());
    }
    if ref_is(reference, 1, 6) {
        return Some(exact_half());
    }
    if ref_is(reference, 1, 5) {
        return Some(exact_sin_pi_5());
    }
    if ref_is(reference, 1, 4) {
        return Some(exact_sqrt2_over_2());
    }
    if ref_is(reference, 1, 3) {
        return Some(exact_sqrt3_over_2());
    }
    if ref_is(reference, 2, 5) {
        return Some(exact_sin_2pi_5());
    }
    if ref_is(reference, 3, 8) {
        return Some(exact_cos_pi_8());
    }
    if ref_is(reference, 3, 10) {
        return Some(exact_sin_3pi_10());
    }
    if ref_is(reference, 5, 12) {
        return Some(exact_sqrt6_plus_sqrt2_over_4());
    }
    if ref_is(reference, 1, 2) {
        return Some(Node::Num(ExactNum::integer(1)));
    }
    None
}

/// Exact value of `tan(reference·π)` for a reference angle in `[0, 1/2)`.
fn principal_tan_value(reference: &BigRational) -> Option<Node> {
    if ref_is(reference, 0, 1) {
        return Some(Node::Num(ExactNum::integer(0)));
    }
    if ref_is(reference, 1, 8) {
        return Some(Node::Subtract(
            Box::new(Node::Sqrt(Box::new(Node::Num(ExactNum::integer(2))))),
            Box::new(Node::Num(ExactNum::integer(1))),
        ));
    }
    if ref_is(reference, 1, 12) {
        return Some(exact_tan_pi_12());
    }
    if ref_is(reference, 1, 10) {
        return Some(exact_tan_pi_10());
    }
    if ref_is(reference, 1, 6) {
        return Some(exact_one_over_sqrt3());
    }
    if ref_is(reference, 1, 5) {
        return Some(exact_tan_pi_5());
    }
    if ref_is(reference, 1, 4) {
        return Some(Node::Num(ExactNum::integer(1)));
    }
    if ref_is(reference, 2, 5) {
        return Some(exact_tan_2pi_5());
    }
    if ref_is(reference, 3, 8) {
        return Some(Node::Add(
            Box::new(Node::Sqrt(Box::new(Node::Num(ExactNum::integer(2))))),
            Box::new(Node::Num(ExactNum::integer(1))),
        ));
    }
    if ref_is(reference, 3, 10) {
        return Some(exact_tan_3pi_10());
    }
    if ref_is(reference, 1, 3) {
        return Some(Node::Sqrt(Box::new(Node::Num(ExactNum::integer(3)))));
    }
    if ref_is(reference, 5, 12) {
        return Some(exact_tan_5pi_12());
    }
    None
}

/// Apply a sign to an exact trig value, negating numeric results in place so a
/// bare `-1`/`-1/2` is produced rather than a wrapped `Negate` node.
fn apply_trig_sign(negative: bool, value: Node) -> Node {
    if !negative {
        return value;
    }
    match value {
        Node::Num(n) if n.is_zero() => Node::Num(n),
        Node::Num(n) => Node::Num(-n),
        other => Node::Negate(Box::new(other)),
    }
}

/// Reduce `sin(k·π)` using the full 2π period to `(negative, reference)`, where
/// `reference ∈ [0, 1/2]` is the first-quadrant reference angle and `negative`
/// carries the quadrant sign. `cos` reduces through this via `cos(k·π) =
/// sin((1/2 − k)·π)`.
fn reduce_sin_to_principal(k: &BigRational) -> (bool, BigRational) {
    let two = BigRational::from_integer(BigInt::from(2));
    let one = BigRational::one();
    let half = BigRational::new(BigInt::one(), BigInt::from(2));
    // Map the angle into [0, 2) (period 2π ↔ k-period 2).
    let mut r = k - (k / &two).floor() * &two;
    if r.is_negative() {
        r += &two;
    }
    if r <= one {
        // Quadrants I/II: sin ≥ 0. Reflect about 1/2 (sin(π − x) = sin(x)).
        let reference = if r <= half { r } else { &one - &r };
        (false, reference)
    } else {
        // Quadrants III/IV: sin ≤ 0.
        let u = &r - &one;
        let reference = if u <= half { u } else { &one - &u };
        (true, reference)
    }
}

// --- Circular trigonometric ---
fn try_exact_sin(arg: &Node) -> Option<Node> {
    let k = as_pi_multiple(arg)?;
    let (negative, reference) = reduce_sin_to_principal(&k);
    let value = principal_sin_value(&reference)?;
    Some(apply_trig_sign(negative, value))
}

fn try_exact_cos(arg: &Node) -> Option<Node> {
    let k = as_pi_multiple(arg)?;
    // cos(k·π) = sin((1/2 − k)·π); reuse the sin reduction for correct signs.
    let half = BigRational::new(BigInt::one(), BigInt::from(2));
    let (negative, reference) = reduce_sin_to_principal(&(half - k));
    let value = principal_sin_value(&reference)?;
    Some(apply_trig_sign(negative, value))
}

fn try_exact_tan(arg: &Node) -> Option<Node> {
    if let Node::Num(n) = arg {
        if n.is_zero() {
            return Some(Node::Num(ExactNum::integer(0)));
        }
    }
    let k = as_pi_multiple(arg)?;
    let one = BigRational::one();
    let half = BigRational::new(BigInt::one(), BigInt::from(2));
    // tan has period π: reduce k into [0, 1).
    let floor_k = k.floor();
    let mut t = k - floor_k;
    if t.is_negative() {
        t += &one;
    }
    if t == half {
        return None; // tan(π/2 + nπ) is undefined
    }
    // tan(π − x) = −tan(x): reflect the upper half about 1/2.
    let (negative, reference) = if t <= half {
        (false, t)
    } else {
        (true, &one - &t)
    };
    let value = principal_tan_value(&reference)?;
    Some(apply_trig_sign(negative, value))
}

// --- Inverse circular trigonometric ---
fn try_exact_arcsin(arg: &Node) -> Option<Node> {
    if let Some(r) = as_exact_rational(arg) {
        if r.is_zero() {
            return Some(Node::Num(ExactNum::integer(0)));
        }
        if r.is_one() {
            return Some(pi_over(2));
        }
        if (-r.clone()).is_one() {
            return Some(neg_pi_over(2));
        }
        if r == BigRational::new(1.into(), 2.into()) {
            return Some(pi_over(6));
        }
    }
    try_exact_inverse_surd("arcsin", arg)
}

fn try_exact_arccos(arg: &Node) -> Option<Node> {
    if let Some(r) = as_exact_rational(arg) {
        if r.is_zero() {
            return Some(pi_over(2));
        }
        if r.is_one() {
            return Some(Node::Num(ExactNum::integer(0)));
        }
        if (-r.clone()).is_one() {
            return Some(pi_node());
        }
        if r == BigRational::new(1.into(), 2.into()) {
            return Some(pi_over(3));
        }
        if r == -BigRational::new(1.into(), 2.into()) {
            return Some(k_pi_over(2, 3));
        }
    }
    try_exact_inverse_surd("arccos", arg)
}

fn try_exact_arctan(arg: &Node) -> Option<Node> {
    if let Some(r) = as_exact_rational(arg) {
        if r.is_zero() {
            return Some(Node::Num(ExactNum::integer(0)));
        }
        if r.is_one() {
            return Some(pi_over(4));
        }
        if (-r.clone()).is_one() {
            return Some(neg_pi_over(4));
        }
    }
    try_exact_inverse_surd("arctan", arg)
}

// --- Inverse reciprocal trigonometric ---
fn try_exact_arccsc(arg: &Node) -> Option<Node> {
    try_exact_inverse_surd("arccsc", arg).or_else(|| inverse_reciprocal_exact("arcsin", arg))
}

fn try_exact_arcsec(arg: &Node) -> Option<Node> {
    try_exact_inverse_surd("arcsec", arg).or_else(|| inverse_reciprocal_exact("arccos", arg))
}

fn try_exact_arccot(arg: &Node) -> Option<Node> {
    if let Some(r) = as_exact_rational(arg) {
        if r.is_zero() {
            return Some(Node::Divide(
                Box::new(pi_node()),
                Box::new(Node::Num(ExactNum::integer(2))),
            ));
        }
        if r.is_one() {
            return Some(Node::Divide(
                Box::new(pi_node()),
                Box::new(Node::Num(ExactNum::integer(4))),
            ));
        }
        if (-r.clone()).is_one() {
            return Some(Node::Negate(Box::new(Node::Divide(
                Box::new(pi_node()),
                Box::new(Node::Num(ExactNum::integer(4))),
            ))));
        }
    }
    try_exact_inverse_surd("arccot", arg).or_else(|| inverse_reciprocal_exact("arctan", arg))
}

fn as_integer_big(node: &Node) -> Option<BigInt> {
    match node {
        Node::Num(n) => {
            let r = n.to_rational()?;
            if r.is_integer() {
                Some(r.to_integer())
            } else {
                None
            }
        }
        Node::Negate(inner) => {
            let v = as_integer_big(inner)?;
            Some(-v)
        }
        _ => None,
    }
}

fn ln_two_plus_sqrt3() -> Node {
    ln_of(Node::Add(
        Box::new(Node::Num(ExactNum::integer(2))),
        Box::new(Node::Sqrt(Box::new(Node::Num(ExactNum::integer(3))))),
    ))
}

fn ln_one_plus_sqrt5_over_2() -> Node {
    ln_of(Node::Divide(
        Box::new(Node::Add(
            Box::new(Node::Num(ExactNum::integer(1))),
            Box::new(Node::Sqrt(Box::new(Node::Num(ExactNum::integer(5))))),
        )),
        Box::new(Node::Num(ExactNum::integer(2))),
    ))
}

fn ln_one_plus_sqrt2() -> Node {
    ln_of(Node::Add(
        Box::new(Node::Num(ExactNum::integer(1))),
        Box::new(Node::Sqrt(Box::new(Node::Num(ExactNum::integer(2))))),
    ))
}

fn try_exp_k_ln(k: &Node, ln_side: &Node) -> Option<Node> {
    let exp = as_integer_big(k)?;
    let Node::Function(name, args) = ln_side else {
        return None;
    };
    if name != "ln" || args.len() != 1 {
        return None;
    }
    let base = as_exact_rational(&args[0])?;
    if !base.is_integer() || base <= BigRational::one() {
        return None;
    }
    Some(exact_num(rational_int_pow(&base, &exp)?))
}

fn try_exact_exp(arg: &Node) -> Option<Node> {
    if let Node::Function(name, args) = arg {
        if args.len() == 1 && name == "ln" {
            // exp(ln x) = x since exp is e^x; log/lg are different bases — no cancel.
            return Some(args[0].clone());
        }
    }
    if let Node::Multiply(a, b) = arg {
        return try_exp_k_ln(a, b).or_else(|| try_exp_k_ln(b, a));
    }
    None
}

// --- Hyperbolic ---
fn try_exact_sinh(arg: &Node) -> Option<Node> {
    if let Some((a, neg)) = as_ln_positive_integer(arg) {
        return hyperbolic_at_ln(&a, neg, HyperbolicLnKind::Sinh);
    }
    if let Node::Num(n) = arg {
        if n.to_rational().is_some_and(|r| r.is_zero()) {
            return Some(Node::Num(ExactNum::integer(0)));
        }
    }
    None
}

fn try_exact_cosh(arg: &Node) -> Option<Node> {
    if let Some((a, _)) = as_ln_positive_integer(arg) {
        return hyperbolic_at_ln(&a, false, HyperbolicLnKind::Cosh);
    }
    if let Node::Num(n) = arg {
        if n.to_rational().is_some_and(|r| r.is_zero()) {
            return Some(Node::Num(ExactNum::integer(1)));
        }
    }
    None
}

fn try_exact_tanh(arg: &Node) -> Option<Node> {
    if let Some((a, neg)) = as_ln_positive_integer(arg) {
        return hyperbolic_at_ln(&a, neg, HyperbolicLnKind::Tanh);
    }
    if let Node::Num(n) = arg {
        if n.to_rational().is_some_and(|r| r.is_zero()) {
            return Some(Node::Num(ExactNum::integer(0)));
        }
    }
    None
}

// --- Inverse hyperbolic ---
fn try_exact_arcsinh(arg: &Node) -> Option<Node> {
    if let Some(r) = as_exact_rational(arg) {
        if r.is_zero() {
            return Some(Node::Num(ExactNum::integer(0)));
        }
        if r.is_one() {
            return Some(ln_one_plus_sqrt2());
        }
        if (-r.clone()).is_one() {
            return Some(Node::Negate(Box::new(ln_one_plus_sqrt2())));
        }
        let half = BigRational::new(1.into(), 2.into());
        if r == half {
            return Some(ln_one_plus_sqrt5_over_2());
        }
        if r == -half {
            return Some(Node::Negate(Box::new(ln_one_plus_sqrt5_over_2())));
        }
    }
    None
}

fn try_exact_arccosh(arg: &Node) -> Option<Node> {
    let r = as_exact_rational(arg)?;
    if r.is_one() {
        return Some(Node::Num(ExactNum::integer(0)));
    }
    // arccosh(x) = ln(x + √(x² − 1))
    if r == BigRational::from_integer(BigInt::from(2)) {
        return Some(ln_of(Node::Add(
            Box::new(Node::Num(ExactNum::integer(2))),
            Box::new(Node::Sqrt(Box::new(Node::Num(ExactNum::integer(3))))),
        )));
    }
    if r == BigRational::from_integer(BigInt::from(3)) {
        return Some(ln_of(Node::Add(
            Box::new(Node::Num(ExactNum::integer(3))),
            Box::new(Node::Multiply(
                Box::new(Node::Num(ExactNum::integer(2))),
                Box::new(Node::Sqrt(Box::new(Node::Num(ExactNum::integer(2))))),
            )),
        )));
    }
    None
}

fn try_exact_arctanh(arg: &Node) -> Option<Node> {
    if let Some(r) = as_exact_rational(arg) {
        if r.is_zero() {
            return Some(Node::Num(ExactNum::integer(0)));
        }
        // arctanh(x) = (1/2)·ln((1 + x)/(1 − x))
        let half = BigRational::new(1.into(), 2.into());
        let third = BigRational::new(1.into(), 3.into());
        if r == half {
            return Some(half_ln(BigRational::from_integer(BigInt::from(3))));
        }
        if r == -half {
            return Some(Node::Negate(Box::new(half_ln(BigRational::from_integer(
                BigInt::from(3),
            )))));
        }
        if r == third {
            return Some(half_ln(BigRational::from_integer(BigInt::from(2))));
        }
        if r == -third {
            return Some(Node::Negate(Box::new(half_ln(BigRational::from_integer(
                BigInt::from(2),
            )))));
        }
    }
    if let Some((neg, surd)) = as_signed_standard_surd(arg) {
        if matches!(surd, StandardSurd::InvSqrt3) {
            let value = Node::Multiply(
                Box::new(exact_num(BigRational::new(
                    BigInt::from(1),
                    BigInt::from(2),
                ))),
                Box::new(ln_two_plus_sqrt3()),
            );
            return Some(if neg {
                Node::Negate(Box::new(value))
            } else {
                value
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pi_rational_large_coefficient() {
        let k = BigRational::new(
            BigInt::parse_bytes(b"9007199254740993", 10).unwrap(),
            BigInt::from(7),
        );
        let display = format!("{}", pi_rational_to_node(&k));
        assert!(display.contains("9007199254740993"));
        assert!(display.contains("\\pi"));
    }

    #[test]
    fn test_sinh_ln_two_is_three_quarters() {
        let arg = Node::Function("ln".to_string(), vec![Node::Num(ExactNum::integer(2))]);
        assert_eq!(
            try_exact_sinh(&arg).map(|n| format!("{}", n)),
            Some("\\frac{3}{4}".to_string())
        );
    }

    #[test]
    fn test_arctanh_half_is_half_ln_three() {
        let arg = Node::Num(ExactNum::rational(1, 2));
        let result = format!("{}", try_exact_arctanh(&arg).unwrap());
        assert!(result.contains("ln") && result.contains("3"));
    }

    #[test]
    fn test_tan_pi_6() {
        let arg = Node::Divide(
            Box::new(pi_node()),
            Box::new(Node::Num(ExactNum::integer(6))),
        );
        assert_eq!(
            format!("{}", try_exact_tan(&arg).unwrap()),
            "\\frac{1}{\\sqrt{3}}"
        );
    }

    #[test]
    fn test_arcsin_sqrt2_over_2() {
        let arg = Node::Divide(
            Box::new(Node::Sqrt(Box::new(Node::Num(ExactNum::integer(2))))),
            Box::new(Node::Num(ExactNum::integer(2))),
        );
        assert_eq!(
            format!("{}", try_exact_arcsin(&arg).unwrap()),
            "\\frac{\\pi}{4}"
        );
    }

    #[test]
    fn test_principal_sin_unreduced_fraction() {
        // 2/24 reduces to 1/12 — lookup must not rely on i64 numer/denom.
        let reference = BigRational::new(BigInt::from(2), BigInt::from(24));
        assert!(principal_sin_value(&reference).is_some());
    }

    #[test]
    fn test_log_large_power_of_ten() {
        let n = BigInt::parse_bytes(b"100000000000000000000", 10).unwrap();
        let arg = Node::Num(ExactNum::Rational(BigRational::from_integer(n)));
        assert_eq!(
            try_exact_function_value("log", std::slice::from_ref(&arg)).map(|n| format!("{}", n)),
            Some("20".to_string())
        );
    }

    #[test]
    fn test_exp_two_ln_three() {
        let arg = Node::Multiply(
            Box::new(Node::Num(ExactNum::integer(2))),
            Box::new(Node::Function(
                "ln".to_string(),
                vec![Node::Num(ExactNum::integer(3))],
            )),
        );
        assert_eq!(
            try_exact_exp(&arg).map(|n| format!("{}", n)),
            Some("9".to_string())
        );
    }

    #[test]
    fn test_arccosh_three() {
        let arg = Node::Num(ExactNum::integer(3));
        let result = format!("{}", try_exact_arccosh(&arg).unwrap());
        assert!(result.contains("ln") && result.contains("2"));
    }

    #[test]
    fn test_arccsc_sqrt2_via_surd_table() {
        let arg = Node::Sqrt(Box::new(Node::Num(ExactNum::integer(2))));
        assert_eq!(
            format!("{}", try_exact_arccsc(&arg).unwrap()),
            "\\frac{\\pi}{4}"
        );
    }

    /// Approximate numeric value of an exact result tree (regression helper).
    fn approx_node_f64(node: &Node) -> Option<f64> {
        match node {
            Node::Num(n) => Some(n.to_f64()),
            Node::Negate(inner) => approx_node_f64(inner).map(|v| -v),
            Node::Divide(a, b) => Some(approx_node_f64(a)? / approx_node_f64(b)?),
            Node::Multiply(a, b) => Some(approx_node_f64(a)? * approx_node_f64(b)?),
            Node::Add(a, b) => Some(approx_node_f64(a)? + approx_node_f64(b)?),
            Node::Subtract(a, b) => Some(approx_node_f64(a)? - approx_node_f64(b)?),
            Node::Sqrt(inner) => Some(approx_node_f64(inner)?.sqrt()),
            _ => None,
        }
    }

    fn pi_multiple_node(k: &BigRational) -> Node {
        Node::Multiply(
            Box::new(Node::Num(ExactNum::Rational(k.clone()))),
            Box::new(pi_node()),
        )
    }

    /// Property-style check: exact `sin(k·π)` agrees with float reference on all
    /// constructible denominators and a wide numerator sweep (quadrant regression).
    #[test]
    fn test_exact_sin_matches_float_for_constructible_angles() {
        let denoms = [1i64, 2, 3, 4, 5, 6, 8, 10, 12];
        for &d in &denoms {
            for n in -24i64..=48 {
                let k = BigRational::new(BigInt::from(n), BigInt::from(d));
                let arg = pi_multiple_node(&k);
                let Some(exact) = try_exact_sin(&arg) else {
                    continue;
                };
                let Some(actual) = approx_node_f64(&exact) else {
                    continue;
                };
                let expected = ((n as f64 / d as f64) * std::f64::consts::PI).sin();
                assert!(
                    (actual - expected).abs() < 1e-9,
                    "sin({n}π/{d}): exact≈{actual}, float≈{expected}"
                );
            }
        }
    }

    /// Same sweep as `sin`: exact `cos(k·π)` vs float reference (quadrant regression).
    #[test]
    fn test_exact_cos_matches_float_for_constructible_angles() {
        let denoms = [1i64, 2, 3, 4, 5, 6, 8, 10, 12];
        for &d in &denoms {
            for n in -24i64..=48 {
                let k = BigRational::new(BigInt::from(n), BigInt::from(d));
                let arg = pi_multiple_node(&k);
                let Some(exact) = try_exact_cos(&arg) else {
                    continue;
                };
                let Some(actual) = approx_node_f64(&exact) else {
                    continue;
                };
                let expected = ((n as f64 / d as f64) * std::f64::consts::PI).cos();
                assert!(
                    (actual - expected).abs() < 1e-9,
                    "cos({n}π/{d}): exact≈{actual}, float≈{expected}"
                );
            }
        }
    }

    /// Same sweep as `sin`: exact `tan(k·π)` vs float reference; skips undefined π/2 + nπ.
    #[test]
    fn test_exact_tan_matches_float_for_constructible_angles() {
        let denoms = [1i64, 2, 3, 4, 5, 6, 8, 10, 12];
        for &d in &denoms {
            for n in -24i64..=48 {
                let k = BigRational::new(BigInt::from(n), BigInt::from(d));
                let arg = pi_multiple_node(&k);
                let Some(exact) = try_exact_tan(&arg) else {
                    continue;
                };
                let Some(actual) = approx_node_f64(&exact) else {
                    continue;
                };
                let expected = ((n as f64 / d as f64) * std::f64::consts::PI).tan();
                assert!(
                    (actual - expected).abs() < 1e-9,
                    "tan({n}π/{d}): exact≈{actual}, float≈{expected}"
                );
            }
        }
    }

    #[test]
    fn test_exp_log_and_lg_do_not_cancel() {
        let x = Node::Variable("x".to_string());
        for name in ["log", "lg"] {
            let arg = Node::Function(name.to_string(), vec![x.clone()]);
            assert_eq!(try_exact_exp(&arg), None);
        }
        assert_eq!(
            try_exact_exp(&Node::Function(
                "log".to_string(),
                vec![Node::Num(ExactNum::integer(10))]
            )),
            None
        );
    }

    #[test]
    fn test_exp_ln_cancels() {
        let x = Node::Variable("x".to_string());
        let arg = Node::Function("ln".to_string(), vec![x.clone()]);
        assert_eq!(try_exact_exp(&arg).unwrap(), x);
    }
}
