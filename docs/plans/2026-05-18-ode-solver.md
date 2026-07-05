# Phase 10: Basic ODE Solver

*2026-05-18*

## Summary

Three ODE classes covering ~80% of what agents encounter:
1. Separable: dy/dx = g(x)*h(y)
2. First-order linear: dy/dx + P(x)*y = Q(x)
3. Second-order constant-coefficient: ay'' + by' + cy = 0

## Interface

Structured parameters (no derivative notation parsing).

**First-order:** user provides f(x,y) where dy/dx = f(x,y). Auto-classify as separable or linear.

**Second-order:** user provides coefficients a, b, c.

## Algorithms

**Separable:** factor f into g(x)*h(y), integrate both sides: ∫dy/h(y) = ∫g(x)dx + C₁.

**Linear:** extract P(x) and Q(x) from f = Q(x) - P(x)*y. Integrating factor e^(∫P dx). Solution: y = e^(-∫P dx) * (∫Q*e^(∫P dx) dx + C₁).

**Constant-coefficient:** solve ar²+br+c=0. Three cases:
- Distinct real r₁,r₂: y = C₁e^(r₁x) + C₂e^(r₂x)
- Repeated r: y = (C₁ + C₂x)e^(rx)
- Complex a±bi: y = e^(ax)(C₁cos(bx) + C₂sin(bx))

## Implementation

1. Create `src/ode.rs` — classification + three solvers
2. Wire into `src/lib.rs`
3. Add MCP tool `solve_ode` with assumptions support
4. Add CLI subcommand `ode` with `--cc` flag
5. Tests for each class
