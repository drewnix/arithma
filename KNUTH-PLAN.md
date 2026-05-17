# Arithma — Algorithmic Foundations Plan

*Author: Knuth (QAI Head of Algorithmic Foundations)*
*Date: 2026-05-17*

---

## Vision

A CAS that is correct before it is fast, fast before it is featureful, and featureful only in ways that the architecture supports cleanly. Rust + WASM means it runs anywhere with no runtime dependencies — that is the differentiator. The mathematics must be exact, the algorithms must be well-chosen, and the code must be readable.

## Current State (Post Session 12)

**Phases 1-2 complete. Phase 3.1 complete. Phase 3.2 core complete. Equation solver classically complete (degree 1-4). Simplifier handles algebraic, trigonometric, logarithmic, and inverse function identities. Full derivative coverage for standard functions. Multivariate polynomial type with recursive representation.** 359 tests pass, 1 ignored (limits unimplemented).

### Session 12 Changes
- **Rational root theorem**: `Polynomial::rational_roots()` finds all rational roots of any-degree polynomial using the rational root theorem. Converts to primitive part for integer coefficients, enumerates ±(divisor of a₀)/(divisor of aₙ), tests via Horner evaluation.
- **Synthetic division**: `Polynomial::deflate(root)` divides out a known root in O(n), reducing degree by 1.
- **Cubic solver (Cardano)**: For irreducible cubics (no rational roots), Cardano's formula computes real roots. Handles all three cases: one real root (h > 0), double root (h = 0), and casus irreducibilis (h < 0, three real roots via trigonometric method).
- **General equation solver**: Degree ≥ 3 polynomials solved by: (1) find all rational roots via rational root theorem, (2) deflate, (3) solve remaining factor with appropriate formula (linear, quadratic, or Cardano). Works for any degree — quintic x⁵-x=0 correctly returns roots {-1, 0, 1}.
- **Implicit multiplication parsing**: Tokenizer inserts `*` for `\frac{a}{b}x`, `2(x+1)`, `(a+b)(c+d)`, and `(expr)number` patterns. Function calls like `\sin(x)` correctly not affected.
- **Environment ExactNum**: Environment stores `ExactNum` internally instead of f64. Custom serde. Last precision leak closed.
- **Ferrari's method**: Quartic solver via resolvent cubic. Classical suite complete through degree 4.
- **Power rules**: `x^a * x^b → x^(a+b)`, `x^a / x^b → x^(a-b)`, `x/x → 1`. Any expression base.
- **Pythagorean identity**: `sin²(x) + cos²(x) → 1` with coefficient and subtraction variants.
- **LaTeX function display**: `\sin(x)` not `sin(x)` in output.
- **Log properties**: `ln(a^b) → b·ln(a)`, `ln(a·b) → ln(a)+ln(b)`, `ln(a/b) → ln(a)-ln(b)`.
- **Trig quotients/reciprocals**: `sin/cos→tan`, `cos/sin→cot`, `1/sin→csc`, `1/cos→sec`, `1/tan→cot`.
- **Even/odd trig**: `sin(-x) → -sin(x)`, `cos(-x) → cos(x)`.
- **Inverse functions**: `ln(e^x) → x`, `exp(ln(x)) → x`, `sqrt(x²) → |x|`.
- **Abs rules**: `|n|→n` for numeric, `|-x|→|x|`, `||x||→|x|`.
- **Power base cases**: `0^n → 0` for n ≥ 0, `1^n → 1`.
- **Derivative expansion**: sec, csc, cot, sinh, cosh, tanh, arcsin, arccos, arctan with chain rule.

### Remaining Ignored Tests (1)
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

### 3.1 — Dense univariate polynomials ✅ (Sessions 10-12)
- `Polynomial` — dense representation, coefficients in `BigRational`, least-degree first
- Operations: add, subtract, multiply, divide-with-remainder, GCD (Euclidean)
- Conversion: `Node` ↔ `Polynomial` for polynomial expressions
- Derivative, integral, square-free decomposition, content, primitive part
- **Session 12:** `rational_roots()` via rational root theorem, `deflate()` via synthetic division
- **Equation solver:** Linear (exact), quadratic (exact when possible, f64 fallback), cubic (Cardano with trigonometric method for casus irreducibilis), any-degree via rational root theorem + deflation

### 3.2 — Multivariate polynomials ✅ (Session 13, in progress)
- `MultiPoly` — recursive representation: polynomial in x with MultiPoly coefficients in y, z, ...
- Variable ordering: lexicographic (alphabetical). First alphabetically is outermost.
- Operations: Add, Sub, Mul, Neg, scalar_mul (all via reference traits)
- Analysis: degree_in, total_degree, variables, leading_coeff, is_zero, is_constant
- Calculus: partial_derivative (any variable)
- Evaluation: evaluate_at (constant substitution), substitute (polynomial substitution with re-normalization)
- Conversion: from_node (auto-detects all variables), to_node, from_univariate, to_univariate
- Display: proper mathematical notation with parenthesization of compound coefficients
- **31 tests** covering arithmetic, derivatives, substitution, three-variable expansion, conversion
- **Remaining:** GCD (sparse modular or subresultant), multivariate division, simplifier integration

### 3.3 — Polynomial factoring
- Square-free factorization ✅ (Session 10)
- Factoring over Q via Hensel lifting (Berlekamp-Zassenhaus) — algorithmically deep, future work

**Estimated effort:** 3.1 complete. 3.2 core complete (Session 13); GCD and simplifier integration remain. 3.3 is a separate project.

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
