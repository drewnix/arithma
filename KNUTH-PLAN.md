# Arithma — Algorithmic Foundations Plan

*Author: Knuth (QAI Head of Algorithmic Foundations)*
*Date: 2026-05-12*

---

## Vision

A CAS that is correct before it is fast, fast before it is featureful, and featureful only in ways that the architecture supports cleanly. Rust + WASM means it runs anywhere with no runtime dependencies — that is the differentiator. The mathematics must be exact, the algorithms must be well-chosen, and the code must be readable.

## Current State (Post Session 08 Cleanup)

The foundation is sound: LaTeX tokenizer → parser → AST → manipulation → evaluation → WASM. Differentiation, basic integration, matrix operations, composition, substitution, and simplification all work. The derivative engine is now clean — nine hardcoded special cases removed, general algorithms verified correct. A `differentiate_and_evaluate` function bypasses the lossy Display round-trip.

### Known Issues
- `Display` implementation doesn't produce correctly parenthesized output (affects `differentiate_latex` string round-trip)
- `Node` type contains `ClosingParen`/`ClosingBrace` — token artifacts in the AST
- All numeric computation uses `f64` — no exact arithmetic
- Simplifier only handles linear combinations of single variables
- 22 matrix tests ignored (determinant, eigenvalues, inverse, rank, multiplication, RREF, linear systems)

## Phase 1: Exact Arithmetic Foundation

**Goal:** Replace `f64` as the primary numeric representation with exact rationals.

### 1.1 — BigRational integration
- Add `num` crate (`num-bigint`, `num-rational`) as dependencies
- Introduce `ExactNum` enum: `Rational(BigRational)`, `Float(f64)`, `Integer(BigInt)`
- `Rational` is the default for all symbolic operations
- `Float` is used only at the evaluation boundary or when the user explicitly requests decimal
- `Integer` is an optimization for the common case (avoids carrying denominator 1)

### 1.2 — Node numeric restructuring
- Replace `Node::Number(f64)` with `Node::Num(ExactNum)`
- Replace `Node::Rational(i64, i64)` — subsumed by `ExactNum::Rational`
- Remove `Node::ClosingParen` and `Node::ClosingBrace` — these belong in the tokenizer
- Update all pattern matches across the codebase

### 1.3 — Evaluator split
- `evaluate_exact(node, env) -> ExactNum` — stays in the exact domain
- `evaluate_f64(node, env) -> f64` — converts to float at the end
- The WASM API calls `evaluate_f64`; internal symbolic operations use `evaluate_exact`

**Estimated effort:** 1-2 sessions. Tests: all existing tests should pass with `f64` evaluation unchanged; new tests for exact arithmetic.

## Phase 2: Display and LaTeX Round-Trip

**Goal:** `Display` output is valid, parseable LaTeX that preserves expression structure.

### 2.1 — Parenthesization
- Add operator precedence to each `Node` variant
- `Display` inserts parentheses when a child has lower precedence than its parent
- `Divide` renders as `\frac{...}{...}` (already correct for some cases)
- `Power` renders base in parens if it's a compound expression

### 2.2 — Round-trip tests
- For every expression in the test suite: parse → format → re-parse → format, assert the two format outputs are identical
- This is the literate-programming version of "the program should be readable": the output should be parseable

**Estimated effort:** 1 session.

## Phase 3: Polynomial Representation

**Goal:** A proper `Polynomial` type for efficient polynomial arithmetic.

### 3.1 — Dense univariate polynomials
- `UnivariatePolynomial<R>` — coefficients in a ring `R` (initially `BigRational`)
- Operations: add, subtract, multiply, divide-with-remainder, GCD
- GCD via the subresultant algorithm (not Euclidean — coefficient growth matters)
- Conversion: `Node` ↔ `UnivariatePolynomial` for polynomial expressions

### 3.2 — Multivariate polynomials
- Recursive representation: a polynomial in x whose coefficients are polynomials in y, z, ...
- Term ordering: lexicographic (simplest, sufficient for most purposes)
- GCD via the sparse modular algorithm if performance requires it

### 3.3 — Polynomial factoring
- Square-free factorization (necessary for integration)
- Factoring over Q via Hensel lifting (Berlekamp-Zassenhaus)
- This is algorithmically deep but essential for a serious CAS

**Estimated effort:** 2-3 sessions for 3.1 + 3.2. Factoring (3.3) is a separate project.

## Phase 4: Simplification Engine

**Goal:** A principled simplification engine that produces canonical forms.

### 4.1 — Canonical form for rational expressions
- Normalize: cancel common polynomial factors in numerator/denominator
- Canonical sign: leading coefficient positive
- Collect like terms with proper polynomial representation

### 4.2 — Algebraic simplification rules
- Trigonometric identities (sin²+cos²=1, double angle, etc.)
- Logarithmic properties (ln(ab) = ln(a)+ln(b), etc.)
- Power rules (proper handling of x^a * x^b = x^(a+b))
- Rule application strategy: bottom-up, repeated until fixed point

### 4.3 — The simplification contract
- `simplify(expr) -> expr` is idempotent: `simplify(simplify(e)) == simplify(e)`
- Two expressions are mathematically equal if and only if their simplified forms are identical (for the supported expression domain)
- This is the hardest invariant to maintain and the most important

**Estimated effort:** 2-3 sessions. This is the intellectually hardest part.

## Phase 5: Calculus Improvements

### 5.1 — Integration by parts
- Pattern matching for ∫u·dv
- Tabular integration for polynomial × exponential/trig

### 5.2 — Partial fraction decomposition
- Requires polynomial factoring (Phase 3.3)
- Essential for rational function integration

### 5.3 — Series expansion
- Taylor/Maclaurin via repeated differentiation
- Formal power series arithmetic

### 5.4 — The Risch algorithm (long-term)
- Decides whether an elementary antiderivative exists
- Full implementation is a significant project
- Start with the transcendental case (Risch-Norman)

## Phase 6: Modular Crate Architecture

**Goal:** Each mathematical domain is an independent crate.

```
arithma/
├── arithma-core/        # Node, ExactNum, simplification engine
├── arithma-parse/       # Tokenizer, parser, LaTeX rendering
├── arithma-poly/        # Polynomial arithmetic, GCD, factoring
├── arithma-calculus/    # Differentiation, integration, series
├── arithma-linalg/     # Matrix operations over exact fields
├── arithma-numtheory/  # Primes, modular arithmetic, GCD
├── arithma-wasm/       # WASM bindings
└── arithma-cli/        # CLI interface
```

Each crate depends only on `arithma-core`. Each can be compiled to WASM independently. A distributed deployment loads only the crates needed for the task.

**This restructuring happens when the mathematical foundation (Phases 1-4) is solid, not before.**

## Principles

1. **Correct first.** Every algorithm is verified against known results. Tests use exact arithmetic, not floating-point approximation, wherever possible.
2. **Well-chosen algorithms.** Not the first algorithm that works — the right algorithm for the data structure. Subresultant GCD, not Euclidean. Horner evaluation, not naive. The choice matters at scale.
3. **Readable code.** Programs are literature. Every non-trivial algorithm has a comment citing the source (TAOCP section, paper, or textbook). A reader should be able to verify the implementation against the reference.
4. **No hardcoded answers.** The disease was diagnosed and excised in Session 08. It does not return.

---

*The mathematics does not care about our schedule, but I care about both.*
