# Result Status: The Evidence Taxonomy

Every Arithma tool response carries a machine-readable statement of *what kind of
evidence* backs the result. This is the foundation of the verification direction
(Discussion #63): an agent consuming Arithma's output must be able to distinguish
"this is an algebraic identity" from "this agreed at 12 test points" from "this
transformation is believed sound but was not independently checked." Those are
different epistemic states, and conflating them is precisely the error an
agent-facing CAS exists to prevent.

## The five statuses

| Status | Meaning | Evidence carried |
|---|---|---|
| `exact` | The result follows from a decision procedure or a complete, sound algebraic algorithm. Canonical-form equality over ℚ, derivative rules, exact rational arithmetic, Berlekamp–Zassenhaus factorization, Gaussian elimination over ℚ. | — |
| `verified` | The result was independently checked numerically. Not a proof: agreement at *n* points. | `points_tested`, optionally `counterexample` (when the *verdict itself* is "not equal" and the counterexample is the evidence) |
| `heuristic` | A transformation was applied that is believed sound, but the result was not independently verified (e.g. too few valid test points in the domain). | `caveats` explain why |
| `unable_to_compute` | The tool understood the request but could not produce an answer. This is an honest "I don't know," distinct from a protocol error. | `reason` |
| `provably_impossible` | The tool *proved* no answer exists in the requested class (e.g. Risch/Liouville non-elementarity). This is a theorem, not a failure. | `certificate` — the reason, human-readable |

Two design rules inherited from the verify_chain design work:

1. **Numeric evidence never masquerades as proof.** A status can be *downgraded*
   by the pipeline (an exact step followed by an unverified rewrite is at best
   `heuristic`) but never upgraded: no amount of point-testing produces `exact`.
2. **The counterexample is the diagnosis.** When a check fails, the response
   carries the specific point and both values. No generative repair.

## JSON payload

The MCP `tools/call` result gains a `result_status` object as a sibling of
`content`:

```json
{
  "content": [{ "type": "text", "text": "\\frac{x^3}{3}" }],
  "result_status": {
    "status": "exact"
  }
}
```

```json
{
  "result_status": {
    "status": "verified",
    "points_tested": 12,
    "caveats": ["transcendental rewrite checked numerically, not algebraically"]
  }
}
```

```json
{
  "result_status": {
    "status": "provably_impossible",
    "certificate": "e^{x^2} has no elementary antiderivative (Risch: no solution to the Risch differential equation)"
  }
}
```

**The `verdict` field.** Tools whose result *is* a yes/no claim (`verify`,
`equivalent`, `verify_chain`) additionally carry a machine-readable
`verdict`: `"pass"`, `"fail"`, or `"inconclusive"` — one vocabulary across
all three tools, so no consumer ever parses prose to learn an outcome.
Verdict and status are orthogonal: "not equal, counterexample attached" is a
`fail` verdict carried by well-earned `verified` evidence.

```json
{
  "result_status": {
    "status": "verified",
    "verdict": "fail",
    "points_tested": 4,
    "counterexample": { "point": { "x": 0.5 }, "lhs": 2.25, "rhs": 1.25 }
  }
}
```

**Extensibility contract.** Consumers switch on the `status` string and ignore
unknown fields. New evidence fields and new caveat strings are non-breaking
additions. New `status` values are additions reserved for a version bump and
announced in advance (`approximate` — for single-point floating-point evaluation
— is the designated first candidate; until then such results are `verified` with
`points_tested: 1` and an explanatory caveat).

## Backward compatibility

This feature is strictly additive. Specifically:

- **Happy-path text is byte-identical.** For `exact` and `verified` results, the
  `content[0].text` an MCP client sees does not change. The status object is a
  new sibling field, which JSON-RPC clients ignore if unknown.
- **Loud cases get a text marker.** When status is `heuristic`,
  `unable_to_compute`, or `provably_impossible`, the text gains a marker line
  (e.g. `[provably impossible] <certificate>`). This is a deliberate repair, not
  a break: previously, `integrate` on a non-elementary integrand returned the
  explanation *prose as if it were an antiderivative* — indistinguishable from
  success without reading English. That was the bug this taxonomy exists to fix.
- **CLI output is unchanged** except for the same marker in the impossible /
  unable cases.
- **No library signature changes.** New module `src/status.rs`, new functions
  alongside existing ones. The `NON_ELEMENTARY:` error-string convention inside
  the library is unchanged (the status layer interprets it at the tool
  boundary). Existing tests pass unmodified.
- **WASM bindings untouched** this iteration.

## How each status is earned (per tool)

The heart of the design: a status must be *earned* by the mechanism that
justifies it, never asserted by optimism.

| Tool | Classification mechanism |
|---|---|
| `format` | `exact` always — parsing and canonical printing make no equivalence claim beyond structure. |
| `simplify` | If input and output are both polynomial/rational over ℚ (field ops + integer powers only), canonicalization is a decision procedure → `exact`. If transcendental subexpressions are present, run the numeric self-check (`verify_identity(input, output)`): pass → `verified` with point count; insufficient valid points → `heuristic` with caveat. A self-check *failure* is a simplifier bug surfaced in production: `heuristic` with a loud caveat carrying the counterexample. |
| `differentiate` | Derivative rules are complete and sound → `exact`; final simplification inherits the simplify classification (minimum of the two). |
| `integrate` (indefinite) | Differentiation round-trip: d/dx of the antiderivative, compare to integrand. Structural match after simplification → `exact` (the round-trip is algebraic — this is why `integral_of` can reach `exact` where `implies` cannot). Numeric-only agreement → `verified`. Risch non-elementarity → `provably_impossible` with certificate. |
| `integrate` (definite) | The FTC path first checks the integrand for singularities inside [a, b] (exact roots for polynomial denominators, sign-change/magnitude scan otherwise) and refuses improper integrals. It then inherits the antiderivative's round-trip status; special-value evaluations are `exact`. |
| `substitute` | Capture-avoiding substitution is algebraic → `exact`. |
| `solve` | Symbolic root formulas (rational-root, quadratic) → `exact`. Cubic/quartic paths that degrade to f64 root-finding → `verified` with an f64 caveat — the status conditions on the code path taken, not the tool name. Inequalities via sign analysis → `exact`. (Back-substitution self-audit is a planned follow-up.) |
| `solve_system` | Exact Gaussian elimination / substitution over ℚ → `exact`. |
| `factor` | Berlekamp–Zassenhaus is exact → `exact`. |
| `partial_fractions` | Exact rational arithmetic → `exact`. |
| `limit` | Symbolic result corroborated numerically by sampling the approach (when point and result are numeric): agreement → `verified`; error contracting but not yet within tolerance → quiet `heuristic` ("slow convergence", never a false alarm); contradiction → loud `heuristic`; corroboration unavailable (symbolic parameters) → quiet `heuristic` with caveat. |
| `taylor_series` | Exact rational coefficient recurrences → `exact`, with truncation-order caveat. |
| `evaluate` | Exact-rational path → `exact`. Floating-point path → `verified` with `points_tested: 1` and caveat `"floating-point evaluation (f64)"`. |
| `matrix` | Exact arithmetic over ℚ / symbolic entries → `exact`. Numeric eigenvalue root-finding (detected by floating-point output) → `verified` with an f64 caveat; complex pairs are explicit as re ± im·i, recovered by deflation or refused — never fabricated. |
| `equivalent` | Structural or difference-zero match → `exact`. Numeric-only agreement → `verified` with point count. Disagreement → the *"not equivalent"* verdict is `verified` with the counterexample as evidence. Carries a machine-readable `verdict`. |
| `verify` | PASS → `verified` with point count (never `exact` — this tool is numeric by definition). FAIL → `verified` carrying the counterexample. INCONCLUSIVE → `unable_to_compute` with reason. Carries a machine-readable `verdict`. |
| `verify_chain` | Per-relation mechanisms (see the verify_chain section below); chain status = minimum across steps; `implies` capped at `verified`. Carries `verdict`, per-step `mechanism`, `first_failure`, `weakest_step`. |
| `solve_ode` | Closed-form paths → `exact`. Series solutions → `exact` coefficients with truncation caveat. |

Errors of protocol (missing parameters, unparseable LaTeX) remain JSON-RPC
errors (`isError: true`); they are not mathematical results and get no status.
Library error strings that represent *mathematical limitations* ("no technique
applies") will migrate to `unable_to_compute` as the library error taxonomy is
refactored — a follow-up, not this change.

## verify_chain

The chain verifier consumes these statuses directly: a chain's status is the
**minimum** across its steps (one numeric step makes the whole chain
`verified`, never `exact`), and audit witnesses are exactly the evidence
fields defined here. This document is therefore the schema contract between
the two features.

A chain is an ordered list of steps; each step after the first (the anchor)
declares a relation to its predecessor. How each relation earns its status:

| Relation | Mechanism | Can earn `exact`? |
|---|---|---|
| `equals` | Syntactic identity (structural tree equality) → unit-normal form (u·1, u+0, u^1, −(−u): identities in every interpretation, no side conditions) → canonical form over ℚ (poly/rational fragment only) → **in-fragment: exact rational evaluation at sample points, no floating-point tolerance anywhere** → outside the fragment: assumption-aware f64 sampling | Yes, inside the fragment. In-fragment disagreement is a *disproof*: exact arithmetic exhibits a point where the values differ, so a provably false step like x = x + 10⁻¹⁵ is refuted, never tolerated. Transcendental agreement caps at `verified` — structural agreement after simplification is only as trustworthy as the simplifier's rewrite rules, which is precisely what a chain verifier must not assume. |
| `derivative_of` | Derivative rules (complete, sound), then the `equals` ladder on the result | Yes |
| `integral_of` | Differentiation round-trip: d/dx(step) compared to predecessor. Constants of integration vanish under d/dx and cannot cause a false fail. | Yes — the round-trip is algebraic. |
| `substitution` | Capture-avoiding substitution, then the `equals` ladder (follows variable-set changes) | Yes |
| `solution_of` | Substitute the claimed root into the equation; exact arithmetic decides membership. A checker, not a finder. | Yes, for roots inside the ℚ fragment, with a caveat: membership is proven, completeness of the solution set is not claimed. Irrational roots (x = √2 for x² = 2) currently land at `verified` — algebraic-number membership belongs to the certificate work. |
| `implies` | Solve the antecedent, check every solution against the consequent. A violating solution refutes the implication and is the counterexample. | **No — capped at `verified` by design.** Finitely many checked solutions are evidence, not proof of implication. |
| `factored_form_of` | The `equals` ladder (expansion happens in canonicalization) | Yes |

Per-step results carry `verdict`, `mechanism` (so over-claims are auditable),
and a full status object. The chain-level `result_status` carries `steps`,
`first_failure`, and `weakest_step`; its evidence is the weakest step's
report when the chain passes or is inconclusive, and the **first failing
step's report when the chain fails** — the diagnosis is never outranked by
a passing step. Failing steps carry the counterexample — the counterexample
is the diagnosis, and no generative repair is attempted.

On a FAIL chain, `weakest_step` still marks the evidence floor and may
differ from `first_failure`; consumers explaining a failure should follow
`first_failure`, not `weakest_step`.

**Equality notion.** In-fragment `exact` means equality in the rational
function field ℚ(x₁,…,xₙ) — the standard CAS convention, in which removable
domain differences do not exist (`0·(1/x) = 0`, and `(x²−1)/(x−1) = x+1`).
Pointwise equality of partial functions is a different notion; declaring
and reconciling the two across tools is `ar-equality-notion` (Carl F4).

Sampling notes: the built-in constants `e` and `π` are never treated as
free variables (a "counterexample" that rebinds Euler's constant is a lie);
sample points where both sides are undefined test domain agreement, not
values, and carry no evidence; a substitution that would capture a bound
summation/product index is a *step-level* `inconclusive` naming the capture
— the rest of the chain still reports (audit trail over abort); exact-
arithmetic counterexamples carry `lhs_exact`/`rhs_exact` strings alongside
the f64 renderings, because two distinct rationals can share an f64 image.

For incremental use (checking one new step against the last accepted one),
send a two-step chain.
