# Arithma

A computer algebra system for AI agents. Written in Rust. Exact arithmetic,
not floating-point approximation. LaTeX in, LaTeX out.

Arithma exists because AI agents deserve a mathematics tool that is *correct*.
Not approximately correct, not usually correct, but correct in the way that
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

**589 tests, zero failures.** Every algorithm is verified against known results.
The simplifier has a verified idempotency contract:
`simplify(simplify(e)) = simplify(e)`.

## What it does

**Algebra.** Polynomial factoring over Q via the Berlekamp-Zassenhaus algorithm
(modular factoring, Hensel lifting, factor recombination). Multivariate
polynomial GCD. Simplification with trigonometric identities, logarithmic
properties, and power rules. Partial fraction decomposition. Expression
equivalence checking. Assumption system for domain-aware simplification:
declare `x > 0` and `sqrt(x^2)` simplifies to `x` instead of `|x|`.

**Calculus.** Differentiation with full chain rule. Integration via 8 techniques:
polynomial rules, transcendental table, integration by parts (tabular method),
u-substitution, trig powers (all parities), inverse trig, partial fractions,
and trig substitution. Taylor/Maclaurin series with exact rational coefficients.
Symbolic limits via direct substitution, GCD cancellation, and L'Hopital's rule.

**Equation solving.** Linear through quartic, exactly (Cardano, Ferrari).
Degree 5+ via Berlekamp-Zassenhaus factoring into solvable pieces.

**Linear algebra.** Determinant, inverse, eigenvalues, rank, transpose,
multiplication, Ax = b, and RREF.

## MCP server

The `arithma-mcp` binary speaks [MCP](https://modelcontextprotocol.io) over
stdio. It gives Claude or any MCP-compatible AI agent access to 13 tools:

| Tool | Purpose |
|------|---------|
| `simplify` | Reduce an expression to canonical form |
| `differentiate` | Symbolic derivative with respect to any variable |
| `integrate` | Indefinite and definite integrals |
| `substitute` | Replace a variable with an expression |
| `solve` | Solve equations (any degree, via factoring) |
| `factor` | Irreducible factoring over Q (Berlekamp-Zassenhaus) |
| `partial_fractions` | Decompose P(x)/Q(x) into partial fractions |
| `limit` | Symbolic limits |
| `taylor_series` | Series expansion with exact coefficients |
| `evaluate` | Numerical evaluation with variable assignments |
| `matrix` | Determinant, inverse, eigenvalues, rank, RREF, Ax=b |
| `equivalent` | Check if two expressions are mathematically equal |

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

## Building

```
cargo build --release --bin arithma-mcp   # MCP server (1.6 MB binary)
cargo test                                # run all 589 tests
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
  integration.rs       -- symbolic integration (8 techniques)
  series.rs            -- Taylor/Maclaurin series
  limits.rs            -- symbolic limits
  matrix.rs            -- matrix operations
  expression.rs        -- equation solving (degree 1-4 + factoring)
  evaluator.rs         -- numerical evaluation
  bin/arithma-mcp.rs   -- MCP server (JSON-RPC 2.0 over stdio)
```

~15.5K lines of Rust. Expressions are trees of `Node` variants. `ExactNum`
wraps `BigRational` for exact arithmetic, falling back to `f64` only for
transcendental constants and function results. The parser reads LaTeX; the
display implementation writes LaTeX. Round-trip stability is tested.

## License

MIT. See [LICENSE](LICENSE).
