//! Arithma MCP server: request handlers.
//!
//! Every tool response carries a `result_status` object describing the kind
//! of evidence behind the result — see `docs/result-status.md` for the
//! taxonomy, the per-tool earning rules, and the compatibility contract
//! (happy-path text stays byte-identical; loud statuses add a marker line).

use serde_json::{json, Value};

use arithma::assumptions::Assumptions;
use arithma::chain::{verify_chain, ChainResult, ChainStepInput, Relation};
use arithma::derivative::differentiate_latex;
use arithma::exact::ExactNum;
use arithma::integration::{definite_integral_exact_latex, integrate_latex};
use arithma::matrix::parse_latex_matrix;
use arithma::series::{
    taylor_series_latex, taylor_series_latex_symbolic, taylor_series_multivar_latex,
};
use arithma::simplify::Simplifiable;
use arithma::special_functions::recognize_special_form_latex;
use arithma::status::Verdict;
use arithma::status::{
    classify_integral, classify_limit, classify_simplify, classify_verify, free_variables,
    Certificate, ProofCertificate, StatusReport,
};
use arithma::substitute::substitute_latex;
use arithma::tokenizer::normalize_var;
use arithma::{
    build_expression_tree, factor_over_q, parse_latex, parse_latex_raw, partial_fractions_latex,
    Environment, Evaluator, Node, Polynomial, Tokenizer,
};

pub fn json_rpc_error(id: Option<Value>, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })
}

pub fn handle_initialize(id: Option<Value>, _params: &Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "arithma",
                "version": "0.1.0"
            }
        }
    })
}

pub fn handle_tools_list(id: Option<Value>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "tools": tools_schema()
        }
    })
}

fn assumptions_schema() -> Value {
    json!({
        "type": "object",
        "description": "Optional variable assumptions. Map variable names to arrays of properties: \"positive\", \"nonnegative\", \"negative\", \"nonzero\", \"real\", \"integer\". Example: {\"x\": [\"positive\"], \"n\": [\"integer\"]}",
        "additionalProperties": {
            "type": "array",
            "items": {
                "type": "string",
                "enum": ["positive", "nonnegative", "negative", "nonzero", "real", "integer"]
            }
        }
    })
}

fn tools_schema() -> Value {
    json!([
        {
            "name": "format",
            "description": "Parse LaTeX and return canonical form without simplification. Use to normalize messy input (spacing, implicit multiplication, nested braces) while preserving algebraic structure.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "expr": {
                        "type": "string",
                        "description": "LaTeX expression to format, e.g. \"\\frac{2}{2+{\\pi}}+.5{\\pi}\""
                    }
                },
                "required": ["expr"]
            }
        },
        {
            "name": "simplify",
            "description": "Simplify a mathematical expression. Returns the simplified form in LaTeX. Handles polynomial normalization, trigonometric identities, logarithmic properties, and multivariate GCD cancellation. Supports optional assumptions about variables (e.g. positive, integer) to enable additional simplifications like sqrt(x^2) → x when x > 0. The response's result_status distinguishes exact canonicalization (\"exact\") from numerically checked transcendental rewrites (\"verified\").",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "expr": {
                        "type": "string",
                        "description": "LaTeX expression to simplify, e.g. \"x^2 + 2x + 1\" or \"\\frac{x^2 - 1}{x - 1}\""
                    },
                    "assumptions": assumptions_schema()
                },
                "required": ["expr"]
            }
        },
        {
            "name": "differentiate",
            "description": "Compute the derivative of an expression with respect to a variable. Supports polynomials, trigonometric, exponential, logarithmic, and composed functions via chain rule.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "expr": {
                        "type": "string",
                        "description": "LaTeX expression to differentiate, e.g. \"x^3 + \\sin(x)\""
                    },
                    "variable": {
                        "type": "string",
                        "description": "Variable to differentiate with respect to",
                        "default": "x"
                    },
                    "assumptions": assumptions_schema()
                },
                "required": ["expr"]
            }
        },
        {
            "name": "integrate",
            "description": "Compute the integral of an expression. Without bounds: returns the indefinite integral (antiderivative). With lower and upper bounds: returns the definite integral (a number). If no elementary antiderivative exists, the response's result_status is \"provably_impossible\" with the reason — a theorem, not a failure.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "expr": {
                        "type": "string",
                        "description": "LaTeX expression to integrate, e.g. \"3x^2\" or \"\\sin(x)\""
                    },
                    "variable": {
                        "type": "string",
                        "description": "Variable of integration",
                        "default": "x"
                    },
                    "lower": {
                        "type": "string",
                        "description": "Lower bound for definite integral as LaTeX expression (e.g. \"0\", \"\\\\pi\", \"1/2\"). Omit for indefinite."
                    },
                    "upper": {
                        "type": "string",
                        "description": "Upper bound for definite integral as LaTeX expression (e.g. \"1\", \"\\\\pi/2\", \"\\\\infty\"). Omit for indefinite."
                    },
                    "assumptions": assumptions_schema()
                },
                "required": ["expr"]
            }
        },
        {
            "name": "substitute",
            "description": "Substitute a value or expression for a variable in an expression. Returns the simplified result.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "expr": {
                        "type": "string",
                        "description": "LaTeX expression, e.g. \"x^2 + 2x + 1\""
                    },
                    "variable": {
                        "type": "string",
                        "description": "Variable to replace"
                    },
                    "value": {
                        "type": "string",
                        "description": "LaTeX expression or number to substitute, e.g. \"3\" or \"y + 1\""
                    },
                    "assumptions": assumptions_schema()
                },
                "required": ["expr", "variable", "value"]
            }
        },
        {
            "name": "solve",
            "description": "Solve a single equation for a variable. Input should contain '=' sign. Returns exact solutions when possible (rational roots, quadratic formula, Cardano's formula for cubics, Ferrari's method for quartics). For systems of equations, use solve_system instead.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "equation": {
                        "type": "string",
                        "description": "LaTeX equation to solve, e.g. \"x^2 - 5x + 6 = 0\" or \"2x + 1 = 7\""
                    },
                    "variable": {
                        "type": "string",
                        "description": "Variable to solve for",
                        "default": "x"
                    },
                    "assumptions": assumptions_schema()
                },
                "required": ["equation"]
            }
        },
        {
            "name": "solve_system",
            "description": "Solve a system of equations. Linear systems use exact Gaussian elimination over Q. Polynomial systems (where at least one equation is linear) use substitution. Returns exact solutions. Handles unique, multiple, parametric, and inconsistent systems.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "equations": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "List of equations, each containing '='. E.g. [\"x + y = 3\", \"2x - y = 1\"]"
                    },
                    "variables": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Variables to solve for. E.g. [\"x\", \"y\"]"
                    }
                },
                "required": ["equations", "variables"]
            }
        },
        {
            "name": "factor",
            "description": "Factor a polynomial into irreducible factors over Q using the Berlekamp-Zassenhaus algorithm. Returns content and monic irreducible factors with multiplicities. Example: x^4 - 1 → (x - 1)(x + 1)(x^2 + 1).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "expr": {
                        "type": "string",
                        "description": "LaTeX polynomial to factor, e.g. \"x^4 - 1\" or \"x^6 - 1\""
                    },
                    "variable": {
                        "type": "string",
                        "description": "Main variable of the polynomial",
                        "default": "x"
                    }
                },
                "required": ["expr"]
            }
        },
        {
            "name": "partial_fractions",
            "description": "Decompose a rational function P(x)/Q(x) into partial fractions. Factors the denominator, then splits into terms with irreducible denominators. Example: 1/(x^2-1) → 1/(2(x-1)) - 1/(2(x+1)).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "numerator": {
                        "type": "string",
                        "description": "LaTeX expression for the numerator polynomial, e.g. \"1\" or \"x^2 + 1\""
                    },
                    "denominator": {
                        "type": "string",
                        "description": "LaTeX expression for the denominator polynomial, e.g. \"x^2 - 1\" or \"x^3 - 1\""
                    },
                    "variable": {
                        "type": "string",
                        "description": "Main variable",
                        "default": "x"
                    }
                },
                "required": ["numerator", "denominator"]
            }
        },
        {
            "name": "limit",
            "description": "Compute the limit of an expression as a variable approaches a point. Supports one-sided limits (append + or - to the point, e.g. \"0+\" for right-sided). Returns +∞ or -∞ for divergent limits. Handles 0/0 forms, exponential indeterminate forms, and limits at infinity.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "expr": {
                        "type": "string",
                        "description": "LaTeX expression, e.g. \"\\frac{\\sin(x)}{x}\" or \"\\frac{x^2-1}{x-1}\""
                    },
                    "variable": {
                        "type": "string",
                        "description": "Variable approaching the limit point",
                        "default": "x"
                    },
                    "point": {
                        "type": "string",
                        "description": "The point the variable approaches. Accepts numbers (\"0\", \"1\", \"3.14\"), infinity (\"inf\", \"\\\\infty\", \"-inf\"), or one-sided limits (\"0+\" for right, \"0-\" for left, \"3+\", \"3-\").",
                        "default": "0"
                    },
                    "assumptions": assumptions_schema()
                },
                "required": ["expr"]
            }
        },
        {
            "name": "taylor_series",
            "description": "Compute the Taylor (or Maclaurin) series expansion of an expression around a center point. Returns exact rational coefficients when possible. Supports multivariate: pass comma-separated variables (e.g. \"x,y\") and centers (e.g. \"0,0\") for total-degree truncation.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "expr": {
                        "type": "string",
                        "description": "LaTeX expression to expand, e.g. \"\\sin(x)\" or \"e^x\" or \"\\frac{1}{1-x}\""
                    },
                    "variable": {
                        "type": "string",
                        "description": "Variable to expand in",
                        "default": "x"
                    },
                    "center": {
                        "type": ["number", "string"],
                        "description": "Center of the expansion. Use a number (0 for Maclaurin) or a LaTeX expression for symbolic centers (e.g. \"a\" or \"\\\\alpha\").",
                        "default": 0
                    },
                    "order": {
                        "type": "integer",
                        "description": "Maximum degree of the expansion",
                        "default": 5
                    },
                    "assumptions": assumptions_schema()
                },
                "required": ["expr"]
            }
        },
        {
            "name": "evaluate",
            "description": "Evaluate a mathematical expression numerically with given variable values. Returns an exact rational result when possible, otherwise a decimal approximation.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "expr": {
                        "type": "string",
                        "description": "LaTeX expression to evaluate, e.g. \"x^2 + 2x + 1\" or \"\\sin(\\pi/6)\""
                    },
                    "variables": {
                        "type": "object",
                        "description": "Variable assignments as key-value pairs, e.g. {\"x\": 3, \"y\": 4}",
                        "additionalProperties": { "type": "number" }
                    },
                    "assumptions": assumptions_schema()
                },
                "required": ["expr"]
            }
        },
        {
            "name": "matrix",
            "description": "Perform matrix operations: determinant, inverse, eigenvalues, rank, transpose, multiply, solve (Ax=b), or RREF. Matrices use LaTeX pmatrix notation.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "operation": {
                        "type": "string",
                        "description": "Operation to perform",
                        "enum": ["determinant", "inverse", "eigenvalues", "rank", "transpose", "multiply", "solve", "rref"]
                    },
                    "matrix": {
                        "type": "string",
                        "description": "LaTeX matrix, e.g. \"\\begin{pmatrix} 1 & 2 \\\\ 3 & 4 \\end{pmatrix}\""
                    },
                    "matrix_b": {
                        "type": "string",
                        "description": "Second matrix for multiply (A*B) or solve (Ax=b). For solve, this is the column vector b."
                    },
                    "assumptions": assumptions_schema()
                },
                "required": ["operation", "matrix"]
            }
        }
        ,{
            "name": "equivalent",
            "description": "Check if two mathematical expressions are equivalent. Simplifies both and compares, then spot-checks numerically at several points. Returns whether they are equivalent and the simplified forms.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "expr_a": {
                        "type": "string",
                        "description": "First LaTeX expression"
                    },
                    "expr_b": {
                        "type": "string",
                        "description": "Second LaTeX expression"
                    },
                    "assumptions": assumptions_schema()
                },
                "required": ["expr_a", "expr_b"]
            }
        },
        {
            "name": "verify",
            "description": "Numerically verify that two expressions are equal by evaluating both at multiple test points. Returns PASS with the number of points tested, or FAIL with a specific counterexample showing where the expressions disagree. Use this to cross-check symbolic results. Supports assumptions to filter test points (e.g. only test positive values when x is assumed positive). The response's result_status carries the evidence: points tested, and the counterexample on FAIL. Note: agreement at n points is evidence, never proof.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "expr_a": {
                        "type": "string",
                        "description": "First LaTeX expression (LHS)"
                    },
                    "expr_b": {
                        "type": "string",
                        "description": "Second LaTeX expression (RHS)"
                    },
                    "variables": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Variables in the expressions. Defaults to [\"x\"] if not provided."
                    },
                    "assumptions": assumptions_schema()
                },
                "required": ["expr_a", "expr_b"]
            }
        },
        {
            "name": "verify_chain",
            "description": "Verify a chain of mathematical reasoning steps. Each step after the first declares its relation to the previous step: equals, derivative_of, integral_of, substitution, implies, solution_of, or factored_form_of. Each relation is checked by the mechanism appropriate to it (canonical forms over Q, differentiation round-trips, substitution, numeric sampling) and reports a machine-readable verdict (pass/fail/inconclusive) plus the evidence class backing it. The chain status is the MINIMUM evidence across steps — one numeric step makes the whole chain 'verified', never 'exact'. A failing step carries the counterexample: the counterexample is the diagnosis. For incremental (step-at-a-time) use, send a two-step chain of the previous and the new step.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "steps": {
                        "type": "array",
                        "description": "Ordered reasoning steps. The first step is the anchor and declares no relation.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "label": {
                                    "type": "string",
                                    "description": "Optional human-readable step name, echoed in results"
                                },
                                "expr": {
                                    "type": "string",
                                    "description": "LaTeX expression for this step. For solution_of use the form \"x = 2\"; for implies both steps must be equations."
                                },
                                "relation": {
                                    "type": "string",
                                    "enum": ["equals", "derivative_of", "integral_of", "substitution", "implies", "solution_of", "factored_form_of"],
                                    "description": "Relation of this step to the previous one. Default: equals. derivative_of: this step is d/d(variable) of the previous. integral_of: this step is an antiderivative of the previous (checked by differentiation round-trip; can earn exact). substitution: this step is the previous with variable := value. implies: previous equation implies this equation (checked at the antecedent's solutions; capped at verified). solution_of: this step (variable = value) solves the previous equation (membership only, not completeness). factored_form_of: this step is a factored form of the previous.",
                                    "default": "equals"
                                },
                                "variable": {
                                    "type": "string",
                                    "description": "Variable for derivative_of / integral_of / substitution / implies / equation comparison. If omitted, it is inferred when the relevant expression has exactly one free variable; ambiguity is an error, never a silent default."
                                },
                                "value": {
                                    "type": "string",
                                    "description": "LaTeX value substituted for the variable (substitution steps only)"
                                }
                            },
                            "required": ["expr"]
                        }
                    },
                    "assumptions": assumptions_schema()
                },
                "required": ["steps"]
            }
        },
        {
            "name": "solve_ode",
            "description": "Solve an ordinary differential equation. First-order: provide expr where dy/dx = expr. Second-order constant-coefficient: provide a, b, c for ay''+by'+cy=0. General linear with polynomial coefficients: provide poly_coeffs for power series solution.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "expr": {
                        "type": "string",
                        "description": "RHS of first-order ODE dy/dx = expr, e.g. \"x \\cdot y\" or \"-2y + x\""
                    },
                    "indep": {
                        "type": "string",
                        "description": "Independent variable",
                        "default": "x"
                    },
                    "dep": {
                        "type": "string",
                        "description": "Dependent variable",
                        "default": "y"
                    },
                    "a": {
                        "type": "number",
                        "description": "Coefficient of y'' for second-order constant-coefficient ODE"
                    },
                    "b": {
                        "type": "number",
                        "description": "Coefficient of y' for second-order constant-coefficient ODE"
                    },
                    "c": {
                        "type": "number",
                        "description": "Coefficient of y for second-order constant-coefficient ODE"
                    },
                    "poly_coeffs": {
                        "type": "array",
                        "description": "Polynomial coefficients for general linear ODE series solution. Array of arrays: poly_coeffs[i] = coefficients of a_i(x) for a_0(x)·y + a_1(x)·y' + ... + a_k(x)·y^(k) = 0. Example: [[6], [0, -2], [1]] for y'' - 2xy' + 6y = 0",
                        "items": {
                            "type": "array",
                            "items": { "type": "number" }
                        }
                    },
                    "order": {
                        "type": "integer",
                        "description": "Truncation degree for power series solution (default: 10)"
                    },
                    "initial_values": {
                        "type": "array",
                        "description": "Initial values [y(0), y'(0), ...] for initial value problem. Length must match ODE order.",
                        "items": { "type": "number" }
                    },
                    "assumptions": assumptions_schema()
                }
            }
        }
    ])
}

pub fn handle_tools_call(id: Option<Value>, params: &Value) -> Value {
    let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    // verify_chain carries structured per-step detail beyond the shared
    // (text, status) shape, so it assembles its own response.
    if tool_name == "verify_chain" {
        return match tool_verify_chain(&args) {
            Ok((text, status_json)) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "content": [{ "type": "text", "text": text }],
                    "result_status": status_json
                }
            }),
            Err(e) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "content": [{ "type": "text", "text": format!("Error: {}", e) }],
                    "isError": true
                }
            }),
        };
    }

    let result = match tool_name {
        "format" => tool_format(&args),
        "simplify" => tool_simplify(&args),
        "differentiate" => tool_differentiate(&args),
        "integrate" => tool_integrate(&args),
        "substitute" => tool_substitute(&args),
        "solve" => tool_solve(&args),
        "solve_system" => tool_solve_system(&args),
        "factor" => tool_factor(&args),
        "partial_fractions" => tool_partial_fractions(&args),
        "limit" => tool_limit(&args),
        "taylor_series" => tool_taylor_series(&args),
        "evaluate" => tool_evaluate(&args),
        "matrix" => tool_matrix(&args),
        "equivalent" => tool_equivalent(&args),
        "verify" => tool_verify(&args),
        "solve_ode" => tool_solve_ode(&args),
        _ => Err(format!("Unknown tool: {}", tool_name)),
    };

    match result {
        Ok((text, status)) => {
            // Loud statuses put a marker line in front of the text so agents
            // that only read text still see them; quiet statuses leave the
            // text byte-identical to the pre-status behavior.
            let text = match status.marker() {
                Some(marker) if text.is_empty() => marker,
                Some(marker) => format!("{}\n{}", marker, text),
                None => text,
            };
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "content": [{ "type": "text", "text": text }],
                    "result_status": status.to_json()
                }
            })
        }
        // Protocol errors (missing params, unparseable input) are not
        // mathematical results and carry no status.
        Err(e) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "content": [{ "type": "text", "text": format!("Error: {}", e) }],
                "isError": true
            }
        }),
    }
}

/// Every tool returns its text plus the evidence classification for it.
type ToolResult = Result<(String, StatusReport), String>;

fn get_str<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(|v| v.as_str())
}

fn get_str_or<'a>(args: &'a Value, key: &str, default: &'a str) -> &'a str {
    get_str(args, key).unwrap_or(default)
}

fn get_var(args: &Value, default: &str) -> String {
    normalize_var(get_str_or(args, "variable", default))
}

fn env_from_args(args: &Value) -> Result<Environment, String> {
    match args.get("assumptions") {
        Some(v) if !v.is_null() => {
            let assumptions = Assumptions::from_json(v)?;
            Ok(Environment::with_assumptions(assumptions))
        }
        _ => Ok(Environment::new()),
    }
}

fn parse_and_simplify_with_env(expr_str: &str, env: &Environment) -> Result<String, String> {
    parse_latex(expr_str, env).map(|node| format!("{node}"))
}

/// Three-way replay outcome: a replay check that conflates
/// "couldn't confirm" with "actively refuted" is decorative. The fix is
/// to canonicalize the *difference* to zero — the same decision procedure
/// verify_chain's equals already uses.
enum ReplayOutcome {
    /// The replay confirmed the result (difference canonicalizes to zero).
    Confirmed,
    /// The replay contradicted the result (difference canonicalizes to a
    /// provably nonzero value). The result should NOT be certified exact.
    Contradicted,
    /// The replay was inconclusive (difference didn't simplify cleanly).
    /// Fall through to by_construction only if the algorithm is independently
    /// a certified decision procedure.
    Inconclusive,
}

/// Check whether `lhs - rhs` is zero by canonicalization and, when the
/// difference contains free variables, by exact rational evaluation at
/// sample points — the same mechanism verify_chain's equals uses.
fn difference_is_zero(lhs: &Node, rhs: &Node, env: &Environment) -> ReplayOutcome {
    use arithma::simplify::Simplifiable;
    use arithma::status::is_algebraic_exact;

    if format!("{lhs}") == format!("{rhs}") {
        return ReplayOutcome::Confirmed;
    }

    let diff = Node::Subtract(Box::new(lhs.clone()), Box::new(rhs.clone()));
    let d = match diff.simplify(env) {
        Ok(d) => d,
        Err(_) => return ReplayOutcome::Inconclusive,
    };

    if matches!(&d, Node::Num(n) if n.is_zero()) {
        return ReplayOutcome::Confirmed;
    }

    if !is_algebraic_exact(&d) {
        return ReplayOutcome::Inconclusive;
    }

    // Constant (no free variables): one exact evaluation decides.
    let vars = free_variables(&[&d]);
    if vars.is_empty() {
        return match Evaluator::evaluate_exact(&d, &Environment::new()) {
            Ok(ExactNum::Rational(r)) if *r.numer() == num_bigint::BigInt::from(0) => {
                ReplayOutcome::Confirmed
            }
            Ok(ExactNum::Rational(_)) => ReplayOutcome::Contradicted,
            _ => ReplayOutcome::Inconclusive,
        };
    }

    // Free variables present: evaluate at exact rational sample points.
    // A single nonzero evaluation is a genuine disproof (Contradicted).
    // Agreement at sample points without a degree bound is evidence, not
    // proof — return Inconclusive, which falls to by_construction for
    // genuine decision procedures. Only the degree-bounded grid in
    // verify_chain's interpolation_identity_Q earns exact from sampling.
    let sample_values: &[i64] = &[0, 1, -1, 2, -2, 3, 7];
    for val in sample_values.iter() {
        let mut pt_env = Environment::new();
        for (i, v) in vars.iter().enumerate() {
            pt_env.set_exact(v, ExactNum::integer(*val + i as i64));
        }
        match Evaluator::evaluate_exact(&d, &pt_env) {
            Ok(ExactNum::Rational(r)) => {
                if *r.numer() != num_bigint::BigInt::from(0) {
                    return ReplayOutcome::Contradicted;
                }
            }
            _ => continue,
        }
    }
    ReplayOutcome::Inconclusive
}

/// Back-substitution replay for solve: substitute each root, check the
/// residual via difference-to-zero. Three outcomes the three-way replay protocol.
fn replay_solve_check(
    expr: &Node,
    var: &str,
    solutions: &[Node],
    env: &Environment,
) -> StatusReport {
    for root in solutions {
        let substituted = arithma::substitute::substitute_variable(expr, var, root)
            .unwrap_or_else(|_| expr.clone());
        let outcome = match &substituted {
            Node::Equation(l, r) => {
                let ls = l.simplify(env).unwrap_or_else(|_| *l.clone());
                let rs = r.simplify(env).unwrap_or_else(|_| *r.clone());
                difference_is_zero(&ls, &rs, env)
            }
            other => {
                let zero = Node::Num(ExactNum::integer(0));
                let residual = other.simplify(env).unwrap_or_else(|_| other.clone());
                difference_is_zero(&residual, &zero, env)
            }
        };
        match outcome {
            ReplayOutcome::Confirmed => continue,
            ReplayOutcome::Contradicted => {
                return StatusReport::heuristic().with_caveat(
                    "back-substitution check CONTRADICTED: residual is provably nonzero after substituting root",
                );
            }
            ReplayOutcome::Inconclusive => {
                return StatusReport::exact(Certificate::by_construction(
                    "symbolic_root_formula — exact algebraic solver (replay inconclusive)",
                ));
            }
        }
    }
    StatusReport::exact(Certificate::replay(
        "substitution_check",
        "each root substituted into the equation yields residual zero",
    ))
}

/// Classify a `raw → simplified` step where the raw form is a LaTeX string
/// produced by our own machinery (derivative output, substitution output).
/// Falls back to a quiet heuristic if the raw form will not re-parse, which
/// would itself be a printer/parser disagreement worth hearing about.
fn classify_simplify_of(raw_latex: &str, simplified: &Node, env: &Environment) -> StatusReport {
    match parse_latex_raw(raw_latex) {
        Ok(raw) => classify_simplify(&raw, simplified, env),
        Err(e) => StatusReport::heuristic()
            .with_caveat(&format!("could not classify simplification: {}", e)),
    }
}

fn tool_format(args: &Value) -> ToolResult {
    let expr = get_str(args, "expr").ok_or("Missing required parameter: expr")?;
    let text = parse_latex_raw(expr).map(|node| format!("{node}"))?;
    Ok((
        text,
        StatusReport::exact(Certificate::by_construction(
            "canonical_printing — no equivalence claim",
        )),
    ))
}

fn tool_simplify(args: &Value) -> ToolResult {
    let expr = get_str(args, "expr").ok_or("Missing required parameter: expr")?;
    let env = env_from_args(args)?;
    let input = parse_latex_raw(expr)?;
    let output = parse_latex(expr, &env)?;
    let status = classify_simplify(&input, &output, &env);
    Ok((format!("{output}"), status))
}

fn tool_differentiate(args: &Value) -> ToolResult {
    let expr = get_str(args, "expr").ok_or("Missing required parameter: expr")?;
    let var = get_var(args, "x");
    let result = differentiate_latex(expr, &var)?;
    let env = env_from_args(args)?;
    let output = parse_latex(&result, &env)?;
    // Derivative rules are complete and sound (exact); the final
    // simplification step inherits the simplify classification.
    let status = classify_simplify_of(&result, &output, &env);
    Ok((format!("{output}"), status))
}

fn tool_integrate(args: &Value) -> ToolResult {
    let expr = get_str(args, "expr").ok_or("Missing required parameter: expr")?;
    let var = get_var(args, "x");

    let has_lower = args.get("lower").and_then(|v| v.as_str());
    let has_upper = args.get("upper").and_then(|v| v.as_str());
    let env = env_from_args(args)?;

    if let (Some(lower), Some(upper)) = (has_lower, has_upper) {
        return match definite_integral_exact_latex(expr, &var, lower, upper) {
            Ok(r) => {
                let value = parse_latex(&r, &env)?;
                // FTC path: certify the antiderivative by round-trip when we
                // can recompute it; the exact evaluation at the bounds adds
                // no further uncertainty. The special-value path (antiderivative
                // non-elementary yet the definite integral known) is a table
                // of proven results.
                let status = match integrate_latex(expr, &var) {
                    Ok(anti) => match (parse_latex(expr, &env), parse_latex(&anti, &env)) {
                        (Ok(integrand), Ok(a)) => classify_integral(&integrand, &a, &var, &env),
                        _ => StatusReport::heuristic()
                            .with_caveat("could not classify the antiderivative round-trip"),
                    },
                    Err(_) => StatusReport::exact(Certificate::by_construction(
                        "special_value_table — proven result from standard tables",
                    )),
                };
                Ok((format!("{value}"), status))
            }
            Err(e) if e.starts_with("NON_ELEMENTARY:") => {
                Ok((String::new(), non_elementary_status(&e, expr, &var)))
            }
            Err(e) => Err(e),
        };
    }

    match integrate_latex(expr, &var) {
        Ok(r) => {
            let antiderivative = parse_latex(&r, &env)?;
            let status = match parse_latex(expr, &env) {
                Ok(integrand) => classify_integral(&integrand, &antiderivative, &var, &env),
                Err(e) => StatusReport::heuristic()
                    .with_caveat(&format!("could not classify round-trip: {}", e)),
            };
            Ok((format!("{antiderivative}"), status))
        }
        Err(e) if e.starts_with("NON_ELEMENTARY:") => {
            Ok((String::new(), non_elementary_status(&e, expr, &var)))
        }
        Err(e) => Err(e),
    }
}

/// Classify a Risch non-elementarity reason into a proof method.
fn classify_risch_method(reason: &str) -> &'static str {
    if reason.contains("Rothstein-Trager") {
        "rothstein-trager"
    } else if reason.contains("differential equation")
        || reason.contains("Risch DE")
        || reason.contains("Cannot integrate the degree-")
    {
        "risch-de"
    } else {
        "risch"
    }
}

/// Build the provably_impossible status for a NON_ELEMENTARY error and, when
/// the integrand's antiderivative is a recognized special function (erf, Ei,
/// li), attach the named form — strictly more information than the
/// impossibility alone. Unrecognized integrands keep the bare certificate.
fn non_elementary_status(error: &str, integrand_latex: &str, var: &str) -> StatusReport {
    let reason = error.replacen("NON_ELEMENTARY: ", "", 1);
    let method = classify_risch_method(&reason);
    let proof = ProofCertificate::new(
        method,
        &reason,
        "This integral has no formula using elementary functions \
         (polynomials, exponentials, logarithms, trigonometric). \
         This is a theorem, not a limitation of the tool.",
    );
    let status = StatusReport::provably_impossible(proof);
    match recognize_special_form_latex(integrand_latex, var) {
        Some((name, form)) => status.with_special_form(&name, &form),
        None => status,
    }
}

fn tool_substitute(args: &Value) -> ToolResult {
    let expr = get_str(args, "expr").ok_or("Missing required parameter: expr")?;
    let var =
        normalize_var(get_str(args, "variable").ok_or("Missing required parameter: variable")?);
    let value = get_str(args, "value").ok_or("Missing required parameter: value")?;
    let subs = vec![(var, value.to_string())];
    let result = substitute_latex(expr, &subs)?;
    let env = env_from_args(args)?;
    match parse_latex(&result, &env) {
        Ok(output) => {
            // Substitution itself is algebraic; the follow-up simplification
            // inherits the simplify classification.
            let status = classify_simplify_of(&result, &output, &env);
            Ok((format!("{output}"), status))
        }
        Err(_) => Ok((
            result,
            StatusReport::exact(Certificate::by_construction(
                "capture_avoiding_substitution",
            )),
        )),
    }
}

fn tool_solve(args: &Value) -> ToolResult {
    let equation = get_str(args, "equation").ok_or("Missing required parameter: equation")?;
    let var = get_var(args, "x");

    let mut tokenizer = Tokenizer::new(equation);
    let tokens = tokenizer.tokenize();
    if let Some(err) = tokenizer.errors.into_iter().next() {
        return Err(err);
    }
    let expr = build_expression_tree(tokens)?;

    // Check if it's an inequality — solved by exact sign analysis.
    if matches!(
        expr,
        Node::Greater(_, _) | Node::GreaterEqual(_, _) | Node::Less(_, _) | Node::LessEqual(_, _)
    ) {
        return arithma::solve_inequality(&expr, &var).map(|t| {
            (
                t,
                StatusReport::exact(Certificate::by_construction(
                    "sign_analysis — exact polynomial sign analysis",
                )),
            )
        });
    }

    let result = match arithma::expression::solve_full(&expr, &var) {
        Ok(r) => r,
        Err(e) if e == "No solution (contradiction)" => {
            let proof = ProofCertificate::new(
                "contradiction",
                "The equation reduces to a nonzero constant equal to zero — a contradiction.",
                "This equation has no solutions. It simplifies to a contradiction \
                 (a nonzero number equal to zero), which is impossible for any value \
                 of the variable.",
            );
            return Ok((
                "No solution (contradiction)".to_string(),
                StatusReport::provably_impossible(proof),
            ));
        }
        Err(e) => return Err(e),
    };

    // No expressible solutions: classify the impossibility.
    if result.solutions.is_empty() && result.complex_omitted > 0 {
        let degree = result.complex_omitted;

        // Abel-Ruffini: irreducible factors of degree ≥ 5 have no
        // closed-form radical solution. Roots may exist (even real ones),
        // but cannot be expressed using radicals.
        if let Some(reason_str) = &result.impossibility_reason {
            let proof = ProofCertificate::new(
                "abel-ruffini",
                reason_str,
                "This polynomial has irreducible factors of degree 5 or higher. \
                 By the Abel-Ruffini theorem, their roots cannot be expressed \
                 using radicals (nth roots, addition, multiplication). The roots \
                 exist as real or complex numbers but have no closed-form formula. \
                 This is a theorem, not a limitation of the tool.",
            );
            return Ok((
                format!(
                    "No closed-form solution ({degree} root{} not expressible in radicals)",
                    if degree == 1 { "" } else { "s" }
                ),
                StatusReport::provably_impossible(proof),
            ));
        }

        let (method, reason, explanation) = if degree == 2 {
            (
                "negative-discriminant",
                "The quadratic has no real roots: discriminant is negative, \
                 so both roots are complex."
                    .to_string(),
                "This equation has no real solutions. Both roots are complex. \
                 This is a theorem (negative discriminant), not a limitation of the tool."
                    .to_string(),
            )
        } else {
            (
                "all-roots-complex",
                format!("No real roots exist: all {degree} roots are complex."),
                format!(
                    "This equation has no real solutions. All {degree} roots are complex. \
                     This is proved by exhaustive analysis of the polynomial's roots."
                ),
            )
        };
        let proof = ProofCertificate::new(method, &reason, &explanation);
        return Ok((
            format!(
                "No real solutions ({degree} complex root{} omitted)",
                if degree == 1 { "" } else { "s" }
            ),
            StatusReport::provably_impossible(proof),
        ));
    }

    let mut parts: Vec<String> = result
        .solutions
        .iter()
        .map(|s| format!("{} = {}", var, s))
        .collect();
    if result.complex_omitted > 0 {
        parts.push(format!(
            "({} complex root{} omitted)",
            result.complex_omitted,
            if result.complex_omitted == 1 { "" } else { "s" }
        ));
    }
    if parts.is_empty() {
        Ok((
            "No solutions found".to_string(),
            StatusReport::exact(Certificate::by_construction(
                "no_solutions — exhaustive search found no roots",
            )),
        ))
    } else {
        let text = parts.join(", ");
        if text.contains('.') {
            let status = StatusReport::verified(1)
                .with_caveat("floating-point root-finding (f64 precision), not symbolic radicals");
            Ok((text, status))
        } else {
            // Back-substitution check: substitute each root into the
            // equation and verify the residual simplifies to zero.
            // Three outcomes: confirmed → exact(replay),
            // contradicted → heuristic, inconclusive → exact(by_construction).
            let env = Environment::new();
            let status = replay_solve_check(&expr, &var, &result.solutions, &env);
            Ok((text, status))
        }
    }
}

fn tool_solve_system(args: &Value) -> ToolResult {
    let eq_arr = args
        .get("equations")
        .and_then(|v| v.as_array())
        .ok_or("Missing required parameter: equations (array of strings)")?;
    let var_arr = args
        .get("variables")
        .and_then(|v| v.as_array())
        .ok_or("Missing required parameter: variables (array of strings)")?;

    let mut equations = Vec::new();
    for eq_val in eq_arr {
        let eq_str = eq_val.as_str().ok_or("Each equation must be a string")?;
        let mut tokenizer = Tokenizer::new(eq_str);
        let tokens = tokenizer.tokenize();
        if let Some(err) = tokenizer.errors.into_iter().next() {
            return Err(format!("Parse error in '{}': {}", eq_str, err));
        }
        let expr = build_expression_tree(tokens)
            .map_err(|e| format!("Parse error in '{}': {}", eq_str, e))?;
        equations.push(expr);
    }

    let vars: Vec<String> = var_arr
        .iter()
        .map(|v| {
            v.as_str()
                .map(normalize_var)
                .unwrap_or_else(|| "x".to_string())
        })
        .collect();

    // Exact Gaussian elimination / substitution over Q.
    let text = match arithma::solve_system(&equations, &vars)? {
        arithma::SystemSolution::Unique(solutions) => {
            let parts: Vec<String> = solutions
                .iter()
                .map(|(var, val)| format!("{} = {}", var, val))
                .collect();
            parts.join(", ")
        }
        arithma::SystemSolution::Multiple(sets) => {
            let mut lines: Vec<String> = Vec::new();
            for (i, solutions) in sets.iter().enumerate() {
                let parts: Vec<String> = solutions
                    .iter()
                    .map(|(var, val)| format!("{} = {}", var, val))
                    .collect();
                lines.push(format!("Solution {}: {}", i + 1, parts.join(", ")));
            }
            lines.join("\n")
        }
        arithma::SystemSolution::Parametric {
            solutions,
            free_vars,
        } => {
            let mut parts = vec![format!(
                "Parametric solution (free: {})",
                free_vars.join(", ")
            )];
            for (var, val) in &solutions {
                parts.push(format!("{} = {}", var, val));
            }
            parts.join(", ")
        }
        arithma::SystemSolution::NoSolution => "No solution (inconsistent system)".to_string(),
    };
    // Back-substitution check: substitute the solution vector into
    // each original equation and verify all residuals are zero.
    let env = Environment::new();
    let status = match arithma::solve_system(&equations, &vars) {
        Ok(arithma::SystemSolution::Unique(ref solutions)) => {
            let subs: Vec<(String, Node)> = solutions
                .iter()
                .map(|(v, n)| (v.clone(), n.clone()))
                .collect();
            let mut confirmed_all = true;
            for eq in &equations {
                let substituted =
                    arithma::substitute::substitute(eq, &subs).unwrap_or_else(|_| eq.clone());
                let outcome = match &substituted {
                    Node::Equation(l, r) => {
                        let ls = l.simplify(&env).unwrap_or_else(|_| *l.clone());
                        let rs = r.simplify(&env).unwrap_or_else(|_| *r.clone());
                        difference_is_zero(&ls, &rs, &env)
                    }
                    other => {
                        let zero = Node::Num(ExactNum::integer(0));
                        let residual = other.simplify(&env).unwrap_or_else(|_| other.clone());
                        difference_is_zero(&residual, &zero, &env)
                    }
                };
                match outcome {
                    ReplayOutcome::Confirmed => continue,
                    ReplayOutcome::Contradicted => {
                        return Ok((
                            text,
                            StatusReport::heuristic().with_caveat(
                                "back-substitution check CONTRADICTED: residual provably nonzero",
                            ),
                        ));
                    }
                    ReplayOutcome::Inconclusive => {
                        confirmed_all = false;
                        break;
                    }
                }
            }
            if confirmed_all {
                StatusReport::exact(Certificate::replay(
                    "system_substitution_check",
                    "solution vector substituted into each equation yields zero",
                ))
            } else {
                StatusReport::exact(Certificate::by_construction(
                    "gaussian_elimination — exact arithmetic over Q",
                ))
            }
        }
        _ => StatusReport::exact(Certificate::by_construction(
            "gaussian_elimination — exact arithmetic over Q",
        )),
    };
    Ok((text, status))
}

fn tool_factor(args: &Value) -> ToolResult {
    let expr = get_str(args, "expr").ok_or("Missing required parameter: expr")?;
    let var = get_var(args, "x");

    let mut tokenizer = Tokenizer::new(expr);
    let tokens = tokenizer.tokenize();
    let node = build_expression_tree(tokens)?;
    let poly =
        Polynomial::from_node(&node, &var).map_err(|e| format!("Not a polynomial: {}", e))?;

    let (content, factors) = factor_over_q(&poly);

    let mut parts: Vec<String> = Vec::new();

    let content_str = format!(
        "{}",
        arithma::Node::Num(arithma::ExactNum::rational(
            content.numer().try_into().unwrap_or(1),
            content.denom().try_into().unwrap_or(1),
        ))
    );
    if content_str != "1" {
        parts.push(content_str);
    }

    // Group factors with multiplicities
    let mut grouped: Vec<(String, usize)> = Vec::new();
    for f in &factors {
        let s = format!("{}", f);
        if let Some(entry) = grouped.iter_mut().find(|(fs, _)| *fs == s) {
            entry.1 += 1;
        } else {
            grouped.push((s, 1));
        }
    }

    for (f_str, m) in &grouped {
        if *m == 1 {
            parts.push(format!("({})", f_str));
        } else {
            parts.push(format!("({})^{}", f_str, m));
        }
    }

    // Replay check: multiply factors back, take the difference with the
    // input, canonicalize to zero. Three outcomes the three-way replay protocol.
    let env = Environment::new();
    let mut product_node: Node = Node::Num(ExactNum::rational(
        content.numer().try_into().unwrap_or(1),
        content.denom().try_into().unwrap_or(1),
    ));
    for f in &factors {
        product_node = Node::Multiply(Box::new(product_node), Box::new(f.to_node()));
    }
    let product_expanded = product_node.simplify(&env).unwrap_or(product_node);
    let input_expanded = node.simplify(&env).unwrap_or_else(|_| node.clone());
    let cert = match difference_is_zero(&product_expanded, &input_expanded, &env) {
        ReplayOutcome::Confirmed => Certificate::replay(
            "factor_multiply_back",
            "product of factors equals input polynomial",
        ),
        ReplayOutcome::Contradicted => {
            return Ok((
                parts.join(" \\cdot "),
                StatusReport::heuristic()
                    .with_caveat("factor multiply-back CONTRADICTED: product differs from input"),
            ));
        }
        ReplayOutcome::Inconclusive => {
            Certificate::by_construction("berlekamp_zassenhaus — exact factorization over Q")
        }
    };

    if parts.is_empty() {
        Ok(("1".to_string(), StatusReport::exact(cert)))
    } else {
        let mut result = parts.join(" \\cdot ");
        if factors.len() == 1 && factors[0].degree().unwrap_or(0) > 1 {
            result.push_str("  \\quad\\text{(irreducible over }\\mathbb{Q}\\text{)}");
        }
        Ok((result, StatusReport::exact(cert)))
    }
}

fn tool_partial_fractions(args: &Value) -> ToolResult {
    let num = get_str(args, "numerator").ok_or("Missing required parameter: numerator")?;
    let den = get_str(args, "denominator").ok_or("Missing required parameter: denominator")?;
    let var = get_var(args, "x");
    let result = partial_fractions_latex(num, den, &var)?;

    // Replay check: parse the result, multiply by the denominator,
    // simplify, and compare to the numerator.
    let env = Environment::new();
    let cert = match (
        parse_latex(&result, &env),
        parse_latex(num, &env),
        parse_latex(den, &env),
    ) {
        (Ok(pf), Ok(num_node), Ok(den_node)) => {
            // Compare via difference-to-zero, not Display strings.
            // simplify(pf·den − num) reduces to 0 via canonical_form_Q even
            // when the two Display forms differ structurally.
            let reconstructed = Node::Multiply(Box::new(pf), Box::new(den_node));
            let reconstructed_s = reconstructed.simplify(&env).unwrap_or(reconstructed);
            let num_s = num_node.simplify(&env).unwrap_or(num_node);
            match difference_is_zero(&reconstructed_s, &num_s, &env) {
                ReplayOutcome::Confirmed => Certificate::replay(
                    "partial_fractions_multiply_back",
                    "partial fractions times denominator equals numerator",
                ),
                ReplayOutcome::Contradicted => {
                    return Ok((
                        result,
                        StatusReport::heuristic().with_caveat(
                            "partial-fractions multiply-back CONTRADICTED: product differs from numerator",
                        ),
                    ));
                }
                ReplayOutcome::Inconclusive => Certificate::by_construction(
                    "exact_rational_arithmetic — partial fraction decomposition",
                ),
            }
        }
        _ => Certificate::by_construction(
            "exact_rational_arithmetic — partial fraction decomposition",
        ),
    };
    Ok((result, StatusReport::exact(cert)))
}

fn tool_limit(args: &Value) -> ToolResult {
    let expr = get_str(args, "expr").ok_or("Missing required parameter: expr")?;
    let var = get_var(args, "x");
    let point_str = args
        .get("point")
        .map(|v| {
            v.as_str()
                .map(|s| s.to_string())
                .or_else(|| v.as_f64().map(|f| f.to_string()))
                .unwrap_or_else(|| "0".to_string())
        })
        .unwrap_or_else(|| "0".to_string());
    let result = arithma::limits::limit_latex_str(expr, &var, &point_str)?;
    let env = env_from_args(args)?;
    let text = parse_and_simplify_with_env(&result, &env)?;
    let status = match parse_latex(expr, &env) {
        Ok(expr_node) => classify_limit(&expr_node, &var, &point_str, &result, &env),
        Err(e) => StatusReport::heuristic().with_caveat(&format!("could not classify: {}", e)),
    };
    Ok((text, status))
}

fn tool_taylor_series(args: &Value) -> ToolResult {
    let expr = get_str(args, "expr").ok_or("Missing required parameter: expr")?;
    let var = get_var(args, "x");
    let order = args.get("order").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
    let env = env_from_args(args)?;

    // Coefficients come from exact rational recurrences; the caveat records
    // that a truncated polynomial equals the function only as a series.
    let status = || {
        StatusReport::exact(Certificate::by_construction("exact_rational_recurrence — Taylor coefficients from exact arithmetic")).with_caveat(&format!(
            "Taylor polynomial truncated at order {}; equality holds as series expansion, not as identity",
            order
        ))
    };

    // Multivariate: var contains comma (e.g., "x,y")
    if var.contains(',') {
        let vars: Vec<&str> = var.split(',').map(|s| s.trim()).collect();
        let default_centers = vec!["0"; vars.len()].join(",");
        let center_str = args
            .get("center")
            .and_then(|v| v.as_str())
            .unwrap_or(&default_centers);
        let centers: Vec<&str> = center_str.split(',').map(|s| s.trim()).collect();
        if centers.len() == 1 && vars.len() > 1 {
            let c = centers[0];
            let centers_expanded: Vec<&str> = vec![c; vars.len()];
            let result = taylor_series_multivar_latex(expr, &vars, &centers_expanded, order)?;
            return parse_and_simplify_with_env(&result, &env).map(|t| (t, status()));
        }
        let result = taylor_series_multivar_latex(expr, &vars, &centers, order)?;
        return parse_and_simplify_with_env(&result, &env).map(|t| (t, status()));
    }

    let center_val = args.get("center");
    let is_numeric = center_val
        .map(|v| v.is_number() || v.is_null())
        .unwrap_or(true);

    if is_numeric {
        let center = center_val.and_then(|v| v.as_f64()).unwrap_or(0.0);
        let result = taylor_series_latex(expr, &var, center, order)?;
        parse_and_simplify_with_env(&result, &env).map(|t| (t, status()))
    } else {
        let center_str = center_val
            .and_then(|v| v.as_str())
            .ok_or("center must be a number or LaTeX expression")?;
        let center_str = &normalize_var(center_str);
        let result = taylor_series_latex_symbolic(expr, &var, center_str, order)?;
        parse_and_simplify_with_env(&result, &env).map(|t| (t, status()))
    }
}

fn tool_evaluate(args: &Value) -> ToolResult {
    let expr_str = get_str(args, "expr").ok_or("Missing required parameter: expr")?;

    let mut tokenizer = Tokenizer::new(expr_str);
    let tokens = tokenizer.tokenize();
    let expr = build_expression_tree(tokens)?;

    let env_simplified = env_from_args(args)?;
    let simplified = expr
        .simplify(&env_simplified)
        .unwrap_or_else(|_| expr.clone());

    let mut env = env_from_args(args)?;
    if let Some(vars) = args.get("variables").and_then(|v| v.as_object()) {
        for (k, v) in vars {
            let key = normalize_var(k);
            if let Some(f) = v.as_f64() {
                if f == f.floor() && f.abs() < 1e15 {
                    env.set_exact(&key, ExactNum::integer(f as i64));
                } else {
                    env.set(&key, f);
                }
            }
        }
    }

    match Evaluator::evaluate_exact(&simplified, &env) {
        Ok(val) => {
            // The exact evaluator can still carry a float if one entered the
            // computation; only a rational result is exact arithmetic.
            let status = match &val {
                ExactNum::Rational(_) => StatusReport::exact(Certificate::by_construction(
                    "exact_rational_arithmetic — evaluated in exact Q arithmetic",
                )),
                ExactNum::Float(_) => StatusReport::verified(1)
                    .with_caveat("floating-point evaluation (f64 precision)"),
            };
            Ok((format!("{}", arithma::Node::Num(val)), status))
        }
        Err(_) => match Evaluator::evaluate(&simplified, &env) {
            Ok(val) => Ok((
                val.to_string(),
                StatusReport::verified(1).with_caveat("floating-point evaluation (f64 precision)"),
            )),
            Err(_) => Ok((
                format!("{}", simplified),
                StatusReport::heuristic()
                    .with_caveat("could not fully evaluate; returning simplified form"),
            )),
        },
    }
}

fn tool_matrix(args: &Value) -> ToolResult {
    let op = get_str(args, "operation").ok_or("Missing required parameter: operation")?;
    let matrix_str = get_str(args, "matrix").ok_or("Missing required parameter: matrix")?;
    let env = env_from_args(args)?;

    let a = parse_latex_matrix(matrix_str, &env)?;

    let text = match op {
        "determinant" => {
            let det = a.determinant(&env)?;
            let simplified = det.simplify(&env).unwrap_or(det);
            format!("{}", simplified)
        }
        "inverse" => a.inverse(&env)?.to_latex(),
        "eigenvalues" => {
            let vals = a.eigenvalues(&env)?;
            let strs: Vec<String> = vals.iter().map(|v| format!("{}", v)).collect();
            strs.join(", ")
        }
        "rank" => a.rank(&env)?.to_string(),
        "transpose" => a.transpose().to_latex(),
        "rref" => a.rref(&env)?.to_latex(),
        "multiply" => {
            let b_str = get_str(args, "matrix_b").ok_or("multiply requires matrix_b parameter")?;
            let b = parse_latex_matrix(b_str, &env)?;
            a.multiply(&b, &env)?.to_latex()
        }
        "solve" => {
            let b_str = get_str(args, "matrix_b")
                .ok_or("solve requires matrix_b parameter (column vector b in Ax=b)")?;
            let b = parse_latex_matrix(b_str, &env)?;
            a.solve(&b, &env)?.to_latex()
        }
        _ => {
            return Err(format!(
                "Unknown matrix operation: {}. Use: determinant, inverse, eigenvalues, rank, transpose, multiply, solve, rref",
                op
            ))
        }
    };
    // The exact claim must be conditioned on the code path actually taken,
    // not on the tool name. Exact arithmetic never
    // prints a decimal point; a '.' in the output means a floating-point
    // routine ran (numeric eigenvalue root-finding, float entries).
    let status = if text.contains('.') {
        let mut s =
            StatusReport::verified(1).with_caveat("floating-point computation (f64 precision)");
        if op == "eigenvalues" && text.contains('i') {
            s = s.with_caveat("complex eigenvalues expressed with i as a symbol");
        }
        s
    } else if op == "inverse" {
        // Replay check: multiply A × A⁻¹ and verify the result is I.
        // Three outcomes the three-way replay protocol.
        match a.inverse(&env) {
            Ok(inv) => match a.multiply(&inv, &env) {
                Ok(product) => {
                    let identity = arithma::matrix::Matrix::identity(a.rows);
                    let product_latex = product.to_latex();
                    let identity_latex = identity.to_latex();
                    if product_latex == identity_latex {
                        StatusReport::exact(Certificate::replay(
                            "inverse_multiply_check",
                            "A × A⁻¹ equals the identity matrix",
                        ))
                    } else {
                        // For matrices, a non-identity product after exact
                        // arithmetic means the inverse is wrong — downgrade.
                        StatusReport::heuristic()
                            .with_caveat("inverse multiply-back CONTRADICTED: A × A⁻¹ ≠ I")
                    }
                }
                Err(_) => StatusReport::exact(Certificate::by_construction(
                    "exact_rational_arithmetic — matrix operations over Q",
                )),
            },
            Err(_) => StatusReport::exact(Certificate::by_construction(
                "exact_rational_arithmetic — matrix operations over Q",
            )),
        }
    } else {
        StatusReport::exact(Certificate::by_construction(
            "exact_rational_arithmetic — matrix operations over Q",
        ))
    };
    Ok((text, status))
}

fn tool_equivalent(args: &Value) -> ToolResult {
    let a_str = get_str(args, "expr_a").ok_or("Missing required parameter: expr_a")?;
    let b_str = get_str(args, "expr_b").ok_or("Missing required parameter: expr_b")?;

    let env = env_from_args(args)?;

    let a_tokens = Tokenizer::new(a_str).tokenize();
    let a_expr = build_expression_tree(a_tokens)?;
    let a_simplified = a_expr.simplify(&env).unwrap_or_else(|_| a_expr.clone());

    let b_tokens = Tokenizer::new(b_str).tokenize();
    let b_expr = build_expression_tree(b_tokens)?;
    let b_simplified = b_expr.simplify(&env).unwrap_or_else(|_| b_expr.clone());

    let a_form = format!("{}", a_simplified);
    let b_form = format!("{}", b_simplified);

    // For the is-this-proven tools (equivalent, verify) the evidence tier IS
    // the answer, so it travels in-band in the text: many MCP hosts deliver
    // only content.text to the agent, and a quiet tier there turns "checked
    // at 12 points" and "decision procedure" into the same sentence —
    // exactly the numeric-check-as-proof conflation this tool exists to
    // prevent. A deliberate, documented exception to the quiet-status
    // byte-identity rule (docs/result-status.md).
    if a_form == b_form {
        return Ok((
            format!("Equivalent: true [exact]\nBoth simplify to: {}", a_form),
            StatusReport::exact(Certificate::by_construction(
                "canonical_form_Q — identical canonical forms",
            ))
            .with_verdict(Verdict::Pass),
        ));
    }

    // Structural comparison failed — try simplifying the difference
    let diff = arithma::Node::Subtract(
        Box::new(a_simplified.clone()),
        Box::new(b_simplified.clone()),
    );
    let diff_simplified = diff.simplify(&env).unwrap_or(diff);
    let diff_form = format!("{}", diff_simplified);
    if diff_form == "0" {
        return Ok((
            format!(
                "Equivalent: true [exact]\nSimplified forms differ but difference is zero.\nA simplifies to: {}\nB simplifies to: {}",
                a_form, b_form
            ),
            StatusReport::exact(Certificate::by_construction("difference_zero — difference simplifies to zero")).with_verdict(Verdict::Pass),
        ));
    }

    // Numeric stage: the same assumption-aware sampler the verify tool uses.
    // (Previously an ad-hoc 5-point spot-check that ignored assumptions and
    // could report "likely true" having evaluated zero points.)
    let mut vars = free_variables(&[&a_simplified, &b_simplified]);
    if vars.is_empty() {
        vars.push("x".to_string());
    }
    let result = arithma::verify_identity(&a_simplified, &b_simplified, &vars, env.assumptions());
    let status = classify_verify(&result);

    let text = if result.insufficient_points {
        format!(
            "Equivalent: inconclusive (only {} valid test point{})\nA simplifies to: {}\nB simplifies to: {}",
            result.points_tested,
            if result.points_tested == 1 { "" } else { "s" },
            a_form,
            b_form
        )
    } else if let Some(ref cx) = result.counterexample {
        let point_str: Vec<String> = cx
            .point
            .iter()
            .map(|(var, val)| format!("{} = {}", var, val))
            .collect();
        format!(
            "Equivalent: false\nA simplifies to: {}\nB simplifies to: {}\nCounterexample at {}: A = {:.6}, B = {:.6}",
            a_form,
            b_form,
            point_str.join(", "),
            cx.lhs_value,
            cx.rhs_value
        )
    } else {
        format!(
            "Equivalent: likely true [verified at {} point{} — numeric agreement, not proof]\nA simplifies to: {}\nB simplifies to: {}\nDifference: {}",
            result.points_tested,
            if result.points_tested == 1 { "" } else { "s" },
            a_form,
            b_form,
            diff_form
        )
    };
    Ok((text, status))
}

fn tool_verify(args: &Value) -> ToolResult {
    let a_str = get_str(args, "expr_a").ok_or("Missing required parameter: expr_a")?;
    let b_str = get_str(args, "expr_b").ok_or("Missing required parameter: expr_b")?;

    let a_tokens = Tokenizer::new(a_str).tokenize();
    let a_expr = build_expression_tree(a_tokens)?;

    let b_tokens = Tokenizer::new(b_str).tokenize();
    let b_expr = build_expression_tree(b_tokens)?;

    let variables: Vec<String> = args
        .get("variables")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_else(|| vec!["x".to_string()]);

    let assumptions = args
        .get("assumptions")
        .map(Assumptions::from_json)
        .transpose()?
        .unwrap_or_default();

    let result = arithma::verify_identity(&a_expr, &b_expr, &variables, &assumptions);
    let status = classify_verify(&result);
    Ok((format!("{}", result), status))
}

/// Human-readable one-line description of a step's evidence. Caveats are
/// appended for every variant — a witness attached as a caveat (e.g. from a
/// simplify-assisted retry) is the diagnosis, and dropping it here would
/// make "preserved as a caveat" true in the data structure and false on
/// the wire.
fn describe_status(report: &arithma::status::StatusReport) -> String {
    use arithma::status::ResultStatus;
    let base = match &report.status {
        ResultStatus::Exact => "exact".to_string(),
        ResultStatus::Verified { points_tested } => {
            format!(
                "verified at {} point{}",
                points_tested,
                if *points_tested == 1 { "" } else { "s" }
            )
        }
        ResultStatus::Heuristic => "heuristic".to_string(),
        ResultStatus::UnableToCompute { reason } => format!("unable to compute: {}", reason),
        ResultStatus::ProvablyImpossible { proof } => {
            format!("provably impossible: {}", proof.explanation)
        }
    };
    if report.caveats.is_empty() {
        base
    } else {
        format!("{} — {}", base, report.caveats.join("; "))
    }
}

fn chain_text(chain: &ChainResult) -> String {
    let headline = match chain.verdict {
        Verdict::Pass => {
            let evidence = match chain.weakest_step {
                Some(i) => format!(
                    "weakest evidence: {} at step {} \"{}\"",
                    describe_status(&chain.steps[i].status),
                    i,
                    chain.steps[i].label
                ),
                None => "anchor only".to_string(),
            };
            format!(
                "Chain: PASS ({} step{}; {})",
                chain.steps.len(),
                if chain.steps.len() == 1 { "" } else { "s" },
                evidence
            )
        }
        Verdict::Fail => {
            let i = chain.first_failure.expect("fail verdict has a failure");
            format!("Chain: FAIL at step {} \"{}\"", i, chain.steps[i].label)
        }
        Verdict::Inconclusive => {
            let i = chain
                .steps
                .iter()
                .position(|s| s.verdict == Verdict::Inconclusive)
                .expect("inconclusive verdict has an inconclusive step");
            format!(
                "Chain: INCONCLUSIVE at step {} \"{}\"",
                i, chain.steps[i].label
            )
        }
    };

    let mut lines = vec![headline];
    for (i, step) in chain.steps.iter().enumerate() {
        let line = match step.relation {
            None => format!("  {}. {} — anchor", i, step.label),
            Some(rel) => {
                let mut l = format!(
                    "  {}. {} [{}] — {} ({}; {})",
                    i,
                    step.label,
                    rel.as_str(),
                    step.verdict.as_str(),
                    describe_status(&step.status),
                    step.mechanism
                );
                if let Some(cx) = step.status.counterexample_json() {
                    l.push_str(&format!("\n     counterexample: {}", cx));
                }
                l
            }
        };
        lines.push(line);
    }
    lines.join("\n")
}

fn tool_verify_chain(args: &Value) -> Result<(String, Value), String> {
    let steps_arr = args
        .get("steps")
        .and_then(|v| v.as_array())
        .ok_or("Missing required parameter: steps (array of step objects)")?;

    let mut steps: Vec<ChainStepInput> = Vec::with_capacity(steps_arr.len());
    for (i, s) in steps_arr.iter().enumerate() {
        let obj = s
            .as_object()
            .ok_or_else(|| format!("steps[{}] must be an object", i))?;
        let expr = obj
            .get("expr")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("steps[{}] is missing required field: expr", i))?;
        let relation = match obj.get("relation").and_then(|v| v.as_str()) {
            Some(r) => Relation::parse(r).map_err(|e| format!("steps[{}]: {}", i, e))?,
            None => Relation::Equals,
        };
        steps.push(ChainStepInput {
            label: obj
                .get("label")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            expr: expr.to_string(),
            relation,
            variable: obj
                .get("variable")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            value: obj
                .get("value")
                .and_then(|v| v.as_str())
                .map(str::to_string),
        });
    }

    let env = env_from_args(args)?;
    let chain = verify_chain(&steps, &env)?;

    // Chain-level status: the minimum evidence across steps, with the
    // machine-readable verdict and the full per-step audit trail.
    let mut status_json = chain.status.clone().with_verdict(chain.verdict).to_json();
    status_json["steps"] = Value::Array(
        chain
            .steps
            .iter()
            .map(|step| {
                json!({
                    "label": step.label,
                    "relation": step.relation.map(|r| r.as_str()),
                    "verdict": step.verdict.as_str(),
                    "mechanism": step.mechanism,
                    "status": step.status.to_json(),
                })
            })
            .collect(),
    );
    if let Some(i) = chain.first_failure {
        status_json["first_failure"] = json!(i);
    }
    if let Some(i) = chain.weakest_step {
        status_json["weakest_step"] = json!(i);
    }

    // Loud chain statuses get the same text marker as every other tool.
    let text = chain_text(&chain);
    let text = match chain.status.marker() {
        Some(marker) => format!("{}\n{}", marker, text),
        None => text,
    };
    Ok((text, status_json))
}

fn tool_solve_ode(args: &Value) -> ToolResult {
    if let Some(poly_arr) = args.get("poly_coeffs").and_then(|v| v.as_array()) {
        return tool_solve_ode_series(args, poly_arr);
    }

    let has_cc = args.get("a").is_some() && args.get("b").is_some() && args.get("c").is_some();

    if has_cc {
        let a = args
            .get("a")
            .and_then(|v| v.as_f64())
            .ok_or("Invalid coefficient a")?;
        let b = args
            .get("b")
            .and_then(|v| v.as_f64())
            .ok_or("Invalid coefficient b")?;
        let c = args
            .get("c")
            .and_then(|v| v.as_f64())
            .ok_or("Invalid coefficient c")?;
        let indep = normalize_var(get_str_or(args, "indep", "x"));
        // Closed-form solution via the characteristic equation — exact.
        arithma::ode::solve_constant_coeff_latex(a, b, c, &indep).map(|t| {
            (
                t,
                StatusReport::exact(Certificate::by_construction(
                    "characteristic_equation — closed-form ODE solution",
                )),
            )
        })
    } else {
        let expr =
            get_str(args, "expr").ok_or("Missing expr (first-order) or a,b,c (second-order)")?;
        let indep = normalize_var(get_str_or(args, "indep", "x"));
        let dep = normalize_var(get_str_or(args, "dep", "y"));
        arithma::ode::solve_ode_latex(expr, &indep, &dep).map(|t| {
            (
                t,
                StatusReport::exact(Certificate::by_construction(
                    "closed_form_ode — separable/linear/exact method",
                )),
            )
        })
    }
}

fn f64_to_rational(v: f64) -> num_rational::BigRational {
    use num_bigint::BigInt;
    use num_rational::BigRational;
    if v.fract() == 0.0 && v.is_finite() && v.abs() < i64::MAX as f64 {
        BigRational::from_integer(BigInt::from(v as i64))
    } else {
        let scale = 1_000_000_000i64;
        let scaled = (v * scale as f64).round() as i64;
        let r = BigRational::new(BigInt::from(scaled), BigInt::from(scale));
        r.reduced()
    }
}

fn tool_solve_ode_series(args: &Value, poly_arr: &[Value]) -> ToolResult {
    use num_rational::BigRational;

    if poly_arr.len() < 2 {
        return Err("poly_coeffs must have at least 2 elements (first-order ODE)".to_string());
    }

    let indep = normalize_var(get_str_or(args, "indep", "x"));
    let order = args.get("order").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

    let mut coeffs: Vec<Polynomial> = Vec::new();
    for (i, poly_val) in poly_arr.iter().enumerate() {
        let arr = poly_val
            .as_array()
            .ok_or_else(|| format!("poly_coeffs[{}] must be an array", i))?;
        let cs: Vec<BigRational> = arr
            .iter()
            .map(|v| {
                v.as_f64()
                    .map(f64_to_rational)
                    .ok_or_else(|| format!("poly_coeffs[{}] contains non-numeric value", i))
            })
            .collect::<Result<Vec<_>, _>>()?;
        coeffs.push(Polynomial::from_coeffs(cs, &indep));
    }

    let iv = args.get("initial_values").and_then(|v| v.as_array());

    if let Some(iv_arr) = iv {
        let initial_values: Vec<BigRational> = iv_arr
            .iter()
            .map(|v| {
                v.as_f64()
                    .map(f64_to_rational)
                    .ok_or("initial_values contains non-numeric value".to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;

        let sol = arithma::ode::solve_series_ivp(&coeffs, &initial_values)?;
        let poly = sol.truncate(order, &indep);
        let coeffs_list: Vec<String> = (0..=order).map(|i| format!("{}", sol.coeff(i))).collect();
        Ok((
            format!(
                "y = {} + O({}^{})\nCoefficients: [{}]",
                poly,
                indep,
                order + 1,
                coeffs_list.join(", ")
            ),
            StatusReport::exact(Certificate::by_construction(
                "exact_rational_recurrence — power series coefficients from exact arithmetic",
            ))
            .with_caveat(&format!(
                "power series truncated at order {}; coefficients are exact",
                order
            )),
        ))
    } else {
        let solutions = arithma::ode::solve_series(&coeffs)?;
        let k = solutions.len();
        let mut parts = Vec::new();
        for (i, sol) in solutions.iter().enumerate() {
            let poly = sol.truncate(order, &indep);
            let coeffs_list: Vec<String> =
                (0..=order).map(|j| format!("{}", sol.coeff(j))).collect();
            parts.push(format!(
                "y_{} = {} + O({}^{})\nCoefficients: [{}]",
                i + 1,
                poly,
                indep,
                order + 1,
                coeffs_list.join(", ")
            ));
        }
        Ok((
            format!(
                "Power series solution ({} independent solution{}, {} terms):\n{}",
                k,
                if k == 1 { "" } else { "s" },
                order + 1,
                parts.join("\n\n")
            ),
            StatusReport::exact(Certificate::by_construction(
                "exact_rational_recurrence — power series coefficients from exact arithmetic",
            ))
            .with_caveat(&format!(
                "power series truncated at order {}; coefficients are exact",
                order
            )),
        ))
    }
}
