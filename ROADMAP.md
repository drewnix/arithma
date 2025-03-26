# Arithma Development Roadmap

This roadmap outlines the planned development path for transforming Arithma into a comprehensive Computer Algebra System (CAS) capable of handling advanced mathematical operations for calculus and linear algebra.

## Core Infrastructure

- [x] Basic arithmetic operations
- [x] Variable support
- [x] Equations
- [x] LaTeX parsing and rendering
- [x] Summation notation
- [x] Matrix notation
- [ ] Improve error handling with location information
- [x] Add comprehensive documentation
- [x] Implement unit testing framework with property-based tests

## Expression Manipulation

- [x] Advanced simplification strategies
  - [x] Trigonometric identities
  - [x] Logarithmic properties
  - [x] Algebraic factoring and expansion
  - [ ] Partial fraction decomposition
  - [ ] Polynomial division
- [x] Expression canonicalization
- [x] Variable substitution 
- [x] Function composition
- [ ] Pattern matching for algebraic manipulation

## Calculus

### Differentiation
- [x] Basic differentiation rules
  - [x] Power rule
  - [x] Product rule
  - [x] Quotient rule
  - [x] Chain rule
- [x] Differentiation of elementary functions
  - [x] Trigonometric
  - [x] Exponential
  - [x] Logarithmic
- [ ] Implicit differentiation
- [x] Partial derivatives
- [ ] Higher-order derivatives
- [ ] Directional derivatives and gradients

### Integration
- [x] Basic integration techniques
  - [x] Direct integration of elementary functions
  - [x] Integration by substitution
  - [ ] Integration by parts
- [x] Special function integration
- [x] Definite integrals
- [ ] Improper integrals
- [ ] Numerical integration methods
- [ ] Multiple integrals

### Differential Equations
- [ ] First-order ODEs
  - [ ] Separable equations
  - [ ] Linear equations
- [ ] Second-order ODEs with constant coefficients
- [ ] Systems of ODEs
- [ ] Basic PDEs

### Limits and Series
- [ ] Limit evaluation
- [ ] Series expansion (Taylor, Maclaurin)
- [ ] Convergence tests
- [ ] Power series operations
- [ ] Laurent series

## Linear Algebra

### Matrix Operations
- [x] Matrix representation and display
- [x] Basic operations (addition, multiplication)
  - [x] Support for LaTeX matrix notation with \begin{pmatrix} and \cdot
- [x] Determinant calculation
- [x] Matrix inversion
- [x] Eigenvalue and eigenvector computation
- [ ] LU decomposition
- [ ] QR decomposition
- [ ] Singular value decomposition (SVD)

### Vector Operations
- [ ] Vector representation
- [ ] Dot and cross products
- [ ] Vector projections
- [ ] Basis and dimension
- [ ] Vector spaces and subspaces

### Linear Systems
- [x] Solving systems of linear equations
- [x] Gaussian elimination
- [x] Row echelon form
- [x] Matrix rank calculations
- [ ] Nullspace and range

## Advanced Topics

### Optimization
- [ ] Gradient descent methods
- [ ] Lagrange multipliers
- [ ] Linear programming
- [ ] Nonlinear optimization

### Number Theory
- [ ] Prime number operations
- [ ] Modular arithmetic
- [ ] Extended GCD algorithm
- [ ] Diophantine equations

### Visualization
- [ ] Function plotting in 2D
- [ ] Surface and contour plots in 3D
- [ ] Vector field visualization

### Symbolic Computation
- [ ] Exact arithmetic with fractions and irrationals
- [ ] Symbolic integration
- [ ] Gröbner basis computation
- [ ] Group theory operations

## Frontend and User Experience

- [x] Interactive expression editor
- [ ] Step-by-step solution display
- [ ] Notebook-style interface
- [x] Export capabilities (LaTeX)
- [x] Mobile-friendly interface
- [ ] Keyboard shortcuts
- [ ] History and saved expressions

## Performance and Optimization

- [ ] Expression caching
- [ ] Parallel computation
- [ ] Memory-efficient data structures
- [ ] Compilation to optimized bytecode
- [ ] GPU acceleration for matrix operations

## Integration and Extensibility

- [ ] Plugin system for extensions
- [ ] API for external access
- [ ] Integration with other scientific tools
- [ ] Import/export functionality for other CAS formats
- [ ] Language bindings for Python, Julia, etc.

## Implementation Priority

### Phase 1: Core Mathematical Foundation ✅
1. Complete expression manipulation framework ✅
2. Basic calculus operations (derivatives and simple integrals) ✅
3. Matrix representation and basic operations ✅

### Phase 2: Calculus Expansion 🚧
1. Advanced integration techniques ✅
2. Series expansions 🚧
3. Limits 🚧
4. Differential equations (basic) 🚧

### Phase 3: Linear Algebra 🚧
1. Complete matrix operations ✅
2. Eigenvalues and eigenvectors ✅
3. Matrix decompositions 🚧
4. Linear system solving ✅

### Phase 4: Advanced Features
1. Optimization algorithms
2. Symbolic computation
3. Visualization tools
4. Performance optimizations

### Phase 5: User Experience and Integration
1. Interactive frontend improvements
2. Step-by-step solutions
3. API and integrations
4. Documentation and examples

## Implementation and Testing Notes

### Recent Matrix Improvements (March 2025)
- Added support for parsing matrices in LaTeX notation with `\begin{pmatrix}` environments
- Implemented matrix multiplication with `\cdot` operator in LaTeX
- Fixed tokenizer to properly handle `&` as matrix cell separator
- Basic parsing tests are now passing, but more advanced operations need refinement
- Some matrix operations (determinant, eigenvalues, rank, etc.) have implementations but require additional work