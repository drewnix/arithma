//! Shared metadata for built-in function names (classification, aliases).
//!
//! Canonical listing order (use in simplify, derivative, integration, registry):
//! - Circular trigonometric: sin, cos, tan
//! - Reciprocal trigonometric: csc, sec, cot
//! - Inverse circular trigonometric: arcsin, arccos, arctan
//! - Inverse reciprocal trigonometric: arccsc, arcsec, arccot
//! - Hyperbolic: sinh, cosh, tanh
//! - Reciprocal hyperbolic: csch, sech, coth
//! - Inverse hyperbolic: arcsinh, arccosh, arctanh
//! - Inverse reciprocal hyperbolic: arccsch, arcsech, arccoth
//! - Logarithmic and exponential: log, ln, lg, exp

/// Canonicalize common function aliases to their primary registry name.
pub fn canonical_function_name(name: &str) -> &str {
    match name {
        // Inverse circular trigonometric aliases
        "asin" => "arcsin",
        "acos" => "arccos",
        "atan" => "arctan",
        // Inverse hyperbolic aliases
        "asinh" => "arcsinh",
        "acosh" => "arccosh",
        "atanh" => "arctanh",
        other => other,
    }
}

/// If `power` is exactly `-1`, return the inverse function name (e.g. `\sin^{-1}` → `arcsin`).
/// Only the integer −1 is treated as inverse-function notation; `-1.0`, `-2`, `1/2`, etc.
/// are powers of the function value, not inverse functions.
pub fn inverse_from_minus_one_power(base: &str, power: &str) -> Option<&'static str> {
    if power.trim() != "-1" {
        return None;
    }
    match canonical_function_name(base) {
        // Circular trigonometric
        "sin" => Some("arcsin"),
        "cos" => Some("arccos"),
        "tan" => Some("arctan"),
        // Reciprocal trigonometric
        "csc" => Some("arccsc"),
        "sec" => Some("arcsec"),
        "cot" => Some("arccot"),
        // Hyperbolic
        "sinh" => Some("arcsinh"),
        "cosh" => Some("arccosh"),
        "tanh" => Some("arctanh"),
        // Reciprocal hyperbolic
        "csch" => Some("arccsch"),
        "sech" => Some("arcsech"),
        "coth" => Some("arccoth"),
        _ => None,
    }
}

pub fn is_trig_or_hyperbolic(name: &str) -> bool {
    matches!(
        canonical_function_name(name),
        // Circular trigonometric
        "sin"
            | "cos"
            | "tan"
            // Reciprocal trigonometric
            | "csc"
            | "sec"
            | "cot"
            // Inverse circular trigonometric
            | "arcsin"
            | "arccos"
            | "arctan"
            // Inverse reciprocal trigonometric
            | "arccsc"
            | "arcsec"
            | "arccot"
            // Hyperbolic
            | "sinh"
            | "cosh"
            | "tanh"
            // Reciprocal hyperbolic
            | "csch"
            | "sech"
            | "coth"
            // Inverse hyperbolic
            | "arcsinh"
            | "arccosh"
            | "arctanh"
            // Inverse reciprocal hyperbolic
            | "arccsch"
            | "arcsech"
            | "arccoth"
    )
}

pub fn is_log_or_exp(name: &str) -> bool {
    // Logarithmic and exponential
    matches!(name, "log" | "ln" | "lg" | "exp")
}

/// Special functions arising as non-elementary antiderivatives (erf, Ei, li).
/// Symbolic-only: they parse, print, and differentiate exactly; numeric
/// evaluation is deliberately unimplemented until it carries an error bound.
pub fn is_special_function(name: &str) -> bool {
    matches!(name, "erf" | "Ei" | "li")
}

/// Functions that map exact (rational) inputs to generally irrational values.
/// `simplify` keeps these symbolic instead of collapsing to a float.
pub fn is_transcendental_function(name: &str) -> bool {
    is_trig_or_hyperbolic(name) || is_log_or_exp(name) || is_special_function(name)
}
