// Strict argument validation at the MCP boundary.
//
// Contract under test: every tool's arguments are validated against its
// published inputSchema BEFORE dispatch. A type mismatch, an unknown
// field, or a missing required field is a JSON-RPC -32602 error whose
// message names the offending key — never a silent ignore. The disease
// this prevents: a wrong-typed argument is dropped, the tool runs
// without it, and the caller draws a conclusion from an answer to a
// question they didn't ask.

use arithma_mcp_server::handle_tools_call;
use serde_json::{json, Value};

fn call(tool: &str, args: Value) -> Value {
    handle_tools_call(Some(json!(1)), &json!({"name": tool, "arguments": args}))
}

/// Assert the response is a JSON-RPC invalid-params error naming `key`.
fn assert_rejects(resp: &Value, key: &str) {
    assert!(
        resp.get("result").is_none(),
        "expected protocol error, got result: {}",
        resp
    );
    assert_eq!(resp["error"]["code"], json!(-32602), "resp: {}", resp);
    let msg = resp["error"]["message"].as_str().unwrap();
    assert!(
        msg.contains(key),
        "message must name the offending key '{}': {}",
        key,
        msg
    );
}

// ── The founding case ──────────────────────────────────────────────────
// evaluate with string-typed variable values used to be silently ignored:
// the tool ran unbound and returned an answer to a different question.

#[test]
fn evaluate_string_variable_value_is_rejected() {
    let resp = call(
        "evaluate",
        json!({"expr": "x^2 + 1", "variables": {"x": "3"}}),
    );
    assert_rejects(&resp, "variables.x");
    let msg = resp["error"]["message"].as_str().unwrap();
    assert!(msg.contains("number"), "must state expected type: {}", msg);
}

#[test]
fn evaluate_well_typed_variables_still_work() {
    let resp = call(
        "evaluate",
        json!({"expr": "x^2 + 1", "variables": {"x": 3}}),
    );
    assert!(resp["result"]["isError"].is_null(), "errored: {}", resp);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert_eq!(text, "10");
}

// ── Unknown fields ─────────────────────────────────────────────────────
// A typo'd field name silently reverts the tool to default behavior —
// the caller believes the option took effect.

#[test]
fn unknown_top_level_field_is_rejected() {
    let resp = call("simplify", json!({"expr": "x + x", "exprs": "y"}));
    assert_rejects(&resp, "exprs");
}

#[test]
fn evaluate_typoed_variables_key_is_rejected() {
    let resp = call("evaluate", json!({"expr": "x^2", "vars": {"x": 3}}));
    assert_rejects(&resp, "vars");
}

// ── Missing / wrong-typed required fields ──────────────────────────────

#[test]
fn missing_required_field_is_json_rpc_error() {
    let resp = call("simplify", json!({}));
    assert_rejects(&resp, "expr");
}

#[test]
fn wrong_typed_required_field_is_rejected() {
    let resp = call("simplify", json!({"expr": 5}));
    assert_rejects(&resp, "expr");
    let msg = resp["error"]["message"].as_str().unwrap();
    assert!(msg.contains("string"), "must state expected type: {}", msg);
}

#[test]
fn arguments_must_be_an_object() {
    let resp = call("simplify", json!("x + x"));
    assert!(
        resp.get("result").is_none(),
        "expected protocol error, got result: {}",
        resp
    );
    assert_eq!(resp["error"]["code"], json!(-32602), "resp: {}", resp);
}

// ── Arrays with wrong-typed items ──────────────────────────────────────
// A non-string item in solve_system's variables used to silently become
// "x"; a non-array `variables` in verify was silently ignored entirely.

#[test]
fn verify_variables_must_be_an_array() {
    let resp = call(
        "verify",
        json!({"expr_a": "n + n", "expr_b": "2n", "variables": "n"}),
    );
    assert_rejects(&resp, "variables");
}

#[test]
fn verify_non_string_variable_item_is_rejected() {
    let resp = call(
        "verify",
        json!({"expr_a": "n + n", "expr_b": "2n", "variables": [1]}),
    );
    assert_rejects(&resp, "variables[0]");
}

#[test]
fn solve_system_non_string_variable_item_is_rejected() {
    let resp = call(
        "solve_system",
        json!({"equations": ["x + y = 3", "x - y = 1"], "variables": ["x", 5]}),
    );
    assert_rejects(&resp, "variables[1]");
}

// ── Integer fields ─────────────────────────────────────────────────────
// A wrong-typed `order` used to silently become the default: the caller
// asked for order 8, got order 5, and read the truncation as mathematics.

#[test]
fn taylor_string_order_is_rejected() {
    let resp = call("taylor_series", json!({"expr": "e^x", "order": "8"}));
    assert_rejects(&resp, "order");
}

#[test]
fn taylor_negative_order_is_rejected() {
    let resp = call("taylor_series", json!({"expr": "e^x", "order": -3}));
    assert_rejects(&resp, "order");
}

#[test]
fn taylor_integral_float_order_is_honored() {
    // JSON producers (Python among them) serialize integral floats as 7.0.
    // JSON Schema's "integer" admits them; so must we — and the value must
    // actually take effect, not fall back to the default of 5.
    let resp = call("taylor_series", json!({"expr": "e^x", "order": 7.0}));
    assert!(resp["result"]["isError"].is_null(), "errored: {}", resp);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        text.contains("5040"),
        "order 7 must produce the x^7/7! term (5040): {}",
        text
    );
}

// ── Enums ──────────────────────────────────────────────────────────────

#[test]
fn matrix_unknown_operation_names_allowed_values() {
    let resp = call(
        "matrix",
        json!({"operation": "det", "matrix": "\\begin{pmatrix} 1 & 2 \\\\ 3 & 4 \\end{pmatrix}"}),
    );
    assert_rejects(&resp, "operation");
    let msg = resp["error"]["message"].as_str().unwrap();
    assert!(
        msg.contains("determinant"),
        "must list allowed values: {}",
        msg
    );
}

// ── Assumptions map ────────────────────────────────────────────────────

#[test]
fn assumptions_value_must_be_an_array() {
    let resp = call(
        "simplify",
        json!({"expr": "\\sqrt{x^2}", "assumptions": {"x": "positive"}}),
    );
    assert_rejects(&resp, "assumptions.x");
}

#[test]
fn assumptions_unknown_property_is_rejected() {
    let resp = call(
        "simplify",
        json!({"expr": "\\sqrt{x^2}", "assumptions": {"x": ["positiv"]}}),
    );
    assert_rejects(&resp, "assumptions.x");
}

#[test]
fn explicit_null_optional_field_is_accepted() {
    // Pre-existing contract: a null optional field means "not provided".
    let resp = call("simplify", json!({"expr": "x + x", "assumptions": null}));
    assert!(resp["result"]["isError"].is_null(), "errored: {}", resp);
}

// ── verify_chain steps ─────────────────────────────────────────────────
// A typo'd field inside a step used to be ignored; a typo in "relation"
// would silently check the wrong relation (the default, equals).

#[test]
fn verify_chain_unknown_step_field_is_rejected() {
    let resp = call(
        "verify_chain",
        json!({"steps": [
            {"expr": "(x+1)^2"},
            {"expr": "x^2+2x+1", "relaton": "equals"}
        ]}),
    );
    assert_rejects(&resp, "relaton");
}

#[test]
fn verify_chain_invalid_relation_is_rejected() {
    let resp = call(
        "verify_chain",
        json!({"steps": [
            {"expr": "(x+1)^2"},
            {"expr": "x^2+2x+1", "relation": "equal"}
        ]}),
    );
    assert_rejects(&resp, "relation");
    let msg = resp["error"]["message"].as_str().unwrap();
    assert!(msg.contains("equals"), "must list allowed values: {}", msg);
}

#[test]
fn verify_chain_non_string_step_value_is_rejected() {
    let resp = call(
        "verify_chain",
        json!({"steps": [
            {"expr": "x^2"},
            {"expr": "4", "relation": "substitution", "variable": "x", "value": 2}
        ]}),
    );
    assert_rejects(&resp, "value");
}

// ── solve_ode nested arrays ────────────────────────────────────────────

#[test]
fn solve_ode_non_numeric_poly_coeff_is_rejected() {
    let resp = call("solve_ode", json!({"poly_coeffs": [["a"], [1]]}));
    assert_rejects(&resp, "poly_coeffs[0][0]");
}

#[test]
fn solve_ode_string_coefficient_is_rejected() {
    let resp = call("solve_ode", json!({"a": "1", "b": 0.0, "c": -1.0}));
    assert_rejects(&resp, "a");
}

// ── Widened contract: limit accepts numeric points ─────────────────────
// The implementation always accepted a numeric `point`; the schema said
// string-only. Behavior and schema must agree — the schema widens.

#[test]
fn limit_numeric_point_is_accepted() {
    let resp = call("limit", json!({"expr": "\\frac{\\sin(x)}{x}", "point": 0}));
    assert!(resp["result"]["isError"].is_null(), "errored: {}", resp);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    // The status marker line may precede the value; the value is the last line.
    assert_eq!(text.lines().last().unwrap(), "1");
}

// ── Multivariate taylor center: number must not be silently dropped ────
// A numeric center with comma-separated variables used to fall back to
// the origin silently: center 2 expanded around 0 without a word.

#[test]
fn taylor_multivar_numeric_center_is_honored() {
    let resp = call(
        "taylor_series",
        json!({"expr": "x^3 y", "variable": "x,y", "center": 1, "order": 2}),
    );
    assert!(resp["result"]["isError"].is_null(), "errored: {}", resp);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    // x³y truncated at total degree 2 around the ORIGIN is exactly 0;
    // around (1,1) it is not. A zero here means the numeric center was
    // silently dropped to the default origin.
    assert_ne!(
        text, "0",
        "center 1 must not be silently dropped to the origin"
    );
}
