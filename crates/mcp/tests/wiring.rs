// Wiring tests for the result_status field (docs/result-status.md).
//
// Contract under test:
//   1. Every tool's JSON-RPC result carries `result_status`.
//   2. Happy-path text (exact / verified) is byte-compatible with the
//      pre-status behavior — no markers, no new lines.
//   3. Loud statuses (heuristic, unable_to_compute, provably_impossible)
//      put a bracketed marker in the text.

use arithma_mcp_server::handle_tools_call;
use serde_json::{json, Value};

fn call(tool: &str, args: Value) -> Value {
    handle_tools_call(Some(json!(1)), &json!({"name": tool, "arguments": args}))
}

#[test]
fn every_tool_reports_result_status() {
    let cases: Vec<(&str, Value)> = vec![
        ("format", json!({"expr": "x + x"})),
        ("simplify", json!({"expr": "x^2 + 2x + 1"})),
        ("differentiate", json!({"expr": "x^3 + \\sin(x)"})),
        ("integrate", json!({"expr": "x^2"})),
        (
            "substitute",
            json!({"expr": "x^2", "variable": "x", "value": "3"}),
        ),
        ("solve", json!({"equation": "x^2 - 4 = 0"})),
        (
            "solve_system",
            json!({"equations": ["x + y = 3", "x - y = 1"], "variables": ["x", "y"]}),
        ),
        ("factor", json!({"expr": "x^2 - 1"})),
        (
            "partial_fractions",
            json!({"numerator": "1", "denominator": "x^2 - 1"}),
        ),
        (
            "limit",
            json!({"expr": "\\frac{\\sin(x)}{x}", "point": "0"}),
        ),
        ("taylor_series", json!({"expr": "e^x", "order": 3})),
        ("evaluate", json!({"expr": "2 + 2"})),
        (
            "matrix",
            json!({"operation": "determinant", "matrix": "\\begin{pmatrix} 1 & 2 \\\\ 3 & 4 \\end{pmatrix}"}),
        ),
        ("equivalent", json!({"expr_a": "x + x", "expr_b": "2x"})),
        (
            "verify",
            json!({"expr_a": "2\\sin(x)\\cos(x)", "expr_b": "\\sin(2x)"}),
        ),
        ("solve_ode", json!({"a": 1.0, "b": 0.0, "c": -1.0})),
    ];
    for (tool, args) in cases {
        let resp = call(tool, args);
        assert!(
            resp["result"]["isError"].is_null(),
            "{} errored: {}",
            tool,
            resp
        );
        let status = resp["result"]["result_status"]["status"]
            .as_str()
            .unwrap_or_else(|| panic!("{} missing result_status: {}", tool, resp));
        assert!(!status.is_empty(), "{} has empty status", tool);
    }
}

#[test]
fn protocol_errors_carry_no_status() {
    let resp = call("simplify", json!({}));
    assert_eq!(resp["result"]["isError"], json!(true));
    assert!(resp["result"].get("result_status").is_none());
}

#[test]
fn integrate_nonelementary_is_provably_impossible_and_loud() {
    let resp = call("integrate", json!({"expr": "e^{x^2}"}));
    assert_eq!(
        resp["result"]["result_status"]["status"],
        "provably_impossible"
    );
    assert!(resp["result"]["result_status"]["proof_certificate"]
        .as_object()
        .is_some());
    assert!(
        !resp["result"]["result_status"]["proof_certificate"]["method"]
            .as_str()
            .unwrap()
            .is_empty()
    );
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        text.starts_with("[provably impossible]"),
        "text must carry the marker: {}",
        text
    );
}

// ── Impossibility proof tests ──────────────────────────────────────────
// Each exercises the full pipeline: tool call → structured ProofCertificate
// with method, reason, and explanation fields.

#[test]
fn integrate_exp_x2_risch_de_certificate() {
    let resp = call("integrate", json!({"expr": "e^{x^2}"}));
    let status = &resp["result"]["result_status"];
    assert_eq!(status["status"], "provably_impossible");
    let cert = &status["proof_certificate"];
    assert_eq!(cert["method"], "risch-de");
    assert!(!cert["reason"].as_str().unwrap().is_empty());
    assert!(
        cert["explanation"]
            .as_str()
            .unwrap()
            .contains("elementary functions"),
        "explanation should mention elementary functions"
    );
}

#[test]
fn integrate_exp_x_over_x_rothstein_trager_certificate() {
    let resp = call("integrate", json!({"expr": "\\frac{e^x}{x}"}));
    let status = &resp["result"]["result_status"];
    assert_eq!(status["status"], "provably_impossible");
    let cert = &status["proof_certificate"];
    assert!(
        cert["method"] == "rothstein-trager" || cert["method"] == "risch-de",
        "method should be rothstein-trager or risch-de, got: {}",
        cert["method"]
    );
    assert!(
        status.get("special_function").is_some(),
        "should recognize Ei"
    );
}

#[test]
fn integrate_exp_neg_x2_erf_special_form() {
    let resp = call("integrate", json!({"expr": "e^{-x^2}"}));
    let status = &resp["result"]["result_status"];
    assert_eq!(status["status"], "provably_impossible");
    assert!(status["proof_certificate"].as_object().is_some());
    assert_eq!(status["special_function"], "erf");
    assert!(!status["special_form"].as_str().unwrap().is_empty());
}

#[test]
fn solve_negative_discriminant_is_provably_impossible() {
    let resp = call("solve", json!({"equation": "x^2 + 1 = 0"}));
    let status = &resp["result"]["result_status"];
    assert_eq!(status["status"], "provably_impossible");
    let cert = &status["proof_certificate"];
    assert_eq!(cert["method"], "negative-discriminant");
    assert!(
        cert["explanation"]
            .as_str()
            .unwrap()
            .contains("no real solutions"),
        "explanation should state no real solutions"
    );
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("No real solutions"), "text: {}", text);
}

#[test]
fn solve_contradiction_is_provably_impossible() {
    // 0 = 1 has no solutions.
    let resp = call("solve", json!({"equation": "0 = 1"}));
    let status = &resp["result"]["result_status"];
    assert_eq!(status["status"], "provably_impossible");
    let cert = &status["proof_certificate"];
    assert_eq!(cert["method"], "contradiction");
    assert!(
        cert["explanation"]
            .as_str()
            .unwrap()
            .contains("contradiction"),
        "explanation should mention contradiction"
    );
}

#[test]
fn solve_quartic_all_complex_is_provably_impossible() {
    // x^4 + 1 = 0 has no real roots (all 4 roots are complex).
    let resp = call("solve", json!({"equation": "x^4 + 1 = 0"}));
    let status = &resp["result"]["result_status"];
    assert_eq!(status["status"], "provably_impossible");
    let cert = &status["proof_certificate"];
    assert_eq!(cert["method"], "all-roots-complex");
    assert!(cert["explanation"].as_str().unwrap().contains("4 roots"));
}

#[test]
fn solve_quintic_irreducible_is_unable_to_compute_not_impossible() {
    // x^5 - 2 = 0 has real root ⁵√2 — the solver can't express it, but
    // that is NOT a proof of impossibility (Abel-Ruffini requires a
    // non-solvable Galois group, not just degree ≥ 5).
    let resp = call("solve", json!({"equation": "x^5 - 2 = 0"}));
    let status = &resp["result"]["result_status"];
    assert_eq!(
        status["status"], "unable_to_compute",
        "irreducible quintic must NOT be provably_impossible — x^5-2 has root ⁵√2"
    );
    assert!(
        status.get("proof_certificate").is_none(),
        "unable_to_compute must not carry a proof_certificate"
    );
}

#[test]
fn simplify_polynomial_is_exact_with_bare_text() {
    let resp = call("simplify", json!({"expr": "\\frac{x^2 - 1}{x - 1}"}));
    assert_eq!(resp["result"]["result_status"]["status"], "exact");
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        !text.contains('[') && !text.contains('\n'),
        "happy-path text must stay bare: {}",
        text
    );
}

#[test]
fn simplify_transcendental_rewrite_is_verified_with_points() {
    let resp = call("simplify", json!({"expr": "\\sin(x) + \\sin(x)"}));
    assert_eq!(resp["result"]["result_status"]["status"], "verified");
    assert!(
        resp["result"]["result_status"]["points_tested"]
            .as_u64()
            .unwrap()
            >= 3
    );
}

#[test]
fn integrate_happy_path_is_exact_with_bare_text() {
    let resp = call("integrate", json!({"expr": "x^2"}));
    assert_eq!(resp["result"]["result_status"]["status"], "exact");
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(!text.contains('['), "text: {}", text);
}

#[test]
fn verify_pass_keeps_legacy_text_and_reports_points() {
    let resp = call(
        "verify",
        json!({"expr_a": "2\\sin(x)\\cos(x)", "expr_b": "\\sin(2x)"}),
    );
    assert_eq!(resp["result"]["result_status"]["status"], "verified");
    assert!(
        resp["result"]["result_status"]["points_tested"]
            .as_u64()
            .unwrap()
            >= 3
    );
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.starts_with("Verified: PASS"), "legacy text: {}", text);
}

#[test]
fn verify_fail_carries_counterexample_in_status() {
    let resp = call(
        "verify",
        json!({"expr_a": "\\sin(x)", "expr_b": "\\cos(x)"}),
    );
    assert_eq!(resp["result"]["result_status"]["status"], "verified");
    assert!(
        resp["result"]["result_status"]["counterexample"]["point"].is_object(),
        "counterexample: {}",
        resp["result"]["result_status"]
    );
}

#[test]
fn result_status_reaches_typed_consumers_via_structured_content() {
    // F3: typed MCP SDKs drop unknown TOP-LEVEL fields, so a sibling
    // result_status never reaches them — Claude Code itself strips it.
    // structuredContent is the standards-track surface; the sibling stays
    // for existing raw-JSON consumers. Both must carry the same object.
    for (tool, args) in [
        ("simplify", json!({"expr": "x^2 + 2x + 1"})),
        (
            "verify",
            json!({"expr_a": "\\sin(x)", "expr_b": "\\cos(x)"}),
        ),
        (
            "verify_chain",
            json!({"steps": [{"expr": "(x+1)^2"}, {"expr": "x^2+2x+1"}]}),
        ),
        ("evaluate", json!({"expr": "e^{-50}"})),
    ] {
        let resp = call(tool, args);
        let sibling = &resp["result"]["result_status"];
        let structured = &resp["result"]["structuredContent"]["result_status"];
        assert!(!sibling.is_null(), "{}: sibling missing", tool);
        assert_eq!(
            structured, sibling,
            "{}: structuredContent must mirror the sibling exactly",
            tool
        );
    }
}

#[test]
fn evaluate_unevaluated_echo_is_unable_to_compute() {
    // F8: an unevaluated echo is not a candidate result — the request was
    // a number and no number was produced. heuristic says "use with
    // suspicion"; the honest status is unable_to_compute, with the
    // simplified form preserved in the text for the caller's benefit.
    let resp = call("evaluate", json!({"expr": "x + 1"}));
    let status = &resp["result"]["result_status"];
    assert_eq!(status["status"], "unable_to_compute", "status: {}", status);
    assert_eq!(status["caveats"][0]["code"], "unevaluated");
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        text.contains("x + 1"),
        "simplified form must still reach the caller: {}",
        text
    );
}

#[test]
fn evaluate_float_path_is_approximate_with_digits() {
    // e^{-50} is tiny but well-conditioned: ~13 trustworthy digits.
    let resp = call("evaluate", json!({"expr": "e^{-50}"}));
    let status = &resp["result"]["result_status"];
    assert_eq!(status["status"], "approximate", "status: {}", status);
    let digits = status["significant_digits"].as_u64().unwrap();
    assert!(digits >= 10, "digits: {}", digits);
}

#[test]
fn evaluate_catastrophic_cancellation_refuses_loudly() {
    // (1 - cos x)/x² at x = 1e-8: f64 computes 0, true value ½. A value
    // with zero significant digits is not a result — it must be refused
    // with the mechanism named, never presented with a generic caveat.
    let resp = call(
        "evaluate",
        json!({"expr": "\\frac{1 - \\cos(x)}{x^2}", "variables": {"x": 1e-8}}),
    );
    let status = &resp["result"]["result_status"];
    assert_eq!(status["status"], "unable_to_compute", "status: {}", status);
    assert_eq!(
        status["caveats"][0]["code"], "catastrophic_cancellation",
        "status: {}",
        status
    );
}

#[test]
fn evaluate_exact_path_is_exact() {
    let resp = call("evaluate", json!({"expr": "2 + 2"}));
    assert_eq!(resp["result"]["result_status"]["status"], "exact");
}

#[test]
fn taylor_series_is_exact_with_truncation_caveat() {
    let resp = call("taylor_series", json!({"expr": "e^x", "order": 3}));
    assert_eq!(resp["result"]["result_status"]["status"], "exact");
    let caveats = &resp["result"]["result_status"]["caveats"];
    // F5 contract: caveats are {code, message} pairs — the code is the
    // machine surface, the message the human one.
    assert_eq!(caveats[0]["code"], "truncation", "caveats: {}", caveats);
    assert!(
        caveats[0]["message"].as_str().unwrap().contains("order 3"),
        "caveats: {}",
        caveats
    );
}

#[test]
fn limit_numeric_claim_is_verified() {
    let resp = call(
        "limit",
        json!({"expr": "\\frac{\\sin(x)}{x}", "point": "0"}),
    );
    assert_eq!(resp["result"]["result_status"]["status"], "verified");
}

#[test]
fn matrix_numeric_eigenvalues_are_not_exact() {
    // The eigenvalue routine is numeric root-finding; its floats
    // must not wear the exact badge. (Companion matrix of x³−x−1.)
    let resp = call(
        "matrix",
        json!({"operation": "eigenvalues", "matrix": "\\begin{pmatrix} 0 & 0 & 1 \\\\ 1 & 0 & 1 \\\\ 0 & 1 & 0 \\end{pmatrix}"}),
    );
    let status = &resp["result"]["result_status"];
    assert_ne!(status["status"], "exact", "floats wearing exact: {}", resp);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        text.contains('i'),
        "complex pair must be explicit: {}",
        text
    );
}

#[test]
fn matrix_exact_operations_stay_exact() {
    let resp = call(
        "matrix",
        json!({"operation": "determinant", "matrix": "\\begin{pmatrix} 1 & 2 \\\\ 3 & 4 \\end{pmatrix}"}),
    );
    assert_eq!(resp["result"]["result_status"]["status"], "exact");
}

#[test]
fn solve_numeric_cubic_roots_are_not_exact() {
    // x³−x−1 solves via numeric root-finding (f64), and the
    // float must not wear the exact badge. x²=2 stays symbolic → exact.
    let resp = call("solve", json!({"equation": "x^3 - x - 1 = 0"}));
    assert_ne!(
        resp["result"]["result_status"]["status"], "exact",
        "f64 roots wearing exact: {}",
        resp
    );
    let resp2 = call("solve", json!({"equation": "x^2 = 2"}));
    assert_eq!(resp2["result"]["result_status"]["status"], "exact");
}

#[test]
fn definite_integral_over_pole_is_refused() {
    // ∫₋₁² dx/x diverges (non-integrable pole at 0); the FTC
    // path must not hand out ln(2) as exact.
    let resp = call(
        "integrate",
        json!({"expr": "\\frac{1}{x}", "lower": "-1", "upper": "2"}),
    );
    let ok_refusal = resp["result"]["isError"] == json!(true)
        || resp["result"]["result_status"]["status"] == "unable_to_compute";
    assert!(ok_refusal, "divergent integral not refused: {}", resp);
}

#[test]
fn definite_integral_with_pole_outside_interval_still_works() {
    let resp = call(
        "integrate",
        json!({"expr": "\\frac{1}{x^2}", "lower": "1", "upper": "2"}),
    );
    assert!(resp["result"]["isError"].is_null(), "errored: {}", resp);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert_eq!(text, "\\frac{1}{2}", "∫₁² dx/x² = 1/2, got {}", text);
}
