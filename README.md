# Arithma

A computer algebra system written in Rust. Exact arithmetic, not
floating-point approximation. LaTeX in, LaTeX out. Works as a CLI tool
for humans and as an MCP server for AI agents.

Arithma exists because mathematics tools should be *correct*. Not
approximately correct, not usually correct, but correct in the way that
exact rational arithmetic and well-chosen algorithms make possible.

## Why Arithma

**Single binary, no dependencies.** The MCP server is 1.6 MB. No Python
runtime, no Java, no Wolfram kernel, no network calls. Copy it anywhere and
it works.

**Exact arithmetic.** Every computation uses rational numbers (`BigRational`),
not floating-point. `1/3 + 1/3 + 1/3 = 1`, not `0.9999999999999998`. Results
are deterministic and reproducible.

**Silence over lies.** If Arithma cannot compute something, it says so. It
never guesses, approximates heuristically, or returns an unverified result.
An agent that gets "I can't do this" can try a different approach. An agent
that gets a wrong answer propagates it through its entire reasoning chain.

**Knows what's impossible.** The Risch algorithm doesn't just integrate — it
can *prove* when no elementary antiderivative exists. Ask for ∫e^{-x²}dx and
you get a mathematically rigorous explanation of why no closed form exists,
not silence or a wrong answer. An agent that knows the boundary of what's
computable can reason about that boundary.

**1134 tests, zero failures.** Every algorithm is verified against known results.
The simplifier has a verified idempotency contract:
`simplify(simplify(e)) = simplify(e)`.

## What it does

**Algebra.** Polynomial factoring over Q via the Berlekamp-Zassenhaus algorithm
(modular factoring, Hensel lifting, factor recombination). Multivariate
polynomial GCD. Simplification with trigonometric identities, logarithmic
properties, power rules, rational content GCD cancellation, and sign
normalization. Factored display for expressions with repeated polynomial
factors. Partial fraction decomposition. Expression equivalence checking.
Greek letter LaTeX parsing (`\alpha` through `\omega`). Implicit
multiplication (`u(3-2u)`, `α(x+1)`). **Radical simplification** with
square-factor extraction (`√12 → 2√3`, `√(4a²) → 2|a|`), like-radical
combination (`√8 + √2 → 3√2`), and common-denominator combination
(`1/x + 1/(x+1) → (2x+1)/(x(x+1))`). Assumption system for
domain-aware simplification: declare `x > 0` and `sqrt(x^2)` simplifies
to `x` instead of `|x|`.

**Calculus.** Differentiation with full chain rule. Integration via heuristic
methods (polynomial rules, transcendental table, integration by parts, u-sub,
trig powers, inverse trig, partial fractions, trig substitution) plus the
**Risch algorithm** for transcendental integration — the decision procedure
that can prove an integral has no elementary closed form. Handles both
exponential extensions (∫r(x)·e^{g(x)}dx, including rational-coefficient
integrands like ∫((1-x)/x²)·e^x dx), logarithmic extensions
(∫r(x)·f(ln(x))dx, including ∫ln(x)/x² dx with ln(x) absorption),
**multi-extension towers** (integrands with both exp and ln, like
∫(exp(x)·ln(x) + exp(x)/x) dx = exp(x)·ln(x)) via two-level Risch
towers with inner DE solving over Q(x)[ln(x)], **biquadratic
integration** (∫1/(ax⁴+bx²+c)dx via quadratic-in-x² factoring with
exact radical coefficients), **general quartic integration** via
Ferrari's method and algebraic number fields (∫1/(x⁴+x+1)dx works),
**higher-power irreducible quadratic** integration via the Ostrogradsky
reduction formula (∫1/(x²+1)²dx, ∫1/(x²+1)³dx, etc.), and
**hyperbolic substitution** (∫1/√(x²±a²)dx = ln|x+√(x²±a²)|).
All via Hermite reduction and the Rothstein-Trager resultant method.
Parametric integration for linear and **quadratic** denominators
(`∫1/(x+a)dx = ln|x+a|`, `∫1/(x²+a)dx = (1/√a)·arctan(x/√a)`,
`∫(px+q)/(ax²+bx+c)dx` with the full completing-the-square formula).
**Exact definite integration** via FTC with symbolic special-value
evaluation: `∫₀¹ 1/(x²+1)dx = π/4`.
Taylor/Maclaurin series with exact rational coefficients, including
symbolic-center expansion (`f(x)` around `x=a` with symbolic `a`).
Symbolic limits via direct substitution, GCD cancellation, and L'Hopital's
rule.

**Equation solving.** Linear through quartic, exactly (Cardano, Ferrari).
Degree 5+ via Berlekamp-Zassenhaus factoring into solvable pieces.
**Exact radical roots** — `x²-2=0` returns `±√2`, not `±1.414...`.
Rational equations with the variable in a denominator (`1/x = 2` → `x = 1/2`)
via automatic denominator clearing. **Complex root reporting** — when a
polynomial has complex roots that can't be expressed in reals, the solver
reports how many were omitted (e.g., `x³-2=0` → `x = ∛2 (2 complex roots
omitted)`). **Parametric solving** — `solve(ax²+bx+c=0, x)` returns the
quadratic formula with symbolic coefficients. **Systems of equations** —
linear systems via exact Gaussian elimination over Q (handles unique,
parametric, and inconsistent systems), polynomial systems via
substitution when at least one equation is linear. **Inequality solving** —
polynomial and rational inequalities via root-finding + sign chart analysis,
returning solutions in standard interval notation
(`x²-4 > 0` → `(-∞, -2) ∪ (2, ∞)`).

**Symbolic summation.** Closed-form evaluation of Σ expressions:
polynomial sums via Faulhaber's formulas (Σk, Σk², Σk³, Σk⁴),
geometric series (Σr^k), telescoping sums (Σ(1/k - 1/(k+1))),
and general polynomial bodies via linearity (Σ(2k-1) = n²).
Works with symbolic bounds (`Σ_{k=1}^{n} k = n(n+1)/2`)
and evaluates to exact numbers when bounds are constant
(`Σ_{k=1}^{100} k = 5050`).

**ODEs.** Three classes: separable (`dy/dx = g(x)*h(y)`), first-order linear
(`dy/dx + P(x)*y = Q(x)` via integrating factor), and second-order
constant-coefficient (`ay'' + by' + cy = 0` — distinct real, repeated, and
complex roots). Returns general solutions with arbitrary constants.

**Linear algebra.** Determinant, inverse, eigenvalues (numerical up to
4×4 via characteristic polynomial + Cardano/Ferrari, including matrices
with decimal entries; symbolic for 2×2 and 3×3 matrices with variable
entries), rank, transpose, multiplication, Ax = b, and RREF.
**Algebraic number fields** — exact arithmetic in Q(α) for algebraic α,
used for quartic integration and Risch algorithm extensions.

## MCP server

The `arithma-mcp` binary speaks [MCP](https://modelcontextprotocol.io) over
stdio. It gives Claude or any MCP-compatible AI agent access to 16 tools:

| Tool | Purpose |
|------|---------|
| `simplify` | Reduce an expression to canonical form |
| `differentiate` | Symbolic derivative with respect to any variable |
| `integrate` | Indefinite/definite integrals; proves non-elementary when applicable |
| `substitute` | Replace a variable with an expression |
| `solve` | Solve equations or inequalities (any degree, interval notation for inequalities) |
| `solve_system` | Solve systems of linear/polynomial equations |
| `factor` | Irreducible factoring over Q (Berlekamp-Zassenhaus) |
| `partial_fractions` | Decompose P(x)/Q(x) into partial fractions |
| `limit` | Symbolic limits |
| `taylor_series` | Series expansion with exact coefficients |
| `evaluate` | Numerical evaluation with variable assignments |
| `matrix` | Determinant, inverse, eigenvalues, rank, RREF, Ax=b |
| `equivalent` | Check if two expressions are mathematically equal |
| `verify` | Numerically cross-check two expressions at multiple test points |
| `solve_ode` | Solve first-order and second-order constant-coeff ODEs |

Every tool accepts LaTeX and returns LaTeX. Nine tools accept an optional
`assumptions` parameter for domain-aware simplification:

```json
{
  "expr": "\\sqrt{x^2}",
  "assumptions": {"x": ["positive"]}
}
```

Valid assumptions: `positive`, `nonnegative`, `negative`, `nonzero`, `real`,
`integer`.

### Setup

**Claude Code** -- add to `.claude/settings.json` (project) or
`~/.claude/settings.json` (global):

```json
{
  "mcpServers": {
    "arithma": {
      "command": "/path/to/arithma-mcp"
    }
  }
}
```

**Claude Desktop** -- add to your config file
(`~/Library/Application Support/Claude/claude_desktop_config.json` on macOS):

```json
{
  "mcpServers": {
    "arithma": {
      "command": "/path/to/arithma-mcp"
    }
  }
}
```

## Command line

Arithma also works as a standalone CLI tool. Run with no arguments for an
interactive REPL, or pass a subcommand for one-shot computation:

```
$ arithma simplify "x^2 + 2x + 1"
x^{2} + 2x + 1

$ arithma diff "x^3 + \sin(x)" x
3x^{2} + \cos(x)

$ arithma integrate "3x^2" x
x^{3} + C

$ arithma solve "x^2 - 4 = 0"
x = 2
x = -2

$ arithma solve "x^2 - 2 = 0"
x = \sqrt{2}
x = -\sqrt{2}

$ arithma solve "x + y = 3, x - y = 1" "x, y"
x = 2
y = 1

$ arithma solve "x + y = 5, x * y = 6" "x, y"
Solution 1:
  x = 3
  y = 2
Solution 2:
  x = 2
  y = 3

$ arithma simplify "\sqrt{12}"
2\sqrt{3}

$ arithma simplify "\sqrt{8} + \sqrt{2}"
3\sqrt{2}

$ arithma integrate "\frac{1}{x^2 + a}" x
\frac{1}{\sqrt(a)} \cdot \arctan(\frac{2x}{2\sqrt(a)}) + C

$ arithma factor "x^4 - 1"
(x + 1) * (x - 1) * (x^2 + 1)

$ arithma eval "x^2 + 1" x=3
10

$ arithma taylor "\sin(x)" x 0 5
\frac{1}{120} \cdot x^{5} - \frac{1}{6} \cdot x^{3} + x

$ arithma ode "x^2" x y
y = C_{1} + \frac{1}{3} \cdot x^{3}

$ arithma ode --cc 1 0 1
y = C_{1} \cdot \cos(x) + C_{2} \cdot \sin(x)

$ arithma simplify "\sum_{k=1}^{n} k^2"
\frac{n \cdot (n + 1) \cdot (2n + 1)}{6}

$ arithma simplify "\sum_{k=0}^{n} 2^k"
2^{n + 1} - 1

$ arithma solve "x^2 - 4 > 0"
(-∞, -2) ∪ (2, ∞)

$ arithma solve "\frac{x-1}{x+2} > 0"
(-∞, -2) ∪ (1, ∞)

$ arithma integrate "\exp(-x^2)" x
No elementary antiderivative exists. The Risch algorithm proves that
the differential equation q' + (-2x)·q = 1 has no polynomial solution,
so ∫1·exp(-x^2) dx cannot be expressed in terms of elementary functions.

$ arithma integrate "\ln(x)^2" x
2x + -2x \cdot \ln(x) + x \cdot \ln(x)^{2} + C

$ arithma integrate "\frac{1}{x \cdot \ln(x)}" x
\ln(\ln(x)) + C

$ arithma integrate "\frac{1}{\ln(x)}" x
No elementary antiderivative exists. The Rothstein-Trager resultant
has no rational roots, so the integral cannot be expressed as a sum
of logarithms.

$ arithma integrate "\frac{\exp(x)}{1 + \exp(x)}" x
\ln(1 + \exp(x)) + C

$ arithma integrate "\frac{1}{1 + \exp(x)}" x
-1 \cdot \ln(1 + \exp(x)) + x + C

$ arithma integrate "\frac{1-x}{x^2} \cdot \exp(x)" x
\frac{-1}{x} \cdot \exp(x) + C

$ arithma integrate "\frac{\exp(x)}{x}" x
No elementary antiderivative exists. The Risch algorithm proves that
the differential equation q' + (1)·q = (1)/(x) has no rational
solution, so ∫(1/x)·exp(x) dx cannot be expressed in terms of
elementary functions.

$ arithma integrate "\frac{\ln(x)}{x^2}" x
\frac{-1}{x} + \frac{-1}{x} \cdot \ln(x) + C

$ arithma integrate "\frac{\ln(x)}{x + 1}" x
No elementary antiderivative exists. The integral requires ln(x + 1),
which is outside the single ln(x) extension tower.

$ arithma integrate "\exp(x) \cdot \ln(x)" x
No elementary antiderivative exists. The Risch DE q' + (1)·q = θ
has no solution in Q(x, ln(x)), so the integral cannot be expressed
in terms of elementary functions.

$ arithma integrate "\frac{\ln(x)}{1 + \exp(x)}" x
No elementary antiderivative exists. The two-level Rothstein-Trager
resultant has no constant roots, so the integral cannot be expressed
in terms of elementary functions.

$ arithma integrate "\frac{1}{x^4 + 1}" x
\frac{1}{8} \cdot \sqrt{2} \cdot \ln(|x^{2} + \sqrt{2} \cdot x + 1|) + \frac{\frac{1}{2}}{\sqrt{2}} \cdot \arctan(\frac{2x + \sqrt{2}}{\sqrt{2}}) + -\frac{1}{8} \cdot \sqrt{2} \cdot \ln(|x^{2} - \sqrt{2} \cdot x + 1|) + \frac{\frac{1}{2}}{\sqrt{2}} \cdot \arctan(\frac{2x - \sqrt{2}}{\sqrt{2}}) + C
```

All 11 subcommands: `simplify`, `differentiate` (`diff`), `integrate`,
`solve`, `factor`, `partial-fractions` (`pf`), `evaluate` (`eval`), `limit`,
`taylor`, `substitute` (`sub`), `ode`. Run `arithma --help` for full usage.

## Building

```
cargo build --release                     # both binaries
cargo build --release --bin arithma-mcp   # MCP server only
cargo test                                # run all 1062 tests
```

## Design principles

**Correct first.** Exact arithmetic everywhere. Verified idempotency contract
on the simplifier.

**Well-chosen algorithms.** Berlekamp-Zassenhaus for polynomial factoring.
Subresultant remainder sequence for GCD. Cardano and Ferrari for cubics and
quartics. The choice of algorithm matters more than the speed of implementation.

**No hardcoded answers.** The system computes its results; it does not look
them up from a table of special cases.

## Architecture

```
src/
  node.rs              -- expression AST
  exact.rs             -- exact rational arithmetic (BigRational)
  assumptions.rs       -- variable assumptions (positive, integer, etc.)
  parser.rs            -- LaTeX tokenizer and parser
  simplify.rs          -- rule-based simplification with idempotency contract
  polynomial.rs        -- dense univariate polynomials over Q
  multipoly.rs         -- multivariate polynomials (recursive representation)
  mod_poly.rs          -- polynomial arithmetic over Z_p, Berlekamp-Zassenhaus
  partial_fractions.rs -- partial fraction decomposition via extended GCD
  derivative.rs        -- symbolic differentiation
  integration.rs       -- symbolic integration (heuristics + Risch fallback)
  rational_function.rs -- p(x)/q(x) arithmetic for Risch algorithm
  ext_poly.rs          -- polynomials in tower variable θ over Q(x)
  risch.rs             -- Risch algorithm: Hermite reduction, DE solver
                          (polynomial and rational coefficients),
                          exponential/logarithmic integration, Rothstein-Trager
                          resultant method, non-elementary proofs
  ode.rs               -- ODE solver (separable, linear, constant-coefficient)
  series.rs            -- Taylor/Maclaurin series
  limits.rs            -- symbolic limits
  matrix.rs            -- matrix operations
  expression.rs        -- equation solving (degree 1-4 + factoring)
  systems.rs           -- systems of linear/polynomial equations (Gaussian elimination)
  algebraic.rs         -- algebraic number fields Q(α), AlgPoly, Hermite reduction
  evaluator.rs         -- numerical evaluation
  bin/arithma-mcp.rs   -- MCP server (JSON-RPC 2.0 over stdio)
```

~31K lines of Rust. Expressions are trees of `Node` variants. `ExactNum`
wraps `BigRational` for exact arithmetic, falling back to `f64` only for
transcendental constants and function results. The parser reads LaTeX; the
display implementation writes LaTeX. Round-trip stability is tested.

## License

MIT. See [LICENSE](LICENSE).
