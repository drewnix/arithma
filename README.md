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
not floating-point. $\frac{1}{3} + \frac{1}{3} + \frac{1}{3} = 1$, not
`0.9999999999999998`. Results are deterministic and reproducible.

**Silence over lies.** If Arithma cannot compute something, it says so. It
never guesses, approximates heuristically, or returns an unverified result.
An agent that gets "I can't do this" can try a different approach. An agent
that gets a wrong answer propagates it through its entire reasoning chain.

**Proves what's impossible.** The Risch algorithm doesn't just integrate — it
can *prove* when no elementary antiderivative exists. Ask for
$\int e^{-x^2}\,dx$ and you get a mathematically rigorous explanation of why
no closed form exists, not silence or a wrong answer.

**Every answer declares its evidence.** Each MCP response carries a
`result_status`: `exact` (decision procedure), `verified` (numeric agreement
at n points — evidence, never proof), `heuristic`, `unable_to_compute`, or
`provably_impossible` (with certificate). An agent can tell an algebraic
identity from "agreed at 12 test points" — because those are different
things. Verdict-shaped tools additionally carry a machine-readable
`verdict` (`pass`/`fail`/`inconclusive`) — no consumer ever parses prose to
learn an outcome. See [docs/result-status.md](docs/result-status.md).

**Verifies whole derivations, not just answers.** The `verify_chain` tool
takes an ordered list of reasoning steps — each declaring its relation to
the previous one (`equals`, `derivative_of`, `integral_of`, `substitution`,
`implies`, `solution_of`, `factored_form_of`) — and checks every step by
the mechanism appropriate to its relation. The chain's status is the
*minimum* evidence across steps: one numeric step makes the whole chain
`verified`, never `exact`. A failing step carries the specific
counterexample that refutes it. The counterexample is the diagnosis.

**1688 tests, zero failures.** Every algorithm is verified against known results.
The simplifier has a verified idempotency contract:
`simplify(simplify(e)) = simplify(e)`.

---

## Capabilities

### Algebra & Simplification

| Feature | Example |
|---------|---------|
| Polynomial factoring (Berlekamp-Zassenhaus) | $x^4-1 \to (x+1)(x-1)(x^2+1)$ |
| Radical simplification | $\sqrt{12} \to 2\sqrt{3}$, $\sqrt{4a^2} \to 2\lvert a\rvert$ |
| Like-radical collection | $1 + \sqrt{2} + \sqrt{2} \to 1 + 2\sqrt{2}$ |
| Radical products | $\sqrt{2}\cdot\sqrt{2} \to 2$, $\sqrt{2}\cdot 3\cdot\sqrt{2} \to 6$ |
| Fraction cancellation | $\frac{3x}{x} \to 3$ |
| Trig identities | $\sin^2 + \cos^2 \to 1$, $\frac{\sin}{\cos} \to \tan$ |
| Exact trig values | $\sin\!\left(\frac{\pi}{6}\right) \to \frac{1}{2}$, $\sin\!\left(\frac{\pi}{12}\right) \to \frac{\sqrt{6}-\sqrt{2}}{4}$ |
| Symbolic trig | $\sin(2)$ stays symbolic — no closed form exists |
| Inverse trig values | $\arcsin\!\left(\frac{\sqrt{2}}{2}\right) \to \frac{\pi}{4}$ |
| Hyperbolic values | $\sinh(\ln 2) \to \frac{3}{4}$, $\text{arccosh}(3) \to \ln(2+\sqrt{3})$ |
| Log rules | $\ln(ab) \to \ln a + \ln b$, $\ln(12) \to 2\ln 2 + \ln 3$ |
| Exp/log folding | $\exp(2\ln 3) \to 9$, $\exp(\ln x) \to x$ |
| Partial fractions | Full decomposition via factoring |
| Common denominator | $\frac{1}{x} + \frac{1}{x+1} \to \frac{2x+1}{x(x+1)}$ |
| Assumption-aware | $\sqrt{x^2} \to x$ when $x > 0$ |
| Repeating decimals | $0.\overline{3} \to \frac{1}{3}$ |
| Factorial & binomial | $n!$, $\binom{n}{k}$ with exact evaluation |
| GCD / LCM | $\gcd(24, 36) \to 12$ |

### Calculus

| Feature | Example |
|---------|---------|
| Differentiation (chain rule) | $\frac{d}{dx}\sin(x^2) \to 2x\cos(x^2)$ |
| All 24 trig/hyperbolic functions | sin, cos, ..., arccoth — derivatives and integrals |
| 8 integration methods | polynomial, parts, u-sub, trig, partial fractions, ... |
| Risch algorithm | proves non-elementarity with certificate |
| Multi-extension towers | $\int(\!e^x\ln x + \frac{e^x}{x})\,dx = e^x\ln x$ |
| Parametric integration | $\int\frac{dx}{x^2+a} = \frac{1}{\sqrt{a}}\arctan\!\frac{x}{\sqrt{a}}$ |
| Exact definite integrals | $\int_0^1\frac{dx}{x^2+1} = \frac{\pi}{4}$ |
| Taylor series | exact rational coefficients, symbolic center |
| Limits | L'Hôpital, series expansion, one-sided, at infinity |

### Equation Solving

| Feature | Example |
|---------|---------|
| Degree 1–4 exact | Cardano, Ferrari |
| Exact radical roots | $x^2-2=0 \to x = \pm\sqrt{2}$ |
| Parametric | $ax^2+bx+c=0 \to \frac{-b \pm \sqrt{b^2-4ac}}{2a}$ |
| Systems | exact Gaussian elimination, polynomial substitution |
| Inequalities | $x^2-4 > 0 \to (-\infty,-2) \cup (2,\infty)$ |
| Rational equations | $\frac{1}{x} = 2 \to x = \frac{1}{2}$ |

### Summation & Products

| Feature | Example |
|---------|---------|
| Faulhaber's formulas | $\sum k^2 = \frac{n(n+1)(2n+1)}{6}$ |
| Geometric series | $\sum r^k = \frac{r^{n+1}-1}{r-1}$ |
| Telescoping | $\sum\!\left(\frac{1}{k} - \frac{1}{k+1}\right) = \frac{n}{n+1}$ |
| Product notation | $\prod_{k=1}^{5} k = 120$ |
| Symbolic products | $\prod_{k=1}^{n} c = c^n$ |

### ODEs & Series

| Feature | Example |
|---------|---------|
| Separable | $\frac{dy}{dx} = g(x)\cdot h(y)$ |
| First-order linear | integrating factor |
| Second-order constant-coefficient | $ay'' + by' + cy = 0$ |
| Power series solutions | Hermite, Legendre, arbitrary order |
| Formal power series | lazy eval, composition, Lagrange inversion |

### Linear Algebra

| Feature | Example |
|---------|---------|
| Determinant, inverse | exact over $\mathbb{Q}$ |
| Eigenvalues | symbolic ($2\times 2$, $3\times 3$), numerical (up to $4\times 4$) |
| Systems | $Ax = b$, RREF |
| Algebraic number fields | exact arithmetic in $\mathbb{Q}(\alpha)$ |

### Verification

| Feature | Description |
|---------|-------------|
| Reasoning-chain verification | `verify_chain`: per-step relations (`equals`, `derivative_of`, `integral_of`, `substitution`, `implies`, `solution_of`, `factored_form_of`), per-step verdict + mechanism, chain status = minimum evidence across steps |
| Numeric cross-check | 12 test points, assumption-aware, counterexample on FAIL |
| Exact refutation | inside the polynomial/rational fragment, disagreements are established in exact rational arithmetic — no floating-point tolerance; `x = x + 10^{-15}` is refuted, not tolerated |
| Expression equivalence | simplify-and-compare, then assumption-aware sampling |
| Non-elementarity proofs | Risch algorithm certificates |
| Result status | evidence taxonomy on every response — `exact` / `verified` / `heuristic` / `unable_to_compute` / `provably_impossible` |
| Machine-readable verdicts | `pass` / `fail` / `inconclusive` on `verify`, `equivalent`, `verify_chain` — outcomes are fields, not prose |
| Self-checking | transcendental simplifications and integration round-trips are independently verified before being blessed |

---

## MCP Server

The `arithma-mcp` binary speaks [MCP](https://modelcontextprotocol.io) over
stdio. 17 tools with LaTeX I/O:

| Tool | Purpose |
|------|---------|
| `format` | Parse and normalize LaTeX without simplifying |
| `simplify` | Reduce an expression to canonical form |
| `differentiate` | Symbolic derivative |
| `integrate` | Indefinite/definite; proves non-elementary when applicable |
| `substitute` | Replace a variable with an expression |
| `solve` | Equations or inequalities |
| `solve_system` | Systems of linear/polynomial equations |
| `factor` | Irreducible factoring over $\mathbb{Q}$ |
| `partial_fractions` | Decompose $P(x)/Q(x)$ |
| `limit` | Symbolic limits |
| `taylor_series` | Series expansion with exact coefficients |
| `evaluate` | Numerical evaluation |
| `matrix` | Determinant, inverse, eigenvalues, rank, RREF, $Ax=b$ |
| `equivalent` | Check if two expressions are equal |
| `verify` | Numerically cross-check at multiple test points |
| `verify_chain` | Verify a multi-step derivation, step by step, with per-step verdicts and evidence |
| `solve_ode` | First-order, constant-coeff, and power series |

`verify_chain` takes an ordered list of steps, each declaring its relation
to the previous one:

```json
{
  "steps": [
    { "label": "f",  "expr": "x^3 - x" },
    { "label": "factored", "expr": "x(x-1)(x+1)", "relation": "factored_form_of" },
    { "label": "f'", "expr": "3x^2 - 1", "relation": "derivative_of", "variable": "x" }
  ]
}
```

Each step reports `verdict` (`pass`/`fail`/`inconclusive`), the `mechanism`
that actually ran (`canonical_form_Q`, `differentiation_roundtrip`, …), and
its evidence class. A wrong step comes back with the counterexample that
refutes it. For incremental use, send a two-step chain of the previous and
new step.

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
cargo test --workspace                    # run all 1688 tests
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
