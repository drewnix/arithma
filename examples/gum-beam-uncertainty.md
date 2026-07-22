# Verified uncertainty propagation: a GUM beam example

This example runs a complete measurement-uncertainty budget — in the sense
of the *Guide to the Expression of Uncertainty in Measurement* (JCGM
100:2008, "GUM") — through Arithma's MCP tools, with **every intermediate
step independently verified** and every result carrying a machine-readable
evidence status.

The point is not that a CAS can differentiate a stress formula. The point
is what a *verified* uncertainty budget looks like: each sensitivity
coefficient is computed symbolically, checked by an independent mechanism,
evaluated in exact rational arithmetic, and the combined uncertainty
matches the hand calculation as the *same rational number* — not "to six
decimal places."

## The measurement model

A simply supported rectangular steel beam with a central point load,
computed in five stages:

| Stage | Quantity | Model |
|-------|----------|-------|
| 1 | Bending moment | $M = \dfrac{FL}{4}$ |
| 2 | Section modulus | $S = \dfrac{bh^2}{6}$ |
| 3 | Bending stress | $\sigma = \dfrac{M}{S} = \dfrac{3FL}{2bh^2}$ |
| 4 | Deflection | $\delta = \dfrac{FL^3}{48EI},\quad I = \dfrac{bh^3}{12}$ |
| 5 | Safety factor | $\mathrm{SF} = \dfrac{Y}{\sigma}$ |

Six parameters with standard uncertainties. Units are chosen (N, mm, MPa)
so that every input is an **integer** — which keeps the entire computation
in exact rational arithmetic, no floating point anywhere:

| Parameter | Value | Standard uncertainty | Relative |
|-----------|-------|---------------------|----------|
| Load $F$ | 1000 N | 10 N | 1% |
| Span $L$ | 2000 mm | 10 mm | 0.5% |
| Width $b$ | 50 mm | 0.5 mm | 1% |
| Height $h$ | 100 mm | 1 mm | 1% |
| Modulus $E$ | 200000 MPa | 2000 MPa | 1% |
| Yield $Y$ | 250 MPa | 2.5 MPa | 1% |

## Step 1 — verify the model composition

Before propagating anything, prove the composed formulas are what you
think they are. The `verify_chain` tool checks each substitution as a
typed step:

```json
{"name": "verify_chain", "arguments": {"steps": [
  {"label": "safety factor vs stress",              "expr": "\\frac{Y}{s}"},
  {"label": "stress from moment and modulus s = M/S", "expr": "\\frac{Y S}{M}",
   "relation": "substitution", "variable": "s", "value": "\\frac{M}{S}"},
  {"label": "bending moment M = FL/4",               "expr": "\\frac{4 Y S}{F L}",
   "relation": "substitution", "variable": "M", "value": "\\frac{F L}{4}"},
  {"label": "section modulus S = bh^2/6",            "expr": "\\frac{2 Y b h^2}{3 F L}",
   "relation": "substitution", "variable": "S", "value": "\\frac{b h^2}{6}"}
]}}
```

```
Chain: PASS (4 steps; weakest evidence: exact at step 1 "stress from moment and modulus s = M/S")
  0. safety factor vs stress — anchor
  1. stress from moment and modulus s = M/S [substitution] — pass (exact; substitute+difference_zero_Q)
  2. bending moment M = FL/4 [substitution] — pass (exact; substitute+canonical_form_Q)
  3. section modulus S = bh^2/6 [substitution] — pass (exact; substitute+canonical_form_Q)
```

The deflection stage verifies the same way ($I = bh^3/12$ substituted into
$FL^3/48EI$ gives $FL^3/4Ebh^3$ — `PASS`, `exact`). The chain's overall
status is the **minimum** evidence across its steps: had any substitution
only been checkable numerically, the whole chain would say `verified`,
never `exact`.

## Step 2 — symbolic sensitivity coefficients

GUM propagates uncertainty through first-order sensitivity coefficients
$c_i = \partial \sigma / \partial x_i$. The `differentiate` tool produces
each one symbolically:

| Input | Call | Result | Status |
|-------|------|--------|--------|
| $F$ | `differentiate(\frac{3 F L}{2 b h^2}, F)` | $\dfrac{3L}{2h^2 b}$ | `exact` |
| $L$ | `differentiate(…, L)` | $\dfrac{3F}{2h^2 b}$ | `exact` |
| $b$ | `differentiate(…, b)` | $\dfrac{-3LF}{2h^2 b^2}$ | `exact` |
| $h$ | `differentiate(…, h)` | $\dfrac{-3LF}{h^3 b}$ | `exact` |

Note the structure: relative sensitivities are 1 for $F$, $L$, $b$ — but
**2 for $h$**, because $h$ enters squared. This factor of 2 (a factor of
4 in variance) is the engineering heart of the example.

## Step 3 — verify each derivative independently

A computed derivative is a claim. Each one is checked as a two-step chain
using the `derivative_of` relation, which re-derives and compares by an
independent mechanism:

```json
{"name": "verify_chain", "arguments": {"steps": [
  {"label": "stress",    "expr": "\\frac{3 F L}{2 b h^2}"},
  {"label": "dsigma_dh", "expr": "\\frac{-3L \\cdot F}{h^{3} \\cdot b}",
   "relation": "derivative_of", "variable": "h"}
]}}
```

All four pass `exact`. The mechanisms differ per case —
`derivative_rules+unit_normal_form`, `+canonical_form_Q`,
`+interpolation_identity_Q` — and each response names the one that
actually ran, so an auditor knows *how* each claim was established, not
just that it was.

## Step 4 — evaluate at the operating point, exactly

With integer inputs, `evaluate` stays in exact rational arithmetic:

| Quantity | Result | Status |
|----------|--------|--------|
| $\sigma$ | $6$ MPa | `exact` |
| $c_F$ | $3/500$ MPa/N | `exact` |
| $c_L$ | $3/1000$ MPa/mm | `exact` |
| $c_b$ | $-3/25$ MPa/mm | `exact` |
| $c_h$ | $-3/25$ MPa/mm | `exact` |
| $\mathrm{SF}$ | $125/3$ | `exact` |

## Step 5 — combined uncertainty (law of propagation)

$u_c^2(\sigma) = \sum_i (c_i\, u_i)^2$, assembled as one exact evaluation:

```json
{"name": "evaluate", "arguments": {"expr":
  "(\\frac{3}{500} \\cdot 10)^2 + (\\frac{3}{1000} \\cdot 10)^2 + (\\frac{-3}{25} \\cdot \\frac{1}{2})^2 + (\\frac{-3}{25} \\cdot 1)^2"}}
```

```
9/400        [exact]
```

So $u_c(\sigma) = \sqrt{9/400} = 3/20$ MPa **exactly** — a relative
combined standard uncertainty of exactly $1/40 = 2.5\%$, matching the
hand GUM calculation as the same rational number. With coverage factor
$k = 2$:

$$\sigma = (6.00 \pm 0.30)\ \text{MPa} \quad (k = 2)$$

(A status note worth reading: `simplify(\sqrt{9/400})` currently returns
$3/20$ at tier `verified` — checked at 12 points — rather than `exact`,
because the simplifier's radical rewrite is classified as a numeric
self-check. The answer is right and provable by squaring; the tool
refuses to *claim* more than its mechanism established. Statuses are
earned, never assumed — that refusal is the contract working.)

## Step 6 — the ranked budget

Variance contributions $(c_i u_i)^2$, exact and ranked:

| Rank | Input | $(c_i u_i)^2$ | Share of $u_c^2$ |
|------|-------|--------------|------------------|
| 1 | Height $h$ | $9/625$ | **64%** |
| 2 | Load $F$ | $9/2500$ | 16% |
| 2 | Width $b$ | $9/2500$ | 16% |
| 4 | Span $L$ | $9/10000$ | 4% |

The engineering conclusion the budget exists to deliver: although $h$ has
the *same relative tolerance* as $F$ and $b$ (1%), it contributes **64%**
of the output variance — four times any other input — because its squared
exponent doubles its sensitivity. To tighten the stress uncertainty,
tighten the height tolerance first; improving the load cell is nearly
pointless.

## Why this is different from a spreadsheet

Every number above carries evidence:

- The **model composition** is proved, not assumed (`verify_chain`,
  `exact`).
- Every **derivative** is independently re-derived and compared, with the
  comparison mechanism named in the response.
- Every **arithmetic step** is exact over ℚ — the combined uncertainty is
  $3/20$, not $0.15000000000000002$.
- Anything the engine could *not* establish exactly says so, in a
  machine-readable status — an agent consuming these responses can
  distinguish "proved" from "checked at 12 points" without parsing prose.

A wrong sensitivity coefficient in a real uncertainty budget survives
review easily — it is one partial derivative among dozens, and nobody
re-derives them all. Here, re-deriving them all is one tool call each.

## Reproducing this example

Every call above is a single JSON-RPC line to the `arithma-mcp` binary on
stdin. For instance:

```sh
printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"differentiate","arguments":{"expr":"\\frac{3 F L}{2 b h^2}","variable":"h"}}}' | arithma-mcp
```

All outputs shown were generated by the binary built from this tree.
