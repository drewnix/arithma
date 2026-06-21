# Arithma — Architecture and Design

*A mathematical truth engine for AI agents.*

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
- RUBI-style rule tables (6700+ patterns — the algorithmic approach is the right foundation)

---

## Current State

**1062 tests. 0 failures. 15 MCP tools. ~31K lines of Rust. Binary under 3 MB. Zero clippy warnings.**

---

## Capabilities

### Parsing and Display

- **LaTeX round-trip**: parse LaTeX input, produce LaTeX output. All operations accept and return LaTeX.
- **Greek letters**: `\alpha` → `α` internally, `\alpha` on output. `normalize_var()` at all API boundaries.
- **Symbolic constants**: `\pi` is `Variable("π")` (symbolic, not float). `\mathrm{e}` for Euler's number.
- **Parser hardening**: implicit multiplication (`u(3-2u)`, `α(x+1)`), space-separated variables, sign normalization in fractions (`-3/(-2b-1)` → `3/(2b+1)`).
- **Operator reservation**: `\int`, `\prod`, `\oint` not tokenized as variables. LaTeX spacing (`\,`, `\;`, `\quad`) stripped.
- **Leibniz detection**: `\frac{d}{dx}` and `\frac{\partial}{\partial x}` error helpfully instead of parsing as fractions.

### Exact Arithmetic

- **BigRational**: all arithmetic in exact rational numbers. No floating-point until the user explicitly asks for evaluation.
- **Radical preservation**: `√12 → 2√3`, `√(4a²) → 2|a|` (assumption-aware). Like-radical combination: `√8+√2 → 3√2`.
- **Symbolic sqrt**: simplifier preserves `√2` as symbolic, never evaluates to float.

### Simplification

- **Polynomial normalization**: canonical form for polynomial expressions.
- **Trig identities**: sin²+cos² → 1, sin(-x) → -sin(x), cos(-x) → cos(x), k·sin/cos → k·tan.
- **Logarithmic rules**: ln(a·b) → ln(a)+ln(b), ln(a^b) → b·ln(a), ln(e^x) → x, exp(ln(x)) → x.
- **Special-value evaluation**: sin(kπ) → 0 for integer k, cos(nπ) → (-1)^n, sin(π/2) → 1, cos(π/2) → 0, arctan(1) → π/4, ln(1) → 0, tan(π/4) → 1.
- **Rational content GCD**: `(-32α+32)/(16α+8)` → `(-4α+4)/(2α+1)`. Fraction coefficient cancellation for integer GCDs.
- **Common-denominator combination**: `1/x + 1/(x+1)` → `(2x+1)/(x(x+1))`.
- **Like function term collection**: `3·exp(x) + 5·exp(x)` → `8·exp(x)`, `a·sin(x) + b·sin(x)` → `(a+b)·sin(x)`.
- **Factored display**: repeated/multiple factors shown in factored form: `48/(16α³+24α²+12α+2)` → `24/(2α+1)³`.
- **Negation normalization**: `f·(-g) → -(f·g)`, nested negations eliminated.
- **Assumption system**: 6 property types (positive, nonneg, negative, nonzero, real, integer). `√(x²) → x` when x ≥ 0. Conservative default.
- **Idempotency contract**: simplification is stable — applying it twice gives the same result.

### Differentiation

- Full chain rule, product rule, quotient rule.
- All standard functions: trig, inverse trig, exp, ln, hyperbolic.
- Partial derivatives via the `differentiate` tool with variable specification.

### Integration

**8 classical techniques:**
- Polynomial term-by-term
- Transcendental (exp, trig, log)
- Integration by parts (IBP)
- u-substitution
- Trig power reduction (all parities)
- Inverse trig
- Partial fractions (via Berlekamp-Zassenhaus factoring over Q)
- Trig substitution

**Risch decision procedure (transcendental case):**
- Hermite reduction for rational functions in extension variables.
- Rothstein-Trager resultant method for logarithmic rational integration.
- Risch DE solver for polynomial and rational coefficient exponential integration.
- Multi-extension towers: exp-over-log and log-over-exp, both polynomial and rational integrands.
- θ₁-in-denominator content extraction for separable tower factorizations.
- Non-elementarity proofs: when no elementary antiderivative exists, Arithma proves it and reports it as a result (not an error).

**Parametric integration:**
- Linear denominators: `∫1/(x+a)dx = ln|x+a|`
- Quadratic denominators: `∫1/(x²+a)dx = (1/√a)·arctan(x/√a)`, completing-the-square for `∫(px+q)/(ax²+bx+c)dx`
- Biquadratic: `∫1/(x⁴+px²+q)dx` with exact radical coefficients
- General quartic: `∫1/(x⁴+x+1)dx` via Ferrari's method and algebraic number fields Q(s)
- Higher-power irreducible quadratic: `∫1/(x²+1)²dx`, `∫1/(x²+1)³dx` via Ostrogradsky reduction
- Hyperbolic substitution: `∫1/√(x²±a²)dx = ln|x+√(x²±a²)|`

**Definite integration:**
- Exact via FTC: symbolic substitution of bounds, special-value evaluation.
- `∫₀¹ 1/(x²+1)dx = π/4`, `∫₁ᵉ 1/x dx = 1`, `∫₀ᵖⁱ sin(x)dx = 2`.
- MCP bounds accept LaTeX strings (e.g., `\pi`, `1/2`).

### Equation Solving

- **Degree 1-4**: exact closed-form solutions (linear, quadratic formula, Cardano, Ferrari).
- **Degree ≥ 5**: Berlekamp-Zassenhaus factoring, solve each irreducible factor ≤ 4.
- **Exact radical roots**: `solve(x²-2=0)` → `±√2`, not `±1.414...`.
- **Rational equations**: automatic denominator clearing: `1/x = 2` → `x = 1/2`.
- **Parametric equations**: `solve(ax²+bx+c=0, x)` → `(-b ± √(b²-4ac))/(2a)`. Differentiation-based coefficient extraction for symbolic coefficients.
- **Systems of equations**: linear systems via exact Gaussian elimination over Q (unique, parametric, inconsistent). Polynomial systems via recursive substitution when at least one equation is linear. CLI: `arithma solve "eq1, eq2" "x, y"`. MCP: `solve_system` tool.
- **Complex root reporting**: `solve_full()` returns solution count and omitted-complex-root count.

### Polynomial Algebra

- Dense univariate polynomials over Q with full arithmetic.
- Multivariate polynomials (`MultiPoly`).
- **Berlekamp-Zassenhaus factoring**: 4-layer pipeline (rational roots → Berlekamp mod p → Hensel lifting → factor combination). Handles non-monic leading coefficients.
- **Partial fraction decomposition**: via factoring. Correct content factor for non-monic linear denominators.

### Matrix Operations

- Parsing: `\begin{pmatrix}...\end{pmatrix}` LaTeX input.
- Determinant, inverse, eigenvalues, eigenvectors.
- Characteristic polynomial computation.
- Symbolic eigenvalues for 2×2 and 3×3 matrices with variable entries (candidate search + deflation).
- Numerical eigenvalues up to 4×4 (characteristic polynomial + Cardano/Ferrari).
- Decimal matrix entries supported via float-to-rational conversion.

### Series and Limits

- **Taylor expansion**: univariate around numeric or symbolic center, with exact coefficients.
- **Limits**: direct substitution, L'Hopital's rule.

### ODEs

- **Separable**: auto-detects g(x)·h(y) factorization.
- **First-order linear**: integrating factor method.
- **Second-order constant-coefficient**: discriminant-based (distinct real, repeated, complex roots).
- Returns general solutions with C₁, C₂.

### MCP Server

15 tools with LaTeX I/O: `simplify`, `differentiate`, `integrate`, `solve`, `solve_system`, `factor`, `partial_fractions`, `evaluate`, `substitute`, `taylor_series`, `limit`, `solve_ode`, `matrix`, `equivalent`. Hand-rolled JSON-RPC, under 3 MB binary. All tools accept optional `assumptions` parameter.

### CLI

Subcommand interface: `arithma simplify|diff|integrate|solve|factor|pf|eval|limit|taylor|sub|ode`. REPL fallback for interactive use. Definite integrals: `arithma integrate <expr> [var] [lo hi]`.

---

## Architecture

### AST (`Node`)

All mathematical expressions are represented as a tree of `Node` variants:
- `Num(ExactNum)` — exact rational or float
- `Variable(String)` — symbolic variables and constants (including `π`)
- Binary operators: `Add`, `Subtract`, `Multiply`, `Divide`, `Power`
- Unary: `Negate`, `Sqrt`, `Abs`
- `Function(String, Vec<Node>)` — named function calls
- `Equation(Node, Node)` — for equation solving
- `Summation`, `Piecewise` — structural

### Number System (`ExactNum`)

Two variants: `Rational(BigRational)` for exact computation, `Float(f64)` for numerical fallback. All internal computation uses `Rational` wherever possible. Float is a last resort.

### Polynomial Infrastructure

- `Polynomial` — dense univariate over Q with coefficient access, arithmetic, GCD, rational roots, deflation.
- `MultiPoly` — sparse multivariate for content GCD and multi-variable simplification.
- `ExtPoly` — polynomial in tower variable θ with Q(x) rational function coefficients, for the Risch algorithm.
- `RationalFunction` — p(x)/q(x) with full arithmetic, for Hermite reduction and Rothstein-Trager.
- `ModPoly` — polynomials over Z/pZ, for Berlekamp factoring.
- `NumberField` — algebraic number field Q(α) with exact BigRational arithmetic, for quartic integration and Risch extensions.
- `AlgPoly` — univariate polynomials with Q(α) coefficients, with GCD and Hermite reduction.

### Integration Pipeline

```
input expression
    → pattern match (polynomial, trig, exp, log, inverse trig)
    → try each technique in order
    → if all fail, build differential extension tower
    → Risch algorithm (Hermite + Rothstein-Trager / Risch DE)
    → return antiderivative or proof of non-elementarity
```

### Crate Structure

Currently a single crate. Workspace split planned when compile times become a bottleneck (~31K lines, approaching threshold):

```
arithma/
├── arithma-core/        # Node, ExactNum, simplification
├── arithma-parse/       # Tokenizer, parser, LaTeX rendering
├── arithma-poly/        # Polynomial arithmetic, GCD, factoring
├── arithma-calculus/    # Differentiation, integration, series, ODE
├── arithma-linalg/     # Matrix operations
├── arithma-wasm/       # WASM bindings
└── arithma-mcp/        # MCP server
```

---

## Design Principles

1. **Correctness beats coverage.** Every result is provably correct. We never guess. If we can't compute something, we say so.
2. **Exact before approximate.** `BigRational` arithmetic throughout. Float only when the user asks for numerical evaluation.
3. **LaTeX is the interface.** Agents speak LaTeX. We parse it and produce it. No intermediate format for users to learn.
4. **Deterministic.** Same input, same output. No randomness, no heuristics that change behavior across runs.
5. **Small footprint.** Under 5 MB, zero runtime dependencies, compiles to WASM. Runs anywhere.
6. **Algorithmic over heuristic.** The Risch algorithm over pattern tables. Berlekamp-Zassenhaus over trial division. The right algorithm is more maintainable than the right collection of special cases.

---

## What Done Looks Like

Arithma reaches a natural resting point when an AI agent with access to it can:

1. **Verify** any undergraduate-level mathematical claim
2. **Compute** derivatives, integrals, solutions, factorizations, series, limits, and matrix operations with exact arithmetic
3. **Know the boundary** — distinguish "I can't compute this yet" from "this has no elementary closed form"
4. **Reason under constraints** — simplify with assumptions about variable domains
5. **Solve basic ODEs** — the three classes that cover 80% of applied mathematics
6. **Do all of this** from a single binary under 5 MB with zero dependencies, deterministic output, and sub-second response times

That's roughly 35-40% of Mathematica's CAS core coverage, with 100% correctness on everything we claim to compute, at 1/3000th the deployment footprint.
