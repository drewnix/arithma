mod utils;

use utils::test_helpers::eval_equation;

#[test]
fn test_division_by_zero() {
    let result = std::panic::catch_unwind(|| {
        eval_equation("x / 0 = 5", 4.0);
    });

    // Check that division by zero caused a panic
    assert!(result.is_err(), "Expected division by zero to cause a panic");
}

#[test]
fn test_negative_numbers() {
    eval_equation("x + (-2) = 6", 8.0);
}
