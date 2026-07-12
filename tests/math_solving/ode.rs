use arithma::ode::{solve_constant_coeff_latex, solve_ode_latex};

#[test]
fn test_separable_x_squared() {
    // dy/dx = x^2 → y = x^3/3 + C₁
    let result = solve_ode_latex("x^2", "x", "y").unwrap();
    eprintln!("dy/dx = x^2  →  {}", result);
    assert!(result.contains("C_{1}"));
}

#[test]
fn test_separable_constant() {
    // dy/dx = 5 → y = 5x + C₁
    let result = solve_ode_latex("5", "x", "y").unwrap();
    eprintln!("dy/dx = 5    →  {}", result);
    assert!(result.contains("C_{1}"));
}

#[test]
fn test_separable_pure_y() {
    // dy/dx = y → ∫dy/y = x + C₁ → ln|y| = x + C₁
    let result = solve_ode_latex("y", "x", "y").unwrap();
    eprintln!("dy/dx = y    →  {}", result);
    assert!(result.contains("C_{1}"));
}

#[test]
fn test_separable_product() {
    // dy/dx = x*y → separable: ∫dy/y = ∫x dx → ln|y| = x²/2 + C₁
    let result = solve_ode_latex("x \\cdot y", "x", "y").unwrap();
    eprintln!("dy/dx = xy   →  {}", result);
    assert!(result.contains("C_{1}"));
}

#[test]
fn test_linear_first_order() {
    // dy/dx = -2y (equivalently dy/dx + 2y = 0)
    // This is separable AND linear; separable will catch it first
    let result = solve_ode_latex("-2y", "x", "y").unwrap();
    eprintln!("dy/dx = -2y  →  {}", result);
    assert!(result.contains("C_{1}"));
}

#[test]
fn test_cc_distinct_real() {
    // y'' + 3y' + 2y = 0 → r = -1, -2
    let result = solve_constant_coeff_latex(1.0, 3.0, 2.0, "x").unwrap();
    eprintln!("y''+3y'+2y=0 →  {}", result);
    assert!(result.contains("C_{1}"));
    assert!(result.contains("C_{2}"));
}

#[test]
fn test_cc_repeated() {
    // y'' + 2y' + y = 0 → r = -1 (double)
    let result = solve_constant_coeff_latex(1.0, 2.0, 1.0, "x").unwrap();
    eprintln!("y''+2y'+y=0  →  {}", result);
    assert!(result.contains("C_{1}"));
    assert!(result.contains("C_{2}"));
}

#[test]
fn test_cc_complex_pure_imaginary() {
    // y'' + y = 0 → r = ±i → y = C₁cos(x) + C₂sin(x)
    let result = solve_constant_coeff_latex(1.0, 0.0, 1.0, "x").unwrap();
    eprintln!("y''+y=0      →  {}", result);
    assert!(result.contains("C_{1}"));
    assert!(result.contains("C_{2}"));
}

#[test]
fn test_cc_complex_damped() {
    // y'' + 2y' + 5y = 0 → r = -1 ± 2i
    let result = solve_constant_coeff_latex(1.0, 2.0, 5.0, "x").unwrap();
    eprintln!("y''+2y'+5y=0 →  {}", result);
    assert!(result.contains("C_{1}"));
    assert!(result.contains("C_{2}"));
}

#[test]
fn test_cc_different_indep_var() {
    // y'' + y = 0 with t as independent variable
    let result = solve_constant_coeff_latex(1.0, 0.0, 1.0, "t").unwrap();
    eprintln!("y''+y=0 (t)  →  {}", result);
    assert!(result.contains("t"));
}

#[test]
fn test_separable_sin_x() {
    // dy/dx = sin(x) → y = -cos(x) + C₁
    let result = solve_ode_latex("\\sin(x)", "x", "y").unwrap();
    eprintln!("dy/dx=sin(x) →  {}", result);
    assert!(result.contains("C_{1}"));
}

#[test]
fn test_cc_a_nonzero_check() {
    // a=0 should error
    let result = solve_constant_coeff_latex(0.0, 1.0, 1.0, "x");
    assert!(result.is_err());
}
