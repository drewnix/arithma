---
name: using-arithma
description: Use when doing any symbolic or numeric mathematics — simplifying, differentiating, integrating, solving equations, checking whether expressions are equal, or verifying a multi-step derivation — anywhere the arithma CLI or MCP server is available; also when interpreting arithma result statuses, bracketed markers, unexpected parse errors, or unevaluated echoes.
---

# Using Arithma

Arithma is an exact CAS built for agents: every answer carries the *kind of evidence* behind it. Core principle: **compute, don't recall — and let the tool check the tool.** Your own algebra chooses *what* to compute; it is never the evidence for a claim.

## The five rules

1. **Delegate every computation.** Any derivative, simplification, root, limit, or equality you were about to assert from memory → one tool call. You cannot do symbolic algebra reliably; the tool can.
2. **Cross-check with a second tool mechanism** — `equivalent`, substitute a point, differentiate the antiderivative. "I verified it by hand" is not verification.
3. **"Is this proven?" is a status question.** `equivalent`/`verify` answer it directly: `exact` = proven, `verified` = checked at n points. Read the field before assembling your own proof.
4. **Verify steps as you derive.** Check each new step as a two-step `verify_chain` call, then run the full chain at the end — per-step checking catches errors before they compound with chain length. On a bad chain, `first_failure` locates the bug; `weakest_step` names what deserves stronger treatment.
5. **A refusal or impossibility result is an answer.** Report `provably_impossible` as a theorem (certificate + `special_form` when present: ∫e^{−x²}dx = (√π/2)·erf(x), not elementary). Route around `unable_to_compute`; retrying verbatim or filling the gap with a guess loses the information the refusal carried.

## The status contract — what you may claim

| status | meaning | you may say |
|---|---|---|
| `exact` | decision procedure ran | proven — as a *rational-function identity* (removable points excepted; see Boundaries) |
| `verified` | agreed at n points | evidence at n points — **never** "proven"; if the reported symbolic `Difference` is nonzero while points agree, the two sides differ below f64 tolerance — treat as open and cite the difference |
| `heuristic` | believed sound, unchecked | needs an independent check before building on it |
| `unable_to_compute` | honest refusal, reason given | say so; try another route |
| `provably_impossible` | theorem, with certificate | report as the answer |

No amount of point-agreement upgrades to `exact`. Verdict-shaped tools also return `verdict` (pass/fail/inconclusive) — switch on fields, never parse prose. Full contract: `docs/result-status.md`.

## Surfaces

**MCP server** (preferred): 17 tools; `verify`, `equivalent`, `verify_chain` exist **only** here. **CLI**: `arithma <cmd> "latex" [var]` — same taxonomy as bracketed markers on loud results.

## Spellings — check here before diagnosing a "bug"

- Multi-letter names are *single variables*: write `3 \cdot a \cdot b`, since `3ab` is 3·(variable "ab") — and `ab` vs `ba` are different variables, not a commutativity failure.
- Use rationals: `\frac{1}{3}`, since decimals exit exact arithmetic and a result touching a float can never earn `exact`.
- Write large integers as powers: `10^{30}`, since long digit literals degrade to float.
- Write explicit `\cdot` between a brace group and a function: `\frac{\sqrt{\pi}}{2} \cdot \erf(x)`. Spell erf as `\erf` (`\operatorname` currently mis-parses); erf/Ei/li are symbolic-only (they parse, print, differentiate; numeric evaluation refuses).
- Write `\abs(x)` for absolute value; `e` and `π` are constants; `e^x` ≡ `\exp(x)`.
- An `evaluate` that echoes your input unchanged is a silent refusal — report no number.

## Boundaries (route around)

No complex numbers, no relational assumptions (x < y), improper integrals refused. In derivative cascades, substitute early — pin variables as soon as sound — since intermediate expressions grow geometrically and very large ones stall. A call that does not return promptly is stalled, not thinking: kill it, reorder the cascade, split the step, and report `unable_to_compute` with the reason. Equality is rational-function equality: `(x²−1)/(x−1) = x+1`; removable points don't count.
