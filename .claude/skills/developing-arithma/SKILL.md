---
name: developing-arithma
description: Use when building, testing, releasing, or contributing to the arithma codebase — adding features, fixing bugs, running CI checks, bumping versions, creating releases, or onboarding to the project structure. Also use when encountering build failures, stale binaries, or CI lint errors.
---

# Developing Arithma

## Project Structure

Cargo workspace with three crates:

| Crate | Path | Binary | Role |
|-------|------|--------|------|
| `arithma` | `.` (root) | — | Core library (lib + cdylib for WASM) |
| `arithma-cli` | `crates/cli/` | `arithma` | Interactive REPL and command-line tool |
| `arithma-mcp-server` | `crates/mcp/` | `arithma-mcp` | MCP server (JSON-RPC over stdio) |

Version is defined once in `[workspace.package]` in the root `Cargo.toml` and inherited by all crates via `version.workspace = true`.

## Build & Install

```bash
make build          # cargo build --release --workspace
make install        # build + copy binaries to ~/.local/bin/
make install PREFIX=/usr/local   # custom install location
make mcp            # build only the MCP server
```

### The Workspace Build Trap

`cargo build` at the repo root builds **only the library** — not the CLI or MCP binaries. Always use `cargo build --workspace` or `make build`. If a binary seems stale after changes, this is almost certainly why.

## Pre-Commit Discipline

Run before every commit — CI enforces all three:

```bash
cargo fmt -- --check
RUSTFLAGS="--allow=unexpected_cfgs" cargo clippy --workspace -- -D warnings
cargo test --all
```

Or: `make check` (runs all three in sequence).

Do not accumulate warnings. Clippy with `-D warnings` means warnings are errors.

## Testing

```bash
cargo test --all              # run everything (lib + cli + mcp)
make test                     # same thing
cargo test -p arithma         # lib tests only
cargo test -p arithma-cli     # CLI integration tests only
```

**Reliable test surfaces:** CLI and stdio (piped MCP commands) are always available. The MCP server connection is sometimes absent in development sessions — don't rely on it for verification.

**X1 sentinel:** When verifying a fix, choose a test input that can only produce the correct output through the fixed code path. If the old (broken) code could accidentally pass, the test proves nothing.

## Release Process

Releases are tagged on `main`. The flow:

1. Merge all feature branches to `main`
2. Run `make release V=x.y.z` — this:
   - Checks the working tree is clean
   - Updates `[workspace.package] version` in root `Cargo.toml`
   - Runs `cargo check` to update `Cargo.lock`
   - Commits: `release: vx.y.z`
   - Tags: `vx.y.z`
3. Push: `git push origin main --tags`
4. The `v*` tag triggers `.github/workflows/release.yml`, which builds cross-platform binaries (Linux x86_64, macOS aarch64, Windows x86_64) for both `arithma` and `arithma-mcp` and creates a GitHub release with artifacts.

**Version scheme:** semver. Pre-1.0, bump minor for capability additions, patch for fixes.

## Makefile Targets

```
make help       # list all targets
make build      # release build (full workspace)
make install    # build + install to ~/.local/bin
make release    # tag a release (V=x.y.z required)
make test       # cargo test --all
make check      # fmt + clippy + test
make fmt        # check formatting
make clippy     # lint
make wasm       # build WASM module + copy to frontend
make mcp        # build MCP server only
make clean      # cargo clean
```

## Standing Rules

- **No internal references in open-source code.** No team member names, session numbers, or organizational processes in source, comments, tests, or commit messages.
- **No uncertified exact.** The `StatusReport::exact()` constructor requires a `Certificate` — compiler-enforced. If you add a new exact path, you must provide a certificate.
- **No false impossibility.** `provably_impossible` requires a proof the tool can actually verify. If you can't prove it (e.g., Galois group computation), use `unable_to_compute`.
