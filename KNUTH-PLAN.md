# Arithma — Algorithmic Foundations Plan

*Author: Knuth (QAI Head of Algorithmic Foundations)*
*Date: 2026-05-12*

---

## Vision

A CAS that is correct before it is fast, fast before it is featureful, and featureful only in ways that the architecture supports cleanly. Rust + WASM means it runs anywhere with no runtime dependencies — that is the differentiator. The mathematics must be exact, the algorithms must be well-chosen, and the code must be readable.

## Current State (Post Session 10)

**Phases 1-2 complete. Simplifier deepened.** 194 tests pass, 5 ignored (unimplemented: limits, infimum; floating-point edge cases: cot, sec at singularities).

### Session 10 Changes
- **\frac parser bug fixed**: Tokenizer now consumes both brace groups of `\frac{A}{B}` and emits `(A) / (B)`. Eliminates the class of bugs where `\frac` as first operand misparsed.
- **Shunting-yard function-pop fix**: Functions are now popped after their closing delimiter (standard algorithm behavior). Previously `\sin(x) + 1` misparsed as `sin(x + 1)`. Same for `\sqrt`, `\cos`, etc.
- **Divide Display**: `Node::Divide` renders as `\frac{left}{right}` for proper LaTeX output and round-tripping.
- **Multiply Display**: Coefficient 1 omitted (`1x` → `x`, `-1x` → `-x`).
- **Simplifier deepened**: Add simplifies children first, falls back gracefully when `collect_terms` can't handle the expression. Sqrt and Function nodes now simplify when args are numeric.
- **Eigenvalue tests un-ignored**: Function simplification unlocked `sqrt(4) → 2` which the eigenvalue quadratic formula needs.
- **Linear system solve un-ignored**: Was producing correct results all along — the test expected x=2, y=1 for a system whose actual solution is x=8/5, y=9/5. Fourth wrong-expectation test found and fixed.

### Remaining Ignored Tests (5)
- `test_sec_function`: sec(π/2) — floating-point `cos(π/2)` ≠ 0.0 exactly
- `test_cot_function`: same class of floating-point singularity issue
- `test_sec_function_undefined`: same
- `test_inf_function`: infimum function not properly implemented
- `test_lim_function`: limits not implemented

## Phase 1: Exact Arithmetic Foundation

**Goal:** Replace `f64` as the primary numeric representation with exact rationals.

### 1.1 — BigRational integration ✅ (Session 09)
- Added `num-bigint`, `num-rational`, `num-traits`, `num-integer` as dependencies
- Introduced `ExactNum` enum: `Rational(BigRational)`, `Float(f64)`
- `Rational` is the default for all symbolic operations
- `Float` is used for transcendentals (`e`, `pi`) and results of transcendental functions
- Dropped the planned `Integer(BigInt)` variant — `BigRational` with denominator 1 handles integers efficiently enough; premature optimization avoided

### 1.2 — Node numeric restructuring ✅ (Session 09)
- Replaced `Node::Number(f64)` and `Node::Rational(i64, i64)` with `Node::Num(ExactNum)`
- Removed `Node::ClosingParen` and `Node::ClosingBrace` — dead code, never produced by parser
- Updated all pattern matches across 14 source and test files
- Parser collapses `\frac{a}{b}` with integer arguments to `Node::Num(ExactNum::rational(a, b))`

### 1.3 — Evaluator split ✅ (Session 09)
- `evaluate_exact(node, env) -> ExactNum` — stays in the exact domain
- `evaluate(node, env) -> f64` — delegates to evaluate_exact, converts at the end
- Verified: 1/3 + 1/6 = 1/2 exactly, 2/3 * 3/4 = 1/2 exactly
- Function calls (sin, cos, etc.) produce ExactNum::Float since they are transcendental

**Phase 1 complete.** 175 tests pass, 0 failed, 11 ignored.

## Phase 2: Display and LaTeX Round-Trip ✅ (Sessions 09-10)

**Goal:** `Display` output is valid, parseable LaTeX that preserves expression structure.

### 2.1 — Parenthesization ✅ (Session 09)
- Precedence method on Node, `fmt_child` inserts parens when child precedence < parent
- Power bases parenthesized for compound expressions, exponents use `{}`
- Subtract/Divide right children parenthesized at same precedence (associativity)

### 2.2 — \frac parser fix and Divide Display ✅ (Session 10)
- Tokenizer handles `\frac{A}{B}` by consuming both brace groups → `(A) / (B)`
- Shunting-yard pops functions after closing delimiters (standard algorithm)
- `Node::Divide` renders as `\frac{left}{right}`
- `Node::Multiply` omits coefficient 1: `1x` → `x`

### 2.3 — Round-trip tests ✅ (Session 10)
- 9 round-trip tests: integer, fraction, addition, polynomial, frac-addition, nested-frac, power, negate, function+constant
- Tests verify parse → format → re-parse → format stability and value preservation

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
