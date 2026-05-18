# Berlekamp-Zassenhaus Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Factor polynomials over Q into irreducible factors using the Berlekamp-Zassenhaus algorithm.

**Architecture:** Four-layer pipeline: ModPoly arithmetic (Z_p[x]) → Berlekamp factoring (mod p) → Hensel lifting (mod p^k) → factor recombination (Z[x]). Layers 1-2 this session, layers 3-4 next session.

**Tech Stack:** Rust, i64 arithmetic for mod-p coefficients (p < 2^31), existing `Polynomial` type for Q[x]/Z[x].

**Reference:** TAOCP Volume 2, Section 4.6.2.

---

### Task 1: ModPoly core type and constructors

**Files:**
- Create: `src/mod_poly.rs`
- Modify: `src/lib.rs` (add `pub mod mod_poly;`)

**Step 1: Create the type and constructors**

```rust
/// Dense polynomial over Z_p (integers mod a prime p).
/// coeffs[i] = coefficient of x^i, each in [0, p-1].
/// Invariant: trailing zeros stripped. Empty vec = zero polynomial.
pub struct ModPoly {
    coeffs: Vec<i64>,
    p: i64,
}
```

Constructors: `zero(p)`, `one(p)`, `from_coeffs(coeffs, p)` (reduces and strips),
`x_poly(p)` (the polynomial x).

**Step 2: Basic queries**

`degree()`, `leading_coeff()`, `is_zero()`, `coeff(i)`, `modulus()`.

**Step 3: Write tests for constructors**

Verify zero-stripping, coefficient reduction, degree computation.

**Step 4: Commit**

---

### Task 2: ModPoly arithmetic

**Files:**
- Modify: `src/mod_poly.rs`

**Step 1: Helper — `mod_reduce(val, p) -> i64`**

Returns val mod p in [0, p-1]. Handle negative values correctly.

**Step 2: Helper — `mod_inverse(a, p) -> i64`**

Extended Euclidean algorithm. Returns a^(-1) mod p. Panics if gcd(a,p) ≠ 1.

**Step 3: Arithmetic — add, sub, neg, scalar_mul, mul**

All results reduced mod p. Implement as free functions or methods.
Multiplication is schoolbook O(n²) — fine for our degree range.

**Step 4: make_monic — divide all coefficients by leading coefficient**

Requires mod_inverse of leading_coeff.

**Step 5: Tests**

- Arithmetic identities over Z_5 and Z_7
- mul then div_rem recovers operands
- make_monic

**Step 6: Commit**

---

### Task 3: ModPoly division, GCD, and powmod

**Files:**
- Modify: `src/mod_poly.rs`

**Step 1: div_rem — polynomial long division over Z_p**

Same algorithm as Polynomial::div_rem but using mod_inverse for leading coeff division.
Returns (quotient, remainder).

**Step 2: gcd — Euclidean algorithm, result monic**

**Step 3: powmod(base, exp, modulus) — repeated squaring**

Compute base^exp mod modulus, all mod p. Algorithm:
```
result = 1
base = base mod modulus
while exp > 0:
    if exp is odd: result = (result * base) mod modulus
    base = (base * base) mod modulus
    exp >>= 1
return result
```

This is the critical operation for Berlekamp — it computes x^(ip) mod f(x).

**Step 4: Tests**

- div_rem: verify quotient * divisor + remainder == dividend
- gcd: known examples
- powmod: x^5 mod (x^3+x+1) mod 5, verify by expanding

**Step 5: Commit**

---

### Task 4: Conversion between Polynomial and ModPoly

**Files:**
- Modify: `src/mod_poly.rs`

**Step 1: `ModPoly::from_polynomial(poly: &Polynomial, p: i64) -> ModPoly`**

Convert Polynomial (BigRational coefficients) to ModPoly by:
1. Take primitive part (integer coefficients)
2. Reduce each coefficient mod p

**Step 2: `ModPoly::to_polynomial(var: &str) -> Polynomial`**

Lift i64 coefficients to BigRational. Used after Hensel lifting.

**Step 3: Tests**

Round-trip: poly → mod_poly → poly recovers (mod p).

**Step 4: Commit**

---

### Task 5: Berlekamp's Q-matrix and null space

**Files:**
- Modify: `src/mod_poly.rs`

**Step 1: `berlekamp_matrix(f: &ModPoly) -> Vec<Vec<i64>>`**

Build the n×n matrix Q where n = deg(f).
Row i = coefficients of x^(i·p) mod f(x).
- Row 0 = [1, 0, 0, ...] (x^0 = 1)
- Compute x^p mod f via powmod, that's row 1
- Row i = (row_{i-1} * x^p) mod f — reuse previous row

**Step 2: `null_space_mod(matrix: &[Vec<i64>], n: usize, p: i64) -> Vec<Vec<i64>>`**

Compute null space of (Q - I) via Gaussian elimination over Z_p.
Return basis vectors. The first basis vector is always [1, 0, ..., 0].

**Step 3: Tests**

- For irreducible f mod p: null space has dimension 1 (just the constant)
- For f = g·h mod p: null space has dimension 2

**Step 4: Commit**

---

### Task 6: Berlekamp splitting and full factorization

**Files:**
- Modify: `src/mod_poly.rs`
- Create: `tests/factoring.rs`

**Step 1: `berlekamp_split(f: &ModPoly, basis: &[ModPoly]) -> Vec<ModPoly>`**

For each basis vector v (beyond the trivial constant):
- For c = 0, 1, ..., p-1: compute g = gcd(f, v - c)
- If g is non-trivial (1 < deg < deg(f)), it's a factor
- Recurse on the factors until all are irreducible

**Step 2: `factor_mod_p(f: &ModPoly) -> Vec<ModPoly>`**

The public entry point:
1. Make f monic and square-free (gcd with derivative)
2. Build Q matrix
3. Compute null space — if dimension is 1, f is irreducible, return [f]
4. Otherwise, split using basis vectors
5. Return sorted list of irreducible factors

**Step 3: Tests in `tests/factoring.rs`**

- x^2 - 1 mod 5 → (x-1)(x+1) = (x+4)(x+1) mod 5
- x^4 - 1 mod 5 → four linear factors
- x^3 + x + 1 mod 2 → irreducible
- x^6 - 1 mod 7 → factors
- Product verification: multiply all factors, compare to original

**Step 4: Commit**

---

### Task 7: Hensel lifting (if time permits)

**Files:**
- Modify: `src/mod_poly.rs`

**Step 1: `hensel_lift_pair(f, g, h, s, t, m, p)` — single-step linear lift**

Given:
- f ≡ g·h (mod m), where m = p^k for current k
- s·g + t·h ≡ 1 (mod m) (Bezout coefficients)

Compute g*, h*, s*, t* such that f ≡ g*·h* (mod m·p).

Algorithm (TAOCP):
1. e = f - g·h (the "error", zero mod m)
2. (q, r) = (s·e) div_rem h → both mod m·p
3. h* = h + r, g* = g + t·e + q·g
4. Update Bezout coefficients

**Step 2: `hensel_lift(f, factors, p, target_k)` — multi-factor lift**

Lift r factors from mod p to mod p^target_k using binary tree strategy:
pair factors, lift each pair, repeat.

**Step 3: Tests**

- Lift factors of x^2-1 from mod 5 to mod 25, mod 125
- Verify product matches f at each level

**Step 4: Commit**

---

### Not this session (Task 8+): Factor recombination

For next session:
- Mignotte bound computation
- Subset enumeration with degree/coefficient pruning
- Integration with `Polynomial::factor()` public API
- Integration with simplifier and solver
