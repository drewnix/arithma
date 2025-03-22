use arithma::matrix::parse_latex_matrix;
use arithma::Environment;
use arithma::Evaluator;

#[test]
fn test_parse_matrices() {
    let env = Environment::default();

    // Test parsing a matrix in pmatrix environment
    let latex_a = r"\begin{pmatrix} 5 & 2 \\ -9 & 11 \end{pmatrix}";
    let matrix_a = parse_latex_matrix(latex_a, &env).unwrap();

    // Check dimensions
    assert_eq!(matrix_a.rows, 2);
    assert_eq!(matrix_a.cols, 2);

    // Check elements
    let a_00 = Evaluator::evaluate(matrix_a.get(0, 0).unwrap(), &env).unwrap();
    assert_eq!(a_00, 5.0);

    let a_01 = Evaluator::evaluate(matrix_a.get(0, 1).unwrap(), &env).unwrap();
    assert_eq!(a_01, 2.0);

    let a_10 = Evaluator::evaluate(matrix_a.get(1, 0).unwrap(), &env).unwrap();
    assert_eq!(a_10, -9.0);

    let a_11 = Evaluator::evaluate(matrix_a.get(1, 1).unwrap(), &env).unwrap();
    assert_eq!(a_11, 11.0);

    // Test parsing a column vector
    let latex_b = r"\begin{pmatrix} 1 \\ 2 \end{pmatrix}";
    let matrix_b = parse_latex_matrix(latex_b, &env).unwrap();

    // Check dimensions
    assert_eq!(matrix_b.rows, 2);
    assert_eq!(matrix_b.cols, 1);
}
