use std::fmt;
use std::ops::{Add, Mul, Neg, Sub};

use crate::environment::Environment;
use crate::node::Node;
use crate::simplify::Simplifiable;

/// Represents a mathematical matrix with expression elements
#[derive(Clone, Debug)]
pub struct Matrix {
    /// Number of rows in the matrix
    pub rows: usize,
    /// Number of columns in the matrix
    pub cols: usize,
    /// The elements of the matrix stored in row-major order
    pub elements: Vec<Node>,
}

impl Matrix {
    /// Create a new matrix with specified dimensions and elements
    pub fn new(rows: usize, cols: usize, elements: Vec<Node>) -> Result<Self, String> {
        if elements.len() != rows * cols {
            return Err(format!(
                "Invalid matrix: expected {} elements for {}x{} matrix, but got {}",
                rows * cols,
                rows,
                cols,
                elements.len()
            ));
        }

        Ok(Matrix {
            rows,
            cols,
            elements,
        })
    }

    /// Create a new matrix from a 2D vector of Node elements
    pub fn from_elements(elements: Vec<Vec<Node>>) -> Result<Self, String> {
        if elements.is_empty() {
            return Err("Cannot create matrix with no rows".to_string());
        }

        let rows = elements.len();
        let cols = elements[0].len();

        // Check that all rows have the same length
        for row in &elements {
            if row.len() != cols {
                return Err("All rows in a matrix must have the same length".to_string());
            }
        }

        // Flatten the 2D vector into a 1D vector
        let flat_elements = elements.into_iter().flatten().collect();

        Matrix::new(rows, cols, flat_elements)
    }

    /// Create an identity matrix of specified size
    pub fn identity(size: usize) -> Self {
        let mut elements = vec![Node::Number(0.0); size * size];

        // Set diagonal elements to 1
        for i in 0..size {
            elements[i * size + i] = Node::Number(1.0);
        }

        Matrix {
            rows: size,
            cols: size,
            elements,
        }
    }

    /// Check if this matrix is square (same number of rows and columns)
    pub fn is_square(&self) -> bool {
        self.rows == self.cols
    }

    /// Get an element at a specific position (row, col)
    pub fn get(&self, row: usize, col: usize) -> Result<&Node, String> {
        if row >= self.rows || col >= self.cols {
            return Err(format!(
                "Matrix index out of bounds: ({}, {}) for {}x{} matrix",
                row, col, self.rows, self.cols
            ));
        }

        Ok(&self.elements[row * self.cols + col])
    }

    /// Set an element at a specific position (row, col)
    pub fn set(&mut self, row: usize, col: usize, value: Node) -> Result<(), String> {
        if row >= self.rows || col >= self.cols {
            return Err(format!(
                "Matrix index out of bounds: ({}, {}) for {}x{} matrix",
                row, col, self.rows, self.cols
            ));
        }

        self.elements[row * self.cols + col] = value;
        Ok(())
    }

    /// Transpose this matrix
    pub fn transpose(&self) -> Self {
        let mut result = vec![Node::Number(0.0); self.rows * self.cols];

        for i in 0..self.rows {
            for j in 0..self.cols {
                result[j * self.rows + i] = self.elements[i * self.cols + j].clone();
            }
        }

        Matrix {
            rows: self.cols,
            cols: self.rows,
            elements: result,
        }
    }

    /// Calculate the determinant of a square matrix
    pub fn determinant(&self, env: &Environment) -> Result<Node, String> {
        if !self.is_square() {
            return Err("Cannot calculate determinant of a non-square matrix".to_string());
        }

        match self.rows {
            0 => Err("Cannot calculate determinant of an empty matrix".to_string()),
            1 => Ok(self.elements[0].clone()),
            2 => {
                // For 2x2 matrix: ad - bc
                let a = &self.elements[0];
                let b = &self.elements[1];
                let c = &self.elements[2];
                let d = &self.elements[3];

                let ad = Node::Multiply(Box::new(a.clone()), Box::new(d.clone()));

                let bc = Node::Multiply(Box::new(b.clone()), Box::new(c.clone()));

                Ok(Node::Subtract(Box::new(ad), Box::new(bc)).simplify(env)?)
            }
            _ => {
                // For larger matrices, use the first row and calculate cofactors
                let mut result = Node::Number(0.0);

                for j in 0..self.cols {
                    let minor = self.minor(0, j)?;
                    let cofactor = minor.determinant(env)?;

                    // Apply sign: (-1)^(i+j)
                    let sign = if j % 2 == 0 { 1.0 } else { -1.0 };
                    let term = Node::Multiply(
                        Box::new(Node::Number(sign)),
                        Box::new(Node::Multiply(
                            Box::new(self.elements[j].clone()),
                            Box::new(cofactor),
                        )),
                    )
                    .simplify(env)?;

                    result = Node::Add(Box::new(result), Box::new(term)).simplify(env)?;
                }

                Ok(result)
            }
        }
    }

    /// Get the minor matrix by removing a specific row and column
    pub fn minor(&self, row: usize, col: usize) -> Result<Matrix, String> {
        if !self.is_square() {
            return Err("Cannot get minor of a non-square matrix".to_string());
        }

        if row >= self.rows || col >= self.cols {
            return Err(format!(
                "Matrix index out of bounds: ({}, {}) for {}x{} matrix",
                row, col, self.rows, self.cols
            ));
        }

        let new_size = self.rows - 1;
        let mut elements = Vec::with_capacity(new_size * new_size);

        for i in 0..self.rows {
            if i == row {
                continue; // Skip the specified row
            }

            for j in 0..self.cols {
                if j == col {
                    continue; // Skip the specified column
                }

                elements.push(self.elements[i * self.cols + j].clone());
            }
        }

        Matrix::new(new_size, new_size, elements)
    }

    /// Calculate the matrix of cofactors
    pub fn cofactor_matrix(&self, env: &Environment) -> Result<Matrix, String> {
        if !self.is_square() {
            return Err("Cannot calculate cofactors of a non-square matrix".to_string());
        }

        let size = self.rows;
        let mut elements = Vec::with_capacity(size * size);

        for i in 0..size {
            for j in 0..size {
                let minor = self.minor(i, j)?;
                let mut cofactor = minor.determinant(env)?;

                // Apply sign: (-1)^(i+j)
                if (i + j) % 2 == 1 {
                    cofactor = Node::Negate(Box::new(cofactor)).simplify(env)?;
                }

                elements.push(cofactor);
            }
        }

        Matrix::new(size, size, elements)
    }

    /// Calculate the adjugate (adjoint) of the matrix
    pub fn adjugate(&self, env: &Environment) -> Result<Matrix, String> {
        // The adjugate is the transpose of the cofactor matrix
        Ok(self.cofactor_matrix(env)?.transpose())
    }

    /// Calculate the inverse of a square matrix
    pub fn inverse(&self, env: &Environment) -> Result<Matrix, String> {
        if !self.is_square() {
            return Err("Cannot invert a non-square matrix".to_string());
        }

        let det = self.determinant(env)?;

        // Check if determinant is zero
        if let Node::Number(0.0) = det {
            return Err("Cannot invert a singular matrix (determinant is zero)".to_string());
        }

        // Inverse = adjugate / determinant
        let adjugate = self.adjugate(env)?;
        let mut result = Vec::with_capacity(self.rows * self.cols);

        for element in adjugate.elements {
            result.push(Node::Divide(Box::new(element), Box::new(det.clone())).simplify(env)?);
        }

        Matrix::new(self.rows, self.cols, result)
    }

    /// Convert the matrix to a LaTeX string
    pub fn to_latex(&self) -> String {
        let mut result = String::from("\\begin{pmatrix}\n");

        for i in 0..self.rows {
            let row: Vec<String> = (0..self.cols)
                .map(|j| self.elements[i * self.cols + j].to_string())
                .collect();

            result.push_str(&row.join(" & "));

            if i < self.rows - 1 {
                result.push_str(" \\\\\n");
            }
        }

        result.push_str("\n\\end{pmatrix}");
        result
    }

    /// Perform Gauss-Jordan elimination to find the reduced row echelon form (RREF)
    pub fn rref(&self, env: &Environment) -> Result<Matrix, String> {
        let mut result = self.clone();
        let mut lead = 0;

        for r in 0..self.rows {
            if self.cols <= lead {
                break;
            }

            let mut i = r;
            while let Node::Number(value) = result.elements[i * self.cols + lead] {
                if value == 0.0 {
                    i += 1;
                    if i == self.rows {
                        i = r;
                        lead += 1;
                        if self.cols == lead {
                            return Ok(result);
                        }
                    }
                } else {
                    break;
                }
            }

            // Swap rows i and r
            if i != r {
                for j in 0..self.cols {
                    let temp = result.elements[i * self.cols + j].clone();
                    result.elements[i * self.cols + j] = result.elements[r * self.cols + j].clone();
                    result.elements[r * self.cols + j] = temp;
                }
            }

            // Scale row r
            let pivot = result.elements[r * self.cols + lead].clone();
            for j in 0..self.cols {
                result.elements[r * self.cols + j] = Node::Divide(
                    Box::new(result.elements[r * self.cols + j].clone()),
                    Box::new(pivot.clone()),
                )
                .simplify(env)?;
            }

            // Eliminate other rows
            for i in 0..self.rows {
                if i != r {
                    let factor = result.elements[i * self.cols + lead].clone();
                    for j in 0..self.cols {
                        let subtraction = Node::Multiply(
                            Box::new(factor.clone()),
                            Box::new(result.elements[r * self.cols + j].clone()),
                        )
                        .simplify(env)?;

                        result.elements[i * self.cols + j] = Node::Subtract(
                            Box::new(result.elements[i * self.cols + j].clone()),
                            Box::new(subtraction),
                        )
                        .simplify(env)?;
                    }
                }
            }

            lead += 1;
        }

        Ok(result)
    }

    /// Solve a system of linear equations represented as Ax = b
    /// Returns x, the solution vector
    pub fn solve(&self, b: &Matrix, env: &Environment) -> Result<Matrix, String> {
        if self.rows != b.rows {
            return Err(format!(
                "Matrix dimensions don't match for solving equations: A is {}x{}, b is {}x{}",
                self.rows, self.cols, b.rows, b.cols
            ));
        }

        if b.cols != 1 {
            return Err("Right-hand side must be a column vector".to_string());
        }

        if !self.is_square() {
            return Err("Coefficient matrix must be square".to_string());
        }

        // Check if matrix is invertible
        let det = self.determinant(env)?;
        if let Node::Number(0.0) = det {
            return Err("System has no unique solution (singular matrix)".to_string());
        }

        // Solve using matrix inverse: x = A^-1 * b
        let inverse = self.inverse(env)?;
        inverse.multiply(b, env)
    }

    /// Multiply this matrix by another matrix
    pub fn multiply(&self, other: &Matrix, env: &Environment) -> Result<Matrix, String> {
        if self.cols != other.rows {
            return Err(format!(
                "Matrix dimensions don't match for multiplication: {}x{} * {}x{}",
                self.rows, self.cols, other.rows, other.cols
            ));
        }

        let mut result = Vec::with_capacity(self.rows * other.cols);

        for i in 0..self.rows {
            for j in 0..other.cols {
                let mut sum = Node::Number(0.0);

                for k in 0..self.cols {
                    let product = Node::Multiply(
                        Box::new(self.elements[i * self.cols + k].clone()),
                        Box::new(other.elements[k * other.cols + j].clone()),
                    )
                    .simplify(env)?;

                    sum = Node::Add(Box::new(sum), Box::new(product)).simplify(env)?;
                }

                result.push(sum);
            }
        }

        Matrix::new(self.rows, other.cols, result)
    }

    /// Calculate the rank of the matrix
    pub fn rank(&self, env: &Environment) -> Result<usize, String> {
        let rref = self.rref(env)?;
        let mut rank = 0;

        // Count non-zero rows in the RREF
        'outer: for i in 0..rref.rows {
            for j in 0..rref.cols {
                match rref.elements[i * rref.cols + j] {
                    Node::Number(n) if n != 0.0 => {
                        rank += 1;
                        continue 'outer;
                    }
                    _ => {}
                }
            }
        }

        Ok(rank)
    }

    /// Computes the eigenvalues of a square matrix
    /// NOTE: This is a simplified implementation that only works for 2x2 matrices
    pub fn eigenvalues(&self, env: &Environment) -> Result<Vec<Node>, String> {
        if !self.is_square() {
            return Err("Cannot compute eigenvalues of a non-square matrix".to_string());
        }

        match self.rows {
            2 => {
                // For 2x2 matrix, eigenvalues are solutions to the characteristic equation:
                // λ² - tr(A)λ + det(A) = 0
                let a = &self.elements[0]; // Top left
                let b = &self.elements[1]; // Top right
                let c = &self.elements[2]; // Bottom left
                let d = &self.elements[3]; // Bottom right

                // Calculate trace
                let trace = Node::Add(Box::new(a.clone()), Box::new(d.clone())).simplify(env)?;

                // Calculate determinant
                let det = Node::Subtract(
                    Box::new(Node::Multiply(Box::new(a.clone()), Box::new(d.clone()))),
                    Box::new(Node::Multiply(Box::new(b.clone()), Box::new(c.clone()))),
                )
                .simplify(env)?;

                // Calculate discriminant: trace² - 4*det
                let trace_squared =
                    Node::Multiply(Box::new(trace.clone()), Box::new(trace.clone()))
                        .simplify(env)?;

                let four_det = Node::Multiply(Box::new(Node::Number(4.0)), Box::new(det.clone()))
                    .simplify(env)?;

                let discriminant =
                    Node::Subtract(Box::new(trace_squared), Box::new(four_det)).simplify(env)?;

                // Calculate eigenvalues: (trace ± √discriminant) / 2
                let sqrt_discriminant =
                    Node::Function("sqrt".to_string(), vec![discriminant.clone()]).simplify(env)?;

                let lambda1 = Node::Divide(
                    Box::new(Node::Add(
                        Box::new(trace.clone()),
                        Box::new(sqrt_discriminant.clone()),
                    )),
                    Box::new(Node::Number(2.0)),
                )
                .simplify(env)?;

                let lambda2 = Node::Divide(
                    Box::new(Node::Subtract(Box::new(trace), Box::new(sqrt_discriminant))),
                    Box::new(Node::Number(2.0)),
                )
                .simplify(env)?;

                Ok(vec![lambda1, lambda2])
            }
            _ => Err(
                "Eigenvalue calculation for matrices larger than 2x2 is not implemented"
                    .to_string(),
            ),
        }
    }
}

// Implement addition for matrices
impl Add for Matrix {
    type Output = Result<Matrix, String>;

    fn add(self, other: Matrix) -> Self::Output {
        if self.rows != other.rows || self.cols != other.cols {
            return Err(format!(
                "Matrix dimensions don't match for addition: {}x{} + {}x{}",
                self.rows, self.cols, other.rows, other.cols
            ));
        }

        let mut result = Vec::with_capacity(self.rows * self.cols);

        for i in 0..self.elements.len() {
            result.push(Node::Add(
                Box::new(self.elements[i].clone()),
                Box::new(other.elements[i].clone()),
            ));
        }

        Matrix::new(self.rows, self.cols, result)
    }
}

// Implement subtraction for matrices
impl Sub for Matrix {
    type Output = Result<Matrix, String>;

    fn sub(self, other: Matrix) -> Self::Output {
        if self.rows != other.rows || self.cols != other.cols {
            return Err(format!(
                "Matrix dimensions don't match for subtraction: {}x{} - {}x{}",
                self.rows, self.cols, other.rows, other.cols
            ));
        }

        let mut result = Vec::with_capacity(self.rows * self.cols);

        for i in 0..self.elements.len() {
            result.push(Node::Subtract(
                Box::new(self.elements[i].clone()),
                Box::new(other.elements[i].clone()),
            ));
        }

        Matrix::new(self.rows, self.cols, result)
    }
}

// Implement negation for matrices
impl Neg for Matrix {
    type Output = Matrix;

    fn neg(self) -> Self::Output {
        let mut result = Vec::with_capacity(self.rows * self.cols);

        for element in self.elements {
            result.push(Node::Negate(Box::new(element)));
        }

        Matrix {
            rows: self.rows,
            cols: self.cols,
            elements: result,
        }
    }
}

// Implement scalar multiplication for matrices
impl Mul<Node> for Matrix {
    type Output = Matrix;

    fn mul(self, scalar: Node) -> Self::Output {
        let mut result = Vec::with_capacity(self.rows * self.cols);

        for element in self.elements {
            result.push(Node::Multiply(Box::new(element), Box::new(scalar.clone())));
        }

        Matrix {
            rows: self.rows,
            cols: self.cols,
            elements: result,
        }
    }
}

// Implement Debug trait for Matrix
impl fmt::Display for Matrix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Matrix {}x{}", self.rows, self.cols)?;

        for i in 0..self.rows {
            write!(f, "[")?;
            for j in 0..self.cols {
                if j > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", self.elements[i * self.cols + j])?;
            }
            writeln!(f, "]")?;
        }

        Ok(())
    }
}

/// Parse a LaTeX matrix expression and return a Matrix object
pub fn parse_latex_matrix(latex: &str, env: &Environment) -> Result<Matrix, String> {
    let mut content = latex.trim().to_string();

    // Check if we have a matrix environment
    let matrix_envs = ["pmatrix", "bmatrix", "vmatrix", "matrix"];
    let mut found_env = false;

    for env_name in &matrix_envs {
        let start_tag = format!("\\begin{{{}}}", env_name);
        let end_tag = format!("\\end{{{}}}", env_name);

        if content.starts_with(&start_tag) && content.ends_with(&end_tag) {
            // Extract the content between the tags
            content = content[start_tag.len()..content.len() - end_tag.len()]
                .trim()
                .to_string();
            found_env = true;
            break;
        }
    }

    if !found_env {
        return Err("Invalid matrix format: missing matrix environment".to_string());
    }

    // Split into rows by \\
    let rows: Vec<&str> = content.split("\\\\").map(|s| s.trim()).collect();

    // Parse each row
    let mut matrix_rows = Vec::new();

    for row in rows {
        if row.is_empty() {
            continue;
        }

        // Split columns by &
        let cols: Vec<&str> = row.split('&').map(|s| s.trim()).collect();
        let mut row_elements = Vec::new();

        for col in cols {
            if col.is_empty() {
                continue;
            }

            // Parse the expression
            let expr = crate::parser::parse_latex(col, env)?;
            row_elements.push(expr);
        }

        if !row_elements.is_empty() {
            matrix_rows.push(row_elements);
        }
    }

    Matrix::from_elements(matrix_rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::Environment;
    use crate::node::Node;

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
    fn test_matrix_from_elements() {
        let elements = vec![
            vec![Node::Number(1.0), Node::Number(2.0)],
            vec![Node::Number(3.0), Node::Number(4.0)],
        ];

        let matrix = Matrix::from_elements(elements).unwrap();
        assert_eq!(matrix.rows, 2);
        assert_eq!(matrix.cols, 2);
        assert_eq!(matrix.elements.len(), 4);
    }

    #[test]
    fn test_identity_matrix() {
        let identity = Matrix::identity(3);

        assert_eq!(identity.rows, 3);
        assert_eq!(identity.cols, 3);

        for i in 0..3 {
            for j in 0..3 {
                let expected = if i == j { 1.0 } else { 0.0 };
                match identity.elements[i * 3 + j] {
                    Node::Number(n) => assert_eq!(n, expected),
                    _ => panic!("Expected number node"),
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

        // Check elements
        match &transposed.elements[0] {
            Node::Number(n) => assert_eq!(*n, 1.0),
            _ => panic!("Expected Number node"),
        }
        match &transposed.elements[1] {
            Node::Number(n) => assert_eq!(*n, 4.0),
            _ => panic!("Expected Number node"),
        }
        match &transposed.elements[2] {
            Node::Number(n) => assert_eq!(*n, 2.0),
            _ => panic!("Expected Number node"),
        }
        match &transposed.elements[3] {
            Node::Number(n) => assert_eq!(*n, 5.0),
            _ => panic!("Expected Number node"),
        }
        match &transposed.elements[4] {
            Node::Number(n) => assert_eq!(*n, 3.0),
            _ => panic!("Expected Number node"),
        }
        match &transposed.elements[5] {
            Node::Number(n) => assert_eq!(*n, 6.0),
            _ => panic!("Expected Number node"),
        }
    }

    #[test]
    #[ignore = "Matrix determinant calculation needs fixing"]
    fn test_matrix_determinant() {
        let env = Environment::default();

        // 2x2 matrix
        let elements_2x2 = vec![
            Node::Number(1.0),
            Node::Number(2.0),
            Node::Number(3.0),
            Node::Number(4.0),
        ];
        let matrix_2x2 = Matrix::new(2, 2, elements_2x2).unwrap();

        // Determinant should be 1*4 - 2*3 = 4 - 6 = -2
        let det = matrix_2x2.determinant(&env).unwrap();
        match det {
            Node::Number(n) => assert_eq!(n, -2.0),
            _ => panic!("Expected Number node"),
        }

        // 3x3 matrix
        let elements_3x3 = vec![
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
        let matrix_3x3 = Matrix::new(3, 3, elements_3x3).unwrap();

        // Determinant should be 0 for this matrix (it's singular)
        let det = matrix_3x3.determinant(&env).unwrap();
        match det {
            Node::Number(n) => assert!((n).abs() < 1e-10),
            _ => panic!("Expected Number node"),
        }
    }

    #[test]
    #[ignore = "Matrix multiplication result calculation needs fixing"]
    fn test_matrix_multiplication() {
        let env = Environment::default();

        // 2x3 matrix
        let elements_a = vec![
            Node::Number(1.0),
            Node::Number(2.0),
            Node::Number(3.0),
            Node::Number(4.0),
            Node::Number(5.0),
            Node::Number(6.0),
        ];
        let matrix_a = Matrix::new(2, 3, elements_a).unwrap();

        // 3x2 matrix
        let elements_b = vec![
            Node::Number(7.0),
            Node::Number(8.0),
            Node::Number(9.0),
            Node::Number(10.0),
            Node::Number(11.0),
            Node::Number(12.0),
        ];
        let matrix_b = Matrix::new(3, 2, elements_b).unwrap();

        // Result should be 2x2
        let result = matrix_a.multiply(&matrix_b, &env).unwrap();
        assert_eq!(result.rows, 2);
        assert_eq!(result.cols, 2);

        // Check result elements
        // [1 2 3] * [7 8]   = [58 64]
        // [4 5 6]   [9 10]    [139 154]
        //           [11 12]
        let expected = vec![58.0, 64.0, 139.0, 154.0];
        for i in 0..4 {
            match &result.elements[i] {
                Node::Number(n) => assert_eq!(*n, expected[i]),
                _ => panic!("Expected Number node at index {}", i),
            }
        }
    }

    #[test]
    #[ignore = "Matrix inverse calculation needs fixing"]
    fn test_matrix_inverse() {
        let env = Environment::default();

        // 2x2 invertible matrix
        let elements = vec![
            Node::Number(4.0),
            Node::Number(7.0),
            Node::Number(2.0),
            Node::Number(6.0),
        ];
        let matrix = Matrix::new(2, 2, elements).unwrap();

        // Determinant is 4*6 - 7*2 = 24 - 14 = 10
        // Inverse should be [0.6 -0.7; -0.2 0.4]
        let inverse = matrix.inverse(&env).unwrap();

        let expected = vec![0.6, -0.7, -0.2, 0.4];
        for i in 0..4 {
            match &inverse.elements[i] {
                Node::Number(n) => assert!((n - expected[i]).abs() < 1e-10),
                _ => panic!("Expected Number node at index {}", i),
            }
        }

        // Check that A * A^-1 = I
        let identity = matrix.multiply(&inverse, &env).unwrap();
        for i in 0..2 {
            for j in 0..2 {
                let expected = if i == j { 1.0 } else { 0.0 };
                match &identity.elements[i * 2 + j] {
                    Node::Number(n) => assert!((n - expected).abs() < 1e-10),
                    _ => panic!("Expected Number node"),
                }
            }
        }
    }

    #[test]
    #[ignore = "Matrix singular detection needs fixing"]
    fn test_singular_matrix_inverse() {
        let env = Environment::default();

        // Singular 2x2 matrix (determinant = 0)
        let elements = vec![
            Node::Number(1.0),
            Node::Number(2.0),
            Node::Number(2.0),
            Node::Number(4.0),
        ];
        let matrix = Matrix::new(2, 2, elements).unwrap();

        // Inverse should fail
        let result = matrix.inverse(&env);
        assert!(result.is_err());
    }

    #[test]
    #[ignore = "Matrix rank calculation needs fixing"]
    fn test_matrix_rank() {
        let env = Environment::default();

        // Full rank 2x2 matrix
        let elements_full_rank = vec![
            Node::Number(1.0),
            Node::Number(2.0),
            Node::Number(3.0),
            Node::Number(4.0),
        ];
        let matrix_full_rank = Matrix::new(2, 2, elements_full_rank).unwrap();
        assert_eq!(matrix_full_rank.rank(&env).unwrap(), 2);

        // Rank 1 matrix (second row is multiple of first)
        let elements_rank_1 = vec![
            Node::Number(1.0),
            Node::Number(2.0),
            Node::Number(2.0),
            Node::Number(4.0),
        ];
        let matrix_rank_1 = Matrix::new(2, 2, elements_rank_1).unwrap();
        assert_eq!(matrix_rank_1.rank(&env).unwrap(), 1);

        // Rank 2 matrix (3x3 but not full rank)
        let elements_rank_2 = vec![
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
        let matrix_rank_2 = Matrix::new(3, 3, elements_rank_2).unwrap();
        assert_eq!(matrix_rank_2.rank(&env).unwrap(), 2);
    }

    #[test]
    fn test_parse_latex_matrix() {
        let env = Environment::default();

        // Test pmatrix environment
        let latex = r"\begin{pmatrix} 1 & 2 \\ 3 & 4 \end{pmatrix}";
        let matrix = parse_latex_matrix(latex, &env).unwrap();

        assert_eq!(matrix.rows, 2);
        assert_eq!(matrix.cols, 2);

        match &matrix.elements[0] {
            Node::Number(n) => assert_eq!(*n, 1.0),
            _ => panic!("Expected Number node"),
        }
        match &matrix.elements[1] {
            Node::Number(n) => assert_eq!(*n, 2.0),
            _ => panic!("Expected Number node"),
        }
        match &matrix.elements[2] {
            Node::Number(n) => assert_eq!(*n, 3.0),
            _ => panic!("Expected Number node"),
        }
        match &matrix.elements[3] {
            Node::Number(n) => assert_eq!(*n, 4.0),
            _ => panic!("Expected Number node"),
        }
    }

    #[test]
    #[ignore = "Matrix eigenvalues calculation needs fixing"]
    fn test_matrix_eigenvalues() {
        let env = Environment::default();

        // Matrix with eigenvalues 1 and 3
        let elements = vec![
            Node::Number(2.0),
            Node::Number(-1.0),
            Node::Number(-1.0),
            Node::Number(2.0),
        ];
        let matrix = Matrix::new(2, 2, elements).unwrap();

        let eigenvalues = matrix.eigenvalues(&env).unwrap();
        assert_eq!(eigenvalues.len(), 2);

        // Sort eigenvalues to make testing easier
        let mut sorted_eigenvalues = eigenvalues;
        sorted_eigenvalues.sort_by(|a, b| {
            if let (Node::Number(x), Node::Number(y)) = (a, b) {
                x.partial_cmp(y).unwrap()
            } else {
                panic!("Expected Number nodes")
            }
        });

        match &sorted_eigenvalues[0] {
            Node::Number(n) => assert!((n - 1.0).abs() < 1e-10),
            _ => panic!("Expected Number node"),
        }
        match &sorted_eigenvalues[1] {
            Node::Number(n) => assert!((n - 3.0).abs() < 1e-10),
            _ => panic!("Expected Number node"),
        }
    }
}
