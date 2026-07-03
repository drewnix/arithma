# Arithma

A computer algebra system written in Rust. Exact arithmetic, not
floating-point approximation. LaTeX in, LaTeX out. Works as a CLI tool
for humans and as an MCP server for AI agents.

Arithma exists because mathematics tools should be *correct*. Not
approximately correct, not usually correct, but correct in the way that
exact rational arithmetic and well-chosen algorithms make possible.

## Why Arithma

**Single binary, no dependencies.** The MCP server is 2.5 MB. No Python
runtime, no Java, no Wolfram kernel, no network calls. Copy it anywhere and
it works.

**Exact arithmetic.** Every computation uses rational numbers (`BigRational`),
not floating-point. `1/3 + 1/3 + 1/3 = 1`, not `0.9999999999999998`. Results
are deterministic and reproducible.

**Silence over lies.** If Arithma cannot compute something, it says so. It
never guesses, approximates heuristically, or returns an unverified result.
An agent that gets "I can't do this" can try a different approach. An agent
that gets a wrong answer propagates it through its entire reasoning chain.

**Proves what's impossible.** The Risch algorithm doesn't just integrate — it
can *prove* when no elementary antiderivative exists. Ask for ∫e^{-x²}dx and
you get a mathematically rigorous explanation of why no closed form exists,
not silence or a wrong answer.

**1356 tests, zero failures.** Every algorithm is verified against known results.
The simplifier has a verified idempotency contract:
`simplify(simplify(e)) = simplify(e)`.

---

## Capabilities

### Algebra & Simplification

| Feature | Example |
|---------|---------|
| Polynomial factoring (Berlekamp-Zassenhaus) | `x⁴-1 → (x+1)(x-1)(x²+1)` |
| Radical simplification | `√12 → 2√3`, `√(4a²) → 2\|a\|` |
| Like-radical collection | `1 + √2 + √2 → 1 + 2√2` |
| Radical products | `√2·√2 → 2`, `√2·3·√2 → 6` |
| Fraction cancellation | `3x/x → 3` |
| Trig identities | `sin²+cos² → 1`, `sin/cos → tan` |
| Log rules | `ln(a·b) → ln(a)+ln(b)`, `ln(12) → 2·ln(2)+ln(3)` |
| Partial fractions | Full decomposition via factoring |
| Common denominator | `1/x + 1/(x+1) → (2x+1)/(x(x+1))` |
| Assumption-aware | `√(x²) → x` when `x > 0` |
| Symbolic trig | `sin(2)` stays symbolic; `sin(π/6) → 1/2` |
| Repeating decimals | `0.\overline{3} → 1/3` |
| Factorial & binomial | `n!`, `\binom{n}{k}` |
| GCD / LCM | `\gcd(24, 36) → 12` |

### Calculus

| Feature | Example |
|---------|---------|
| Differentiation (chain rule) | `d/dx sin(x²) → 2x·cos(x²)` |
| 8 integration methods | polynomial, parts, u-sub, trig, partial fractions, ... |
| Risch algorithm | proves non-elementarity with explanation |
| Multi-extension towers | `∫(exp(x)·ln(x) + exp(x)/x) dx = exp(x)·ln(x)` |
| Parametric integration | `∫1/(x²+a)dx = (1/√a)·arctan(x/√a)` |
| Exact definite integrals | `∫₀¹ 1/(x²+1)dx = π/4` |
| Taylor series | exact rational coefficients, symbolic center |
| Limits | L'Hôpital, series expansion, one-sided, at infinity |

### Equation Solving

| Feature | Example |
|---------|---------|
| Degree 1–4 exact | Cardano, Ferrari |
| Exact radical roots | `x²-2=0 → ±√2` |
| Parametric | `solve(ax²+bx+c=0, x)` → quadratic formula |
| Systems | exact Gaussian elimination, polynomial substitution |
| Inequalities | `x²-4 > 0 → (-∞,-2) ∪ (2,∞)` |
| Rational equations | `1/x = 2 → x = 1/2` |

### Summation & Products

| Feature | Example |
|---------|---------|
| Faulhaber's formulas | `Σk² = n(n+1)(2n+1)/6` |
| Geometric series | `Σr^k = (r^{n+1}-1)/(r-1)` |
| Telescoping | `Σ(1/k - 1/(k+1)) = n/(n+1)` |
| Product notation | `∏_{k=1}^{5} k = 120` |
| Symbolic products | `∏_{k=1}^{n} c = c^n` |

### ODEs & Series

| Feature | Example |
|---------|---------|
| Separable | `dy/dx = g(x)·h(y)` |
| First-order linear | integrating factor |
| Second-order constant-coefficient | `ay'' + by' + cy = 0` |
| Power series solutions | Hermite, Legendre, arbitrary order |
| Formal power series | lazy eval, composition, Lagrange inversion |

### Linear Algebra

| Feature | Example |
|---------|---------|
| Determinant, inverse | exact over Q |
| Eigenvalues | symbolic (2×2, 3×3), numerical (up to 4×4) |
| Systems | Ax = b, RREF |
| Algebraic number fields | exact arithmetic in Q(α) |

### Verification

| Feature | Example |
|---------|---------|
| Numeric cross-check | 12 test points, assumption-aware |
| Expression equivalence | simplify-and-compare |
| Non-elementarity proofs | Risch algorithm certificates |

---

## MCP Server

The `arithma-mcp` binary speaks [MCP](https://modelcontextprotocol.io) over
stdio. 16 tools with LaTeX I/O:

| Tool | Purpose |
|------|---------|
| `format` | Parse and normalize LaTeX without simplifying |
| `simplify` | Reduce an expression to canonical form |
| `differentiate` | Symbolic derivative |
| `integrate` | Indefinite/definite; proves non-elementary when applicable |
| `substitute` | Replace a variable with an expression |
| `solve` | Equations or inequalities |
| `solve_system` | Systems of linear/polynomial equations |
| `factor` | Irreducible factoring over Q |
| `partial_fractions` | Decompose P(x)/Q(x) |
| `limit` | Symbolic limits |
| `taylor_series` | Series expansion with exact coefficients |
| `evaluate` | Numerical evaluation |
| `matrix` | Determinant, inverse, eigenvalues, rank, RREF, Ax=b |
| `equivalent` | Check if two expressions are equal |
| `verify` | Numerically cross-check at multiple test points |
| `solve_ode` | First-order, constant-coeff, and power series |

All tools accept an optional `assumptions` parameter:

```json
{
  "expr": "\\sqrt{x^2}",
  "assumptions": {"x": ["positive"]}
}
```

Valid assumptions: `positive`, `nonnegative`, `negative`, `nonzero`, `real`,
`integer`.

### Setup

**Claude Code** — add to `.claude/settings.json`:

```json
{
  "mcpServers": {
    "arithma": {
      "command": "/path/to/arithma-mcp"
    }
  }
}
```

**Claude Desktop** — add to your config file
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

---

## Web Calculator

The `frontend/` directory contains a browser-based calculator built with
React and [MathLive](https://cortexjs.io/mathlive/). Arithma compiles to
WebAssembly and runs entirely client-side — same exact-arithmetic engine,
no server, no network calls.

```bash
wasm-pack build --target web --out-dir frontend/public/pkg
cd frontend && npm install && npm run dev
```

Then open `http://localhost:5173`.

---

## Command Line

Interactive REPL or one-shot subcommands:

```
$ arithma simplify "x^2 + 2x + 1"
x^{2} + 2x + 1

$ arithma diff "x^3 + \sin(x)" x
3x^{2} + \cos(x)

$ arithma integrate "3x^2" x
x^{3} + C

$ arithma solve "x^2 - 2 = 0"
x = \sqrt{2}
x = -\sqrt{2}

$ arithma simplify "\sum_{k=1}^{n} k^2"
\frac{n \cdot (n + 1) \cdot (2n + 1)}{6}

$ arithma solve "x^2 - 4 > 0"
(-∞, -2) ∪ (2, ∞)

$ arithma integrate "\exp(-x^2)" x
No elementary antiderivative exists. The Risch algorithm proves that
the differential equation q' + (-2x)·q = 1 has no polynomial solution,
so ∫1·exp(-x^2) dx cannot be expressed in terms of elementary functions.

$ arithma integrate "\frac{1}{x^4 + 1}" x
\frac{1}{8}·√2·ln(|x²+√2·x+1|) + ... + C
```

All 13 subcommands: `format`, `simplify`, `differentiate` (`diff`), `integrate`,
`solve`, `factor`, `prime-factorize` (`factorint`), `partial-fractions` (`pf`),
`evaluate` (`eval`), `limit`, `taylor`, `substitute` (`sub`), `ode`.

---

## Building

Cargo workspace: math engine library (root) + CLI (`crates/cli/`) + MCP server (`crates/mcp/`).

```
cargo build --release --workspace         # all crates
cargo build --release -p arithma-cli      # CLI only
cargo build --release -p arithma-mcp-server # MCP server only
cargo test --workspace                    # run all 1356 tests
```

---

## Design Principles

1. **Correct first.** Exact arithmetic everywhere. Verified idempotency.
2. **Well-chosen algorithms.** Berlekamp-Zassenhaus, Cardano, Ferrari, Risch. The algorithm matters more than the implementation speed.
3. **No hardcoded answers.** The system computes results; it does not look them up.
4. **LaTeX is the interface.** Agents speak LaTeX. We parse it and produce it.
5. **Deterministic.** Same input, same output. No randomness.
6. **Small footprint.** Under 5 MB, zero dependencies, compiles to WASM.

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development conventions,
CI discipline, and PR workflow.

## License

MIT. See [LICENSE](LICENSE).
