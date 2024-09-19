
use arithma::*;
use serde_json::json;

fn evaluate_mathjson(mathjson: serde_json::Value, env: &Environment) -> Result<f64, String> {
    let node = mathjson_to_node(&mathjson)?;
    Evaluator::evaluate(&node, env)
}

// 1. Derivatives
#[test]
fn test_derivative_polynomial() {
    let env = Environment::new();

    // Derivative of 3x^2 + 2x + 1 with respect to x
    let derivative = json!([
        "Derivative",
        [
            "Add",
            [
                "Add",
                ["Multiply", 3, ["Power", "x", 2]],
                ["Multiply", 2, "x"]
            ],
            1
        ],
        "x",
        1
    ]);
    let mut env_with_x = env.clone();
    env_with_x.set("x", 1.0); // Set x = 1
    assert_eq!(evaluate_mathjson(derivative, &env_with_x).unwrap(), 8.0); // The derivative at x = 1
}

#[test]
fn test_derivative_trigonometric() {
    let env = Environment::new();

    // Derivative of sin(x) with respect to x
    let derivative = json!(["Derivative", ["Sin", "x"], "x", 1]);
    let mut env_with_x = env.clone();
    env_with_x.set("x", 0.0); // Set x = 0
    assert_eq!(evaluate_mathjson(derivative, &env_with_x).unwrap(), 1.0); // cos(0) = 1
}

// 2. Integrals
#[test]
fn test_indefinite_integral_polynomial() {
    let env = Environment::new();

    // Indefinite integral of 3x^2 with respect to x
    let integral = json!(["Integral", ["Multiply", 3, ["Power", "x", 2]], "x"]);
    let mut env_with_x = env.clone();
    env_with_x.set("x", 2.0); // Set x = 2
    assert_eq!(evaluate_mathjson(integral, &env_with_x).unwrap(), 8.0); // Integral of 3x^2 is x^3, so result is 8
}

#[test]
fn test_definite_integral_exponential() {
    let env = Environment::new();

    // Definite integral of e^x from 0 to 1
    let integral = json!(["Integral", ["Power", "ExponentialE", "x"], "x", 0, 1]);
    let result = evaluate_mathjson(integral, &env);
    assert_eq!(result.unwrap(), std::f64::consts::E - 1.0); // Integral of e^x from 0 to 1 is e - 1
}

// 3. Limits
#[test]
fn test_limit_at_infinity() {
    let env = Environment::new();

    // Limit of 1/x as x approaches infinity
    let limit = json!(["Limit", ["Divide", 1, "x"], "x", "Infinity"]);
    assert_eq!(evaluate_mathjson(limit, &env).unwrap(), 0.0); // Limit of 1/x as x -> ∞ is 0
}

#[test]
fn test_limit_at_zero() {
    let env = Environment::new();

    // Limit of sin(x)/x as x approaches 0
    let limit = json!(["Limit", ["Divide", ["Sin", "x"], "x"], "x", 0]);
    assert_eq!(evaluate_mathjson(limit, &env).unwrap(), 1.0); // The limit is 1
}

// 4. Series Expansions
#[test]
fn test_taylor_series_expansion() {
    let env = Environment::new();

    // First-order Taylor series expansion of e^x at x=0
    let series = json!(["Series", ["Power", "ExponentialE", "x"], "x", 1]);
    let mut env_with_x = env.clone();
    env_with_x.set("x", 1.0); // Set x = 1
    assert_eq!(evaluate_mathjson(series, &env_with_x).unwrap(), 2.718); // First-order Taylor expansion should give e^1 = 2.718
}

// 5. Partial Derivatives
#[test]
fn test_partial_derivative() {
    let env = Environment::new();

    // Partial derivative of x^2 + y^2 with respect to x
    let partial_derivative = json!([
        "PartialDerivative",
        ["Add", ["Power", "x", 2], ["Power", "y", 2]],
        "x"
    ]);
    let mut env_with_xy = env.clone();
    env_with_xy.set("x", 1.0);
    env_with_xy.set("y", 2.0);
    assert_eq!(
        evaluate_mathjson(partial_derivative, &env_with_xy).unwrap(),
        2.0
    ); // Partial derivative with respect to x is 2x = 2
}

// 6. Differential Equations
#[test]
fn test_first_order_differential_equation() {
    let env = Environment::new();

    // Solve dy/dx = 2x for y(0) = 1
    let diff_eq = json!([
        "SolveDifferentialEquation",
        ["Derivative", "y", "x", 1],
        ["Multiply", 2, "x"],
        "y",
        0,
        1
    ]);
    let mut env_with_x = env.clone();
    env_with_x.set("x", 1.0); // Set x = 1
    assert_eq!(evaluate_mathjson(diff_eq, &env_with_x).unwrap(), 1.0 + 1.0); // The solution is y = x^2 + 1, so y(1) = 2
}

#[test]
fn test_second_order_differential_equation() {
    let env = Environment::new();

    // Solve d^2y/dx^2 = -y with y(0) = 0, y'(0) = 1 (Simple Harmonic Oscillator)
    let diff_eq = json!([
        "SolveDifferentialEquation",
        ["Derivative", "y", "x", 2],
        ["Multiply", -1, "y"],
        "y",
        0,
        0,
        1
    ]);
    let mut env_with_x = env.clone();
    env_with_x.set("x", std::f64::consts::PI / 2.0); // Set x = π/2
    assert_eq!(evaluate_mathjson(diff_eq, &env_with_x).unwrap(), 1.0); // The solution should be y = sin(x), so y(π/2) = 1
}
