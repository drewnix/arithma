mod utils;

use utils::test_helpers::eval_equation;


#[test]
#[ignore]
fn test_addition() {
    eval_equation("x + 3 = 7", 4.0);
}

#[test]
#[ignore]
fn test_subtraction() {
    eval_equation("x - 5 = 10", 15.0);
}

#[test]
#[ignore]
fn test_multiplication() {
    eval_equation("x * 4 = 20", 5.0);
}

#[test]
#[ignore]
fn test_division() {
    eval_equation("x / 2 = 10", 20.0);
}