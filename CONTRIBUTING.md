# Contributing to Arithma

Thanks for your interest in contributing. This document covers the conventions
that keep the codebase consistent.

## Before you start

Read `ARCHITECTURE.md` for the project's scope and design principles. The short
version:

- **Correctness beats coverage.** A wrong answer is worse than "I can't compute
  this." Never introduce a heuristic that can silently produce incorrect results.
- **Exact before approximate.** Use `BigRational` arithmetic. Float is a last
  resort for numerical evaluation.
- **Algorithmic over heuristic.** Prefer a well-chosen algorithm over a
  collection of pattern-matched special cases.

If you're unsure whether a feature fits the project's scope, open a Discussion
before writing code.

## Development workflow

### Building and testing

```bash
cargo build                                  # debug build
cargo fmt                                    # format code
cargo clippy --tests -- -D warnings          # lint (zero warnings policy)
cargo test                                   # run full test suite
```

**Run all three checks before every commit.** CI enforces them — a PR that fails
fmt, clippy, or tests will not be merged. A pre-commit hook is available in
`.git/hooks/pre-commit`.

### Pull requests

- **One concern per PR.** A bug fix and a new feature should be separate PRs.
- **Reference the issue.** Use `Fixes #N` or `Closes #N` in the PR body.
- **Small PRs merge faster.** If a feature can be split into independent pieces
  (e.g., parser change + simplifier change), consider separate PRs.
- **Design discussions before large PRs.** If a change touches the AST, parser
  architecture, or simplifier rule ordering, open a Discussion first. Small,
  self-contained changes (adding an operator, exposing an existing internal
  function) can go straight to PR.

### Commit messages

Follow conventional commits: `feat:`, `fix:`, `docs:`, `test:`, `refactor:`.
Keep the first line under 72 characters. A body is welcome for non-obvious
changes.

### Tests

- Every new feature needs tests. Every bug fix needs a regression test.
- **Integration tests** go in `tests/`. **Unit tests** go in `mod tests` inside
  the source file.
- For integration work: differentiate the result and check it against the
  integrand. This round-trip is the strongest correctness check we have.
- When modifying existing tests, keep the original test case and add new ones
  rather than replacing.

## Code conventions

- **No comments by default.** Add one only when the *why* is non-obvious.
- **No `#[allow(clippy::...)]`** without a comment explaining why the lint
  doesn't apply.
- **Variable normalization:** call `normalize_var()` at API boundaries. Greek
  letter names (`alpha`, `beta`) become Unicode (`α`, `β`) internally.
- **Error handling:** return `Result<T, String>` with a message that helps the
  caller understand what went wrong. Don't panic in library code.
- **Formatting:** `cargo fmt` is authoritative. Don't fight it.

## Project structure

```
src/
├── lib.rs              # public API surface
├── node.rs             # AST (Node enum)
├── exact.rs            # exact number type (BigRational / Float)
├── tokenizer.rs        # LaTeX tokenizer
├── parser.rs           # token stream → AST
├── simplify.rs         # simplification rules
├── evaluator.rs        # numeric evaluation
├── polynomial.rs       # dense univariate polynomials over Q
├── limits.rs           # limit computation
├── integration.rs      # symbolic integration
├── derivative.rs       # symbolic differentiation
├── series.rs           # Taylor series
├── fps.rs              # formal power series
├── ode.rs              # ODE solving
├── integer.rs          # integer number theory (prime factorization)
├── main.rs             # CLI binary
└── bin/arithma-mcp.rs  # MCP server binary
```

## What we don't build

Some things are explicitly out of scope. See ARCHITECTURE.md for the full list,
but commonly requested items that don't fit:

- RUBI-style pattern tables (we use algorithms, not rule collections)
- Physics / statistics / geometry modules (application layers)
- Hundreds of special functions (agents can look those up)
- Visualization (different tool)
