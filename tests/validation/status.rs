// Tests for the result-status taxonomy (docs/result-status.md).
//
// The JSON shapes asserted here are the schema contract consumed by MCP
// clients and, next sprint, by verify_chain. Changing a shape is a
// contract change; add fields instead.

use arithma::status::{
    caveat_codes, classify_integral, classify_simplify, is_algebraic_exact, Certificate,
    ProofCertificate, ResultStatus, StatusReport,
};
use arithma::{parse_latex, parse_latex_raw, Environment};

#[test]
fn exact_report_serializes_with_certificate() {
    let cert = Certificate::by_construction("test_algorithm");
    let json = StatusReport::exact(cert).to_json();
    assert_eq!(json["status"], "exact");
    assert!(json.get("points_tested").is_none());
    assert!(json.get("certificate").is_some());
    assert_eq!(json["certificate"]["kind"], "decision_procedure");
    assert_eq!(json["certificate"]["witness"], "test_algorithm");
    assert_eq!(json["certificate"]["checked"], true);
}

#[test]
fn exact_replay_certificate_serializes() {
    let cert = Certificate::replay("factor_multiply_back", "product equals input");
    let json = StatusReport::exact(cert).to_json();
    assert_eq!(json["status"], "exact");
    assert_eq!(json["certificate"]["kind"], "factor_multiply_back");
    assert_eq!(json["certificate"]["witness"], "product equals input");
    assert_eq!(json["certificate"]["checked"], true);
}

#[test]
fn verified_report_carries_point_count() {
    let json = StatusReport::verified(12).to_json();
    assert_eq!(json["status"], "verified");
    assert_eq!(json["points_tested"], 12);
}

#[test]
fn heuristic_report_serializes() {
    let json = StatusReport::heuristic().to_json();
    assert_eq!(json["status"], "heuristic");
}

#[test]
fn unable_to_compute_carries_reason() {
    let json = StatusReport::unable_to_compute("only 1 valid test point in domain").to_json();
    assert_eq!(json["status"], "unable_to_compute");
    assert_eq!(json["reason"], "only 1 valid test point in domain");
}

#[test]
fn provably_impossible_carries_structured_proof_certificate() {
    let proof = ProofCertificate::new(
        "risch-de",
        "The DE q' + f·q = g has no rational solution",
        "No elementary antiderivative exists.",
    );
    let json = StatusReport::provably_impossible(proof).to_json();
    assert_eq!(json["status"], "provably_impossible");
    assert_eq!(json["proof_certificate"]["method"], "risch-de");
    assert_eq!(
        json["proof_certificate"]["reason"],
        "The DE q' + f·q = g has no rational solution"
    );
    assert_eq!(
        json["proof_certificate"]["explanation"],
        "No elementary antiderivative exists."
    );
}

#[test]
fn caveats_serialize_as_code_message_pairs() {
    // F5: consumers must never have to regex prose — every caveat carries
    // a stable machine code alongside the human message.
    let json = StatusReport::verified(1)
        .with_caveat(
            arithma::status::caveat_codes::F64_PRECISION,
            "floating-point evaluation (f64)",
        )
        .to_json();
    assert_eq!(json["caveats"][0]["code"], "f64_precision");
    assert_eq!(
        json["caveats"][0]["message"],
        "floating-point evaluation (f64)"
    );
}

#[test]
fn uncertified_exact_downgrade_carries_coded_caveat() {
    // The to_json gate's own caveat obeys the same contract.
    // exact() requires a certificate; an UNCHECKED one must be downgraded
    // at the to_json gate.
    let report = StatusReport::exact(Certificate {
        kind: "test".to_string(),
        witness: "test".to_string(),
        checked: false,
    });
    let json = report.to_json();
    assert_eq!(json["status"], "heuristic");
    assert_eq!(json["caveats"][0]["code"], "uncertified_exact");
}

// Text markers: quiet statuses produce no marker (happy-path output stays
// byte-identical); loud statuses produce a bracketed marker line.

#[test]
fn exact_has_no_marker() {
    assert!(StatusReport::exact(Certificate::by_construction("test"))
        .marker()
        .is_none());
}

#[test]
fn verified_marker_carries_point_count() {
    let m = StatusReport::verified(12).marker().unwrap();
    assert!(m.starts_with("[verified]"), "got: {}", m);
    assert!(m.contains("12"), "should name point count, got: {}", m);
}

#[test]
fn verified_marker_includes_caveats_when_present() {
    let m = StatusReport::verified(1)
        .with_caveat(
            caveat_codes::F64_PRECISION,
            "floating-point evaluation (f64)",
        )
        .marker()
        .unwrap();
    assert!(m.contains("[verified]"), "got: {}", m);
    assert!(
        m.contains("floating-point"),
        "caveat should appear in marker, got: {}",
        m
    );
}

#[test]
fn provably_impossible_marker_includes_explanation() {
    let proof = ProofCertificate::new("risch", "formal reason", "Risch: no solution");
    let m = StatusReport::provably_impossible(proof).marker().unwrap();
    assert_eq!(m, "[provably impossible] Risch: no solution");
}

#[test]
fn special_form_serializes_on_provably_impossible() {
    let proof = ProofCertificate::new(
        "risch",
        "no elementary antiderivative",
        "No elementary antiderivative exists.",
    );
    let json = StatusReport::provably_impossible(proof)
        .with_special_form("erf", "\\frac{\\sqrt{\\pi}}{2}\\erf(x)")
        .to_json();
    assert_eq!(json["status"], "provably_impossible");
    assert_eq!(json["proof_certificate"]["method"], "risch");
    assert_eq!(json["special_function"], "erf");
    assert_eq!(json["special_form"], "\\frac{\\sqrt{\\pi}}{2}\\erf(x)");
}

#[test]
fn special_form_absent_when_not_attached() {
    let proof = ProofCertificate::new("risch", "reason", "explanation");
    let json = StatusReport::provably_impossible(proof).to_json();
    assert!(json.get("special_function").is_none());
    assert!(json.get("special_form").is_none());
}

#[test]
fn provably_impossible_marker_names_special_form_when_present() {
    let proof = ProofCertificate::new(
        "risch",
        "formal reason",
        "no elementary antiderivative exists",
    );
    let m = StatusReport::provably_impossible(proof)
        .with_special_form("erf", "\\frac{\\sqrt{\\pi}}{2}\\erf(x)")
        .marker()
        .unwrap();
    assert_eq!(
        m,
        "[provably impossible] no elementary antiderivative exists — antiderivative in special functions: \\frac{\\sqrt{\\pi}}{2}\\erf(x)"
    );
}

#[test]
fn unable_to_compute_marker_includes_reason() {
    let m = StatusReport::unable_to_compute("insufficient test points")
        .marker()
        .unwrap();
    assert_eq!(m, "[unable to compute] insufficient test points");
}

#[test]
fn unable_to_compute_marker_includes_caveats() {
    // A witness attached as a caveat must reach the wire, not just live
    // in the data structure — "preserved as a caveat" is only true if the
    // marker renderer includes it.
    let m = StatusReport::unable_to_compute("only 0 valid test points")
        .with_caveat(
            caveat_codes::SELF_CHECK_FAILED,
            "the simplified derivative disagreed at {\"x\": 0.5}",
        )
        .marker()
        .unwrap();
    assert!(
        m.contains("disagreed at"),
        "caveats must reach the marker text, got: {}",
        m
    );
}

#[test]
fn heuristic_marker_includes_caveats() {
    let m = StatusReport::heuristic()
        .with_caveat(
            caveat_codes::NOT_CORROBORATED,
            "transcendental rewrite not independently verified",
        )
        .marker()
        .unwrap();
    assert_eq!(
        m,
        "[heuristic] transcendental rewrite not independently verified"
    );
}

#[test]
fn heuristic_marker_has_default_text_without_caveats() {
    let m = StatusReport::heuristic().marker().unwrap();
    assert_eq!(m, "[heuristic] result not independently verified");
}

// --- Detector: which expressions admit exact canonicalization over Q ---

#[test]
fn polynomials_and_rational_functions_are_algebraic_exact() {
    let env = Environment::new();
    for expr in [
        "x^2 + 2x + 1",
        "\\frac{x^2 - 1}{x - 1}",
        "x^{-2} + 3",
        "2/3 + x y",
    ] {
        let node = parse_latex_raw(expr).unwrap();
        assert!(
            is_algebraic_exact(&node),
            "expected algebraic-exact: {}",
            expr
        );
        let _ = env; // silence unused when loop body changes
    }
}

#[test]
fn transcendental_and_non_field_expressions_are_not_algebraic_exact() {
    for expr in ["\\sin(x)", "\\sqrt{x}", "x^{1/2}", "e^x + 1", "x^y"] {
        let node = parse_latex_raw(expr).unwrap();
        assert!(
            !is_algebraic_exact(&node),
            "expected NOT algebraic-exact: {}",
            expr
        );
    }
    // Constructed directly: bare |x| currently tokenizes to plain x (a
    // separate parser issue), so the LaTeX round-trip can't exercise Abs.
    use arithma::Node;
    let abs = Node::Abs(Box::new(Node::Variable("x".to_string())));
    assert!(!is_algebraic_exact(&abs));
}

#[test]
fn float_literals_are_not_algebraic_exact() {
    // 0.5 parses to an exact rational in this codebase IF the tokenizer
    // rationalizes decimals; this test pins whatever the honest answer is:
    // an ExactNum::Float anywhere in the tree disqualifies exactness.
    use arithma::{ExactNum, Node};
    let node = Node::Add(
        Box::new(Node::Num(ExactNum::Float(0.1))),
        Box::new(Node::Variable("x".to_string())),
    );
    assert!(!is_algebraic_exact(&node));
}

// --- classify_simplify: exact for poly/rational, verified for transcendental ---

#[test]
fn simplify_polynomial_classifies_exact() {
    let env = Environment::new();
    let input = parse_latex_raw("x^2 + 2x + 1 - x^2").unwrap();
    let output = parse_latex("x^2 + 2x + 1 - x^2", &env).unwrap();
    let report = classify_simplify(&input, &output, &env);
    assert_eq!(report.status, ResultStatus::Exact);
    assert!(
        report.certificate().is_some(),
        "exact must carry a certificate"
    );
    assert!(
        report.certificate().unwrap().checked,
        "certificate must be checked"
    );
}

#[test]
fn simplify_transcendental_classifies_verified_with_points() {
    // sin(x) + sin(x) → 2 sin(x): a genuine rewrite involving a
    // transcendental atom, so equivalence is certified numerically. (The
    // rewrite itself is sound like-term collection, but without rule-level
    // provenance the classifier must under-claim: verified, not exact.)
    let env = Environment::new();
    let input = parse_latex_raw("\\sin(x) + \\sin(x)").unwrap();
    let output = parse_latex("\\sin(x) + \\sin(x)", &env).unwrap();
    let report = classify_simplify(&input, &output, &env);
    match report.status {
        arithma::status::ResultStatus::Verified { points_tested } => {
            assert!(points_tested >= 3, "too few points: {}", points_tested)
        }
        other => panic!("expected Verified, got {:?}", other),
    }
}

#[test]
fn simplify_identity_transformation_is_exact_even_for_transcendental() {
    // If the simplifier returns the input unchanged, the equivalence claim
    // is trivial (x ≡ x) and needs no numeric evidence.
    let env = Environment::new();
    let node = parse_latex_raw("\\operatorname{atan}(x) + 1").unwrap();
    let report = classify_simplify(&node, &node, &env);
    assert_eq!(report.status, ResultStatus::Exact);
    assert!(report.certificate().is_some());
}

#[test]
fn simplify_self_check_failure_is_loud() {
    // Simulate a simplifier bug: claim x+1 "simplified to" x+2.
    let env = Environment::new();
    let input = parse_latex_raw("\\sin(x) + 1").unwrap();
    let output = parse_latex("\\sin(x) + 2", &env).unwrap();
    let report = classify_simplify(&input, &output, &env);
    assert_eq!(report.status, arithma::status::ResultStatus::Heuristic);
    assert!(
        report
            .caveats
            .iter()
            .any(|c| c.code == caveat_codes::SELF_CHECK_FAILED),
        "caveats should flag the self-check failure: {:?}",
        report.caveats
    );
}

// --- classify_integral: the differentiation round-trip ---

#[test]
fn integral_round_trip_structural_match_is_exact() {
    let env = Environment::new();
    let integrand = parse_latex("x^2", &env).unwrap();
    let antiderivative = parse_latex("\\frac{x^3}{3}", &env).unwrap();
    let report = classify_integral(&integrand, &antiderivative, "x", &env);
    assert_eq!(report.status, ResultStatus::Exact);
    let cert = report.certificate().expect("exact must carry certificate");
    assert_eq!(cert.kind, "differentiation_round_trip");
    assert!(cert.checked);
}

#[test]
fn integral_round_trip_transcendental_is_at_least_verified() {
    let env = Environment::new();
    let integrand = parse_latex("\\cos(x)", &env).unwrap();
    let antiderivative = parse_latex("\\sin(x)", &env).unwrap();
    let report = classify_integral(&integrand, &antiderivative, "x", &env);
    match report.status {
        arithma::status::ResultStatus::Exact | arithma::status::ResultStatus::Verified { .. } => {}
        other => panic!("expected Exact or Verified, got {:?}", other),
    }
}

#[test]
fn integral_round_trip_mismatch_is_loud() {
    let env = Environment::new();
    let integrand = parse_latex("\\cos(x)", &env).unwrap();
    let wrong = parse_latex("\\sin(2x)", &env).unwrap();
    let report = classify_integral(&integrand, &wrong, "x", &env);
    assert_eq!(report.status, arithma::status::ResultStatus::Heuristic);
    assert!(
        report
            .caveats
            .iter()
            .any(|c| c.code == caveat_codes::SELF_CHECK_FAILED && c.message.contains("round-trip")),
        "caveats should flag round-trip failure: {:?}",
        report.caveats
    );
}

// --- classify_verify: the verify tool's verdict statuses ---
// A FAIL verdict is not a degraded result — the counterexample IS the
// evidence for "not equal." Both PASS and FAIL are verified verdicts;
// only insufficient sampling is unable_to_compute.

#[test]
fn verify_pass_maps_to_verified_with_point_count() {
    use arithma::status::classify_verify;
    let env = Environment::new();
    let lhs = parse_latex("2\\sin(x)\\cos(x)", &env).unwrap();
    let rhs = parse_latex("\\sin(2x)", &env).unwrap();
    let result = arithma::verify_identity(
        &lhs,
        &rhs,
        &["x".to_string()],
        &arithma::assumptions::Assumptions::new(),
    );
    let report = classify_verify(&result);
    match report.status {
        arithma::status::ResultStatus::Verified { points_tested } => {
            assert!(points_tested >= 3)
        }
        other => panic!("expected Verified, got {:?}", other),
    }
    assert!(report.counterexample_json().is_none());
}

#[test]
fn verify_fail_maps_to_verified_with_counterexample() {
    use arithma::status::classify_verify;
    let env = Environment::new();
    let lhs = parse_latex("\\sin(x)", &env).unwrap();
    let rhs = parse_latex("\\cos(x)", &env).unwrap();
    let result = arithma::verify_identity(
        &lhs,
        &rhs,
        &["x".to_string()],
        &arithma::assumptions::Assumptions::new(),
    );
    let report = classify_verify(&result);
    assert!(matches!(
        report.status,
        arithma::status::ResultStatus::Verified { .. }
    ));
    let cx = report
        .counterexample_json()
        .expect("counterexample present");
    assert!(cx.get("point").is_some());
    assert!(cx.get("lhs").is_some());
    assert!(cx.get("rhs").is_some());
    // And it serializes into the payload:
    let json = report.to_json();
    assert!(json.get("counterexample").is_some());
}

#[test]
fn verify_inconclusive_maps_to_unable_to_compute() {
    use arithma::status::classify_verify;
    use arithma::verify::VerifyResult;
    let result = VerifyResult {
        passed: false,
        points_tested: 1,
        counterexample: None,
        insufficient_points: true,
        domain_mismatches: 0,
        starved_range_lengths: None,
    };
    let report = classify_verify(&result);
    match report.status {
        arithma::status::ResultStatus::UnableToCompute { ref reason } => {
            assert!(
                reason.contains("1"),
                "reason should count points: {}",
                reason
            )
        }
        ref other => panic!("expected UnableToCompute, got {:?}", other),
    }
}

// --- classify_limit: numeric corroboration along the approach path ---

#[test]
fn limit_correct_finite_claim_is_verified() {
    use arithma::status::classify_limit;
    let env = Environment::new();
    let expr = parse_latex("\\frac{\\sin(x)}{x}", &env).unwrap();
    let report = classify_limit(&expr, "x", "0", "1", &env);
    assert!(
        matches!(
            report.status,
            arithma::status::ResultStatus::Verified { .. }
        ),
        "expected Verified, got {:?}",
        report
    );
}

#[test]
fn limit_correct_infinite_claim_is_verified() {
    use arithma::status::classify_limit;
    let env = Environment::new();
    let expr = parse_latex("\\frac{1}{x^2}", &env).unwrap();
    let report = classify_limit(&expr, "x", "0", "\\infty", &env);
    assert!(
        matches!(
            report.status,
            arithma::status::ResultStatus::Verified { .. }
        ),
        "expected Verified, got {:?}",
        report
    );
}

#[test]
fn limit_wrong_claim_is_loud() {
    use arithma::status::classify_limit;
    let env = Environment::new();
    let expr = parse_latex("\\frac{\\sin(x)}{x}", &env).unwrap();
    let report = classify_limit(&expr, "x", "0", "2", &env);
    assert_eq!(report.status, arithma::status::ResultStatus::Heuristic);
    assert!(
        report
            .caveats
            .iter()
            .any(|c| c.code == caveat_codes::CORROBORATION_FAILED),
        "should flag corroboration failure: {:?}",
        report.caveats
    );
}

#[test]
fn limit_symbolic_claim_is_heuristic_with_caveat() {
    use arithma::status::classify_limit;
    let env = Environment::new();
    let expr = parse_latex("\\frac{a x}{x}", &env).unwrap();
    let report = classify_limit(&expr, "x", "0", "a", &env);
    assert_eq!(report.status, arithma::status::ResultStatus::Heuristic);
    assert!(
        report
            .caveats
            .iter()
            .any(|c| c.code == caveat_codes::NOT_CORROBORATED),
        "should note lack of corroboration: {:?}",
        report.caveats
    );
}

#[test]
fn limit_slow_convergence_is_not_a_false_alarm() {
    // 1/ln(x) → 0 as x → ∞ is correct but converges too slowly to land
    // within tolerance at the sample points. That is "consistent but
    // uncorroborated", never "FAILED".
    use arithma::status::classify_limit;
    let env = Environment::new();
    let expr = parse_latex("\\frac{1}{\\ln(x)}", &env).unwrap();
    let report = classify_limit(&expr, "x", "inf", "0", &env);
    assert_eq!(report.status, arithma::status::ResultStatus::Heuristic);
    assert!(
        report
            .caveats
            .iter()
            .all(|c| c.code != caveat_codes::CORROBORATION_FAILED),
        "slow convergence must not be reported as failure: {:?}",
        report.caveats
    );
    assert!(
        report
            .caveats
            .iter()
            .any(|c| c.code == caveat_codes::SLOW_CONVERGENCE),
        "caveat should name slow convergence: {:?}",
        report.caveats
    );
}

#[test]
fn limit_slow_divergence_is_not_a_false_alarm() {
    // ln(x) → ∞ but only reaches ~11.5 at x = 10^5 — growing, correct,
    // below the corroboration threshold.
    use arithma::status::classify_limit;
    let env = Environment::new();
    let expr = parse_latex("\\ln(x)", &env).unwrap();
    let report = classify_limit(&expr, "x", "inf", "\\infty", &env);
    assert_eq!(report.status, arithma::status::ResultStatus::Heuristic);
    assert!(
        report
            .caveats
            .iter()
            .all(|c| c.code != caveat_codes::CORROBORATION_FAILED),
        "slow divergence must not be reported as failure: {:?}",
        report.caveats
    );
}
