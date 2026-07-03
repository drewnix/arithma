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

**1517 tests. 0 failures. 16 MCP tools. ~40K lines of Rust. MCP binary 2.5 MB. Zero clippy warnings.**

---

## Capabilities

### Parsing and Display

- **LaTeX round-trip**: parse LaTeX input, produce LaTeX output. All operations accept and return LaTeX.
- **Greek letters**: `\alpha` → `α` internally, `\alpha` on output. `normalize_var()` at all API boundaries.
- **Symbolic constants**: `\pi` is `Variable("π")` (symbolic, not float). `e` is `Variable("e")` (symbolic, resolved to float during evaluation — same treatment as `π`).
- **Parser hardening**: implicit multiplication (`u(3-2u)`, `α(x+1)`), space-separated variables, sign normalization in fractions (`-3/(-2b-1)` → `3/(2b+1)`).
- **Operator reservation**: `\int`, `\prod`, `\oint` not tokenized as variables. LaTeX spacing (`\,`, `\;`, `\quad`) stripped.
- **Leibniz detection**: `\frac{d}{dx}` and `\frac{\partial}{\partial x}` error helpfully instead of parsing as fractions.

### Exact Arithmetic

- **BigRational**: all arithmetic in exact rational numbers. No floating-point until the user explicitly asks for evaluation.
- **Radical preservation**: `√12 → 2√3`, `√(4a²) → 2|a|` (assumption-aware). Like-radical combination: `√8+√2 → 3√2`.
- **Symbolic sqrt**: simplifier preserves `√2` as symbolic, never evaluates to float. `√a·√a → a`, `(√x)² → |x|` (or `x` with nonneg assumption).
- **Repeating decimals**: `0.\overline{3} → 1/3` exactly, parsed at the tokenizer level. Supports non-repeating prefixes: `0.1\overline{6} → 1/6`.
- **Factorial**: `n!` postfix and `\factorial{n}`. Exact evaluation via BigRational. `factorial_u64` for values ≤ 20!, `factorial_exact` for arbitrary size.
- **Integer number theory**: prime factorization, GCD/LCM (`\gcd`, `\lcm` as multi-argument functions), square-factor extraction.

### Simplification

- **Polynomial normalization**: canonical form for polynomial expressions.
- **Trig identities**: sin²+cos² → 1, sin(-x) → -sin(x), cos(-x) → cos(x), k·sin/cos → k·tan.
- **Logarithmic rules**: ln(a·b) → ln(a)+ln(b), ln(a^b) → b·ln(a), ln(e^x) → x, exp(ln(x)) → x. **Integer factorization**: ln(12) → 2·ln(2) + ln(3).
- **Special-value evaluation**: sin(kπ) → 0 for integer k, cos(nπ) → (-1)^n, sin(π/2) → 1, cos(π/2) → 0, arctan(1) → π/4, ln(1) → 0, tan(π/4) → 1. Trig functions with non-special numeric args (sin(2), cos(3)) stay symbolic — no closed form exists.
- **Shared factor cancellation**: `(3·x)/x → 3`, `(2·(ln(2)+ln(3)))/(ln(2)+ln(3)) → 2`.
- **Rational content GCD**: `(-32α+32)/(16α+8)` → `(-4α+4)/(2α+1)`. Fraction coefficient cancellation for integer GCDs.
- **Common-denominator combination**: `1/x + 1/(x+1)` → `(2x+1)/(x(x+1))`.
- **Like function term collection**: `3·exp(x) + 5·exp(x)` → `8·exp(x)`, `a·sin(x) + b·sin(x)` → `(a+b)·sin(x)`.
- **Factored display**: repeated/multiple factors shown in factored form: `48/(16α³+24α²+12α+2)` → `24/(2α+1)³`.
- **Negation normalization**: `f·(-g) → -(f·g)`, nested negations eliminated.
- **Assumption system**: 6 property types (positive, nonneg, negative, nonzero, real, integer). `√(x²) → x` when x ≥ 0. Conservative default.
- **f64 → rational canonicalization**: float coefficients near simple rationals (denominators ≤ 100) are converted to exact BigRational. `0.5·x → (1/2)·x`, `0.333...·x → (1/3)·x`. Improves equivalence detection.
- **Numeric verification**: `verify` tool evaluates two expressions at 12 deterministic test points, reports PASS or FAIL with specific counterexample. Multi-variable support. **Assumption-aware**: test points are filtered by stated assumptions — `verify(√(x²), x, {x: positive})` correctly skips negative test points instead of producing spurious counterexamples.
- **Idempotency contract**: simplification is stable — applying it twice gives the same result.

### Differentiation

- Full chain rule, product rule, quotient rule.
- All 24 trig/hyperbolic functions: sin, cos, tan, csc, sec, cot, arcsin, arccos, arctan, arccsc, arcsec, arccot, sinh, cosh, tanh, csch, sech, coth, arcsinh, arccosh, arctanh, arccsch, arcsech, arccoth. Plus exp, ln.
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

**Logarithm convention:** Indefinite integrals use `ln|·|` (real-valued convention). `∫1/x dx = ln|x| + C`, `∫tan(x) dx = -ln|cos(x)| + C`. The Risch algorithm produces complex logarithms without absolute value signs, following the differential algebra framework. The two conventions coexist: classical techniques emit `ln|·|`, Risch emits `ln(·)`. The `verify` tool evaluates numerically and does not distinguish between them — this is a known boundary where spurious mismatches can occur at negative arguments.

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
- **Inequality solving**: polynomial and rational inequalities via root-finding + sign chart. Returns standard interval notation: `x²-4 > 0` → `(-∞, -2) ∪ (2, ∞)`. Handles >, >=, <, <= with proper endpoint inclusion. Rational inequalities exclude poles from solution set.
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

### Symbolic Summation

- **Faulhaber's formulas**: closed-form evaluation of Σk^p for p=0..4. `Σ_{k=1}^{n} k² = n(n+1)(2n+1)/6`.
- **Geometric series**: `Σ_{k=0}^{n} r^k = (r^{n+1}-1)/(r-1)`. Handles coefficients.
- **Telescoping sums**: detects g(k)-g(k+1) pattern before body simplification. `Σ(1/k - 1/(k+1)) = n/(n+1)`.
- **Telescoping via partial fractions**: `Σ 1/(k(k+1))` decomposes to `1/k - 1/(k+1)` automatically.
- **Symbolic coefficients**: `Σ a·k²` decomposes into symbolic coefficient × Faulhaber. Handles linear combinations: `Σ (a·k² + b·k)`.
- **General polynomial bodies**: linearity decomposition. `Σ(2k-1) = n²`.
- **Constant/numeric bounds**: evaluates to exact number when possible. `Σ_{k=1}^{100} k = 5050`.

### Symbolic Product Notation

- **Constant body**: `∏_{k=1}^{n} c = c^n`.
- **Factorial**: `∏_{k=1}^{n} k = n!` (numeric bounds evaluate exactly).
- **Geometric products**: `∏_{k=0}^{n} r^k = r^{n(n+1)/2}`.
- **Odd-number products**: `∏_{k=1}^{n}(2k-1)` with exact evaluation for numeric bounds.
- Shared parser with summation via `IndexedNotation` enum.

### Series and Limits

- **Taylor expansion**: univariate around numeric or symbolic center, with exact coefficients. Parametric expressions (e.g., `n/(1+(n-1)a)` expanded in `a`) produce symbolic coefficients.
- **Limits**: direct substitution, polynomial GCD cancellation, L'Hopital's rule, Taylor series expansion for higher-order indeterminate forms. Limits at infinity via polynomial degree comparison. Exponential indeterminate forms (1^∞, 0^0). One-sided limits with signed infinity results (lim_{x→0+} 1/x = +∞). Direction parsed from point string: `"0+"`, `"0-"`.

### ODEs

- **Separable**: auto-detects g(x)·h(y) factorization.
- **First-order linear**: integrating factor method.
- **Second-order constant-coefficient**: discriminant-based (distinct real, repeated, complex roots).
- **Power series solutions**: general linear ODEs with polynomial coefficients at ordinary points. Derives coefficient recurrence from the ODE structure, wraps in `FormalPowerSeries::from_fn` with internal cache. Returns k independent solutions for order-k ODE, with initial condition matching. Handles Hermite, Legendre, and arbitrary-order equations.
- Returns general solutions with C₁, C₂.

### Formal Power Series

- Lazy coefficient evaluation with `Rc<RefCell<FpsInner>>` for self-referential recurrences.
- Generator variants: explicit, closure, sum, diff, neg, scalar-mul, product, inverse, quotient.
- Composition `f(g(x))` with g-power cache for O(n²) per coefficient.
- Compositional inverse (reversion) via Lagrange inversion: g_n = (1/n)·[x^{n-1}](x/f(x))^n.
- Formal derivative and integral.
- Built-in series: exp, sin, cos, geometric, ln(1+x).
- Truncation to `Polynomial` for finite-term output.

### MCP Server

16 tools with LaTeX I/O: `format`, `simplify`, `differentiate`, `integrate`, `solve`, `solve_system`, `factor`, `partial_fractions`, `evaluate`, `substitute`, `taylor_series`, `limit`, `solve_ode`, `matrix`, `equivalent`, `verify`. Hand-rolled JSON-RPC, under 3 MB binary. All tools accept optional `assumptions` parameter. `solve_ode` accepts `poly_coeffs` for general linear ODEs with polynomial coefficients (power series solution). `format` parses and normalizes LaTeX without simplifying — useful for canonicalizing messy input.

### CLI

Subcommand interface: `arithma format|simplify|diff|integrate|solve|factor|pf|eval|limit|taylor|sub|ode`. REPL fallback for interactive use. Definite integrals: `arithma integrate <expr> [var] [lo hi]`. `format` parses and re-emits canonical LaTeX without simplification.

---

## Architecture

### AST (`Node`)

All mathematical expressions are represented as a tree of `Node` variants:
- `Num(ExactNum)` — exact rational or float
- `Variable(String)` — symbolic variables and constants (including `π`)
- Binary operators: `Add`, `Subtract`, `Multiply`, `Divide`, `Power`
- Unary: `Negate`, `Sqrt`, `Abs`, `Factorial`
- `Function(String, Vec<Node>)` — named function calls
- `Equation(Node, Node)` — for equation solving
- `Summation`, `Product`, `Piecewise` — structural

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
- `FormalPowerSeries` — lazy formal power series with `Rc<RefCell>` caching. Self-referential recurrences (inverse, quotient, ODE coefficients) via the borrow-clone-compute pattern.

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

Cargo workspace with three members:

```
arithma/                 # root: math engine library (lib only, no binaries)
├── src/                 # all math modules, WASM bindings
├── tests/               # integration tests
├── crates/
│   ├── cli/             # arithma CLI binary
│   └── mcp/             # arithma-mcp server binary
└── frontend/            # React + MathLive web calculator (WASM)
```

The root crate is the public API — downstream Rust projects depend on `arithma`. The CLI and MCP server are thin wrappers. WASM builds target the root crate (`wasm-pack build --target web`).

---

## Design Principles

1. **Correctness beats coverage.** For polynomial and rational function operations, equivalence is decided exactly via canonical forms. For transcendental expressions, equivalence is verified by deterministic numerical evaluation — high-confidence but not a proof (Richardson's theorem makes the general problem undecidable). Every integration result is verified by differentiating back and checking against the integrand. If we can't compute something, we say so.
2. **Exact before approximate.** `BigRational` arithmetic throughout. Float only when the user asks for numerical evaluation.
3. **LaTeX is the interface.** Agents speak LaTeX. We parse it and produce it. No intermediate format for users to learn.
4. **Deterministic.** Same input, same output. No randomness, no heuristics that change behavior across runs.
5. **Small footprint.** Under 5 MB, zero runtime dependencies, compiles to WASM. Runs anywhere.
6. **Algorithmic over heuristic.** The Risch algorithm over pattern tables. Berlekamp-Zassenhaus over trial division. The right algorithm is more maintainable than the right collection of special cases.

---

## Direction

Arithma's computation surface is mature. The next phase is **verification infrastructure** — making Arithma the mathematical reasoning verification layer for AI agents. See [Discussion #63](https://github.com/drewnix/arithma/discussions/63) for the full design.

**In progress:**
- **Structured result status** on all tool outputs (`exact`, `numerically_consistent`, `heuristic`, `unable_to_compute`, `provably_impossible`) — so agents know the strength of evidence behind every result
- **Reasoning-chain verification** with typed step relations (`equals`, `derivative_of`, `integral_of`, `substitution`, `implies`, `solution_of`, `factored_form_of`) — check each step of a derivation using the appropriate primitive
- **Impossibility proofs as first-class output** — structured proof certificates for Risch non-elementarity, special function recognition (erf, Ei, Li)

**The thesis:** the biggest gap in AI-assisted mathematics isn't missing features — it's missing verification. An agent that can check each step of its own reasoning, catch errors at the exact point they occur, and prove when something is mathematically impossible changes the class of problems agents can reliably solve.

## What Done Looks Like

Arithma reaches completion when an AI agent with access to it can:

1. **Verify each step** of a multi-step derivation, catching errors at the specific point they occur
2. **Trust the evidence level** — know whether a result is algebraically exact, numerically consistent, or heuristic
3. **Know the boundary** — get a structured proof when no closed form exists, with the special function name when recognized
4. **Compute** derivatives, integrals, solutions, factorizations, series, limits, and matrix operations with exact arithmetic
5. **Reason under constraints** — simplify with assumptions about variable domains
6. **Do all of this** from a single binary under 5 MB with zero dependencies, deterministic output, and sub-second response times
