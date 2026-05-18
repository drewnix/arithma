# Arithma CAS MCP

A computer algebra system written in Rust. Exact arithmetic, not floating-point
approximation. LaTeX in, LaTeX out. A single binary with no runtime dependencies.

Arithma exists because AI agents deserve a mathematics tool that is *correct*. 
Not approximately correct, not usually correct, but correct in the way that exact
rational arithmetic and well-chosen algorithms make possible. The MCP server gives
any Claude session (or any MCP-compatible client) access to symbolic mathematics
that would otherwise require a Python runtime, a Wolfram kernel, or faith in
floating-point.

## What it does

**Algebra.** Exact rational arithmetic. Polynomial factoring over Q via the
Berlekamp-Zassenhaus algorithm (modular factoring, Hensel lifting, factor
recombination). Multivariate polynomial GCD. Simplification with a verified
idempotency contract: `simplify(simplify(e)) = simplify(e)`. Trigonometric
identities, logarithmic properties, power rules. Partial fraction decomposition.
Expression equivalence checking with symbolic and numerical verification.
Assumption system: declare variable constraints (positive, nonnegative, negative,
nonzero, real, integer) to enable domain-aware simplification ---
`sqrt(x^2) -> x` when x > 0, `|x| -> x` when x >= 0, `(-1)^(2n) -> 1`
when n is integer.

**Calculus.** Differentiation with chain rule, product rule, quotient rule, and the
general f^g formula. Integration via polynomial rules, transcendental table,
integration by parts (tabular method), u-substitution, trig powers (all parities
including mixed products), inverse trig antiderivatives, partial fractions, and
trig substitution for square roots of quadratics. Taylor and Maclaurin series
with exact rational coefficients. Symbolic limits through direct substitution,
GCD cancellation, and L'Hopital's rule.

**Equation solving.** Linear through quartic, exactly. Rational root theorem with
synthetic division. Cardano's formula for cubics (including the trigonometric method
for *casus irreducibilis*). Ferrari's method for quartics. Degree 5 and above via
Berlekamp-Zassenhaus factoring into solvable pieces.

**Linear algebra.** Determinant, inverse, eigenvalues, rank, transpose, multiplication,
linear system solving (Ax = b), and row echelon form.

**Series.** Taylor and Maclaurin expansion via repeated symbolic differentiation.
Coefficients are exact rationals, not floating-point approximations:
`sin(x)` to order 5 gives `x - \frac{1}{6}x^3 + \frac{1}{120}x^5`, not
`x - 0.16667x^3 + 0.00833x^5`.

## The MCP server

The `arithma-mcp` binary speaks MCP (Model Context Protocol) over stdio. It gives
Claude or any MCP-compatible AI agent access to 13 tools:

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

Every tool accepts LaTeX and returns LaTeX. No intermediate formats, no ambiguity.
Nine tools accept an optional `assumptions` parameter for domain-aware simplification:

```json
{
  "expr": "\\sqrt{x^2}",
  "assumptions": {"x": ["positive"]}
}
```

Valid assumptions: `positive`, `nonnegative`, `negative`, `nonzero`, `real`, `integer`.

### Adding to Claude Code

In your project's `.claude/settings.json` (or `~/.claude/settings.json` for global):

```json
{
  "mcpServers": {
    "arithma": {
      "command": "/path/to/arithma-mcp"
    }
  }
}
```

Replace `/path/to/arithma-mcp` with the path to the built binary (e.g.,
`/Users/you/arithma/target/release/arithma-mcp`).

### Adding to Claude Desktop

In your Claude Desktop configuration file:

- **macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Windows:** `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "arithma": {
      "command": "/path/to/arithma-mcp"
    }
  }
}
```

Restart Claude Desktop after editing. The 13 tools will appear in your tool list.

### Building the MCP server

```
cargo build --release --bin arithma-mcp
```

The binary lands at `target/release/arithma-mcp`. It is approximately 1.5 MB,
statically linked, with no runtime dependencies: no Python, no Java, no
Wolfram, no network calls.

## Building and testing

```
cargo build            # debug build
cargo build --release  # optimized build
cargo test             # run all 589 tests
```

## Design principles

**Correct first.** Every algorithm is verified against known results. The test
suite uses exact arithmetic, not floating-point approximation, wherever possible.
The simplifier has a verified idempotency contract.

**Well-chosen algorithms.** Not the first algorithm that works but the right
algorithm for the data structure. Berlekamp-Zassenhaus for polynomial factoring.
Subresultant remainder sequence for GCD. Horner evaluation. Cardano and Ferrari
for cubics and quartics. The choice matters.

**Silence over lies.** If arithma cannot compute something, it says so. It never
guesses, approximates heuristically, or returns an unverified result.

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

Expressions are represented as trees of `Node` variants. `ExactNum` wraps
`BigRational` for exact arithmetic, falling back to `f64` only for transcendental
constants (e, pi) and transcendental function results. The parser reads LaTeX;
the display implementation writes LaTeX. Round-trip stability is tested.

## License

MIT. See [LICENSE](LICENSE).
