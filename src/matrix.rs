use std::fmt;
use std::ops::{Add, Mul, Neg, Sub};

use crate::environment::Environment;
use crate::exact::ExactNum;
use crate::node::Node;
use crate::simplify::Simplifiable;
use num_traits::ToPrimitive;

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
        let mut elements = vec![Node::Num(ExactNum::zero()); size * size];

        // Set diagonal elements to 1
        for i in 0..size {
            elements[i * size + i] = Node::Num(ExactNum::one());
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
        let mut result = vec![Node::Num(ExactNum::zero()); self.rows * self.cols];

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
                let mut result = Node::Num(ExactNum::zero());

                for j in 0..self.cols {
                    let minor = self.minor(0, j)?;
                    let cofactor = minor.determinant(env)?;

                    // Apply sign: (-1)^(i+j)
                    let sign = if j % 2 == 0 {
                        ExactNum::one()
                    } else {
                        -ExactNum::one()
                    };
                    let term = Node::Multiply(
                        Box::new(Node::Num(sign)),
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
        if let Node::Num(ref n) = det {
            if n.is_zero() {
                return Err("Cannot invert a singular matrix (determinant is zero)".to_string());
            }
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
            while let Node::Num(ref value) = result.elements[i * self.cols + lead] {
                if value.is_zero() {
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
        if let Node::Num(ref n) = det {
            if n.is_zero() {
                return Err("System has no unique solution (singular matrix)".to_string());
            }
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
                let mut sum = Node::Num(ExactNum::zero());

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
                match &rref.elements[i * rref.cols + j] {
                    Node::Num(n) if !n.is_zero() => {
                        rank += 1;
                        continue 'outer;
                    }
                    _ => {}
                }
            }
        }

        Ok(rank)
    }

    /// Computes the characteristic polynomial det(A - λI) as a Polynomial in λ.
    pub fn characteristic_polynomial(
        &self,
        env: &Environment,
    ) -> Result<crate::polynomial::Polynomial, String> {
        if !self.is_square() {
            return Err(
                "Cannot compute characteristic polynomial of a non-square matrix".to_string(),
            );
        }
        let lambda_var = "__lambda__";
        let lambda = Node::Variable(lambda_var.to_string());

        let mut shifted = self.elements.clone();
        for i in 0..self.rows {
            let idx = i * self.cols + i;
            shifted[idx] = Node::Subtract(Box::new(shifted[idx].clone()), Box::new(lambda.clone()));
        }

        let shifted_matrix = Matrix {
            rows: self.rows,
            cols: self.cols,
            elements: shifted,
        };

        let det_expr = shifted_matrix.determinant(env)?;
        let det_simplified = det_expr.simplify(env).unwrap_or(det_expr);

        crate::polynomial::Polynomial::from_node(&det_simplified, lambda_var)
            .map_err(|e| format!("Characteristic polynomial extraction failed: {}", e))
    }

    /// Computes the eigenvalues of a square matrix via the characteristic polynomial.
    /// Returns eigenvalues with algebraic multiplicity.
    /// Supports matrices up to 4×4 (Cardano for 3×3, Ferrari for 4×4).
    /// Falls back to symbolic computation when entries contain variables.
    pub fn eigenvalues(&self, env: &Environment) -> Result<Vec<Node>, String> {
        if !self.is_square() {
            return Err("Cannot compute eigenvalues of a non-square matrix".to_string());
        }
        if self.rows > 4 {
            return Err(format!(
                "Eigenvalue computation for {}×{} matrices is not supported (max 4×4)",
                self.rows, self.rows
            ));
        }

        // For purely numeric matrices, compute directly with f64 to avoid
        // precision issues in the symbolic path with Float entries.
        if let Some(numerical) = self.eigenvalues_direct_numeric(env) {
            return Ok(numerical);
        }

        // Try exact path: characteristic polynomial → factor → solve
        if let Ok(char_poly) = self.characteristic_polynomial(env) {
            let (_, factors) = crate::mod_poly::factor_over_q(&char_poly);
            let mut eigenvalues = Vec::new();
            for factor in &factors {
                let expr = factor.to_node();
                let eq = Node::Equation(Box::new(expr), Box::new(Node::Num(ExactNum::zero())));
                if let Ok(roots) = crate::expression::solve_for_variable_exact(&eq, "__lambda__") {
                    for root in roots {
                        eigenvalues.push(Node::Num(root));
                    }
                }
            }
            if !eigenvalues.is_empty() {
                return Ok(eigenvalues);
            }

            // Exact factoring failed — try numerical solve on the characteristic polynomial
            if let Some(numerical) = self.eigenvalues_numerical(&char_poly) {
                return Ok(numerical);
            }
        }

        // Try symbolic
        self.eigenvalues_symbolic(env)
    }

    fn eigenvalues_direct_numeric(&self, env: &Environment) -> Option<Vec<Node>> {
        let n = self.rows;
        if n > 4 {
            return None;
        }
        let mut vals = vec![vec![0.0f64; n]; n];
        for (i, row) in vals.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                let node = &self.elements[i * n + j];
                let v = crate::evaluator::Evaluator::evaluate(node, env).ok()?;
                if v.is_nan() || v.is_infinite() {
                    return None;
                }
                *cell = v;
            }
        }

        let roots = match n {
            1 => vec![vals[0][0]],
            2 => {
                let tr = vals[0][0] + vals[1][1];
                let det = vals[0][0] * vals[1][1] - vals[0][1] * vals[1][0];
                let disc = tr * tr - 4.0 * det;
                if disc < -1e-10 {
                    return None;
                }
                let disc = disc.max(0.0).sqrt();
                vec![(tr + disc) / 2.0, (tr - disc) / 2.0]
            }
            3 => {
                let (a, b, c, d, e, f, g, h, k) = (
                    vals[0][0], vals[0][1], vals[0][2], vals[1][0], vals[1][1], vals[1][2],
                    vals[2][0], vals[2][1], vals[2][2],
                );
                let trace = a + e + k;
                let c3 = -1.0;
                let c2 = trace;
                let c1 = -(a * e - b * d + a * k - c * g + e * k - f * h);
                let c0 = a * (e * k - f * h) - b * (d * k - f * g) + c * (d * h - e * g);
                let mut r = crate::expression::solve_cubic_f64_pub(c3, c2, c1, c0);
                // Fill in missing repeated roots using the trace identity
                while r.len() < 3 {
                    let missing: f64 = trace - r.iter().sum::<f64>();
                    r.push(missing / (3 - r.len()) as f64);
                }
                r
            }
            4 => {
                let (a, b, c, d) = (vals[0][0], vals[0][1], vals[0][2], vals[0][3]);
                let (e, f, g, h) = (vals[1][0], vals[1][1], vals[1][2], vals[1][3]);
                let (i, j, k, l) = (vals[2][0], vals[2][1], vals[2][2], vals[2][3]);
                let (m, nn, o, p) = (vals[3][0], vals[3][1], vals[3][2], vals[3][3]);

                let trace = a + f + k + p;
                let sum2 = (a * f - b * e)
                    + (a * k - c * i)
                    + (a * p - d * m)
                    + (f * k - g * j)
                    + (f * p - h * nn)
                    + (k * p - l * o);
                let det3sum = {
                    let m3 = |r0: [f64; 3], r1: [f64; 3], r2: [f64; 3]| -> f64 {
                        r0[0] * (r1[1] * r2[2] - r1[2] * r2[1])
                            - r0[1] * (r1[0] * r2[2] - r1[2] * r2[0])
                            + r0[2] * (r1[0] * r2[1] - r1[1] * r2[0])
                    };
                    m3([f, g, h], [j, k, l], [nn, o, p])
                        + m3([a, c, d], [i, k, l], [m, o, p])
                        + m3([a, b, d], [e, f, h], [m, nn, p])
                        + m3([a, b, c], [e, f, g], [i, j, k])
                };

                let det4 = {
                    let mat = [[a, b, c, d], [e, f, g, h], [i, j, k, l], [m, nn, o, p]];
                    let minor_det = |r: [[f64; 3]; 3]| -> f64 {
                        r[0][0] * (r[1][1] * r[2][2] - r[1][2] * r[2][1])
                            - r[0][1] * (r[1][0] * r[2][2] - r[1][2] * r[2][0])
                            + r[0][2] * (r[1][0] * r[2][1] - r[1][1] * r[2][0])
                    };
                    let mut det = 0.0;
                    for col in 0..4 {
                        let mut sub = [[0.0; 3]; 3];
                        for row in 1..4 {
                            let mut sc = 0;
                            for (cc, &val) in mat[row].iter().enumerate() {
                                if cc == col {
                                    continue;
                                }
                                sub[row - 1][sc] = val;
                                sc += 1;
                            }
                        }
                        let sign = if col % 2 == 0 { 1.0 } else { -1.0 };
                        det += sign * mat[0][col] * minor_det(sub);
                    }
                    det
                };

                // λ⁴ - trace·λ³ + sum2·λ² - det3sum·λ + det4 = 0
                crate::expression::solve_quartic_f64_pub(1.0, -trace, sum2, -det3sum, det4)
            }
            _ => return None,
        };

        if roots.is_empty() {
            return None;
        }

        Some(
            roots
                .into_iter()
                .map(|r| Node::Num(ExactNum::from_f64(r)))
                .collect(),
        )
    }

    fn eigenvalues_numerical(
        &self,
        char_poly: &crate::polynomial::Polynomial,
    ) -> Option<Vec<Node>> {
        let deg = char_poly.degree()?;
        if deg > 4 {
            return None;
        }
        let coeff = |i: usize| -> f64 { char_poly.coeff(i).to_f64().unwrap_or(0.0) };
        let roots = match deg {
            1 => {
                let a = coeff(1);
                let b = coeff(0);
                if a.abs() < 1e-15 {
                    return None;
                }
                vec![-b / a]
            }
            2 => {
                let a = coeff(2);
                let b = coeff(1);
                let c = coeff(0);
                let disc = b * b - 4.0 * a * c;
                if disc < -1e-10 {
                    return None;
                }
                let disc = disc.max(0.0).sqrt();
                vec![(-b + disc) / (2.0 * a), (-b - disc) / (2.0 * a)]
            }
            3 => crate::expression::solve_cubic_f64_pub(coeff(3), coeff(2), coeff(1), coeff(0)),
            4 => crate::expression::solve_quartic_f64_pub(
                coeff(4),
                coeff(3),
                coeff(2),
                coeff(1),
                coeff(0),
            ),
            _ => return None,
        };
        if roots.is_empty() {
            return None;
        }
        Some(
            roots
                .into_iter()
                .map(|r| Node::Num(ExactNum::from_f64(r)))
                .collect(),
        )
    }

    /// Symbolic eigenvalue computation for matrices with variable entries.
    fn eigenvalues_symbolic(&self, env: &Environment) -> Result<Vec<Node>, String> {
        match self.rows {
            1 => Ok(vec![self.elements[0].clone()]),
            2 => self.eigenvalues_symbolic_2x2(env),
            3 => self.eigenvalues_symbolic_3x3(env),
            _ => Err("Symbolic eigenvalues for 4×4+ not yet implemented".to_string()),
        }
    }

    /// Symbolic eigenvalues for a 2×2 matrix via the quadratic formula.
    fn eigenvalues_symbolic_2x2(&self, env: &Environment) -> Result<Vec<Node>, String> {
        let a = &self.elements[0];
        let d = &self.elements[3];
        let b = &self.elements[1];
        let c = &self.elements[2];

        let trace = Node::Add(Box::new(a.clone()), Box::new(d.clone())).simplify(env)?;
        let det = Node::Subtract(
            Box::new(Node::Multiply(Box::new(a.clone()), Box::new(d.clone()))),
            Box::new(Node::Multiply(Box::new(b.clone()), Box::new(c.clone()))),
        )
        .simplify(env)?;

        let trace_sq = Node::Power(
            Box::new(trace.clone()),
            Box::new(Node::Num(ExactNum::integer(2))),
        )
        .simplify(env)?;
        let four_det = Node::Multiply(Box::new(Node::Num(ExactNum::integer(4))), Box::new(det))
            .simplify(env)?;
        let discriminant = Node::Subtract(Box::new(trace_sq), Box::new(four_det)).simplify(env)?;
        let sqrt_disc = Node::Sqrt(Box::new(discriminant)).simplify(env)?;

        let two = Node::Num(ExactNum::integer(2));
        let lambda1 = Node::Divide(
            Box::new(Node::Add(
                Box::new(trace.clone()),
                Box::new(sqrt_disc.clone()),
            )),
            Box::new(two.clone()),
        )
        .simplify(env)?;
        let lambda2 = Node::Divide(
            Box::new(Node::Subtract(Box::new(trace), Box::new(sqrt_disc))),
            Box::new(two),
        )
        .simplify(env)?;

        Ok(vec![lambda1, lambda2])
    }

    /// Symbolic eigenvalues for a 3×3 matrix.
    /// Tries to find a root among row sums, column sums, and diagonal elements,
    /// then deflates to a quadratic.
    fn eigenvalues_symbolic_3x3(&self, env: &Environment) -> Result<Vec<Node>, String> {
        let lambda_var = "__lambda__";
        let lambda = Node::Variable(lambda_var.to_string());

        // Build det(A - λI)
        let mut shifted = self.elements.clone();
        for i in 0..self.rows {
            let idx = i * self.cols + i;
            shifted[idx] = Node::Subtract(Box::new(shifted[idx].clone()), Box::new(lambda.clone()));
        }
        let shifted_matrix = Matrix {
            rows: self.rows,
            cols: self.cols,
            elements: shifted,
        };
        let det_expr = shifted_matrix.determinant(env)?;
        let det_simplified = det_expr.simplify(env).unwrap_or(det_expr);

        // Compute trace s₁, sum of 2×2 principal minors s₂, determinant s₃
        let s1 = Node::Add(
            Box::new(Node::Add(
                Box::new(self.elements[0].clone()),
                Box::new(self.elements[4].clone()),
            )),
            Box::new(self.elements[8].clone()),
        )
        .simplify(env)?;

        // s₂ = (a00*a11 - a01*a10) + (a00*a22 - a02*a20) + (a11*a22 - a12*a21)
        let minor01 = Node::Subtract(
            Box::new(Node::Multiply(
                Box::new(self.elements[0].clone()),
                Box::new(self.elements[4].clone()),
            )),
            Box::new(Node::Multiply(
                Box::new(self.elements[1].clone()),
                Box::new(self.elements[3].clone()),
            )),
        )
        .simplify(env)?;
        let minor02 = Node::Subtract(
            Box::new(Node::Multiply(
                Box::new(self.elements[0].clone()),
                Box::new(self.elements[8].clone()),
            )),
            Box::new(Node::Multiply(
                Box::new(self.elements[2].clone()),
                Box::new(self.elements[6].clone()),
            )),
        )
        .simplify(env)?;
        let minor12 = Node::Subtract(
            Box::new(Node::Multiply(
                Box::new(self.elements[4].clone()),
                Box::new(self.elements[8].clone()),
            )),
            Box::new(Node::Multiply(
                Box::new(self.elements[5].clone()),
                Box::new(self.elements[7].clone()),
            )),
        )
        .simplify(env)?;
        let s2 = Node::Add(
            Box::new(Node::Add(Box::new(minor01), Box::new(minor02))),
            Box::new(minor12),
        )
        .simplify(env)?;

        let _s3 = self.determinant(env)?;

        // Generate candidate eigenvalues
        let mut candidates: Vec<Node> = Vec::new();

        // Row sums
        let row_sums: Vec<Node> = (0..3)
            .map(|i| {
                let sum = self.elements[i * 3..i * 3 + 3]
                    .iter()
                    .fold(Node::Num(ExactNum::zero()), |acc, elem| {
                        Node::Add(Box::new(acc), Box::new(elem.clone()))
                    });
                sum.simplify(env).unwrap_or(sum)
            })
            .collect();

        // Check if all row sums are equal — if so, it's an eigenvalue
        let rs0_str = format!("{}", row_sums[0]);
        if row_sums[1..].iter().all(|rs| format!("{}", rs) == rs0_str) {
            candidates.push(row_sums[0].clone());
        }

        // Column sums
        let col_sums: Vec<Node> = (0..3)
            .map(|j| {
                let sum = (0..3).fold(Node::Num(ExactNum::zero()), |acc, i| {
                    Node::Add(Box::new(acc), Box::new(self.elements[i * 3 + j].clone()))
                });
                sum.simplify(env).unwrap_or(sum)
            })
            .collect();

        let cs0_str = format!("{}", col_sums[0]);
        if col_sums[1..].iter().all(|cs| format!("{}", cs) == cs0_str)
            && (cs0_str != rs0_str || !row_sums[1..].iter().all(|rs| format!("{}", rs) == rs0_str))
        {
            candidates.push(col_sums[0].clone());
        }

        // Diagonal elements as candidates
        for i in 0..3 {
            candidates.push(self.elements[i * 3 + i].clone());
        }

        // Node::Num(ExactNum::zero()) as a candidate
        candidates.push(Node::Num(ExactNum::zero()));

        // Verify each candidate by substituting into det(A - λI)
        for candidate in &candidates {
            let substituted = crate::substitute::substitute(
                &det_simplified,
                &[(lambda_var.to_string(), candidate.clone())],
            )?;
            let result = substituted.simplify(env).unwrap_or(substituted);
            if is_zero_node(&result) {
                // Found an eigenvalue r. Deflate to quadratic.
                let r = candidate;

                // Quadratic: λ² + (r - s₁)λ + (s₂ + r² - s₁·r) = 0
                let a_coeff =
                    Node::Subtract(Box::new(r.clone()), Box::new(s1.clone())).simplify(env)?;
                let r_sq = Node::Power(
                    Box::new(r.clone()),
                    Box::new(Node::Num(ExactNum::integer(2))),
                )
                .simplify(env)?;
                let s1_r =
                    Node::Multiply(Box::new(s1.clone()), Box::new(r.clone())).simplify(env)?;
                let b_coeff = Node::Add(
                    Box::new(Node::Subtract(
                        Box::new(Node::Add(Box::new(s2.clone()), Box::new(r_sq))),
                        Box::new(s1_r),
                    )),
                    Box::new(Node::Num(ExactNum::zero())),
                )
                .simplify(env)?;

                // Discriminant: a_coeff² - 4·b_coeff
                let a_sq = Node::Power(
                    Box::new(a_coeff.clone()),
                    Box::new(Node::Num(ExactNum::integer(2))),
                )
                .simplify(env)?;
                let four_b =
                    Node::Multiply(Box::new(Node::Num(ExactNum::integer(4))), Box::new(b_coeff))
                        .simplify(env)?;
                let disc = Node::Subtract(Box::new(a_sq), Box::new(four_b)).simplify(env)?;

                if is_zero_node(&disc) {
                    // Double root
                    let neg_a = Node::Negate(Box::new(a_coeff)).simplify(env)?;
                    let double_root =
                        Node::Divide(Box::new(neg_a), Box::new(Node::Num(ExactNum::integer(2))))
                            .simplify(env)?;
                    return Ok(vec![r.clone(), double_root.clone(), double_root]);
                }

                let sqrt_disc = Node::Sqrt(Box::new(disc)).simplify(env)?;
                let neg_a = Node::Negate(Box::new(a_coeff)).simplify(env)?;
                let two = Node::Num(ExactNum::integer(2));

                let lambda2 = Node::Divide(
                    Box::new(Node::Add(
                        Box::new(neg_a.clone()),
                        Box::new(sqrt_disc.clone()),
                    )),
                    Box::new(two.clone()),
                )
                .simplify(env)?;
                let lambda3 = Node::Divide(
                    Box::new(Node::Subtract(Box::new(neg_a), Box::new(sqrt_disc))),
                    Box::new(two),
                )
                .simplify(env)?;

                return Ok(vec![r.clone(), lambda2, lambda3]);
            }
        }

        Err("Could not find symbolic eigenvalues: no candidate root verified".to_string())
    }
}

/// Check whether a Node expression represents zero.
fn is_zero_node(node: &Node) -> bool {
    match node {
        Node::Num(n) => n.to_f64() == 0.0,
        _ => format!("{}", node) == "0",
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
    use crate::exact::ExactNum;
    use crate::node::Node;

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
    fn test_matrix_from_elements() {
        let elements = vec![
            vec![
                Node::Num(ExactNum::integer(1)),
                Node::Num(ExactNum::integer(2)),
            ],
            vec![
                Node::Num(ExactNum::integer(3)),
                Node::Num(ExactNum::integer(4)),
            ],
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
                match &identity.elements[i * 3 + j] {
                    Node::Num(n) => assert_eq!(n.to_f64(), expected),
                    _ => panic!("Expected Num node"),
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

        // Check elements
        match &transposed.elements[0] {
            Node::Num(n) => assert_eq!(n.to_f64(), 1.0),
            _ => panic!("Expected Num node"),
        }
        match &transposed.elements[1] {
            Node::Num(n) => assert_eq!(n.to_f64(), 4.0),
            _ => panic!("Expected Num node"),
        }
        match &transposed.elements[2] {
            Node::Num(n) => assert_eq!(n.to_f64(), 2.0),
            _ => panic!("Expected Num node"),
        }
        match &transposed.elements[3] {
            Node::Num(n) => assert_eq!(n.to_f64(), 5.0),
            _ => panic!("Expected Num node"),
        }
        match &transposed.elements[4] {
            Node::Num(n) => assert_eq!(n.to_f64(), 3.0),
            _ => panic!("Expected Num node"),
        }
        match &transposed.elements[5] {
            Node::Num(n) => assert_eq!(n.to_f64(), 6.0),
            _ => panic!("Expected Num node"),
        }
    }

    #[test]
    fn test_matrix_determinant() {
        let env = Environment::default();

        // 2x2 matrix
        let elements_2x2 = vec![
            Node::Num(ExactNum::integer(1)),
            Node::Num(ExactNum::integer(2)),
            Node::Num(ExactNum::integer(3)),
            Node::Num(ExactNum::integer(4)),
        ];
        let matrix_2x2 = Matrix::new(2, 2, elements_2x2).unwrap();

        // Determinant should be 1*4 - 2*3 = 4 - 6 = -2
        let det = matrix_2x2.determinant(&env).unwrap();
        match det {
            Node::Num(n) => assert_eq!(n.to_f64(), -2.0),
            _ => panic!("Expected Num node"),
        }

        // 3x3 matrix
        let elements_3x3 = vec![
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
        let matrix_3x3 = Matrix::new(3, 3, elements_3x3).unwrap();

        // Determinant should be 0 for this matrix (it's singular)
        let det = matrix_3x3.determinant(&env).unwrap();
        match det {
            Node::Num(n) => assert!(n.to_f64().abs() < 1e-10),
            _ => panic!("Expected Num node"),
        }
    }

    #[test]
    fn test_matrix_multiplication() {
        let env = Environment::default();

        // 2x3 matrix
        let elements_a = vec![
            Node::Num(ExactNum::integer(1)),
            Node::Num(ExactNum::integer(2)),
            Node::Num(ExactNum::integer(3)),
            Node::Num(ExactNum::integer(4)),
            Node::Num(ExactNum::integer(5)),
            Node::Num(ExactNum::integer(6)),
        ];
        let matrix_a = Matrix::new(2, 3, elements_a).unwrap();

        // 3x2 matrix
        let elements_b = vec![
            Node::Num(ExactNum::integer(7)),
            Node::Num(ExactNum::integer(8)),
            Node::Num(ExactNum::integer(9)),
            Node::Num(ExactNum::integer(10)),
            Node::Num(ExactNum::integer(11)),
            Node::Num(ExactNum::integer(12)),
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
        let expected = [58.0, 64.0, 139.0, 154.0];
        for (i, &exp) in expected.iter().enumerate() {
            match &result.elements[i] {
                Node::Num(n) => assert_eq!(n.to_f64(), exp),
                _ => panic!("Expected Num node at index {}", i),
            }
        }
    }

    #[test]
    fn test_matrix_inverse() {
        let env = Environment::default();

        // 2x2 invertible matrix
        let elements = vec![
            Node::Num(ExactNum::integer(4)),
            Node::Num(ExactNum::integer(7)),
            Node::Num(ExactNum::integer(2)),
            Node::Num(ExactNum::integer(6)),
        ];
        let matrix = Matrix::new(2, 2, elements).unwrap();

        // Determinant is 4*6 - 7*2 = 24 - 14 = 10
        // Inverse should be [0.6 -0.7; -0.2 0.4]
        let inverse = matrix.inverse(&env).unwrap();

        let expected = [0.6, -0.7, -0.2, 0.4];
        for (i, &exp) in expected.iter().enumerate() {
            match &inverse.elements[i] {
                Node::Num(n) => assert!((n.to_f64() - exp).abs() < 1e-10),
                _ => panic!("Expected Num node at index {}", i),
            }
        }

        // Check that A * A^-1 = I
        let identity = matrix.multiply(&inverse, &env).unwrap();
        for i in 0..2 {
            for j in 0..2 {
                let expected = if i == j { 1.0 } else { 0.0 };
                match &identity.elements[i * 2 + j] {
                    Node::Num(n) => assert!((n.to_f64() - expected).abs() < 1e-10),
                    _ => panic!("Expected Num node"),
                }
            }
        }
    }

    #[test]
    fn test_singular_matrix_inverse() {
        let env = Environment::default();

        // Singular 2x2 matrix (determinant = 0)
        let elements = vec![
            Node::Num(ExactNum::integer(1)),
            Node::Num(ExactNum::integer(2)),
            Node::Num(ExactNum::integer(2)),
            Node::Num(ExactNum::integer(4)),
        ];
        let matrix = Matrix::new(2, 2, elements).unwrap();

        // Inverse should fail
        let result = matrix.inverse(&env);
        assert!(result.is_err());
    }

    #[test]
    fn test_matrix_rank() {
        let env = Environment::default();

        // Full rank 2x2 matrix
        let elements_full_rank = vec![
            Node::Num(ExactNum::integer(1)),
            Node::Num(ExactNum::integer(2)),
            Node::Num(ExactNum::integer(3)),
            Node::Num(ExactNum::integer(4)),
        ];
        let matrix_full_rank = Matrix::new(2, 2, elements_full_rank).unwrap();
        assert_eq!(matrix_full_rank.rank(&env).unwrap(), 2);

        // Rank 1 matrix (second row is multiple of first)
        let elements_rank_1 = vec![
            Node::Num(ExactNum::integer(1)),
            Node::Num(ExactNum::integer(2)),
            Node::Num(ExactNum::integer(2)),
            Node::Num(ExactNum::integer(4)),
        ];
        let matrix_rank_1 = Matrix::new(2, 2, elements_rank_1).unwrap();
        assert_eq!(matrix_rank_1.rank(&env).unwrap(), 1);

        // Rank 2 matrix (3x3 but not full rank)
        let elements_rank_2 = vec![
            Node::Num(ExactNum::integer(1)),
            Node::Num(ExactNum::zero()),
            Node::Num(ExactNum::integer(1)),
            Node::Num(ExactNum::integer(2)),
            Node::Num(ExactNum::one()),
            Node::Num(ExactNum::integer(3)),
            Node::Num(ExactNum::integer(3)),
            Node::Num(ExactNum::one()),
            Node::Num(ExactNum::integer(4)),
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
            Node::Num(n) => assert_eq!(n.to_f64(), 1.0),
            _ => panic!("Expected Num node"),
        }
        match &matrix.elements[1] {
            Node::Num(n) => assert_eq!(n.to_f64(), 2.0),
            _ => panic!("Expected Num node"),
        }
        match &matrix.elements[2] {
            Node::Num(n) => assert_eq!(n.to_f64(), 3.0),
            _ => panic!("Expected Num node"),
        }
        match &matrix.elements[3] {
            Node::Num(n) => assert_eq!(n.to_f64(), 4.0),
            _ => panic!("Expected Num node"),
        }
    }

    #[test]
    fn test_matrix_eigenvalues() {
        let env = Environment::default();

        // Matrix with eigenvalues 1 and 3
        let elements = vec![
            Node::Num(ExactNum::integer(2)),
            Node::Num(-ExactNum::one()),
            Node::Num(-ExactNum::one()),
            Node::Num(ExactNum::integer(2)),
        ];
        let matrix = Matrix::new(2, 2, elements).unwrap();

        let eigenvalues = matrix.eigenvalues(&env).unwrap();
        assert_eq!(eigenvalues.len(), 2);

        // Sort eigenvalues to make testing easier
        let mut sorted_eigenvalues = eigenvalues;
        sorted_eigenvalues.sort_by(|a, b| {
            if let (Node::Num(x), Node::Num(y)) = (a, b) {
                x.to_f64().partial_cmp(&y.to_f64()).unwrap()
            } else {
                panic!("Expected Num nodes")
            }
        });

        match &sorted_eigenvalues[0] {
            Node::Num(n) => assert!((n.to_f64() - 1.0).abs() < 1e-10),
            _ => panic!("Expected Num node"),
        }
        match &sorted_eigenvalues[1] {
            Node::Num(n) => assert!((n.to_f64() - 3.0).abs() < 1e-10),
            _ => panic!("Expected Num node"),
        }
    }

    #[test]
    fn test_3x3_eigenvalues_identity() {
        let env = Environment::new();
        let elements = vec![
            Node::Num(ExactNum::integer(1)),
            Node::Num(ExactNum::zero()),
            Node::Num(ExactNum::zero()),
            Node::Num(ExactNum::zero()),
            Node::Num(ExactNum::integer(1)),
            Node::Num(ExactNum::zero()),
            Node::Num(ExactNum::zero()),
            Node::Num(ExactNum::zero()),
            Node::Num(ExactNum::integer(1)),
        ];
        let matrix = Matrix::new(3, 3, elements).unwrap();
        let eigenvalues = matrix.eigenvalues(&env).unwrap();
        assert_eq!(eigenvalues.len(), 3);
        for ev in &eigenvalues {
            match ev {
                Node::Num(n) => assert!(
                    (n.to_f64() - 1.0).abs() < 1e-10,
                    "Expected 1, got {}",
                    n.to_f64()
                ),
                _ => panic!("Expected Num node"),
            }
        }
    }

    #[test]
    fn test_3x3_eigenvalues_diagonal() {
        let env = Environment::new();
        let elements = vec![
            Node::Num(ExactNum::integer(2)),
            Node::Num(ExactNum::zero()),
            Node::Num(ExactNum::zero()),
            Node::Num(ExactNum::zero()),
            Node::Num(ExactNum::integer(3)),
            Node::Num(ExactNum::zero()),
            Node::Num(ExactNum::zero()),
            Node::Num(ExactNum::zero()),
            Node::Num(ExactNum::integer(5)),
        ];
        let matrix = Matrix::new(3, 3, elements).unwrap();
        let mut eigenvalues = matrix.eigenvalues(&env).unwrap();
        assert_eq!(eigenvalues.len(), 3);
        eigenvalues.sort_by(|a, b| {
            if let (Node::Num(x), Node::Num(y)) = (a, b) {
                x.to_f64().partial_cmp(&y.to_f64()).unwrap()
            } else {
                panic!("Expected Num nodes")
            }
        });
        let vals: Vec<f64> = eigenvalues
            .iter()
            .map(|ev| {
                if let Node::Num(n) = ev {
                    n.to_f64()
                } else {
                    panic!()
                }
            })
            .collect();
        assert!((vals[0] - 2.0).abs() < 1e-10);
        assert!((vals[1] - 3.0).abs() < 1e-10);
        assert!((vals[2] - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_3x3_eigenvalues_symmetric() {
        // [[2, -1, 0], [-1, 2, -1], [0, -1, 2]]
        // Eigenvalues: 2-√2, 2, 2+√2
        let env = Environment::new();
        let two = Node::Num(ExactNum::integer(2));
        let neg1 = Node::Num(-ExactNum::one());
        let zero = Node::Num(ExactNum::zero());
        let elements = vec![
            two.clone(),
            neg1.clone(),
            zero.clone(),
            neg1.clone(),
            two.clone(),
            neg1.clone(),
            zero,
            neg1,
            two,
        ];
        let matrix = Matrix::new(3, 3, elements).unwrap();
        let mut eigenvalues = matrix.eigenvalues(&env).unwrap();
        assert_eq!(eigenvalues.len(), 3);
        eigenvalues.sort_by(|a, b| {
            if let (Node::Num(x), Node::Num(y)) = (a, b) {
                x.to_f64().partial_cmp(&y.to_f64()).unwrap()
            } else {
                panic!("Expected Num nodes")
            }
        });
        let vals: Vec<f64> = eigenvalues
            .iter()
            .map(|ev| {
                if let Node::Num(n) = ev {
                    n.to_f64()
                } else {
                    panic!()
                }
            })
            .collect();
        let sqrt2 = std::f64::consts::SQRT_2;
        assert!((vals[0] - (2.0 - sqrt2)).abs() < 1e-8, "Got {}", vals[0]);
        assert!((vals[1] - 2.0).abs() < 1e-8, "Got {}", vals[1]);
        assert!((vals[2] - (2.0 + sqrt2)).abs() < 1e-8, "Got {}", vals[2]);
    }

    #[test]
    fn test_4x4_eigenvalues_diagonal() {
        let env = Environment::new();
        let elements = vec![
            Node::Num(ExactNum::integer(1)),
            Node::Num(ExactNum::zero()),
            Node::Num(ExactNum::zero()),
            Node::Num(ExactNum::zero()),
            Node::Num(ExactNum::zero()),
            Node::Num(ExactNum::integer(2)),
            Node::Num(ExactNum::zero()),
            Node::Num(ExactNum::zero()),
            Node::Num(ExactNum::zero()),
            Node::Num(ExactNum::zero()),
            Node::Num(ExactNum::integer(3)),
            Node::Num(ExactNum::zero()),
            Node::Num(ExactNum::zero()),
            Node::Num(ExactNum::zero()),
            Node::Num(ExactNum::zero()),
            Node::Num(ExactNum::integer(4)),
        ];
        let matrix = Matrix::new(4, 4, elements).unwrap();
        let mut eigenvalues = matrix.eigenvalues(&env).unwrap();
        assert_eq!(eigenvalues.len(), 4);
        eigenvalues.sort_by(|a, b| {
            if let (Node::Num(x), Node::Num(y)) = (a, b) {
                x.to_f64().partial_cmp(&y.to_f64()).unwrap()
            } else {
                panic!("Expected Num nodes")
            }
        });
        let vals: Vec<f64> = eigenvalues
            .iter()
            .map(|ev| {
                if let Node::Num(n) = ev {
                    n.to_f64()
                } else {
                    panic!()
                }
            })
            .collect();
        assert!((vals[0] - 1.0).abs() < 1e-10);
        assert!((vals[1] - 2.0).abs() < 1e-10);
        assert!((vals[2] - 3.0).abs() < 1e-10);
        assert!((vals[3] - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_characteristic_polynomial_3x3() {
        let env = Environment::new();
        let elements = vec![
            Node::Num(ExactNum::integer(2)),
            Node::Num(ExactNum::zero()),
            Node::Num(ExactNum::zero()),
            Node::Num(ExactNum::zero()),
            Node::Num(ExactNum::integer(3)),
            Node::Num(ExactNum::zero()),
            Node::Num(ExactNum::zero()),
            Node::Num(ExactNum::zero()),
            Node::Num(ExactNum::integer(5)),
        ];
        let matrix = Matrix::new(3, 3, elements).unwrap();
        let char_poly = matrix.characteristic_polynomial(&env).unwrap();
        // det(A - λI) = (2-λ)(3-λ)(5-λ) = -λ³ + 10λ² - 31λ + 30
        assert_eq!(char_poly.degree(), Some(3));
    }

    #[test]
    fn test_symbolic_eigenvalues_2x2() {
        let env = Environment::new();
        let a = Node::Variable("a".to_string());
        let b = Node::Variable("b".to_string());
        let elements = vec![a.clone(), b.clone(), Node::Num(ExactNum::zero()), a.clone()];
        let matrix = Matrix::new(2, 2, elements).unwrap();
        let eigenvalues = matrix.eigenvalues(&env).unwrap();
        assert_eq!(
            eigenvalues.len(),
            2,
            "2x2 symbolic should have 2 eigenvalues: {:?}",
            eigenvalues
        );
    }

    #[test]
    fn test_symbolic_eigenvalues_3x3_circulant() {
        // [[1, α, α], [α, 1, α], [α, α, 1]]
        // Eigenvalues: 1+2α (mult 1), 1-α (mult 2)
        let env = Environment::new();
        let one = Node::Num(ExactNum::one());
        let alpha = Node::Variable("α".to_string());
        let elements = vec![
            one.clone(),
            alpha.clone(),
            alpha.clone(),
            alpha.clone(),
            one.clone(),
            alpha.clone(),
            alpha.clone(),
            alpha.clone(),
            one.clone(),
        ];
        let matrix = Matrix::new(3, 3, elements).unwrap();
        let eigenvalues = matrix.eigenvalues(&env).unwrap();
        assert_eq!(
            eigenvalues.len(),
            3,
            "3x3 circulant should have 3 eigenvalues: {:?}",
            eigenvalues
        );

        // Verify numerically at α=0.3: eigenvalues should be 1.6, 0.7, 0.7
        let mut test_env = Environment::new();
        test_env.set("α", 0.3);
        let mut vals: Vec<f64> = eigenvalues
            .iter()
            .map(|ev| crate::evaluator::Evaluator::evaluate(ev, &test_env).unwrap())
            .collect();
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!(
            (vals[0] - 0.7).abs() < 0.01,
            "Expected 0.7, got {}",
            vals[0]
        );
        assert!(
            (vals[1] - 0.7).abs() < 0.01,
            "Expected 0.7, got {}",
            vals[1]
        );
        assert!(
            (vals[2] - 1.6).abs() < 0.01,
            "Expected 1.6, got {}",
            vals[2]
        );
    }

    #[test]
    fn test_symbolic_eigenvalues_3x3_diagonal() {
        // [[a, 0, 0], [0, b, 0], [0, 0, c]]
        // Eigenvalues: a, b, c
        let env = Environment::new();
        let a = Node::Variable("a".to_string());
        let b = Node::Variable("b".to_string());
        let c = Node::Variable("c".to_string());
        let zero = Node::Num(ExactNum::zero());
        let elements = vec![
            a.clone(),
            zero.clone(),
            zero.clone(),
            zero.clone(),
            b.clone(),
            zero.clone(),
            zero.clone(),
            zero.clone(),
            c.clone(),
        ];
        let matrix = Matrix::new(3, 3, elements).unwrap();
        let eigenvalues = matrix.eigenvalues(&env).unwrap();
        assert_eq!(
            eigenvalues.len(),
            3,
            "3x3 diagonal should have 3 eigenvalues: {:?}",
            eigenvalues
        );
    }

    #[test]
    fn test_4x4_eigenvalues_symmetric() {
        // [[2,-1,0,0],[-1,2,-1,0],[0,-1,2,-1],[0,0,-1,2]]
        // Tridiagonal Toeplitz: eigenvalues = 2-2cos(kπ/5) for k=1..4
        let env = Environment::new();
        let two = Node::Num(ExactNum::integer(2));
        let neg1 = Node::Num(-ExactNum::one());
        let zero = Node::Num(ExactNum::zero());
        let elements = vec![
            two.clone(),
            neg1.clone(),
            zero.clone(),
            zero.clone(),
            neg1.clone(),
            two.clone(),
            neg1.clone(),
            zero.clone(),
            zero.clone(),
            neg1.clone(),
            two.clone(),
            neg1.clone(),
            zero.clone(),
            zero.clone(),
            neg1.clone(),
            two.clone(),
        ];
        let matrix = Matrix::new(4, 4, elements).unwrap();
        let mut eigenvalues = matrix.eigenvalues(&env).unwrap();
        assert_eq!(
            eigenvalues.len(),
            4,
            "Expected 4 eigenvalues, got {:?}",
            eigenvalues
        );
        eigenvalues.sort_by(|a, b| {
            if let (Node::Num(x), Node::Num(y)) = (a, b) {
                x.to_f64().partial_cmp(&y.to_f64()).unwrap()
            } else {
                panic!("Expected Num nodes")
            }
        });
        let vals: Vec<f64> = eigenvalues
            .iter()
            .map(|ev| {
                if let Node::Num(n) = ev {
                    n.to_f64()
                } else {
                    panic!()
                }
            })
            .collect();
        let pi = std::f64::consts::PI;
        for (i, &v) in vals.iter().enumerate() {
            let expected = 2.0 - 2.0 * ((i as f64 + 1.0) * pi / 5.0).cos();
            assert!(
                (v - expected).abs() < 1e-6,
                "Eigenvalue {} = {}, expected {}",
                i,
                v,
                expected
            );
        }
    }

    #[test]
    fn test_4x4_eigenvalues_companion() {
        // Companion matrix for x⁴-10x²+1=0 (roots: ±√(5±2√6) ≈ ±3.146, ±0.318)
        let env = Environment::new();
        let zero = Node::Num(ExactNum::zero());
        let one = Node::Num(ExactNum::one());
        let elements = vec![
            zero.clone(),
            zero.clone(),
            zero.clone(),
            Node::Num(-ExactNum::one()),
            one.clone(),
            zero.clone(),
            zero.clone(),
            zero.clone(),
            zero.clone(),
            one.clone(),
            zero.clone(),
            Node::Num(ExactNum::integer(10)),
            zero.clone(),
            zero.clone(),
            one.clone(),
            zero.clone(),
        ];
        let matrix = Matrix::new(4, 4, elements).unwrap();
        let eigenvalues = matrix.eigenvalues(&env).unwrap();
        assert!(
            eigenvalues.len() == 4,
            "Expected 4 eigenvalues for companion matrix, got {}",
            eigenvalues.len()
        );
    }

    #[test]
    fn test_4x4_eigenvalues_integer() {
        // [[5,4,1,1],[4,5,1,1],[1,1,4,2],[1,1,2,4]]
        // All integer eigenvalues: 1, 2, 5, 10
        let env = Environment::new();
        let n = |v: i64| Node::Num(ExactNum::integer(v));
        let elements = vec![
            n(5),
            n(4),
            n(1),
            n(1),
            n(4),
            n(5),
            n(1),
            n(1),
            n(1),
            n(1),
            n(4),
            n(2),
            n(1),
            n(1),
            n(2),
            n(4),
        ];
        let matrix = Matrix::new(4, 4, elements).unwrap();
        let mut eigenvalues = matrix.eigenvalues(&env).unwrap();
        assert_eq!(eigenvalues.len(), 4);
        eigenvalues.sort_by(|a, b| {
            if let (Node::Num(x), Node::Num(y)) = (a, b) {
                x.to_f64().partial_cmp(&y.to_f64()).unwrap()
            } else {
                panic!()
            }
        });
        let vals: Vec<f64> = eigenvalues
            .iter()
            .map(|ev| {
                if let Node::Num(n) = ev {
                    n.to_f64()
                } else {
                    panic!()
                }
            })
            .collect();
        assert!((vals[0] - 1.0).abs() < 1e-8, "Expected 1, got {}", vals[0]);
        assert!((vals[1] - 2.0).abs() < 1e-8, "Expected 2, got {}", vals[1]);
        assert!((vals[2] - 5.0).abs() < 1e-8, "Expected 5, got {}", vals[2]);
        assert!(
            (vals[3] - 10.0).abs() < 1e-8,
            "Expected 10, got {}",
            vals[3]
        );
    }
}
