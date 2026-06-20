# Arithma — A Mathematical Truth Engine for AI Agents

*Author: Knuth (QAI Head of Algorithmic Foundations)*
*Last updated: 2026-05-31, Session 31*

---

## Vision

Arithma is a mathematical verification and computation engine for AI agents. It is correct before it is fast, fast before it is featureful, and featureful only where it helps an agent reason with confidence.

An agent that gets a wrong simplification will propagate that error through its entire reasoning chain. An agent that gets "I cannot compute this" can say so honestly and try a different approach. **Correctness beats coverage, every time.**

The design target is not "everything Mathematica does" but "everything an agent needs to reason mathematically without lying." Rust + WASM means it runs anywhere with no runtime dependencies. The binary stays under 5 MB. The mathematics is exact. Every result is deterministic.

### What We Build

- **Verification**: Is this mathematical claim correct? Simplify both sides, check equivalence.
- **Computation**: What is the answer? Differentiate, integrate, solve, factor, evaluate.
- **Boundaries**: Can this be computed at all? Know when a closed form doesn't exist.

### What We Don't Build

- A programming language (agents already have one)
- Visualization (different tool)
- Physics/statistics/geometry modules (application layers, not foundations)
- Hundreds of special functions (agents can look those up)

---

## Current State (Post Session 34)

**1004 tests pass. 0 failures. 14 MCP tools. ~26K lines of Rust. Binary under 2 MB. Zero clippy warnings.**

Phases 1-5 and 7-8, 10 complete. Phase 9 (Risch) now handles **multi-extension towers** in both tower orderings: the unified tower builder handles logarithmic extensions, exponential extensions, **two-level exp-over-log towers** (polynomial and rational), AND **two-level log-over-exp towers** (polynomial integrands in ln(h(x, exp(g)))). Integration covers polynomials, transcendentals, IBP, u-substitution, trig powers (all parities), inverse trig, partial fractions (via Berlekamp-Zassenhaus factoring), trig substitution, **Risch polynomial-in-exp integration** (independent Risch DE per degree, polynomial AND rational coefficients), **Risch polynomial-in-log integration** (top-down coefficient solving with rational coefficients and ln(x) absorption), **Rothstein-Trager for logarithmic rational integration**, **Rothstein-Trager for exponential rational integration** (with residual computation), **two-level exp-over-log polynomial tower integration** (inner Risch DE solver over Q(x)[ln(x)]), **two-level exp-over-log rational tower integration** (Hermite reduction via per-θ₁-degree linearity, Rothstein-Trager with θ₁-structured resultant, general GCD via θ₁-component decomposition), **two-level log-over-exp polynomial tower integration** (top-down logarithmic descent with structured inner exp integration), **θ₁-in-denominator detection** via content extraction (separable case: D₁(θ₁)·D₂(θ₂) factorization), and **two-level log-over-exp rational tower integration** (h-scaled Rothstein-Trager for non-elementarity detection) — all with non-elementary detection.

**Multi-extension towers (Sessions 24-25):** Two tower orderings supported. **Exp-over-log:** Q(x) ⊂ Q(x, ln(x)) ⊂ Q(x, ln(x), exp(g(x))). Polynomial integrands: inner Risch DE solver via triangular decomposition. Rational integrands: per-θ₁-degree Hermite reduction, two-level Rothstein-Trager, general denominator GCD via θ₁-component decomposition. **Log-over-exp:** Q(x) ⊂ Q(x, exp(g(x))) ⊂ Q(x, exp(g(x)), ln(h(x, exp(g(x))))). Polynomial integrands: top-down logarithmic coefficient solving with structured inner exp integration. Rational correction terms dispatched to inner Rothstein-Trager. Both orderings prove non-elementarity when antiderivatives don't exist in elementary terms.

**Key results:** ∫ln(1+exp(x)) dx → non-elementary ✓ (log-over-exp tower, degree-0 RT fails). ∫exp(x)·ln(1+exp(x)) dx = exp(x)·ln(1+exp(x)) + ln(1+exp(x)) − exp(x) ✓ (log-over-exp elementary). ∫ln(x)/(1+exp(x)) dx → non-elementary ✓ (exp-over-log, RT resultant has θ₁ term). ∫ln(x)/(1+exp(2x)) dx → non-elementary ✓ (degree-2 denominator). ∫exp(x)·ln(x)/(1+exp(x)) dx → non-elementary ✓. ∫exp(x)·ln(x) dx → non-elementary ✓ (reduces to Ei). ∫(exp(x)·ln(x) + exp(x)/x) dx = exp(x)·ln(x) ✓. ∫(exp(x)·ln(x)² + 2·exp(x)·ln(x)/x) dx = exp(x)·ln(x)² ✓. ∫exp(x²)·ln(x) dx → non-elementary ✓. ∫1/(ln(x)·(1+exp(x))) dx → non-elementary ✓ (content extraction, scaled RT). ∫exp(x)/(ln(x)·(1+exp(x))) dx → non-elementary ✓. ∫1/(ln(x)²·(1+exp(x))) dx → non-elementary ✓ (content = θ₁²). ∫1/ln(1+exp(x)) dx → non-elementary ✓ (log-over-exp rational, h-scaled RT). ∫exp(x)/ln(1+exp(x)) dx → non-elementary ✓. Plus all previous: ∫exp(x)/(1+exp(x))dx = ln(1+exp(x)) ✓. ∫1/(1+exp(x))dx = x − ln(1+exp(x)) ✓. ∫((1-x)/x²)·exp(x)dx = −exp(x)/x ✓. ∫exp(x)/x dx → non-elementary ✓. ∫ln(x)/x² dx = −(ln(x)+1)/x ✓. ∫(1/x+ln(x))dx = (x+1)ln(x)−x ✓. ∫ln(x)/(x+1) dx → non-elementary ✓.

Equation solver handles degree 1-4 classically, degree ≥ 5 via factoring. **Exact radical roots (Session 31):** `solve(x²-2=0)` → `±√2`, not `±1.414...` — irrational quadratic roots returned as symbolic Node expressions via `solve_for_variable_nodes`. **Rational equation solving** via automatic denominator clearing: `1/x = 2` → `x = 1/2`. Matrix eigenvalues computed via characteristic polynomial + solver for matrices up to 4×4, with algebraic multiplicity; **decimal matrix entries** now supported via float-to-rational conversion with numerical cubic fallback. Greek letter LaTeX parsing (`\alpha` → `α` internally, `\alpha` on output) with `normalize_var()` at all API boundaries. **Parser hardening (Sessions 29, 31):** implicit multiplication for variable-paren patterns (`u(3-2u)`, `α(x+1)`), space-separated variable multiplication, sign normalization in fractions (`-3/(-2b-1)` → `3/(2b+1)`), LaTeX operator reservation (`\int`, `\prod`, `\oint` not tokenized as variables), LaTeX spacing commands stripped (`\,`, `\;`, `\quad`). **Radical simplification (Session 31):** numeric square-factor extraction (`√12 → 2√3`, `√72 → 6√2`), mixed radicand factoring (`√(4a²) → 2|a|`, `√(9x²) → 3|x|`, assumption-aware: `√(4a²) → 2a` when `a≥0`), like-radical combination (`√8+√2 → 3√2`). Simplifier performs **rational content GCD cancellation** on fractions: `(-32α+32)/(16α+8)` → `(-4α+4)/(2α+1)`. **Common-denominator combination:** `1/x + 1/(x+1)` → `(2x+1)/(x(x+1))`. **Factored display** for denominators (and numerators) with repeated or multiple factors: `48/(16α³+24α²+12α+2)` → `24/(2α+1)³`. Both univariate and multivariate fractions supported. **Parametric integration** for linear and **quadratic** denominators: `∫1/(x+a)dx = ln|x+a|`, `∫1/(x²+a)dx = (1/√a)·arctan(x/√a)`, `∫(px+q)/(ax²+bx+c)dx` via completing-the-square formula (Session 31). **Symbolic-center Taylor expansion:** `taylor_series_symbolic` expands `f(x)` around `x = a` where `a` is a symbolic expression, producing coefficients as exact symbolic expressions. MCP and CLI accept symbolic centers alongside numeric ones (e.g., `taylor "3/(1+2x)" x a 3` → `3/(2a+1) - 6/(2a+1)² · (x-a) + 12/(2a+1)³ · (x-a)²`). Simplifier has verified idempotency contract plus assumption-aware rules. **Fraction coefficient cancellation** in Divide handler: `(k·expr)/m → (k/m)·expr`, `k/(m·expr) → (k/m)/expr`, and **general case** `(k·expr1)/(m·expr2)` cancels integer GCD (Session 32). **Negation extraction** in Multiply: `f·(-g) → -(f·g)`. **Leibniz notation detection (Session 32):** `\frac{d}{dx}(...)` errors helpfully instead of parsing `d` as a variable. **Complex root reporting (Session 32):** `solve_full()` API returns `SolveResult` with solution count and omitted-complex-root count; CLI and MCP report "(N complex roots omitted)" when applicable. Assumption system supports variable constraints across 9 MCP tools. **Symbolic π (Session 34):** `\pi` is now `Variable("π")` instead of `Float(3.14...)`, enabling exact results throughout. Special-value evaluation: sin/cos/tan at rational multiples of π, arctan(0/±1), arcsin(0/±1), ln(1). **Exact definite integration (Session 34):** `definite_integral_exact` substitutes bounds symbolically via FTC — ∫₀¹ 1/(x²+1)dx = π/4, ∫₁ᵉ 1/x dx = 1. MCP bounds accept LaTeX strings. CLI: `arithma integrate <expr> [var] [lo hi]`. **Parametric equation solving (Session 34):** `try_solve_parametric` uses differentiation to extract symbolic coefficients — solve(ax²+bx+c=0, x) returns the quadratic formula, solve(ax+b=0, x) returns -b/a. **Like function term collection (Session 34):** 3·exp(x)+5·exp(x) → 8·exp(x), a·sin(x)+b·sin(x) → (a+b)·sin(x). **Partial derivative notation (Session 34):** `\frac{\partial}{\partial x}` detected and errors helpfully. LaTeX in, LaTeX out.

---

## Roadmap

### Tier 1: Foundation (Complete)

The exact arithmetic, polynomial algebra, simplification, differentiation, and basic integration that everything else builds on.

- **Phase 1: Exact arithmetic** ✅ — BigRational, ExactNum, evaluator split
- **Phase 2: LaTeX round-trip** ✅ — parse ↔ display stability
- **Phase 3: Polynomial algebra** ✅ — dense univariate, multivariate, Berlekamp-Zassenhaus factoring over Q, partial fractions
- **Phase 4: Simplification engine** ✅ — polynomial normalization, trig/log/power rules, idempotency contract (62 tests)
- **Phase 5: Core calculus** ✅ — differentiation (full chain rule), integration (8 techniques), series, limits
- **Phase 7: MCP server** ✅ — 13 tools, LaTeX I/O, hand-rolled JSON-RPC, < 2 MB
- **Phase 8: Assumption system** ✅ — 6 property types, 9 tools with assumptions, 21 tests
- **Phase 10: Basic ODE solving** ✅ — separable, linear, constant-coeff; 19 tests

### Tier 2: Agent Confidence (In Progress)

Features that help an agent *trust* its mathematical reasoning — knowing the boundaries of what's computable and simplifying under real-world constraints.

#### Phase 8: Assumption System ✅

**Completed Session 17.** Variable constraints (positive, nonnegative, negative, nonzero, real, integer) with implication rules. Assumption-aware simplification: `√(x²)` → `x` when x ≥ 0, `|x|` → `x` when x ≥ 0, `(-1)^{2n}` → `1` when n ∈ ℤ. Conservative default preserved. 9 of 12 MCP tools accept optional `assumptions` parameter. 21 new tests.

#### Phase 9: Risch Decision Procedure (Transcendental Case) — In Progress

**Goal:** Decide whether an elementary antiderivative exists. The single most important feature.

**Sessions 1-2 completed (Session 18).** The exponential case is operational:

- **Foundation types:** `RationalFunction` (p(x)/q(x) with full arithmetic), `ExtPoly` (polynomial in tower variable θ with Q(x) coefficients), `DifferentialExtension` (log/exp tower with derivative computation). 64 tests.
- **Hermite reduction:** Splits ∫A/D into rational part + squarefree-denominator integral. Handles single-factor, multi-factor, and polynomial-division cases. Verified by formal differentiation identity.
- **Risch DE solver:** Solves q' + f·q = g over Q[x] or proves no polynomial solution exists. Degree bound + top-down coefficient matching. 11 tests.
- **Exponential integration:** For ∫r(x)·exp(g(x))dx, reduces to Risch DE via Liouville's theorem. Returns elementary antiderivative or proof of non-elementarity.
- **Engine wiring:** Risch inserted as last-resort fallback. Non-elementary results surfaced cleanly via MCP (as success) and CLI (exit code 0).

**Key results:** ∫e^{-x²}dx → "non-elementary" ✓. ∫e^{x³}dx → "non-elementary" ✓. ∫x²·e^{-x²}dx → "non-elementary" ✓. ∫2x·e^{x²}dx = e^{x²} ✓.

**Session 3 completed (Session 19/20).** Logarithmic polynomial integration operational. Top-down coefficient solving for ∫(a₀ + a₁·ln(x) + ... + aₙ·ln(x)ⁿ)dx. 3 tests.

**Session 4 completed (Session 21).** Rothstein-Trager resultant method for logarithmic rational integration:

- **Sylvester matrix determinant:** Cofactor expansion for matrices of ExtPolys, computing R(z) = res_θ(d, a − z·D(d)) symbolically with z as parameter.
- **Constant root finder:** Specializes R(z) ∈ Q(x)[z] at x = x₀, finds Q-roots via rational root theorem, verifies against full R(z).
- **Pattern detector:** Converts Node AST to (numerator, denominator) ExtPoly pairs for rational functions of ln(x).
- **Full pipeline:** Pattern detect → Hermite reduce → Rothstein-Trager → result assembly.
- **Node conversion:** ExtPoly and RationalFunction back to Node AST for result output.

**Key results:** ∫1/(x·ln(x))dx = ln(ln(x)) ✓. ∫1/(x·(ln(x)−1))dx = ln(ln(x)−1) ✓. ∫1/ln(x)dx → "non-elementary" ✓. ∫1/(1+ln(x))dx → "non-elementary" ✓. All verified numerically.

**Session 6 completed (Session 22).** Rational-coefficient Risch DE and base-field rational integration:

- **Generalized DE solver:** `solve_risch_de(s, F, G)` solves s·p' + F·p = G for polynomial p. Three-way degree bound (deg(F) ≷ deg(s)). Backward-compatible wrapper for s=1 case.
- **Rational wrapper:** `solve_risch_de_rational(f, g)` for q' + f·q = g where g ∈ K(x). Squarefree rejection (simple poles → no rational solution), denominator bound computation, polynomial ODE transformation.
- **Base-field integration:** `integrate_rational_base` decomposes via partial fractions, separates rational part from ln(x) coefficient. Rejects ln(x+a) for a≠0.
- **Pipeline upgrades:** Both `integrate_poly_exp` and `integrate_poly_log` now accept rational function coefficients. Log extension uses Δ accumulator for ln(x) absorption across degrees.
- **Tower dispatch fix:** `try_risch_tower` now folds x-polynomial denominators into ExtPoly coefficients instead of discarding them.
- **Parser precedence fix:** `-x^2` → `-(x^2)`. Removed `fixup_negated_power` workaround.

**Key results:** ∫((1-x)/x²)·exp(x)dx = −exp(x)/x ✓. ∫exp(x)/x dx → non-elementary ✓. ∫ln(x)/x² dx = −(ln(x)+1)/x ✓. ∫(1/x+ln(x))dx = (x+1)ln(x)−x ✓. ∫ln(x)/(x+1) dx → non-elementary ✓.

**Session 7 completed (Session 24).** Multi-extension towers (exp + log in same integrand):

- **Two-level tower:** Q(x) ⊂ Q(x, θ₁=ln(x)) ⊂ Q(x, θ₁, θ₂=exp(g(x))). Exp on top, log on bottom.
- **Node parser:** `node_to_two_level` converts mixed exp+ln ASTs into `Vec<ExtPoly>` (polynomial in θ₂, coefficients are ExtPolys in θ₁).
- **Inner DE solver:** `solve_risch_de_in_log_ext` solves q' + f·q = g where g ∈ Q(x)[θ₁], via top-down triangular decomposition into standard Risch DEs.
- **Two-level integrator:** `integrate_two_level_exp_log` drives the outer loop. Degree 0 via `integrate_poly_log`, degree i≥1 via inner DE solver.
- **Wiring:** `try_risch_tower` falls through to `try_risch_two_level` when single-extension tower detects both extensions.

**Key results:** ∫exp(x)·ln(x) dx → non-elementary ✓. ∫(exp(x)·ln(x) + exp(x)/x) dx = exp(x)·ln(x) ✓. ∫(exp(x)·ln(x)² + 2·exp(x)·ln(x)/x) dx = exp(x)·ln(x)² ✓. ∫exp(x²)·ln(x) dx → non-elementary ✓.

**Session 24b: Rational-in-θ₂ with θ₁ coefficients:**

- **Rational parsing:** `extract_two_level_rational` for Divide/Multiply nodes with θ₂ denominator.
- **Per-θ₁-degree Hermite reduction:** exploits linearity — runs existing single-level Hermite on each θ₁-degree independently.
- **Two-level Rothstein-Trager:** Sylvester matrix with θ₁-structured entries, `two_level_det` via cofactor expansion, `find_constant_roots_two_level` with ExtPoly verification.
- **Integration pipeline:** `integrate_rational_two_level` — poly division → Hermite → RT → residual. Degree-1 denominator GCD via polynomial evaluation.

**Key results:** ∫ln(x)/(1+exp(x)) dx → non-elementary ✓. ∫exp(x)·ln(x)/(1+exp(x)) dx → non-elementary ✓.

**Remaining:**
- Algebraic extensions (integration over Q(α) for algebraic α — 2-3 sessions)

**Reference:** Manuel Bronstein, *Symbolic Integration I: Transcendental Functions*.

#### Phase 10: Basic ODE Solving ✅

**Completed Session 17.** Three ODE classes: separable (auto-detects g(x)*h(y) factorization), first-order linear (integrating factor method), second-order constant-coefficient (discriminant-based: distinct real, repeated, complex roots). MCP tool `solve_ode` and CLI command `ode` with `--cc` flag. 19 tests. Returns general solutions with C₁, C₂.

### Tier 3: Computational Power (When Needed)

Features that extend what arithma can compute. Each earns its place by demonstrable agent utility.

#### Phase 11: Formal Power Series

**Goal:** Multiply, compose, and invert power series as algebraic objects. Lazy evaluation — compute coefficients on demand.

**Why this matters:** Generating functions are a powerful reasoning tool. An agent that can manipulate formal power series can solve recurrences, count combinatorial structures, and verify identities that would be impractical to check term-by-term.

- `FormalPowerSeries` type with lazy coefficient computation
- Operations: add, multiply, compose, inverse, derivative, integral
- MCP tool: `power_series` with operations parameter
- Connects to existing Taylor series (a power series is the algebraic object; a Taylor expansion is a specific instance)

**Estimated effort:** 2 sessions.

#### Phase 12: Systems of Equations

**Goal:** Solve systems of polynomial equations and linear systems with symbolic coefficients.

- Linear systems: extend matrix solve to handle symbolic entries (we have the matrix infrastructure)
- Nonlinear substitution: for 2-3 equation systems, solve one for a variable and substitute into others
- Gröbner bases: only if agent use cases demand it — powerful but rarely needed outside research

**Estimated effort:** 2-3 sessions for linear + substitution. Gröbner bases would be 3-4 sessions additional and is not currently planned.

#### Phase 13: Integration Completions

Filling remaining gaps in the integration engine as they arise from agent usage.

- ~~Biquadratic integration: 1/(ax⁴+bx²+c) via quadratic-in-x² factoring~~ ✅ Done (Session 27)
- ~~Parametric quadratic integration: ∫(px+q)/(ax²+bx+c)dx with symbolic a,b,c~~ ✅ Done (Session 31)
- Higher-power irreducible quadratic: (Ax+B)/(x²+bx+c)^k for k ≥ 2 (reduction formula)
- Hyperbolic substitution variants
- Integration of expressions with absolute values
- Better heuristic ordering: try cheap methods before expensive ones

**Not planned:** RUBI-style rule tables (6700+ patterns). The algorithmic approach we have is the right foundation. Rule tables are a maintenance burden that doesn't match our design philosophy.

**Estimated effort:** Ongoing, 1-2 sessions as gaps are identified.

### Tier 4: Architecture (When Scale Demands)

#### Phase 6: Modular Crate Architecture

**Goal:** Split into independent crates when the single-crate approach becomes a development bottleneck.

```
arithma/
├── arithma-core/        # Node, ExactNum, simplification engine
├── arithma-parse/       # Tokenizer, parser, LaTeX rendering
├── arithma-poly/        # Polynomial arithmetic, GCD, factoring
├── arithma-calculus/    # Differentiation, integration, series, ODE
├── arithma-linalg/     # Matrix operations over exact fields
├── arithma-wasm/       # WASM bindings
└── arithma-mcp/        # MCP server
```

Each crate depends only on `arithma-core`. Each can be compiled to WASM independently.

**When:** When the codebase exceeds ~25K lines or when build times become a friction. Currently at ~20K lines — approaching but not yet a bottleneck.

---

## What Done Looks Like

Arithma is done — or rather, at a natural resting point — when an AI agent with access to it can:

1. **Verify** any undergraduate-level mathematical claim (simplification, equivalence, identity)
2. **Compute** derivatives, integrals, solutions, factorizations, series, limits, and matrix operations with exact arithmetic
3. **Know the boundary** — distinguish "I can't compute this yet" from "this has no elementary closed form"
4. **Reason under constraints** — simplify with assumptions about variable domains
5. **Solve basic ODEs** — the three classes that cover 80% of applied mathematics
6. **Do all of this** from a single binary under 5 MB with zero dependencies, deterministic output, and sub-second response times

That's roughly 35-40% of Mathematica's CAS core coverage, with 100% correctness on everything we claim to compute, at 1/3000th the deployment footprint. The coverage is driven by what agents need, not by completeness for its own sake.

---

## Completed Work

### Session 32 (2026-05-31)
- Feature: General fraction coefficient cancellation — `(k·e1)/(m·e2)` cancels integer GCD
- Feature: Leibniz d/dx detection — `\frac{d}{dx}(...)` errors helpfully, tokenizer gains `errors` field
- Feature: Complex root reporting — `SolveResult` struct, `solve_full()` API, CLI/MCP report omitted complex roots
- 9 new tests (960→969), 3 commits

### Session 31 (2026-05-31)
- Feature: Parametric quadratic integration — `∫(px+q)/(ax²+bx+c)dx` with symbolic coefficients
- Feature: `try_decompose_quadratic` for Node-level (a,b,c) extraction from ax²+bx+c
- Numerically verified: `∫₀² 1/(x²+4)dx = π/8`
- Fix: Reserve LaTeX operators (`\int`, `\prod`, `\oint`) in tokenizer
- Fix: Strip LaTeX spacing (`\,`, `\;`, `\quad`) in tokenizer
- Feature: Radical square-factor extraction (`√12 → 2√3`, `√72 → 6√2`)
- Feature: Mixed radicand factoring (`√(4a²) → 2|a|`, `√(9x²) → 3|x|`, assumption-aware)
- Feature: Like-radical combination (`√8+√2 → 3√2`, `√2-√2 → 0`)
- Feature: Exact radical solver roots (`solve(x²-2=0) → ±√2`) via `solve_for_variable_nodes`
- Feature: Fraction coefficient cancellation (`(k·expr)/m`, `k/(m·expr)`)
- Fix: Negation extraction from products (`sin(x)·(-sin(x)) → -sin²(x)`)
- 32 new tests (928→960), 10 commits, driven by dogfooding report

### Session 29 (2026-05-28)
- Parser hardening: implicit multiplication for variable-paren (`u(3-2u)`, `α(x+1)`)
- Parser hardening: space-separated variable multiplication (`x y` → `x * y`)
- Simplifier: sign normalization in fractions (`-3/(-2b-1)` → `3/(2b+1)`)
- Feature: Rational equation solving via denominator clearing (`1/x = 2` → `x = 1/2`)
- Feature: Decimal matrix eigenvalues (float-to-rational + numerical cubic fallback)
- Feature: Parametric integration for linear denominators (`∫1/(x+a)dx = ln|x+a|`)
- 26 new tests (897→923), 4 commits, all from agent feedback (Carl + Ada)

### Session 28 (2026-05-28)
- Rational GCD simplification + factored display (877→889)
- Display normalization: nested negations (889→891)
- Symbolic-center Taylor expansion (891→896)
- Non-monic polynomial factoring fix (896→897)
- 20 new tests, 5 commits

### Session 27 (2026-05-28)
- Fix: Partial fraction content factor for non-monic linear denominators (Ada MSG-016 bug)
- Feature: Greek letter LaTeX parsing (\alpha, \beta, etc. → Unicode internally, \alpha on output)
- Feature: normalize_var() at MCP and CLI boundaries for Greek variable names
- Feature: Eigenvalue computation for 3×3 and 4×4 matrices via characteristic polynomial
- Feature: characteristic_polynomial() method on Matrix
- Feature: Biquadratic integration (1/(ax⁴+bx²+c) via quadratic-in-x² factoring, exact √d coefficients)
- Feature: Symbolic eigenvalues for 2×2 and 3×3 matrices with variable entries
- Simplifier: preserve symbolic sqrt for non-perfect squares
- 28 new tests (849→877), 6 features

### Session 26 (2026-05-25)
- Phase 9: θ₁-in-denominator via content extraction for two-level exp-over-log tower
- Phase 9: Log-over-exp rational integration via h-scaled Rothstein-Trager
- compute_theta1_content: iterative GCD of θ₂-coefficients
- rothstein_trager_two_level: content parameter for scaled z-coefficients
- rothstein_trager_two_level_general: accepts pre-computed Vec<ExtPoly> z-coefficients
- integrate_rational_two_level: content threading through Hermite/RT/GCD pipeline
- compute_log_ext_dd_scaled: h-scaled log-extension derivative
- extract_rational_log_over_exp: parse rational-in-ln(h) expressions
- integrate_rational_log_over_exp: Hermite + h-scaled RT pipeline
- try_risch_two_level: content-extraction + log-over-exp rational dispatch
- 24 new tests (825→849), 8 commits

### Session 24 (2026-05-21)
- Phase 9 Session 7: Multi-extension polynomial towers (exp + ln in same integrand)
- Phase 9 Session 7: node_to_two_level parser, solve_risch_de_in_log_ext, integrate_two_level_exp_log
- Phase 9 Session 7: try_risch_two_level + try_risch_tower wiring
- Phase 9 Session 8: Multi-extension rational towers (rational-in-exp with ln coefficients)
- Phase 9 Session 8: extract_two_level_rational, hermite_reduce_two_level (per-θ₁-degree linearity)
- Phase 9 Session 8: rothstein_trager_two_level, two_level_det, find_constant_roots_two_level
- Phase 9 Session 8: integrate_rational_two_level pipeline, div_rem_two_level_by_extpoly
- 33 new tests (772→805), 12 commits

### Session 22 (2026-05-19/20)
- Parser precedence fix: -x^2 → -(x^2), removed fixup_negated_power workaround
- Phase 9 Session 6: Generalized DE solver (s·p'+F·p=G), solve_risch_de_rational
- Phase 9 Session 6: integrate_rational_base (partial fractions → rational part + ln(x) coeff)
- Phase 9 Session 6: integrate_poly_exp upgraded for rational coefficients
- Phase 9 Session 6: integrate_poly_log upgraded with Δ accumulator for ln(x) absorption
- Phase 9 Session 6: Tower dispatch fix (fold x-polynomial denominators into coefficients)
- 30 new tests (747→772), 9 commits

### Session 21 (2026-05-19)
- Phase 9 Session 4: Rothstein-Trager resultant method (Sylvester matrix, root finder, RT pipeline)
- Phase 9 Session 5: Unified tower builder replacing three pattern detectors
- Phase 9 Session 5: Transcendental scanning + generalized node_to_extpoly (log + exp)
- Phase 9 Session 5: build_tower (scan → classify → convert to ExtPoly num/den)
- Phase 9 Session 5: integrate_poly_exp (independent Risch DE per degree)
- Phase 9 Session 5: integrate_poly_log (refactored standalone)
- Phase 9 Session 5: integrate_rational_ext (generalized Hermite + RT + exp residual)
- Phase 9 Session 5: try_risch_tower (unified dispatcher)
- Phase 9 Session 5: Removed 12 old functions, 23 old tests; net -1100 lines
- Phase 9 Session 5: New capability: rational-in-exp integration (∫exp(x)/(1+exp(x))dx, ∫1/(1+exp(x))dx)
- 742 total tests (30 old removed, 25 new added)

### Session 18 (2026-05-18)
- Phase 9 Session 1: RationalFunction type (p(x)/q(x), full arithmetic, derivative)
- Phase 9 Session 1: ExtPoly type (polynomial in θ with Q(x) coefficients, GCD, extended GCD, SFD)
- Phase 9 Session 1: DifferentialExtension (log/exp towers, derivative computation)
- Phase 9 Session 1: Hermite reduction (split integral into rational + squarefree parts)
- Phase 9 Session 2: Risch DE solver (q' + fq = g, degree bound, coefficient matching)
- Phase 9 Session 2: Exponential pattern detector (extract r(x)·exp(g(x)) from Node AST)
- Phase 9 Session 2: try_risch_exponential (integrate or prove non-elementary)
- Phase 9 Session 2: Integration engine wiring + MCP/CLI non-elementary reporting
- 95 new tests (64 foundation + 31 integration)

### Session 17 (2026-05-18)
- Phase 8: Assumption system — Assumptions struct with 6 property types, implication rules
- Assumption-aware simplification: sqrt(x^2)->x, |x|->x, (-1)^(2n)->1
- Environment integration: assumptions field, with_assumptions() constructor
- MCP server: 9 tools gain optional assumptions parameter with JSON schema
- ExactNum::is_even() for even-integer detection
- Subcommand CLI: 11 commands (simplify, diff, integrate, solve, factor, pf, eval, limit, taylor, sub, ode)
- Phase 10: ODE solver — separable, first-order linear, second-order constant-coefficient
- MCP tool: solve_ode (14th tool)
- Clippy cleanup: zero warnings across lib and all tests
- 40 new tests total (7 assumption unit + 14 assumption integration + 7 ODE unit + 12 ODE integration)

### Session 16 (2026-05-17)
- Both-even mixed trig product integration (Pythagorean expansion + binomial theorem)
- Berlekamp-Zassenhaus polynomial factoring over Q (4-layer pipeline: ModPoly, Q-matrix, Hensel, recombination)
- Partial fraction decomposition (recursive extended GCD splitting)
- Partial fraction integration (log + arctan terms for linear and quadratic denominators)
- MCP server: 13 tools (upgraded factor, new partial_fractions)
- Solver: factor_over_q for degree ≥ 5 polynomials
- Trig substitution: √(a²-x²), √(x²+a²), √(x²-a²)

### Session 15 (2026-05-17)
- Simplification idempotency contract (3 bugs fixed, 62 tests)
- U-substitution for integration
- Inverse trig antiderivatives
- Equivalence checking MCP tool

### Session 14 (2026-05-16)
- Multivariate GCD (primitive PRS)
- Taylor/Maclaurin series
- Symbolic limits (direct substitution, GCD cancellation, L'Hôpital)
- MCP server v1 (8 tools)

### Sessions 09-13
- Phase 1: Exact arithmetic (BigRational, ExactNum)
- Phase 2: LaTeX round-trip
- Phase 3.1-3.2: Polynomial types (univariate + multivariate)
- Equation solver: linear through quartic (Cardano, Ferrari)
- Simplification rules: trig, log, power, inverse functions, abs

### Sessions 08 and earlier
- Project inception and derivative engine
- Paper reviews and research collaboration
- Foundation work

---

## Principles

1. **Correct first.** Every algorithm is verified against known results. Tests use exact arithmetic, not floating-point approximation. An incorrect result is worse than no result.
2. **Well-chosen algorithms.** Not the first algorithm that works — the right algorithm for the data structure. Berlekamp-Zassenhaus, not trial division. Subresultant GCD, not naive Euclidean. The choice matters.
3. **Readable code.** Programs are literature. Every non-trivial algorithm cites its source. A reader should be able to verify the implementation against the reference.
4. **No hardcoded answers.** The disease was diagnosed and excised in Session 08. It does not return.
5. **Silence over lies.** If arithma cannot compute something, it says so. It never guesses, approximates heuristically, or returns an unverified result. An agent's trust in this tool is the product.

---

*The mathematics does not care about our schedule, but I care about both.*
