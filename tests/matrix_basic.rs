use arithma::matrix::Matrix;
use arithma::Environment;
use arithma::Evaluator;
use arithma::Node;

#[test]
fn test_basic_matrix_determinant() {
    let env = Environment::default();

    // Create a simple 2x2 matrix
    let elements = vec![
        Node::Number(1.0),
        Node::Number(2.0),
        Node::Number(3.0),
        Node::Number(4.0),
    ];

    let matrix = Matrix::new(2, 2, elements).unwrap();

    // Calculate the determinant (returns the expression 1*4 - 2*3)
    let det_expr = matrix.determinant(&env).unwrap();

    // Now evaluate the expression to get the numerical result
    let result = Evaluator::evaluate(&det_expr, &env).unwrap();
    assert_eq!(result, -2.0);
}
