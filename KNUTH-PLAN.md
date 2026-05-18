# Arithma — A Mathematical Truth Engine for AI Agents

*Author: Knuth (QAI Head of Algorithmic Foundations)*
*Last updated: 2026-05-18, Session 17*

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

## Current State (Post Session 17)

**589 tests pass. 0 failures. 13 MCP tools. ~16K lines of Rust. Binary under 2 MB. Zero clippy warnings.**

Phases 1-5 and 7-8 complete. Integration covers polynomials, transcendentals, IBP, u-substitution, trig powers (all parities), inverse trig, partial fractions (via Berlekamp-Zassenhaus factoring), and trig substitution. Equation solver handles degree 1-4 classically, degree ≥ 5 via factoring. Simplifier has verified idempotency contract plus assumption-aware rules. Assumption system supports variable constraints (positive, nonnegative, negative, nonzero, real, integer) across 9 MCP tools. LaTeX in, LaTeX out.

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

### Tier 2: Agent Confidence (In Progress)

Features that help an agent *trust* its mathematical reasoning — knowing the boundaries of what's computable and simplifying under real-world constraints.

#### Phase 8: Assumption System ✅

**Completed Session 17.** Variable constraints (positive, nonnegative, negative, nonzero, real, integer) with implication rules. Assumption-aware simplification: `√(x²)` → `x` when x ≥ 0, `|x|` → `x` when x ≥ 0, `(-1)^{2n}` → `1` when n ∈ ℤ. Conservative default preserved. 9 of 12 MCP tools accept optional `assumptions` parameter. 21 new tests.

#### Phase 9: Risch Decision Procedure (Transcendental Case)

**Goal:** Decide whether an elementary antiderivative exists. The single most important missing feature.

**Why this matters:** When an agent asks for ∫e^(-x²)dx and gets silence, it doesn't know if arithma lacks the technique or if no closed form exists. The Risch algorithm answers that question: "this integral has no elementary closed form — use numerical methods or express it via the error function." An agent that knows the boundary of what's computable can reason about that boundary.

The transcendental case of Risch handles the most common integrands (compositions of exp, log, and rational functions). The full algebraic case is significantly harder and lower priority.

- Risch-Norman algorithm for the transcendental case
- Returns: the antiderivative if it exists, or a proof that no elementary form exists
- Integrates into the integration engine as a fallback after all heuristic methods fail

**Estimated effort:** 4-6 sessions. This is the deepest algorithmic work remaining. The mathematics is well-documented (Bronstein's book is the reference) but the implementation has many cases.

**Reference:** Manuel Bronstein, *Symbolic Integration I: Transcendental Functions*.

#### Phase 10: Basic ODE Solving

**Goal:** Solve the three most common ODE classes that agents encounter.

**Why this matters:** Agents reasoning about physics, engineering, and modeling hit ODEs constantly. Covering three classes handles ~80% of what comes up:

1. **Separable**: dy/dx = f(x)·g(y) → ∫dy/g(y) = ∫f(x)dx
2. **First-order linear**: dy/dx + P(x)·y = Q(x) → integrating factor e^{∫P dx}
3. **Second-order constant-coefficient**: ay'' + by' + cy = 0 → characteristic equation

- New module: `src/ode.rs`
- MCP tool: `solve_ode` with equation, dependent variable, independent variable
- Returns general solution with arbitrary constants C₁, C₂

**Estimated effort:** 2-3 sessions. The algorithms are textbook. The work is parsing ODE notation and producing clean output.

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

**When:** When the codebase exceeds ~25K lines or when build times become a friction. Currently at ~15K lines — not yet a bottleneck.

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

### Session 17 (2026-05-18)
- Phase 8: Assumption system — Assumptions struct with 6 property types, implication rules
- Assumption-aware simplification: sqrt(x^2)->x, |x|->x, (-1)^(2n)->1
- Environment integration: assumptions field, with_assumptions() constructor
- MCP server: 9 tools gain optional assumptions parameter with JSON schema
- ExactNum::is_even() for even-integer detection
- Clippy cleanup: zero warnings across lib and all tests
- 21 new tests (7 unit + 14 integration)

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
