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

// Test for using \cdot to multiply matrices in LaTeX
#[test]
fn test_matrix_cdot_parsing() {
    let env = Environment::default();

    // Test string with matrix multiplication using \cdot
    let latex =
        r"\begin{pmatrix} 5 & 2 \\ -9 & 11 \end{pmatrix}\cdot\begin{pmatrix} 1 \\ 2 \end{pmatrix}";

    // Split the string by \cdot
    let parts: Vec<&str> = latex.split("\\cdot").collect();
    assert_eq!(parts.len(), 2);

    // Parse the matrices
    let matrix_a = parse_latex_matrix(parts[0], &env).unwrap();
    let matrix_b = parse_latex_matrix(parts[1], &env).unwrap();

    // Check dimensions for matrix-vector multiplication compatibility
    assert_eq!(matrix_a.cols, matrix_b.rows);
}

#[test]
fn test_matrix_dimension_mismatch() {
    let env = Environment::default();

    // Define matrices with incompatible dimensions
    let latex_a = r"\begin{pmatrix} 5 & 2 \\ -9 & 11 \end{pmatrix}";
    let matrix_a = parse_latex_matrix(latex_a, &env).unwrap();

    let latex_c = r"\begin{pmatrix} 1 & 2 & 3 \\ 4 & 5 & 6 \end{pmatrix}";
    let matrix_c = parse_latex_matrix(latex_c, &env).unwrap();

    // Attempt to multiply incompatible matrices should fail
    let result = matrix_a.multiply(&matrix_c, &env);
    assert!(result.is_err());
}
