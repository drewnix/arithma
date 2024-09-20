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
// fn test_negative_numbers_neg_variable() {
//     eval_equation("-x = 8", -8.0);  // Solves for x, where x = -8
// }

// #[test]
// fn test_combined_negative_numbers() {
//     // eval_equation("x + (-2) = 6", 8.0);
//     let env = Environment::new();
//     let expr = "5 + -3";
//     let tree = build_expression_tree(tokenize(expr)).expect("Failed to build expression tree");
//     let result = Evaluator::evaluate(&tree, &env).expect("Failed to evaluate expression");
//     assert_eq!(result, 2.0);
// }

// #[test]
// fn test_negative_numbers_neg_result() {
//     let env = Environment::new();
//     let expr = "-5";
//     let tree = build_expression_tree(tokenize(expr)).expect("Failed to build expression tree");
//     let result = Evaluator::evaluate(&tree, &env).expect("Failed to evaluate expression");
//     assert_eq!(result, -5.0);
// }

// #[test]
// fn test_negative_numbers_neg_result_unary() {
//     eval_equation("x = -4", -4.0);  // Solves for x, where x = -4
// }
