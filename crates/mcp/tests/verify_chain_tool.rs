//! MCP-layer tests for the verify_chain tool and the machine-readable
//! `verdict` field (an agent must never have to parse prose to
//! learn whether a check passed).

use serde_json::{json, Value};

fn call(tool: &str, arguments: Value) -> Value {
    let params = json!({ "name": tool, "arguments": arguments });
    let response = arithma_mcp_server::handle_tools_call(Some(json!(1)), &params);
    response["result"].clone()
}

fn status_of(result: &Value) -> &Value {
    &result["result_status"]
}

#[test]
fn verify_chain_pass_reports_exact_status_and_verdict() {
    let result = call(
        "verify_chain",
        json!({
            "steps": [
                { "label": "start", "expr": "(x+1)^2" },
                { "label": "expand", "expr": "x^2 + 2x + 1", "relation": "equals" },
                { "label": "derivative", "expr": "2x + 2", "relation": "derivative_of", "variable": "x" }
            ]
        }),
    );
    let status = status_of(&result);
    assert_eq!(status["status"], "exact");
    assert_eq!(status["verdict"], "pass");

    // Per-step detail is machine-readable: verdict and mechanism per step.
    let steps = status["steps"].as_array().expect("steps array");
    assert_eq!(steps.len(), 3);
    assert_eq!(steps[0]["mechanism"], "anchor");
    assert_eq!(steps[1]["verdict"], "pass");
    assert_eq!(steps[1]["relation"], "equals");
    assert!(steps[2]["mechanism"]
        .as_str()
        .unwrap()
        .starts_with("derivative_rules"));

    // Text is human-readable and leads with the chain verdict.
    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(text.starts_with("Chain: PASS"), "got: {}", text);
}

#[test]
fn verify_chain_failure_carries_first_failure_and_counterexample() {
    let result = call(
        "verify_chain",
        json!({
            "steps": [
                { "expr": "(x+1)^2" },
                { "expr": "x^2 + 1" }
            ]
        }),
    );
    let status = status_of(&result);
    assert_eq!(status["verdict"], "fail");
    assert_eq!(status["first_failure"], 1);
    let steps = status["steps"].as_array().unwrap();
    assert_eq!(steps[1]["verdict"], "fail");
    assert!(steps[1]["status"]["counterexample"].is_object());

    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(text.starts_with("Chain: FAIL"), "got: {}", text);
}

#[test]
fn verify_chain_respects_assumptions() {
    // sqrt(x^2) = x holds only for x >= 0; with the assumption the chain
    // passes on the assumed domain.
    let result = call(
        "verify_chain",
        json!({
            "steps": [
                { "expr": "\\sqrt{x^2}" },
                { "expr": "x" }
            ],
            "assumptions": { "x": ["positive"] }
        }),
    );
    assert_eq!(status_of(&result)["verdict"], "pass");
}

#[test]
fn verify_chain_empty_steps_is_a_protocol_error() {
    let params = json!({ "name": "verify_chain", "arguments": { "steps": [] } });
    let response = arithma_mcp_server::handle_tools_call(Some(json!(1)), &params);
    assert_eq!(response["result"]["isError"], true);
}

#[test]
fn verify_chain_unknown_relation_is_a_protocol_error() {
    let params = json!({
        "name": "verify_chain",
        "arguments": { "steps": [
            { "expr": "x" },
            { "expr": "x", "relation": "proves" }
        ]}
    });
    let response = arithma_mcp_server::handle_tools_call(Some(json!(1)), &params);
    assert_eq!(response["result"]["isError"], true);
    // The error names the offending relation and the valid vocabulary.
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("proves"), "got: {}", text);
}

#[test]
fn verify_chain_is_listed_in_the_tool_schema() {
    let response = arithma_mcp_server::handle_tools_list(Some(json!(1)));
    let tools = response["result"]["tools"].as_array().unwrap();
    assert!(tools.iter().any(|t| t["name"] == "verify_chain"));
}

#[test]
fn verify_tool_gains_machine_readable_verdict() {
    let pass = call(
        "verify",
        json!({ "expr_a": "(x+1)^2", "expr_b": "x^2+2x+1" }),
    );
    assert_eq!(status_of(&pass)["verdict"], "pass");

    let fail = call("verify", json!({ "expr_a": "x^2", "expr_b": "x^3" }));
    assert_eq!(status_of(&fail)["verdict"], "fail");
    assert_eq!(status_of(&fail)["status"], "verified");
}

#[test]
fn equivalent_tool_gains_machine_readable_verdict() {
    let pass = call(
        "equivalent",
        json!({ "expr_a": "\\frac{x^2-1}{x-1}", "expr_b": "x+1" }),
    );
    assert_eq!(status_of(&pass)["verdict"], "pass");

    let fail = call("equivalent", json!({ "expr_a": "x^2", "expr_b": "x^3" }));
    assert_eq!(status_of(&fail)["verdict"], "fail");
}

#[test]
fn verify_chain_response_carries_build_provenance() {
    // A replay verdict without build provenance is not reproducible: the
    // chain-level result_status must say which checker build produced it.
    let result = call(
        "verify_chain",
        json!({
            "steps": [
                { "expr": "(x+1)^2" },
                { "expr": "x^2 + 2x + 1" }
            ]
        }),
    );
    let build = &status_of(&result)["build"];

    // The commit is a non-empty identifier ("unknown" only when the
    // binary was built outside a git checkout).
    let commit = build["commit"].as_str().expect("build.commit is a string");
    assert!(!commit.is_empty());

    // The dirty flag is always present and boolean: a dirty build must be
    // distinguishable from a clean one in every chain report.
    assert!(
        build["dirty"].is_boolean(),
        "build.dirty must be a boolean, got: {}",
        build["dirty"]
    );
}
