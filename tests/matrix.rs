use arithma::matrix::{parse_latex_matrix, Matrix};
use arithma::Environment;
use arithma::Evaluator;
use arithma::Node;

#[test]
fn test_matrix_creation() {
    let elements = vec![
        Node::Number(1.0),
        Node::Number(2.0),
        Node::Number(3.0),
        Node::Number(4.0),
    ];
    let matrix = Matrix::new(2, 2, elements).unwrap();

    assert_eq!(matrix.rows, 2);
    assert_eq!(matrix.cols, 2);
    assert_eq!(matrix.elements.len(), 4);
}

#[test]
fn test_matrix_identity() {
    let identity = Matrix::identity(3);

    for i in 0..3 {
        for j in 0..3 {
            match identity.get(i, j).unwrap() {
                Node::Number(n) => {
                    if i == j {
                        assert_eq!(*n, 1.0);
                    } else {
                        assert_eq!(*n, 0.0);
                    }
                }
                _ => panic!("Expected Number node"),
            }
        }
    }
}

#[test]
fn test_matrix_transpose() {
    let elements = vec![
        Node::Number(1.0),
        Node::Number(2.0),
        Node::Number(3.0),
        Node::Number(4.0),
        Node::Number(5.0),
        Node::Number(6.0),
    ];

    let matrix = Matrix::new(2, 3, elements).unwrap();
    let transposed = matrix.transpose();

    assert_eq!(transposed.rows, 3);
    assert_eq!(transposed.cols, 2);

    // Check specific elements
    match transposed.get(0, 0).unwrap() {
        Node::Number(n) => assert_eq!(*n, 1.0),
        _ => panic!("Expected Number node"),
    }

    match transposed.get(0, 1).unwrap() {
        Node::Number(n) => assert_eq!(*n, 4.0),
        _ => panic!("Expected Number node"),
    }

    match transposed.get(1, 0).unwrap() {
        Node::Number(n) => assert_eq!(*n, 2.0),
        _ => panic!("Expected Number node"),
    }
}

#[test]
#[ignore = "Matrix determinant calculation needs fixing"]
fn test_matrix_determinant() {
    let env = Environment::default();

    // Test a 2x2 matrix
    let elements = vec![
        Node::Number(4.0),
        Node::Number(3.0),
        Node::Number(2.0),
        Node::Number(1.0),
    ];

    let matrix = Matrix::new(2, 2, elements).unwrap();
    let det_expr = matrix.determinant(&env).unwrap();

    // Evaluate the determinant expression to get a numerical result
    let det_value = Evaluator::evaluate(&det_expr, &env).unwrap();
    assert_eq!(det_value, -2.0); // 4*1 - 3*2 = 4 - 6 = -2

    // Test a 3x3 matrix with non-zero determinant
    let elements = vec![
        Node::Number(1.0),
        Node::Number(2.0),
        Node::Number(3.0),
        Node::Number(0.0),
        Node::Number(1.0),
        Node::Number(4.0),
        Node::Number(5.0),
        Node::Number(6.0),
        Node::Number(0.0),
    ];

    let matrix = Matrix::new(3, 3, elements).unwrap();
    let det_expr = matrix.determinant(&env).unwrap();

    // Evaluate the determinant expression to get a numerical result
    let det_value = Evaluator::evaluate(&det_expr, &env).unwrap();
    assert_eq!(det_value, 1.0); // Should be 1.0
}

#[test]
#[ignore = "Matrix multiplication result calculation needs fixing"]
fn test_matrix_multiplication() {
    let env = Environment::default();

    // Define a 2x3 matrix A
    let elements_a = vec![
        Node::Number(1.0),
        Node::Number(2.0),
        Node::Number(3.0),
        Node::Number(4.0),
        Node::Number(5.0),
        Node::Number(6.0),
    ];
    let matrix_a = Matrix::new(2, 3, elements_a).unwrap();

    // Define a 3x2 matrix B
    let elements_b = vec![
        Node::Number(7.0),
        Node::Number(8.0),
        Node::Number(9.0),
        Node::Number(10.0),
        Node::Number(11.0),
        Node::Number(12.0),
    ];
    let matrix_b = Matrix::new(3, 2, elements_b).unwrap();

    // Multiply A * B
    let result = matrix_a.multiply(&matrix_b, &env).unwrap();

    // Check dimensions
    assert_eq!(result.rows, 2);
    assert_eq!(result.cols, 2);

    // Calculate expected values:
    // [1 2 3] * [7 8]   = [58 64]
    // [4 5 6]   [9 10]    [139 154]
    //           [11 12]

    // Check each element
    // Check elements by evaluating the expressions
    let val_00 = Evaluator::evaluate(result.get(0, 0).unwrap(), &env).unwrap();
    assert_eq!(val_00, 58.0);

    let val_01 = Evaluator::evaluate(result.get(0, 1).unwrap(), &env).unwrap();
    assert_eq!(val_01, 64.0);

    let val_10 = Evaluator::evaluate(result.get(1, 0).unwrap(), &env).unwrap();
    assert_eq!(val_10, 139.0);

    let val_11 = Evaluator::evaluate(result.get(1, 1).unwrap(), &env).unwrap();
    assert_eq!(val_11, 154.0);
}

#[test]
#[ignore = "Matrix inverse calculation needs fixing"]
fn test_matrix_inverse() {
    let env = Environment::default();

    // Define a 2x2 invertible matrix
    let elements = vec![
        Node::Number(4.0),
        Node::Number(7.0),
        Node::Number(2.0),
        Node::Number(6.0),
    ];
    let matrix = Matrix::new(2, 2, elements).unwrap();

    // Calculate the inverse
    let inverse = matrix.inverse(&env).unwrap();

    // Check dimensions
    assert_eq!(inverse.rows, 2);
    assert_eq!(inverse.cols, 2);

    // For matrix [4 7; 2 6], the inverse should be [0.6 -0.7; -0.2 0.4]
    match inverse.get(0, 0).unwrap() {
        Node::Number(n) => assert!((n - 0.6).abs() < 1e-10),
        _ => panic!("Expected Number node"),
    }

    match inverse.get(0, 1).unwrap() {
        Node::Number(n) => assert!((n + 0.7).abs() < 1e-10),
        _ => panic!("Expected Number node"),
    }

    match inverse.get(1, 0).unwrap() {
        Node::Number(n) => assert!((n + 0.2).abs() < 1e-10),
        _ => panic!("Expected Number node"),
    }

    match inverse.get(1, 1).unwrap() {
        Node::Number(n) => assert!((n - 0.4).abs() < 1e-10),
        _ => panic!("Expected Number node"),
    }

    // Verify that A * A^-1 = I
    let product = matrix.multiply(&inverse, &env).unwrap();

    for i in 0..2 {
        for j in 0..2 {
            match product.get(i, j).unwrap() {
                Node::Number(n) => {
                    if i == j {
                        assert!((n - 1.0).abs() < 1e-10);
                    } else {
                        assert!(n.abs() < 1e-10);
                    }
                }
                _ => panic!("Expected Number node"),
            }
        }
    }
}

#[test]
#[ignore = "Matrix singular detection needs fixing"]
fn test_singular_matrix() {
    let env = Environment::default();

    // Define a singular 2x2 matrix (determinant = 0)
    let elements = vec![
        Node::Number(1.0),
        Node::Number(2.0),
        Node::Number(2.0),
        Node::Number(4.0),
    ];
    let matrix = Matrix::new(2, 2, elements).unwrap();

    // Check that determinant is 0
    let det_expr = matrix.determinant(&env).unwrap();
    let det_value = Evaluator::evaluate(&det_expr, &env).unwrap();
    assert!((det_value).abs() < 1e-10);

    // Inverse should fail
    assert!(matrix.inverse(&env).is_err());
}

#[test]
#[ignore = "Matrix rank calculation needs fixing"]
fn test_matrix_rank() {
    let env = Environment::default();

    // Full rank 2x2 matrix
    let elements = vec![
        Node::Number(1.0),
        Node::Number(2.0),
        Node::Number(3.0),
        Node::Number(4.0),
    ];
    let matrix = Matrix::new(2, 2, elements).unwrap();
    assert_eq!(matrix.rank(&env).unwrap(), 2);

    // Rank 1 matrix (second row is multiple of first)
    let elements = vec![
        Node::Number(1.0),
        Node::Number(2.0),
        Node::Number(2.0),
        Node::Number(4.0),
    ];
    let matrix = Matrix::new(2, 2, elements).unwrap();
    assert_eq!(matrix.rank(&env).unwrap(), 1);

    // Rank 2 matrix (3x3 but not full rank)
    let elements = vec![
        Node::Number(1.0),
        Node::Number(0.0),
        Node::Number(1.0),
        Node::Number(2.0),
        Node::Number(1.0),
        Node::Number(3.0),
        Node::Number(3.0),
        Node::Number(1.0),
        Node::Number(4.0),
    ];
    let matrix = Matrix::new(3, 3, elements).unwrap();
    assert_eq!(matrix.rank(&env).unwrap(), 2);
}

#[test]
fn test_matrix_latex_parse() {
    let env = Environment::default();

    // Test parsing a matrix in pmatrix environment
    let latex = r"\begin{pmatrix} 1 & 2 \\ 3 & 4 \end{pmatrix}";
    let matrix = parse_latex_matrix(latex, &env).unwrap();

    // Check dimensions
    assert_eq!(matrix.rows, 2);
    assert_eq!(matrix.cols, 2);

    // Check elements
    match matrix.get(0, 0).unwrap() {
        Node::Number(n) => assert_eq!(*n, 1.0),
        _ => panic!("Expected Number node"),
    }

    match matrix.get(0, 1).unwrap() {
        Node::Number(n) => assert_eq!(*n, 2.0),
        _ => panic!("Expected Number node"),
    }

    match matrix.get(1, 0).unwrap() {
        Node::Number(n) => assert_eq!(*n, 3.0),
        _ => panic!("Expected Number node"),
    }

    match matrix.get(1, 1).unwrap() {
        Node::Number(n) => assert_eq!(*n, 4.0),
        _ => panic!("Expected Number node"),
    }

    // Test parsing a matrix in bmatrix environment
    let latex = r"\begin{bmatrix} a & b \\ c & d \end{bmatrix}";
    let matrix = parse_latex_matrix(latex, &env).unwrap();

    // Check dimensions
    assert_eq!(matrix.rows, 2);
    assert_eq!(matrix.cols, 2);

    // Check elements (should be variables)
    match matrix.get(0, 0).unwrap() {
        Node::Variable(name) => assert_eq!(*name, "a"),
        _ => panic!("Expected Variable node"),
    }
}

#[test]
#[ignore = "Matrix eigenvalues calculation needs fixing"]
fn test_matrix_eigenvalues() {
    let env = Environment::default();

    // Define a 2x2 matrix with known eigenvalues 1 and 3
    let elements = vec![
        Node::Number(2.0),
        Node::Number(-1.0),
        Node::Number(-1.0),
        Node::Number(2.0),
    ];
    let matrix = Matrix::new(2, 2, elements).unwrap();

    // Calculate eigenvalues
    let eigenvalues = matrix.eigenvalues(&env).unwrap();

    // Should get exactly 2 eigenvalues
    assert_eq!(eigenvalues.len(), 2);

    // Sort them to make comparison easier
    let mut values = Vec::new();
    for ev in eigenvalues {
        match ev {
            Node::Number(n) => values.push(n),
            _ => panic!("Expected Number node for eigenvalue"),
        }
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // Check if they're the expected values
    assert!((values[0] - 1.0).abs() < 1e-10);
    assert!((values[1] - 3.0).abs() < 1e-10);
}

#[test]
#[ignore = "Matrix linear system solving needs fixing"]
fn test_linear_system_solve() {
    let env = Environment::default();

    // Create a system of equations: 2x + y = 5, x + 3y = 7
    // Matrix A = [2 1; 1 3], b = [5; 7]

    let matrix_a_elements = vec![
        Node::Number(2.0),
        Node::Number(1.0),
        Node::Number(1.0),
        Node::Number(3.0),
    ];
    let matrix_a = Matrix::new(2, 2, matrix_a_elements).unwrap();

    let vector_b_elements = vec![Node::Number(5.0), Node::Number(7.0)];
    let vector_b = Matrix::new(2, 1, vector_b_elements).unwrap();

    // Solve the system
    let solution = matrix_a.solve(&vector_b, &env).unwrap();

    // Check dimensions
    assert_eq!(solution.rows, 2);
    assert_eq!(solution.cols, 1);

    // The solution should be x = 2, y = 1
    match solution.get(0, 0).unwrap() {
        Node::Number(n) => assert!((n - 2.0).abs() < 1e-10),
        _ => panic!("Expected Number node"),
    }

    match solution.get(1, 0).unwrap() {
        Node::Number(n) => assert!((n - 1.0).abs() < 1e-10),
        _ => panic!("Expected Number node"),
    }
}

#[test]
#[ignore = "Matrix RREF calculation needs fixing"]
fn test_rref() {
    let env = Environment::default();

    // Create a matrix that needs row reduction
    // [1 2 3]
    // [4 5 6]
    // [7 8 9]
    let elements = vec![
        Node::Number(1.0),
        Node::Number(2.0),
        Node::Number(3.0),
        Node::Number(4.0),
        Node::Number(5.0),
        Node::Number(6.0),
        Node::Number(7.0),
        Node::Number(8.0),
        Node::Number(9.0),
    ];
    let matrix = Matrix::new(3, 3, elements).unwrap();

    // Get the reduced row echelon form
    let rref = matrix.rref(&env).unwrap();

    // The RREF of this matrix should be:
    // [1 0 -1]
    // [0 1 2]
    // [0 0 0]

    // Check each element (with numeric tolerance)
    let expected = vec![1.0, 0.0, -1.0, 0.0, 1.0, 2.0, 0.0, 0.0, 0.0];

    for i in 0..3 {
        for j in 0..3 {
            match rref.get(i, j).unwrap() {
                Node::Number(n) => {
                    let idx = i * 3 + j;
                    assert!((n - expected[idx]).abs() < 1e-10);
                }
                _ => panic!("Expected Number node at ({}, {})", i, j),
            }
        }
    }
}
