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

1. Merge all feature branches to `main`. **Wait for CI green on the release commit before tagging** — the tag triggers artifact builds from exactly that commit.
2. Run `make release V=x.y.z` — this:
   - Checks the working tree is clean and the tag doesn't already exist
   - If `Cargo.toml` already carries version x.y.z (e.g. a merged PR bumped it), skips the bump commit and tags HEAD directly; otherwise updates `[workspace.package] version`, runs `cargo check` to refresh `Cargo.lock`, and commits `release: vx.y.z`
   - Tags: `vx.y.z`
3. Push: `git push origin main vx.y.z` (push the tag by name — `--tags` sends every local tag, including stale ones)
4. The `v*` tag triggers `.github/workflows/release.yml`, which builds cross-platform binaries for both `arithma` and `arithma-mcp`, names each asset with a platform suffix (`arithma-linux-x86_64`, `arithma-macos-aarch64`, `arithma-mcp-windows-x86_64.exe`, …), fails loudly if any of the 6 expected binaries is missing, and publishes a GitHub release with the assets plus `SHA256SUMS.txt`.
5. **Verify:** `gh release view vx.y.z --json assets` — expect 6 platform-suffixed binaries + checksums. A tag without a corresponding release means the workflow failed silently; check `gh run list --workflow=release.yml`.

**Re-cutting a release** (bad artifacts, workflow fix): the workflow runs *from the tag's commit*, so a workflow fix only takes effect if the tag contains it. Delete the release (`gh release delete vX --yes`) and the remote tag (`git push origin :refs/tags/vX`), re-tag on a commit that includes the fix, and push the tag again.

**Version scheme:** semver. Pre-1.0, bump minor for capability additions, patch for fixes. Breaking API changes (removed `pub` items, changed signatures) also require at least a minor bump.

**Known ledger gaps:** v0.2.0's release assets predate the platform-suffix fix (four ambiguous binaries). v0.3.0 is tagged but has no release — its workflow run never produced one; superseded by v0.4.0 within two days, left as-is deliberately.

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
