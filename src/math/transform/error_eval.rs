//! First-order floating-point error propagation.
//!
//! [`evaluate_with_error`] mirrors the evaluator's fold but carries an
//! absolute error bound beside each value: leaves start at conversion
//! error, every operation propagates bounds through its first-order
//! sensitivity, and subtraction of nearly-equal quantities — catastrophic
//! cancellation — is where the bound visibly outgrows the value. The
//! question the bound answers is *how many digits of this f64 are
//! trustworthy*, via [`significant_digits`].
//!
//! Two consumers: the `approximate` result tier of the evaluate tool, and
//! the constant-comparison path of verify_chain, where `|a − b| ≤ bound`
//! replaces a fixed tolerance. That replacement is what lets e^{-50} = 0
//! fail honestly (disagreement ≫ bound) while sin(2π) = 0 still passes
//! (value within its own bound of zero) — a fixed tolerance cannot
//! distinguish the two.
//!
//! Contract: the bound is a first-order upper estimate. It may over-report
//! error (we then under-claim digits — harmless), but it must not
//! meaningfully under-report on the constructs it models. Anything without
//! a sound model returns `Err` — an optimistic bound would be a lie with
//! decimals.

use crate::environment::Environment;
use crate::node::Node;

const EPS: f64 = f64::EPSILON; // 2^-52 ≈ 2.22e-16, conservative (≥ half-ulp)

/// How many leading decimal digits of `value` are trustworthy given an
/// absolute error bound. 16 is the f64 ceiling; 0 means the value is
/// indistinguishable from noise (or from zero, if it is zero with error).
pub fn significant_digits(value: f64, bound: f64) -> u32 {
    if bound == 0.0 {
        return 16;
    }
    if value == 0.0 {
        return 0;
    }
    let rel = bound / value.abs();
    if rel >= 1.0 {
        0
    } else {
        ((-rel.log10()).floor() as u32).min(16)
    }
}

/// Evaluate `node` in f64, returning `(value, absolute_error_bound)`.
///
/// Errors both where the evaluator would error (undefined variable,
/// non-integer factorial) and where no sound first-order error model
/// exists (exotic functions, discontinuous operations at uncertain
/// points). Callers must treat `Err` as "bound unavailable", never as
/// "bound zero".
pub fn evaluate_with_error(node: &Node, env: &Environment) -> Result<(f64, f64), String> {
    let (v, e) = eval(node, env)?;
    if !v.is_finite() || !e.is_finite() {
        return Err("evaluation did not produce a finite value".to_string());
    }
    Ok((v, e))
}

fn eval(node: &Node, env: &Environment) -> Result<(f64, f64), String> {
    match node {
        Node::Num(n) => {
            let v = n.to_f64();
            // Small integers convert exactly; everything else may round.
            let bound = if v.fract() == 0.0 && v.abs() < 2f64.powi(53) {
                0.0
            } else {
                v.abs() * EPS
            };
            Ok((v, bound))
        }
        Node::Variable(var) => {
            if let Some(val) = env.get_exact(var) {
                // The caller's f64 IS the input — the bound measures the
                // algorithm's error on it, not the caller's intent.
                Ok((val.to_f64(), 0.0))
            } else if var == "π" {
                Ok((std::f64::consts::PI, std::f64::consts::PI * EPS))
            } else if var == "e" {
                Ok((std::f64::consts::E, std::f64::consts::E * EPS))
            } else {
                Err(format!("Variable '{}' is not defined.", var))
            }
        }
        Node::Negate(inner) => {
            let (v, e) = eval(inner, env)?;
            Ok((-v, e))
        }
        Node::Abs(inner) => {
            let (v, e) = eval(inner, env)?;
            Ok((v.abs(), e))
        }
        Node::Add(l, r) => {
            let (lv, le) = eval(l, env)?;
            let (rv, re) = eval(r, env)?;
            let v = lv + rv;
            Ok((v, le + re + v.abs() * EPS))
        }
        Node::Subtract(l, r) => {
            let (lv, le) = eval(l, env)?;
            let (rv, re) = eval(r, env)?;
            let v = lv - rv;
            // Cancellation lives here: le + re is absolute, so when
            // |v| ≪ |lv| + |rv| the bound dwarfs the value.
            Ok((v, le + re + v.abs() * EPS))
        }
        Node::Multiply(l, r) => {
            let (lv, le) = eval(l, env)?;
            let (rv, re) = eval(r, env)?;
            let v = lv * rv;
            Ok((v, lv.abs() * re + rv.abs() * le + v.abs() * EPS))
        }
        Node::Divide(l, r) => {
            let (lv, le) = eval(l, env)?;
            let (rv, re) = eval(r, env)?;
            if rv == 0.0 {
                return Err("division by zero".to_string());
            }
            let v = lv / rv;
            Ok((v, (le + v.abs() * re) / rv.abs() + v.abs() * EPS))
        }
        Node::Sqrt(inner) => {
            let (xv, xe) = eval(inner, env)?;
            if xv < 0.0 {
                return Err("square root of negative number".to_string());
            }
            if xv == 0.0 {
                return if xe == 0.0 {
                    Ok((0.0, 0.0))
                } else {
                    // First-order breaks down at the branch point.
                    Err("no error model for sqrt at an uncertain zero".to_string())
                };
            }
            let v = xv.sqrt();
            Ok((v, xe / (2.0 * v) + v * EPS))
        }
        Node::Power(base, exp) => {
            let (bv, be) = eval(base, env)?;
            let (xv, xe) = eval(exp, env)?;
            // Value semantics mirror the evaluator (powf on f64).
            let v = bv.powf(xv);
            if !v.is_finite() {
                return Err("power did not produce a finite value".to_string());
            }
            let mut bound = v.abs() * EPS;
            if be > 0.0 {
                // d/db b^x = x·b^{x-1}; requires the derivative to exist
                // where we stand.
                if bv <= 0.0 {
                    return Err(
                        "no error model for power with uncertain non-positive base".to_string()
                    );
                }
                bound += (xv * bv.powf(xv - 1.0)).abs() * be;
            }
            if xe > 0.0 {
                // d/dx b^x = b^x·ln b; needs b > 0.
                if bv <= 0.0 {
                    return Err(
                        "no error model for power with uncertain exponent and non-positive base"
                            .to_string(),
                    );
                }
                bound += (v * bv.ln()).abs() * xe;
            }
            Ok((v, bound))
        }
        Node::Factorial(inner) => {
            let (xv, xe) = eval(inner, env)?;
            if xe != 0.0 || xv.fract() != 0.0 || xv < 0.0 {
                return Err("factorial requires an exact non-negative integer".to_string());
            }
            let mut v = 1.0f64;
            for k in 2..=(xv as u64) {
                v *= k as f64;
            }
            if !v.is_finite() {
                return Err("factorial overflow".to_string());
            }
            Ok((v, v * EPS * (xv.max(1.0))))
        }
        Node::Summation(index_var, start, end, body) => {
            fold_range(index_var, start, end, body, env, 0.0, |acc, term| {
                let v = acc.0 + term.0;
                (v, acc.1 + term.1 + v.abs() * EPS)
            })
        }
        Node::Product(index_var, start, end, body) => {
            fold_range(index_var, start, end, body, env, 1.0, |acc, term| {
                let v = acc.0 * term.0;
                (
                    v,
                    acc.0.abs() * term.1 + term.0.abs() * acc.1 + v.abs() * EPS,
                )
            })
        }
        Node::Function(name, args) => {
            if args.len() != 1 {
                return Err(format!("no error model for {}-ary function", args.len()));
            }
            let (x, xe) = eval(&args[0], env)?;
            unary_function(name, x, xe)
        }
        // Discontinuous, piecewise, relational, and matrix constructs have
        // no sound first-order model here. Refusing is the honest bound.
        _ => Err("no floating-point error model for this construct".to_string()),
    }
}

/// Shared Σ/Π fold: integer bounds required exactly (mirroring the
/// evaluator's refusal), then the accumulator rule is applied per term.
fn fold_range(
    index_var: &str,
    start: &Node,
    end: &Node,
    body: &Node,
    env: &Environment,
    identity: f64,
    combine: impl Fn((f64, f64), (f64, f64)) -> (f64, f64),
) -> Result<(f64, f64), String> {
    let (sv, se) = eval(start, env)?;
    let (ev, ee) = eval(end, env)?;
    if se != 0.0 || ee != 0.0 || sv.fract() != 0.0 || ev.fract() != 0.0 {
        return Err("range bounds must be exact integers".to_string());
    }
    let (lo, hi) = (sv as i64, ev as i64);
    let mut scoped = env.clone();
    let mut acc = (identity, 0.0);
    for i in lo..=hi {
        scoped.set_exact(index_var, crate::exact::ExactNum::integer(i));
        let term = eval(body, &scoped)?;
        acc = combine(acc, term);
    }
    Ok(acc)
}

/// Unary functions: `δ_out = |f'(x)|·δ_in + intrinsic`, where intrinsic is
/// the function's own evaluation error at the point. For the
/// argument-reduced trig functions the intrinsic term must scale with |x|,
/// not |f(x)| — that is precisely what makes sin(2π) honest about being
/// indistinguishable from zero.
fn unary_function(name: &str, x: f64, xe: f64) -> Result<(f64, f64), String> {
    let (v, derivative, intrinsic) = match name {
        "sin" => (x.sin(), x.cos().abs(), (x.abs() + x.sin().abs()) * EPS),
        "cos" => (x.cos(), x.sin().abs(), (x.abs() + x.cos().abs()) * EPS),
        "tan" => {
            let v = x.tan();
            let d = 1.0 + v * v;
            (v, d, (x.abs() * d + v.abs()) * EPS)
        }
        "exp" => {
            let v = x.exp();
            (v, v, v.abs() * EPS)
        }
        "ln" => {
            if x <= 0.0 {
                return Err("ln requires a positive argument".to_string());
            }
            let v = x.ln();
            (v, 1.0 / x, v.abs() * EPS)
        }
        "log" => {
            if x <= 0.0 {
                return Err("log requires a positive argument".to_string());
            }
            let v = x.log10();
            (v, 1.0 / (x * std::f64::consts::LN_10), v.abs() * EPS)
        }
        "asin" | "arcsin" => {
            if x.abs() >= 1.0 {
                return Err("no error model for asin at the domain boundary".to_string());
            }
            (x.asin(), 1.0 / (1.0 - x * x).sqrt(), x.asin().abs() * EPS)
        }
        "acos" | "arccos" => {
            if x.abs() >= 1.0 {
                return Err("no error model for acos at the domain boundary".to_string());
            }
            (x.acos(), 1.0 / (1.0 - x * x).sqrt(), x.acos().abs() * EPS)
        }
        "atan" | "arctan" => (x.atan(), 1.0 / (1.0 + x * x), x.atan().abs() * EPS),
        "sinh" => (x.sinh(), x.cosh(), x.sinh().abs() * EPS),
        "cosh" => (x.cosh(), x.sinh().abs(), x.cosh().abs() * EPS),
        "tanh" => {
            let v = x.tanh();
            (v, 1.0 - v * v, v.abs() * EPS)
        }
        "abs" => (x.abs(), 1.0, 0.0),
        "erf" => {
            let d = 2.0 / std::f64::consts::PI.sqrt() * (-x * x).exp();
            // erf itself is not in std; refuse rather than approximate the
            // value — but the model exists if a value source appears.
            let _ = d;
            return Err("no f64 evaluation for erf here".to_string());
        }
        _ => return Err(format!("no floating-point error model for '{}'", name)),
    };
    if !v.is_finite() {
        return Err(format!("'{}' did not produce a finite value", name));
    }
    Ok((v, derivative * xe + intrinsic))
}
