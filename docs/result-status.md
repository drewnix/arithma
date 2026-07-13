# Result Status: The Evidence Taxonomy

Every Arithma tool response carries a machine-readable statement of *what kind of
evidence* backs the result. This is the foundation of the verification direction
(Discussion #63): an agent consuming Arithma's output must be able to distinguish
"this is an algebraic identity" from "this agreed at 12 test points" from "this
transformation is believed sound but was not independently checked." Those are
different epistemic states, and conflating them is precisely the error an
agent-facing CAS exists to prevent.

## Schema v2 (breaking changes, one release)

Four consumer-side changes land together so the payload breaks once:

1. **Caveats are `{code, message}` objects**, no longer bare strings. The
   `code` comes from a fixed registry (below) and is the machine surface;
   the `message` is prose with **no contract** — it may be reworded at any
   time. Consumers that matched caveat text must switch on codes.
2. **New status `approximate`** for floating-point values, carrying
   `significant_digits` when first-order error propagation could bound the
   error. Consumers with exhaustive status switches must add the arm.
3. **`result_status` is also emitted inside `structuredContent`** (additive).
   Typed MCP SDKs drop unknown top-level fields, so the sibling alone never
   reached them; both surfaces carry the identical object.
   Also additive: **`error_bound`** — present on every outcome of a bounded
   numeric comparison, carrying the propagated absolute error bound the
   outcome was judged against. The bound is the domain of the outcome: a
   pass certifies agreement only within ±`error_bound`.
4. **`points_tested` semantics documented** (clarification, not a change):
   on PASS it is the sample size; on FAIL it is the number of points
   examined up to and including the counterexample that stopped the search.

## The six statuses

| Status | Meaning | Evidence carried |
|---|---|---|
| `exact` | The result follows from a decision procedure or a complete, sound algebraic algorithm, backed by a checked `certificate`. The certificate proves the result by naming the check (replay or decision procedure) and recording that it passed. At the tool boundary, exact without a checked certificate is downgraded to heuristic. | `certificate` — `{kind, witness, checked}` |
| `verified` | The result was independently checked numerically. Not a proof: agreement at *n* points. | `points_tested`, optionally `counterexample` (when the *verdict itself* is "not equal" and the counterexample is the evidence) |
| `approximate` | A floating-point value. `significant_digits` states how many leading decimal digits are trustworthy, from first-order error propagation through the computation; absent when no bound could be computed — an untracked bound is never defaulted into a number. Weaker than `verified` (nothing checked it independently), stronger than `heuristic` (its precision is stated rather than unknown). | `significant_digits` (optional) |
| `heuristic` | A transformation was applied that is believed sound, but the result was not independently verified (e.g. too few valid test points in the domain). | `caveats` explain why |
| `unable_to_compute` | The tool understood the request but could not produce an answer. This is an honest "I don't know," distinct from a protocol error. Includes floating-point results whose significant digits were entirely destroyed by cancellation or ill-conditioning — a value with zero trustworthy digits is noise, not a result. | `reason` |
| `provably_impossible` | The tool *proved* no answer exists in the requested class (e.g. Risch non-elementarity, negative discriminant, Abel-Ruffini). This is a theorem, not a failure. | `proof_certificate` — structured: `{method, reason, explanation}` |

Three design rules:

1. **Numeric evidence never masquerades as proof.** A status can be *downgraded*
   by the pipeline (an exact step followed by an unverified rewrite is at best
   `heuristic`) but never upgraded: no amount of point-testing produces `exact`.
2. **The counterexample is the diagnosis.** When a check fails, the response
   carries the specific point and both values. No generative repair.
3. **No certificate, no exact.** The tool boundary grants `exact` only after a
   certificate proves it. An empty certificate slot cannot be defaulted into
   anything — the refusal-becomes-default disease dies by construction.

## JSON payload

The MCP `tools/call` result carries `result_status` in two places with the
identical object: as a sibling of `content` (raw-JSON consumers) and inside
`structuredContent` (typed SDKs, which drop unknown top-level fields):

```json
{
  "content": [{ "type": "text", "text": "\\frac{x^3}{3}" }],
  "result_status": {
    "status": "exact",
    "certificate": {
      "kind": "differentiation_round_trip",
      "witness": "d/dx of antiderivative matches integrand structurally",
      "checked": true
    }
  },
  "structuredContent": { "result_status": { "status": "exact", "certificate": { "...": "identical" } } }
}
```

```json
{
  "result_status": {
    "status": "verified",
    "points_tested": 12,
    "caveats": [
      {
        "code": "corroborated",
        "message": "transcendental rewrite checked numerically, not algebraically"
      }
    ]
  }
}
```

```json
{
  "result_status": {
    "status": "approximate",
    "significant_digits": 13,
    "caveats": [
      { "code": "f64_precision", "message": "floating-point evaluation (f64 precision)" }
    ]
  }
}
```

```json
{
  "result_status": {
    "status": "provably_impossible",
    "proof_certificate": {
      "method": "risch-de",
      "reason": "No elementary antiderivative exists. The differential equation q' + (2x)·q = 1 has no rational solution.",
      "explanation": "This integral has no formula using elementary functions (polynomials, exponentials, logarithms, trigonometric). This is a theorem, not a limitation of the tool."
    }
  }
}
```

**Proof certificate methods.** The `method` field classifies the impossibility proof:

| Method | Proof type | Used by |
|---|---|---|
| `risch-de` | Risch differential equation has no rational solution | `integrate` |
| `rothstein-trager` | Rothstein-Trager resultant has no rational roots | `integrate` |
| `risch` | Generic Risch non-elementarity proof | `integrate` |
| `negative-discriminant` | Quadratic discriminant is negative — no real roots | `solve` |
| `all-roots-complex` | All polynomial roots are complex | `solve` |
| `contradiction` | Equation reduces to nonzero = 0 | `solve` |

**Special-function recognition fields.** When a `provably_impossible`
integration result's antiderivative is a recognized special function (erf,
Ei, li — each table entry is a defining identity: DLMF 7.2.1, 6.2.5, 6.2.8),
the status additionally carries `special_function` (the name) and
`special_form` (the full antiderivative as LaTeX). The theorem is unchanged —
erf is not elementary — and the fields are strictly additive per the
extensibility contract below. Recognition is earned: the structural match is
guarded by a numeric differentiation round-trip, and any failure drops the
name and keeps the bare certificate. An unrecognized integrand never gets a
guessed name.

```json
{
  "result_status": {
    "status": "provably_impossible",
    "proof_certificate": {
      "method": "risch-de",
      "reason": "No elementary antiderivative exists. The differential equation q' + (-2x)·q = 1 has no rational solution.",
      "explanation": "This integral has no formula using elementary functions. This is a theorem, not a limitation of the tool."
    },
    "special_function": "erf",
    "special_form": "\\frac{\\sqrt{\\pi}}{2} \\cdot \\erf(x)"
  }
}
```

## Certificate kinds

The `exact` status carries a `certificate` object proving the result. Two
families:

**Replay certificates** — the result was independently verified by replaying a
cheap check in exact arithmetic. Finding is hard, checking is easy.

| Kind | Replay check | Used by |
|---|---|---|
| `factor_multiply_back` | Multiply all factors back, compare to input polynomial | `factor` |
| `substitution_check` | Substitute each root into the equation, verify residual zero | `solve` |
| `differentiation_round_trip` | Differentiate the antiderivative, compare to integrand | `integrate`, `verify_chain` (integral_of) |
| `system_substitution_check` | Substitute solution vector into each equation, verify zero | `solve_system` |
| `inverse_multiply_check` | Multiply A × A⁻¹, compare to identity matrix | `matrix` (inverse) |
| `partial_fractions_multiply_back` | Multiply partial fractions by denominator, compare to numerator | `partial_fractions` |
| `interpolation_identity_Q` | Exact evaluation on a grid exceeding the degree bound (polynomial identity theorem) | `verify_chain` (equals) |
| `exact_rational_sample` | Disagreement in exact rational arithmetic — a disproof | `verify_chain` (equals) |

**Construction certificates** — the algorithm is a decision procedure or
provably complete and sound. The computation IS the proof; no separate replay.

| Kind | Algorithm | Used by |
|---|---|---|
| `decision_procedure` | Canonical-form comparison, sign analysis, syntactic identity, unit-normal-form, etc. | `simplify`, `equivalent`, `verify_chain`, `evaluate`, `solve` (inequality), `taylor_series`, `matrix`, `solve_ode`, etc. |

At the tool boundary, `to_json()` enforces invariant 3: if the status is
`exact` and the certificate is missing or `checked: false`, the output
downgrades to `heuristic` with a caveat. This makes classifier over-claims
structurally impossible.

## Caveats: `{code, message}` pairs

Caveats are orthogonal to the status: domain restrictions, precision notes,
corroboration outcomes, method-scope statements. Each carries a stable
machine `code` from the registry below and a human `message`. **The code is
the contract; the message is not** — consumers must never regex the prose
(the same token can appear in a confirmation and a refutation, and prose is
reworded freely).

| Code | Meaning |
|---|---|
| `f64_precision` | Computed in f64 floating point; exactness not claimed. |
| `self_check_failed` | An independent recomputation disagreed with the result — the tool's own output is under suspicion. |
| `check_unavailable` | The independent check could not run; the result is unvalidated, not contradicted. |
| `check_inconclusive` | The check ran but could not conclude (e.g. insufficient sampling). |
| `not_corroborated` | A symbolic claim with no numeric corroboration path. |
| `corroborated` | Numeric samples agree with the claim. |
| `slow_convergence` | Samples move toward the claim but below tolerance — consistent, not confirming. |
| `corroboration_failed` | Numeric samples contradict the claim. |
| `domain_mismatch` | One side undefined at some sample points; values compared only where both defined. |
| `truncation` | A series result is truncated at a stated order. |
| `sub_resolution` | Two constants agree only within the propagated floating-point error bound — equality at f64 resolution, not proof. |
| `catastrophic_cancellation` | Subtractive cancellation destroyed the significant digits. Often remediable: rewriting the cancelling subtraction (e.g. 1 − cos x = 2sin²(x/2)) can recover the value. |
| `ill_conditioned` | The computation is ill-conditioned at this input (e.g. trig argument reduction at huge arguments); the digits are lost to the condition number itself and NO rewrite recovers them in f64. |
| `margin_band` | The disagreement lies inside the refutation safety margin — larger than the error bound, smaller than the refutation threshold; too uncertain to confirm or refute. |
| `binder_capture` | A substitution was refused because it would capture a bound Σ/Π index. |
| `solver_incomplete` | The solver could not produce the solutions a check needed — a solver limitation, not a theorem. |
| `not_evaluable` | An expression the check needed could not be evaluated numerically. |
| `insufficient_sampling` | The sampler could not gather enough valid test points to conclude anything. |
| `uncertified_exact` | An `exact` claim reached the boundary without a checked certificate and was downgraded. |
| `exact_disagreement` | Disagreement established in exact rational arithmetic — a disproof, not a tolerance judgement. |
| `symbolic_imaginary` | Complex quantities expressed with `i` as a symbol. |
| `chain_structure` | A structural property of the chain (anchors, degenerate shapes), not a mathematical judgement. |
| `unevaluated` | The evaluation returned a simplified-but-unevaluated form instead of a value. |
| `method_scope` | A statement of what the check's method does and does not certify. |
| `complex_omitted` | Complex solutions omitted from a real-valued comparison. |
| `disagreement_witness` | The specific witness of a disagreement, in prose, beside the structured counterexample. |

Adding a code is additive (consumers ignore unknown codes); renaming or
removing one is a breaking schema change.

Two registry rules, learned the expensive way:

- **Every refusal carries a code.** An `unable_to_compute` without a caveat
  code is a shrug with good grammar: five different situations demanding
  five different responses must not collapse into one status with the
  diagnosis living in contract-free prose. When you add an outcome, give it
  a code.
- **A code names the mechanism that actually fired, not a neighbor.**
  `catastrophic_cancellation` and `ill_conditioned` have OPPOSITE remedies
  (rewrite the subtraction vs. nothing helps); the error tracker attributes
  which one occurred rather than guessing. And a coincidence is not a
  category: whether an f64 difference lands on exactly 0.0 does not change
  the code — exact float agreement is the signature of total cancellation,
  the case to trust least, and must never draw a more reassuring code than
  ordinary sub-resolution agreement.

**`points_tested` semantics.** The field's meaning switches on the verdict:
on **PASS** it is the sample size (how many points agreed); on **FAIL** it
is the number of points examined up to and including the counterexample
that stopped the search — a measure of search effort, not of agreement.
Averaging or comparing `points_tested` across mixed verdicts is a category
error. On INCONCLUSIVE the status is `unable_to_compute` and the count, if
any, appears in the `reason`.

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
unknown fields. New evidence fields and new caveat *codes* are non-breaking
additions. New `status` values are reserved for a version bump and announced
in advance (`approximate` was the designated first candidate and landed in
schema v2; floating-point evaluation results previously reported as
`verified` with `points_tested: 1` now report `approximate`).

## Backward compatibility

This feature is strictly additive. Specifically:

- **Happy-path text is byte-identical** — with one deliberate, documented
  exception. For `exact` and `verified` results, the `content[0].text` an MCP
  client sees does not change; the status object is a new sibling field, which
  JSON-RPC clients ignore if unknown. The exception is the is-this-proven
  tools: **`equivalent` carries its evidence tier in-band in the text**
  (`Equivalent: true [exact]`, `Equivalent: likely true [verified at 12
  points — numeric agreement, not proof]`), and `verify` carries its point
  count. Rationale: many MCP hosts deliver only `content.text` to the agent —
  the `result_status` sidecar never arrives — and for these tools the tier
  *is* the answer. A quiet tier turns "decision procedure" and "agreed at 12
  points" into the same sentence, which is precisely the
  numeric-check-as-proof conflation the taxonomy exists to prevent.
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
| `integrate` (indefinite) | Differentiation round-trip: d/dx of the antiderivative, compare to integrand. Structural match after simplification → `exact` (the round-trip is algebraic — this is why `integral_of` can reach `exact` where `implies` cannot). Numeric-only agreement → `verified`. Risch non-elementarity → `provably_impossible` with certificate; when the antiderivative is a recognized special function (erf, Ei, li), the status also carries `special_function`/`special_form` — recognition guarded by a differentiation round-trip, never guessed. |
| `integrate` (definite) | The FTC path first checks the integrand for singularities inside [a, b] (exact roots for polynomial denominators, sign-change/magnitude scan otherwise) and refuses improper integrals. It then inherits the antiderivative's round-trip status; special-value evaluations are `exact`. |
| `substitute` | Capture-avoiding substitution is algebraic → `exact`. |
| `solve` | Symbolic root formulas (rational-root, quadratic) → `exact`. Cubic/quartic paths that degrade to f64 root-finding → `verified` with an f64 caveat. All roots complex (negative discriminant for quadratics, exhaustive for degree ≤ 4) → `provably_impossible` with method `negative-discriminant` / `all-roots-complex`. Contradiction (nonzero = 0) → `provably_impossible` with method `contradiction`. Irreducible degree-≥5 factors → `unable_to_compute` (degree ≥ 5 irreducibility does not prove Abel-Ruffini without Galois group computation; x⁵−2 has radical root ⁵√2). Inequalities via sign analysis → `exact`. |
| `solve_system` | Exact Gaussian elimination / substitution over ℚ → `exact`. |
| `factor` | Berlekamp–Zassenhaus is exact → `exact`. |
| `partial_fractions` | Exact rational arithmetic → `exact`. |
| `limit` | Symbolic result corroborated numerically by sampling the approach (when point and result are numeric): agreement → `verified`; error contracting but not yet within tolerance → quiet `heuristic` ("slow convergence", never a false alarm); contradiction → loud `heuristic`; corroboration unavailable (symbolic parameters) → quiet `heuristic` with caveat. |
| `taylor_series` | Exact rational coefficient recurrences → `exact`, with truncation-order caveat. |
| `evaluate` | Exact-rational path → `exact`. Floating-point path → `approximate` with `significant_digits` from first-order error propagation through the expression tree (leaves start at conversion error, each operation applies its sensitivity, subtraction of near-equal quantities amplifies); when no error model exists for the expression, `approximate` without the field. **Zero surviving digits → `unable_to_compute`** with caveat `catastrophic_cancellation`: (1−cos x)/x² at x = 10⁻⁸ computes 0 in f64 while the true value is ½ — noise is refused, not published with a precision label. An expression that does not evaluate to a number at all → `unable_to_compute` with caveat `unevaluated` (the simplified form still reaches the text). |
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
| `equals` (expressions) | Syntactic identity (structural tree equality) → unit-normal form (u·1, u+0, u^1, −(−u): identities in every interpretation, no side conditions) → canonical form over ℚ (poly/rational fragment only) → **in-fragment: degree-aware exact rational evaluation.** Within budget, agreement on a grid exceeding the difference's per-variable degree bounds is the polynomial identity theorem — a decision, mechanism `interpolation_identity_Q`. Over budget or starved of valid points: bounded exact sampling (still zero tolerance), `verified` with the shortfall named. Outside the fragment: assumption-aware f64 sampling. **Variable-free comparisons** (mechanism `numeric_constant_eval_bounded`) measure the disagreement against the *propagated floating-point error bound* of the two computations rather than a fixed tolerance — e^{-50} = 0 fails honestly (difference ≫ bound) while sin(2π) = 0 passes (value within its own bound of zero); refutation requires a 4× safety margin over the first-order bound. **Resolution gate:** when the bound swamps the comparison scale (`max(|lhs|, |rhs|, 1)` — the unit floor is a documented convention: an absolute bound below 1 counts as resolving evidence for claims about zero), the step is *Inconclusive*, never a pass — agreement inside a bound that admits almost any claim is the absence of resolution, not evidence ((1−cos 10⁻⁸)/10⁻¹⁶ "equals" nothing at f64 precision, including its true value ½; sin(10²⁰) resolves nothing in [−1,1]). This is the same `significant_digits` gate `evaluate` applies before publishing a value, shared by construction. The outcome is **three-way**: agreement inside the bound passes; a disagreement clearing 4× the bound refutes; the band between is *Inconclusive* — the 4× margin exists to prevent false disproofs, and a margin must widen refusal, never agreement. **Invariant: a PASS means what its caveat says** — agreement within the stated bound, enforced, not merely printed. A bounded **pass** reports `approximate`, not `verified` — one f64 agreement within rounding is the paradigm case for the tier. In this context `significant_digits` states the digits the *comparison resolved at its scale* (−log₁₀(bound/scale)), **not** the digits of either value: an f64 residue like sin(2π) ≈ 2.4e-16 has zero correct digits as a value while the comparison resolves ~14 digits at unit scale. Every bounded outcome publishes `error_bound`: the bound is the domain of the outcome. | Yes, inside the fragment — including by interpolation, which is a proof, not a sample. In-fragment disagreement is a *disproof*: exact arithmetic exhibits a point where the values differ, so a provably false step like x = x + 10⁻¹⁵ is refuted, never tolerated; a polynomial constructed to vanish exactly on a fixed sample grid is caught by the degree count. Transcendental agreement caps at `verified`. |
| `equals` (equations) | Two equation-shaped steps are compared by **solution set** (mechanism `solution_set_comparison`): both sides solved, sets compared exactly. This is the semantics under which dividing both sides by 2 is an identity step — residual (pointwise) comparison would refute valid algebra. A solution of one equation missing from the other refutes the step and is the witness. Mixing an equation with an expression is refused with guidance. | **No — capped at `verified`:** the comparison inherits the solver's completeness, which is not proven. |
| `derivative_of` | Derivative rules (complete, sound), then the `equals` ladder on the result. Constant factors differentiate to *literal* zeros (d(c·f) = c·f', no dead f·0 term), so claims scaled by a constant — right or wrong — are checked through the raw path: recognized special-function antiderivatives like (√π/2)·erf(x) pass raw, and a wrong sign or multiple is refuted raw with a counterexample. If the raw comparison is still inconclusive (the residue: the special function survives in the derivative itself, e.g. erf(x)²), the constructed side is simplified and retried — the retry can pass (mechanism prefixed `simplify+`, auditable) but never refute: a disagreement reached only through an unverified transform stays inconclusive with the witness as a caveat, which reaches the rendered text. | Yes |
| `integral_of` | Differentiation round-trip: d/dx(step) compared to predecessor. Constants of integration vanish under d/dx and cannot cause a false fail. Same raw-first, simplify-retry policy as `derivative_of`. | Yes — the round-trip is algebraic. |
| `substitution` | Capture-avoiding substitution, then the `equals` ladder (follows variable-set changes) | Yes |
| `solution_of` | Substitute the claimed root into the equation; exact arithmetic decides membership. A checker, not a finder. A **rounded decimal literal** (x = 1.4142135623 for x² = 2) is provably a NON-root under strict equality, but by design claims *approximate* membership: it passes within the legacy tolerance with a caveat directing the author to supply an exact value for exact membership verification. | Yes, for roots inside the ℚ fragment, with a caveat: membership is proven, completeness of the solution set is not claimed. Irrational roots (x = √2 for x² = 2) currently land at `verified` — algebraic-number membership belongs to the certificate work. |
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
and reconciling the two across tools is tracked as follow-up work.

Sampling notes: the built-in constants `e` and `π` are never treated as
free variables (a "counterexample" that rebinds Euler's constant is a lie);
sample points where both sides are undefined test domain agreement, not
values, and carry no evidence, while a point where exactly one side is
undefined is a domain violation — a counterexample serialized with an
explicit `"undefined"`, never a null; a substitution that would capture a bound
summation/product index is a *step-level* `inconclusive` naming the capture
— the rest of the chain still reports (audit trail over abort); exact-
arithmetic counterexamples carry `lhs_exact`/`rhs_exact` strings alongside
the f64 renderings, because two distinct rationals can share an f64 image.

For incremental use (checking one new step against the last accepted one),
send a two-step chain.
