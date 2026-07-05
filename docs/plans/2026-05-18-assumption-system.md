# Phase 8: Assumption System

*2026-05-18*

## Summary

Add an assumption system that lets agents declare variable properties (positive, nonnegative, negative, nonzero, real, integer) to enable simplifications that are only valid under those constraints. Conservative default: no assumptions = maximum generality. All existing behavior preserved.

## Data Model

New file: `src/assumptions.rs`

```rust
pub enum Assumption {
    Positive,     // x > 0
    NonNegative,  // x >= 0
    Negative,     // x < 0
    NonZero,      // x != 0
    Real,         // x in R
    Integer,      // x in Z
}

pub struct Assumptions {
    props: HashMap<String, HashSet<Assumption>>,
}
```

Implication rules in query methods: Positive implies NonNegative and NonZero. Negative implies NonZero.

## Environment Integration

`Environment` gains `assumptions: Assumptions` field. `Environment::new()` produces empty assumptions. New `Environment::with_assumptions()` constructor.

## Simplification Rules (Session 1)

1. `sqrt(x^2) -> x` when x positive (currently `|x|`)
2. `|x| -> x` when x positive/nonneg; `|x| -> -x` when x negative
3. `(-1)^(2n) -> 1` when n integer

## MCP Interface

All tools gain optional `assumptions` parameter:

```json
{
  "expr": "\\sqrt{x^2}",
  "assumptions": { "x": ["positive"] }
}
```

## Implementation Steps

1. Create `src/assumptions.rs` — Assumption enum, Assumptions struct, JSON parsing
2. Add `assumptions` field to Environment
3. Thread assumptions into simplification rules (sqrt, abs, power)
4. Add assumptions to MCP tool handlers (all 13 tools)
5. Tests for each rule, including no-assumption defaults
