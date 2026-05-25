# Log-Over-Exp Rational Integration: Non-Elementarity Detection

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Detect non-elementarity for rational-in-θ₂ integrands in the log-over-exp tower, e.g., ∫1/ln(1+exp(x)) dx, via h-scaled Rothstein-Trager.

**Architecture:** For the tower Q(x) ⊂ Q(x, θ₁=exp(g)) ⊂ Q(x, θ₁, θ₂=ln(h(x,θ₁))), the derivative D(θ₂)=h'/h has rational θ₁ coefficients. Multiply through by h to get polynomial coefficients: h·D(d)_k = h·D(dₖ) + (k+1)·d_{k+1}·h'. Scale numerator by h too. The Rothstein-Trager resultant becomes res(d, h·a − z·h·D(d)) = h^m · R(z), preserving roots. Refactor RT to accept pre-computed two-level z-coefficients (Vec<ExtPoly>), subsuming the content parameter.

**Tech Stack:** Rust, existing `ExtPoly`, `DifferentialExtension`, `hermite_reduce_two_level`, `two_level_det`, `find_constant_roots_two_level`.

**Reference:** Bronstein, *Symbolic Integration I*, §5.5 (logarithmic case of Rothstein-Trager).

---

## Tasks

### Task 1: Refactor rothstein_trager_two_level to accept Vec<ExtPoly> z-coefficients

Extract the core RT computation into `rothstein_trager_two_level_general(d, a, dd_tl: &[ExtPoly], var)` where dd_tl[j] is the z-coefficient ExtPoly at θ₂-degree j. Make the existing function a wrapper.

### Task 2: Compute h-scaled log-extension derivative

Add `compute_log_ext_dd_scaled(d: &ExtPoly, h: &ExtPoly, h_prime: &ExtPoly, inner_ext: &DifferentialExtension, var) -> Vec<ExtPoly>` that computes h·D(d)_k for each θ₂-degree k.

### Task 3: Add extract_rational_log_over_exp parser

Parse expressions like 1/ln(1+exp(x)) as (numerator, denominator) in the log-over-exp tower, returning (Vec<ExtPoly>, Vec<ExtPoly>).

### Task 4: Implement integrate_rational_log_over_exp pipeline

Hermite reduce → RT with h-scaled coefficients → non-elementarity detection. Elementary results return None (future work).

### Task 5: Wire dispatch and add e2e tests

Hook into try_risch_two_level. Test ∫1/ln(1+exp(x)) dx → non-elementary, etc.
