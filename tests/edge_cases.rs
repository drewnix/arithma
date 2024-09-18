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


// #[test]
// fn test_negative_numbers_base() {
//     eval_equation("-x = 8", -8.0);  // Solves for x, where x = -8
// }

#[test]
fn test_combined_negative_numbers() {
    eval_equation("x + (-2) = 6", 8.0);
}

#[test]
fn test_negative_numbers_neg_result() {
    eval_equation("x + 2 = -5", -7.0);  // Example test for handling negative numbers
}

#[test]
fn test_negative_numbers_neg_result_unary() {
    eval_equation("x = -4", -4.0);  // Solves for x, where x = -4
}
