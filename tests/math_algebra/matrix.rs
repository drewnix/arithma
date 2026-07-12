use arithma::matrix::{parse_latex_matrix, Matrix};
use arithma::Environment;
use arithma::Evaluator;
use arithma::ExactNum;
use arithma::Node;

#[test]
fn test_matrix_creation() {
    let elements = vec![
        Node::Num(ExactNum::integer(1)),
        Node::Num(ExactNum::integer(2)),
        Node::Num(ExactNum::integer(3)),
        Node::Num(ExactNum::integer(4)),
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
                Node::Num(n) => {
                    if i == j {
                        assert_eq!(n.to_f64(), 1.0);
                    } else {
                        assert_eq!(n.to_f64(), 0.0);
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
        Node::Num(ExactNum::integer(1)),
        Node::Num(ExactNum::integer(2)),
        Node::Num(ExactNum::integer(3)),
        Node::Num(ExactNum::integer(4)),
        Node::Num(ExactNum::integer(5)),
        Node::Num(ExactNum::integer(6)),
    ];

    let matrix = Matrix::new(2, 3, elements).unwrap();
    let transposed = matrix.transpose();

    assert_eq!(transposed.rows, 3);
    assert_eq!(transposed.cols, 2);

    // Check specific elements
    match transposed.get(0, 0).unwrap() {
        Node::Num(n) => assert_eq!(n.to_f64(), 1.0),
        _ => panic!("Expected Number node"),
    }

    match transposed.get(0, 1).unwrap() {
        Node::Num(n) => assert_eq!(n.to_f64(), 4.0),
        _ => panic!("Expected Number node"),
    }

    match transposed.get(1, 0).unwrap() {
        Node::Num(n) => assert_eq!(n.to_f64(), 2.0),
        _ => panic!("Expected Number node"),
    }
}

#[test]
fn test_matrix_determinant() {
    let env = Environment::default();

    // Test a 2x2 matrix
    let elements = vec![
        Node::Num(ExactNum::integer(4)),
        Node::Num(ExactNum::integer(3)),
        Node::Num(ExactNum::integer(2)),
        Node::Num(ExactNum::integer(1)),
    ];

    let matrix = Matrix::new(2, 2, elements).unwrap();
    let det_expr = matrix.determinant(&env).unwrap();

    // Evaluate the determinant expression to get a numerical result
    let det_value = Evaluator::evaluate(&det_expr, &env).unwrap();
    assert_eq!(det_value, -2.0); // 4*1 - 3*2 = 4 - 6 = -2

    // Test a 3x3 matrix with non-zero determinant
    let elements = vec![
        Node::Num(ExactNum::integer(1)),
        Node::Num(ExactNum::integer(2)),
        Node::Num(ExactNum::integer(3)),
        Node::Num(ExactNum::integer(0)),
        Node::Num(ExactNum::integer(1)),
        Node::Num(ExactNum::integer(4)),
        Node::Num(ExactNum::integer(5)),
        Node::Num(ExactNum::integer(6)),
        Node::Num(ExactNum::integer(0)),
    ];

    let matrix = Matrix::new(3, 3, elements).unwrap();
    let det_expr = matrix.determinant(&env).unwrap();

    // Evaluate the determinant expression to get a numerical result
    let det_value = Evaluator::evaluate(&det_expr, &env).unwrap();
    assert_eq!(det_value, 1.0); // Should be 1.0
}

#[test]
fn test_matrix_multiplication() {
    let env = Environment::default();

    // Define a 2x3 matrix A
    let elements_a = vec![
        Node::Num(ExactNum::integer(1)),
        Node::Num(ExactNum::integer(2)),
        Node::Num(ExactNum::integer(3)),
        Node::Num(ExactNum::integer(4)),
        Node::Num(ExactNum::integer(5)),
        Node::Num(ExactNum::integer(6)),
    ];
    let matrix_a = Matrix::new(2, 3, elements_a).unwrap();

    // Define a 3x2 matrix B
    let elements_b = vec![
        Node::Num(ExactNum::integer(7)),
        Node::Num(ExactNum::integer(8)),
        Node::Num(ExactNum::integer(9)),
        Node::Num(ExactNum::integer(10)),
        Node::Num(ExactNum::integer(11)),
        Node::Num(ExactNum::integer(12)),
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
fn test_matrix_inverse() {
    let env = Environment::default();

    // Define a 2x2 invertible matrix
    let elements = vec![
        Node::Num(ExactNum::integer(4)),
        Node::Num(ExactNum::integer(7)),
        Node::Num(ExactNum::integer(2)),
        Node::Num(ExactNum::integer(6)),
    ];
    let matrix = Matrix::new(2, 2, elements).unwrap();

    // Calculate the inverse
    let inverse = matrix.inverse(&env).unwrap();

    // Check dimensions
    assert_eq!(inverse.rows, 2);
    assert_eq!(inverse.cols, 2);

    // For matrix [4 7; 2 6], the inverse should be [0.6 -0.7; -0.2 0.4]
    match inverse.get(0, 0).unwrap() {
        Node::Num(n) => assert!((n.to_f64() - 0.6).abs() < 1e-10),
        _ => panic!("Expected Number node"),
    }

    match inverse.get(0, 1).unwrap() {
        Node::Num(n) => assert!((n.to_f64() + 0.7).abs() < 1e-10),
        _ => panic!("Expected Number node"),
    }

    match inverse.get(1, 0).unwrap() {
        Node::Num(n) => assert!((n.to_f64() + 0.2).abs() < 1e-10),
        _ => panic!("Expected Number node"),
    }

    match inverse.get(1, 1).unwrap() {
        Node::Num(n) => assert!((n.to_f64() - 0.4).abs() < 1e-10),
        _ => panic!("Expected Number node"),
    }

    // Verify that A * A^-1 = I
    let product = matrix.multiply(&inverse, &env).unwrap();

    for i in 0..2 {
        for j in 0..2 {
            match product.get(i, j).unwrap() {
                Node::Num(n) => {
                    if i == j {
                        assert!((n.to_f64() - 1.0).abs() < 1e-10);
                    } else {
                        assert!(n.to_f64().abs() < 1e-10);
                    }
                }
                _ => panic!("Expected Number node"),
            }
        }
    }
}

#[test]
fn test_singular_matrix() {
    let env = Environment::default();

    // Define a singular 2x2 matrix (determinant = 0)
    let elements = vec![
        Node::Num(ExactNum::integer(1)),
        Node::Num(ExactNum::integer(2)),
        Node::Num(ExactNum::integer(2)),
        Node::Num(ExactNum::integer(4)),
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
fn test_matrix_rank() {
    let env = Environment::default();

    // Full rank 2x2 matrix
    let elements = vec![
        Node::Num(ExactNum::integer(1)),
        Node::Num(ExactNum::integer(2)),
        Node::Num(ExactNum::integer(3)),
        Node::Num(ExactNum::integer(4)),
    ];
    let matrix = Matrix::new(2, 2, elements).unwrap();
    assert_eq!(matrix.rank(&env).unwrap(), 2);

    // Rank 1 matrix (second row is multiple of first)
    let elements = vec![
        Node::Num(ExactNum::integer(1)),
        Node::Num(ExactNum::integer(2)),
        Node::Num(ExactNum::integer(2)),
        Node::Num(ExactNum::integer(4)),
    ];
    let matrix = Matrix::new(2, 2, elements).unwrap();
    assert_eq!(matrix.rank(&env).unwrap(), 1);

    // Rank 2 matrix (3x3 but not full rank)
    let elements = vec![
        Node::Num(ExactNum::integer(1)),
        Node::Num(ExactNum::integer(0)),
        Node::Num(ExactNum::integer(1)),
        Node::Num(ExactNum::integer(2)),
        Node::Num(ExactNum::integer(1)),
        Node::Num(ExactNum::integer(3)),
        Node::Num(ExactNum::integer(3)),
        Node::Num(ExactNum::integer(1)),
        Node::Num(ExactNum::integer(4)),
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
        Node::Num(n) => assert_eq!(n.to_f64(), 1.0),
        _ => panic!("Expected Number node"),
    }

    match matrix.get(0, 1).unwrap() {
        Node::Num(n) => assert_eq!(n.to_f64(), 2.0),
        _ => panic!("Expected Number node"),
    }

    match matrix.get(1, 0).unwrap() {
        Node::Num(n) => assert_eq!(n.to_f64(), 3.0),
        _ => panic!("Expected Number node"),
    }

    match matrix.get(1, 1).unwrap() {
        Node::Num(n) => assert_eq!(n.to_f64(), 4.0),
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
fn test_matrix_eigenvalues() {
    let env = Environment::default();

    // Define a 2x2 matrix with known eigenvalues 1 and 3
    let elements = vec![
        Node::Num(ExactNum::integer(2)),
        Node::Num(ExactNum::integer(-1)),
        Node::Num(ExactNum::integer(-1)),
        Node::Num(ExactNum::integer(2)),
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
            Node::Num(n) => values.push(n.to_f64()),
            _ => panic!("Expected Number node for eigenvalue"),
        }
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // Check if they're the expected values
    assert!((values[0] - 1.0).abs() < 1e-10);
    assert!((values[1] - 3.0).abs() < 1e-10);
}

#[test]
fn test_linear_system_solve() {
    let env = Environment::default();

    // Create a system of equations: 2x + y = 5, x + 3y = 7
    // Matrix A = [2 1; 1 3], b = [5; 7]
    // Solution: x = 8/5, y = 9/5

    let matrix_a_elements = vec![
        Node::Num(ExactNum::integer(2)),
        Node::Num(ExactNum::integer(1)),
        Node::Num(ExactNum::integer(1)),
        Node::Num(ExactNum::integer(3)),
    ];
    let matrix_a = Matrix::new(2, 2, matrix_a_elements).unwrap();

    let vector_b_elements = vec![
        Node::Num(ExactNum::integer(5)),
        Node::Num(ExactNum::integer(7)),
    ];
    let vector_b = Matrix::new(2, 1, vector_b_elements).unwrap();

    let solution = matrix_a.solve(&vector_b, &env).unwrap();

    assert_eq!(solution.rows, 2);
    assert_eq!(solution.cols, 1);

    match solution.get(0, 0).unwrap() {
        Node::Num(n) => assert!(
            (n.to_f64() - 1.6).abs() < 1e-10,
            "x should be 8/5 = 1.6, got {}",
            n.to_f64()
        ),
        other => panic!("Expected Num node, got {:?}", other),
    }

    match solution.get(1, 0).unwrap() {
        Node::Num(n) => assert!(
            (n.to_f64() - 1.8).abs() < 1e-10,
            "y should be 9/5 = 1.8, got {}",
            n.to_f64()
        ),
        other => panic!("Expected Num node, got {:?}", other),
    }
}

#[test]
fn test_rref() {
    let env = Environment::default();

    // Create a matrix that needs row reduction
    // [1 2 3]
    // [4 5 6]
    // [7 8 9]
    let elements = vec![
        Node::Num(ExactNum::integer(1)),
        Node::Num(ExactNum::integer(2)),
        Node::Num(ExactNum::integer(3)),
        Node::Num(ExactNum::integer(4)),
        Node::Num(ExactNum::integer(5)),
        Node::Num(ExactNum::integer(6)),
        Node::Num(ExactNum::integer(7)),
        Node::Num(ExactNum::integer(8)),
        Node::Num(ExactNum::integer(9)),
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
                Node::Num(n) => {
                    let idx = i * 3 + j;
                    assert!((n.to_f64() - expected[idx]).abs() < 1e-10);
                }
                _ => panic!("Expected Number node at ({}, {})", i, j),
            }
        }
    }
}

// ── Assumption-aware eigenvalues ────────────────────────────

#[test]
fn test_eigenvalues_symbolic_2x2_no_assumptions() {
    // [[1, a], [a, 1]] → eigenvalues 1±|a| (without assumptions)
    let env = Environment::new();
    let a = Node::Variable("a".to_string());
    let one = Node::Num(ExactNum::integer(1));
    let m = Matrix::new(2, 2, vec![one.clone(), a.clone(), a.clone(), one.clone()]).unwrap();
    let vals = m.eigenvalues(&env).unwrap();
    assert_eq!(vals.len(), 2, "Should have 2 eigenvalues");
    let s: Vec<String> = vals.iter().map(|v| format!("{}", v)).collect();
    let joined = s.join(", ");
    assert!(
        joined.contains("|a|") || joined.contains("\\sqrt"),
        "Without assumptions, eigenvalues should contain |a| or √, got: {}",
        joined
    );
}

#[test]
fn test_eigenvalues_symbolic_2x2_with_positive_assumption() {
    // [[1, a], [a, 1]] with a > 0 → eigenvalues 1+a, 1-a
    use arithma::assumptions::{Assumption, Assumptions};
    let mut assumptions = Assumptions::new();
    assumptions.assume("a", Assumption::Positive);
    let env = Environment::with_assumptions(assumptions);
    let a = Node::Variable("a".to_string());
    let one = Node::Num(ExactNum::integer(1));
    let m = Matrix::new(2, 2, vec![one.clone(), a.clone(), a.clone(), one.clone()]).unwrap();
    let vals = m.eigenvalues(&env).unwrap();
    assert_eq!(vals.len(), 2, "Should have 2 eigenvalues");
    let s: Vec<String> = vals.iter().map(|v| format!("{}", v)).collect();
    let joined = s.join(", ");
    assert!(
        !joined.contains("|a|") && !joined.contains("\\sqrt"),
        "With a > 0, eigenvalues should not contain |a| or √, got: {}",
        joined
    );
    // Should contain 1+a and 1-a (or a+1 and 1-a, etc.)
    assert!(
        joined.contains('a'),
        "Eigenvalues should contain 'a', got: {}",
        joined
    );
}

#[test]
fn test_eigenvalues_no_regression_numeric() {
    // [[2, 1], [1, 2]] → eigenvalues 3, 1
    let env = Environment::new();
    let m = Matrix::new(
        2,
        2,
        vec![
            Node::Num(ExactNum::integer(2)),
            Node::Num(ExactNum::integer(1)),
            Node::Num(ExactNum::integer(1)),
            Node::Num(ExactNum::integer(2)),
        ],
    )
    .unwrap();
    let vals = m.eigenvalues(&env).unwrap();
    let mut numeric: Vec<f64> = vals
        .iter()
        .map(|v| Evaluator::evaluate(v, &env).unwrap())
        .collect();
    numeric.sort_by(|a, b| b.partial_cmp(a).unwrap());
    assert!((numeric[0] - 3.0).abs() < 1e-10);
    assert!((numeric[1] - 1.0).abs() < 1e-10);
}

// ── Complex eigenvalues must not be fabricated  ──
// The companion matrix of x³−x−1 has one real eigenvalue (the plastic
// number ≈ 1.3247) and a complex conjugate pair ≈ −0.6624 ± 0.5623i.
// The old "fill in missing repeated roots" loop printed the real part
// of the pair twice — false values, silently.

#[test]
fn eigenvalues_complex_pair_not_fabricated() {
    use arithma::matrix::parse_latex_matrix;
    let env = arithma::Environment::new();
    let m = parse_latex_matrix(
        "\\begin{pmatrix} 0 & 0 & 1 \\\\ 1 & 0 & 1 \\\\ 0 & 1 & 0 \\end{pmatrix}",
        &env,
    )
    .unwrap();
    let vals = m.eigenvalues(&env).unwrap();
    assert_eq!(vals.len(), 3);
    let rendered: Vec<String> = vals.iter().map(|v| format!("{}", v)).collect();
    let all = rendered.join(", ");
    // The real root (plastic number) is present…
    assert!(
        rendered.iter().any(|s| s.starts_with("1.3247")),
        "missing real root: {}",
        all
    );
    // …and the complex pair is EXPLICIT (symbol i), not silently realified.
    assert_eq!(
        rendered.iter().filter(|s| s.contains('i')).count(),
        2,
        "complex pair must appear with imaginary parts: {}",
        all
    );
    assert!(
        rendered.iter().any(|s| s.contains("0.5622")),
        "imaginary magnitude ≈ 0.5623 missing: {}",
        all
    );
}

#[test]
fn eigenvalues_2x2_complex_rotation() {
    use arithma::matrix::parse_latex_matrix;
    let env = arithma::Environment::new();
    // Rotation-like matrix [[0,-1],[1,0]]: eigenvalues ±i.
    let m = parse_latex_matrix("\\begin{pmatrix} 0 & -1 \\\\ 1 & 0 \\end{pmatrix}", &env).unwrap();
    let vals = m.eigenvalues(&env).unwrap();
    assert_eq!(vals.len(), 2);
    let all: String = vals.iter().map(|v| format!("{}, ", v)).collect();
    assert_eq!(
        vals.iter()
            .filter(|v| format!("{}", v).contains('i'))
            .count(),
        2,
        "±i expected: {}",
        all
    );
}

#[test]
fn eigenvalues_repeated_real_still_work() {
    use arithma::matrix::parse_latex_matrix;
    let env = arithma::Environment::new();
    // [[2,1,0],[0,2,0],[0,0,3]]: eigenvalues 2, 2, 3 (defective, repeated).
    let m = parse_latex_matrix(
        "\\begin{pmatrix} 2 & 1 & 0 \\\\ 0 & 2 & 0 \\\\ 0 & 0 & 3 \\end{pmatrix}",
        &env,
    )
    .unwrap();
    let vals = m.eigenvalues(&env).unwrap();
    let mut nums: Vec<f64> = vals
        .iter()
        .map(|v| arithma::Evaluator::evaluate(v, &env).unwrap_or(f64::NAN))
        .collect();
    nums.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let expected = [2.0, 2.0, 3.0];
    assert_eq!(nums.len(), 3, "spectrum incomplete: {:?}", nums);
    for (got, want) in nums.iter().zip(expected.iter()) {
        assert!(
            (got - want).abs() < 1e-9,
            "expected {:?}, got {:?}",
            expected,
            nums
        );
    }
}
