//! Recognition of non-elementary antiderivatives as named special functions.
//!
//! When the Risch machinery proves an integrand has no *elementary*
//! antiderivative, the antiderivative may still be a named special function.
//! Each table row here is a definition, not a heuristic:
//!
//! - erf: d/dx erf(x) = (2/√π)·e^{−x²}            (DLMF 7.2.1)
//! - Ei:  d/dx Ei(x)  = eˣ/x                       (DLMF 6.2.5)
//! - li:  d/dx li(x)  = 1/ln(x)                    (DLMF 6.2.8)
//!
//! A structural match against a definition therefore *proves* the recognized
//! form (up to the constant of integration). The matcher is conservative:
//! it destructures exactly, and every recognized form must additionally pass
//! a numeric differentiation round-trip against the integrand before the
//! name is attached. Failure at any point drops the name and keeps the bare
//! impossibility result — the safe direction. No guessed names, ever.

use crate::environment::Environment;
use crate::exact::ExactNum;
use crate::node::Node;
use crate::polynomial::Polynomial;
use crate::simplify::Simplifiable;
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};

/// A non-elementary antiderivative expressed via a named special function.
#[derive(Debug, Clone, PartialEq)]
pub struct SpecialAntiderivative {
    /// The special function's name: "erf", "Ei", or "li".
    pub function: &'static str,
    /// The full antiderivative (constant factors folded in), e.g.
    /// (√π/2)·erf(x) for the integrand e^{−x²}.
    pub form: Node,
    /// The defining identity that justifies the recognition, human-readable.
    pub identity: String,
}

/// Recognize `integrand` (already simplified) as the derivative of a named
/// special function, up to a factor free of `var`. Returns `None` whenever
/// the match is not exact — unnamed is honest, misnamed is not.
pub fn recognize_special_antiderivative(
    integrand: &Node,
    var: &str,
) -> Option<SpecialAntiderivative> {
    let (coeff, core) = peel_constant_factor(integrand, var);
    let recognized = match_core(&core, var)?;

    let form = match coeff {
        Some(c) => Node::Multiply(Box::new(c), Box::new(recognized.form)),
        None => recognized.form,
    };
    let env = Environment::new();
    let form = form.simplify(&env).unwrap_or(form);

    // Guard: the construction must survive a numeric differentiation
    // round-trip against the integrand. d/dx eliminates the special function,
    // so both sides are elementary and evaluable.
    if !roundtrip_holds(integrand, &form, var) {
        return None;
    }

    Some(SpecialAntiderivative { form, ..recognized })
}

/// LaTeX-level convenience for the tool boundaries (CLI, MCP): recognize the
/// integrand and return `(function name, antiderivative as LaTeX)` ready for
/// `StatusReport::with_special_form`. `None` on parse failure or no match —
/// the caller's bare impossibility result is already correct.
pub fn recognize_special_form_latex(integrand_latex: &str, var: &str) -> Option<(String, String)> {
    let mut tokenizer = crate::tokenizer::Tokenizer::new(integrand_latex);
    let expr = crate::parser::build_expression_tree(tokenizer.tokenize()).ok()?;
    let env = Environment::new();
    let simplified = expr.simplify(&env).unwrap_or(expr);
    let special = recognize_special_antiderivative(&simplified, var)?;
    Some((special.function.to_string(), format!("{}", special.form)))
}

/// Split `expr` into (factor free of `var`, factor containing `var`).
/// ∫k·f dx = k·∫f dx for any k free of the integration variable, so the
/// free factor folds into the recognized form unchanged.
fn peel_constant_factor(expr: &Node, var: &str) -> (Option<Node>, Node) {
    match expr {
        Node::Multiply(a, b) => {
            if is_free_of_var(a, var) {
                let (inner, core) = peel_constant_factor(b, var);
                (Some(combine_mul(a.as_ref().clone(), inner)), core)
            } else if is_free_of_var(b, var) {
                let (inner, core) = peel_constant_factor(a, var);
                (Some(combine_mul(b.as_ref().clone(), inner)), core)
            } else {
                (None, expr.clone())
            }
        }
        Node::Divide(a, b) if is_free_of_var(b, var) && !is_free_of_var(a, var) => {
            let (inner, core) = peel_constant_factor(a, var);
            let reciprocal = Node::Divide(
                Box::new(Node::Num(ExactNum::one())),
                Box::new(b.as_ref().clone()),
            );
            (Some(combine_mul(reciprocal, inner)), core)
        }
        // Free factor in the *numerator* of a variable-bearing quotient:
        // c/den → c · (1/den), and (c·f)/den → c · (f/den). Simplify
        // normalizes every product spelling into this Divide shape before
        // recognition runs, so without this arm no spelling of 3/ln(x) or
        // 3e^{2x}/x could ever be named.
        Node::Divide(a, b) if !is_free_of_var(b, var) => {
            if is_free_of_var(a, var) {
                let core = Node::Divide(
                    Box::new(Node::Num(ExactNum::one())),
                    Box::new(b.as_ref().clone()),
                );
                (Some(a.as_ref().clone()), core)
            } else {
                let (coeff, core_num) = peel_constant_factor(a, var);
                match coeff {
                    Some(c) => (
                        Some(c),
                        Node::Divide(Box::new(core_num), Box::new(b.as_ref().clone())),
                    ),
                    None => (None, expr.clone()),
                }
            }
        }
        Node::Negate(a) => {
            let (inner, core) = peel_constant_factor(a, var);
            let minus_one = Node::Num(ExactNum::integer(-1));
            (Some(combine_mul(minus_one, inner)), core)
        }
        _ => (None, expr.clone()),
    }
}

fn combine_mul(factor: Node, existing: Option<Node>) -> Node {
    match existing {
        Some(e) => Node::Multiply(Box::new(factor), Box::new(e)),
        None => factor,
    }
}

/// Shared conservative predicate (see `Node::is_provably_free_of`): a
/// wrong "free" here would fold a variable-bearing factor into the
/// recognized form, so unknown node kinds are never treated as constants.
fn is_free_of_var(node: &Node, var: &str) -> bool {
    node.is_provably_free_of(var)
}

/// Match the variable-bearing core of the integrand against the table.
fn match_core(core: &Node, var: &str) -> Option<SpecialAntiderivative> {
    match_erf(core, var)
        .or_else(|| match_ei(core, var))
        .or_else(|| match_li(core, var))
}

/// exp(−a·x²) with a > 0 rational → (1/2)·√(π/a)·erf(√a·x).
fn match_erf(core: &Node, var: &str) -> Option<SpecialAntiderivative> {
    let arg = exp_argument(core)?;
    let poly = Polynomial::from_node(arg, var).ok()?;
    if poly.degree() != Some(2) || !poly.coeff(1).is_zero() || !poly.coeff(0).is_zero() {
        return None;
    }
    let a = -poly.coeff(2);
    if !a.is_positive() {
        // exp(+a·x²) integrates to erfi, which is not in the table yet.
        return None;
    }

    let x = Node::Variable(var.to_string());
    let pi = Node::Variable("π".to_string());
    let (scale, erf_arg) = if a.is_one() {
        (sqrt(pi), x)
    } else {
        // For a = p/q: scale = √(π·q/p), argument = √p·x/√q — the
        // conventional spelling (erf(x/√2), not erf(√(1/2)·x)).
        let p = BigRational::from(a.numer().clone());
        let q = BigRational::from(a.denom().clone());
        let scale = sqrt(Node::Divide(
            Box::new(Node::Multiply(Box::new(pi), Box::new(rational_node(&q)))),
            Box::new(rational_node(&p)),
        ));
        let numerator = if p.is_one() {
            x
        } else {
            Node::Multiply(Box::new(sqrt(rational_node(&p))), Box::new(x))
        };
        let arg = if q.is_one() {
            numerator
        } else {
            Node::Divide(Box::new(numerator), Box::new(sqrt(rational_node(&q))))
        };
        (scale, arg)
    };
    let form = Node::Multiply(
        Box::new(Node::Divide(
            Box::new(scale),
            Box::new(Node::Num(ExactNum::two())),
        )),
        Box::new(Node::Function("erf".to_string(), vec![erf_arg])),
    );
    Some(SpecialAntiderivative {
        function: "erf",
        form,
        identity: "d/dx erf(u) = (2/√π)·e^{-u²} (DLMF 7.2.1)".to_string(),
    })
}

/// exp(b·x)/x with b ≠ 0 rational → Ei(b·x).
fn match_ei(core: &Node, var: &str) -> Option<SpecialAntiderivative> {
    let (numerator, denominator) = as_quotient(core)?;
    if !matches!(&denominator, Node::Variable(name) if name == var) {
        return None;
    }
    let arg = exp_argument(&numerator)?;
    let poly = Polynomial::from_node(arg, var).ok()?;
    if poly.degree() != Some(1) || !poly.coeff(0).is_zero() {
        return None;
    }
    let b = poly.coeff(1);
    if b.is_zero() {
        return None;
    }

    let x = Node::Variable(var.to_string());
    let ei_arg = if b.is_one() {
        x
    } else {
        Node::Multiply(Box::new(rational_node(&b)), Box::new(x))
    };
    Some(SpecialAntiderivative {
        function: "Ei",
        form: Node::Function("Ei".to_string(), vec![ei_arg]),
        identity: "d/dx Ei(u) = e^u/u (DLMF 6.2.5)".to_string(),
    })
}

/// 1/ln(x) → li(x).
fn match_li(core: &Node, var: &str) -> Option<SpecialAntiderivative> {
    let (numerator, denominator) = as_quotient(core)?;
    let is_unit_numerator = matches!(&numerator, Node::Num(n) if n.is_one());
    if !is_unit_numerator {
        return None;
    }
    match &denominator {
        Node::Function(name, args)
            if name == "ln"
                && args.len() == 1
                && matches!(&args[0], Node::Variable(v) if v == var) =>
        {
            Some(SpecialAntiderivative {
                function: "li",
                form: Node::Function("li".to_string(), vec![Node::Variable(var.to_string())]),
                identity: "d/dx li(u) = 1/ln(u) (DLMF 6.2.8)".to_string(),
            })
        }
        _ => None,
    }
}

/// The argument of exp, whether spelled exp(u) or (post-simplify) e^u.
fn exp_argument(node: &Node) -> Option<&Node> {
    match node {
        Node::Function(name, args) if name == "exp" && args.len() == 1 => Some(&args[0]),
        Node::Power(base, exponent) => match base.as_ref() {
            Node::Variable(v) if v == "e" => Some(exponent),
            _ => None,
        },
        _ => None,
    }
}

/// View a node as numerator/denominator: Divide directly, or a Multiply
/// carrying a negative power.
fn as_quotient(node: &Node) -> Option<(Node, Node)> {
    match node {
        Node::Divide(a, b) => Some((a.as_ref().clone(), b.as_ref().clone())),
        _ => None,
    }
}

/// Build a radical as `Node::Sqrt`, which displays as `\sqrt{u}`.
/// (`Function("sqrt", …)` would display as `\sqrt(u)` — not valid LaTeX,
/// and a second spelling of the same object.)
fn sqrt(node: Node) -> Node {
    Node::Sqrt(Box::new(node))
}

fn rational_node(r: &BigRational) -> Node {
    Node::Num(ExactNum::Rational(r.clone()))
}

/// Numeric self-check: d/dx(form) must agree with the integrand at sample
/// points where both evaluate. At least one point must succeed. Any failure
/// drops the recognition — under-claiming is the safe direction.
fn roundtrip_holds(integrand: &Node, form: &Node, var: &str) -> bool {
    let derivative = match crate::derivative::differentiate(form, var) {
        Ok(d) => d,
        Err(_) => return false,
    };
    // Simplify before evaluating: the product rule leaves terms like
    // (d/dx √π)·erf(x) — an exact zero multiplied by a function that
    // (deliberately) refuses numeric evaluation. Folding the zeros away
    // leaves the derivative erf/Ei/li-free and hence evaluable.
    let env = Environment::new();
    let derivative = derivative.simplify(&env).unwrap_or(derivative);
    let mut checked = 0usize;
    for point in [0.7_f64, 1.9, 3.3] {
        let mut env = Environment::new();
        env.set(var, point);
        let lhs = crate::evaluator::Evaluator::evaluate(&derivative, &env);
        let rhs = crate::evaluator::Evaluator::evaluate(integrand, &env);
        match (lhs, rhs) {
            (Ok(l), Ok(r)) if l.is_finite() && r.is_finite() => {
                if (l - r).abs() > 1e-9 * r.abs().max(1.0) {
                    return false;
                }
                checked += 1;
            }
            // A point where either side fails to evaluate carries no
            // evidence for or against; skip it.
            _ => continue,
        }
    }
    checked > 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::build_expression_tree;
    use crate::tokenizer::Tokenizer;

    fn parse(latex: &str) -> Node {
        let mut tokenizer = Tokenizer::new(latex);
        build_expression_tree(tokenizer.tokenize()).unwrap()
    }

    fn simplified(latex: &str) -> Node {
        let env = Environment::new();
        let node = parse(latex);
        node.simplify(&env).unwrap_or(node)
    }

    #[test]
    fn test_match_erf_on_plain_gaussian_core() {
        let core = simplified("\\exp(-x^2)");
        let m = match_erf(&core, "x");
        assert!(m.is_some(), "match_erf missed core {:?}", core);
    }

    #[test]
    fn test_erf_roundtrip_guard_accepts_correct_form() {
        let integrand = simplified("\\exp(-x^2)");
        let m = match_erf(&integrand, "x").expect("matcher");
        assert!(
            roundtrip_holds(&integrand, &m.form, "x"),
            "round-trip rejected the correct form {:?}",
            m.form
        );
    }

    #[test]
    fn test_erf_roundtrip_guard_rejects_wrong_form() {
        // The guard exists to catch matcher bugs: a wrong recognized form
        // must be rejected, not shipped.
        let integrand = simplified("\\exp(-x^2)");
        let wrong = Node::Function("erf".to_string(), vec![Node::Variable("x".to_string())]);
        assert!(
            !roundtrip_holds(&integrand, &wrong, "x"),
            "round-trip accepted a form missing the √π/2 factor"
        );
    }
}
