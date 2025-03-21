# Arithma 

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

Arithma is a Computer Algebra System (CAS) written in Rust with WebAssembly
support and a typescript front-end. It provides symbolic mathematics 
capabilities with a focus on elegance, performance, and extensibility.

## Features

- **Symbolic Mathematics**: Manipulate algebraic expressions, solve equations, perform calculus operations
- **LaTeX Integration**: Parse and render mathematical expressions in LaTeX format
- **Web Ready**: WebAssembly compilation for seamless integration with the Front End
- **Comprehensive Mathematical Capabilities**:
  - ✅ Variable substitution
  - ✅ Basic arithmetic operations
  - ✅ Function composition
  - ✅ Differentiation (including product, quotient, chain rules)
  - ✅ Basic integration techniques
  - ✅ Definite integrals

## Getting Started

### Prerequisites

- Rust (latest stable version)
- wasm-pack (for WebAssembly compilation)
- Node.js and npm (for the frontend)

### Installation

1. **Clone the repository**:
   ```
   git clone https://github.com/drewnix/arithma.git
   cd arithma
   ```

2. **Build the Rust backend**:
   ```
   cargo build
   ```

3. **Build the WebAssembly package**:
   ```
   wasm-pack build --target web
   ```

4. **Install frontend dependencies**:
   ```
   cd frontend
   npm install
   ```

### Running the Application

- **Backend CLI**:
  ```
  cargo run
  ```

- **Frontend development server**:
  ```
  cd frontend
  npm run dev
  ```

## Design and Architecture

Arithma is designed around a core expression tree (AST) representation with 
modules for parsing, manipulation, and evaluation:

1. **Expression Representation**: Mathematical expressions are represented as 
abstract syntax trees (ASTs) using the `Node` enum
2. **Parsing**: LaTeX expressions are tokenized and parsed into AST nodes
3. **Manipulation**: Modules for differentiation, integration, substitution, and
simplification transform expression trees
4. **Evaluation**: Expressions can be numerically evaluated with the `Evaluator`
5. **WebAssembly**: WASM bindings expose functionality to JavaScript environments

## Development and Testing

### Build and Test Commands

#### Rust Backend
- Build: `cargo build`
- Run: `cargo run`
- Test all: `cargo test`
- Test specific: `cargo test algebra_tests::test_basic_operations`
- Format: `cargo fmt`
- Lint: `cargo clippy`
- Build WASM: `wasm-pack build --target web`

#### TypeScript/React Frontend
- Install: `cd frontend && npm install`
- Dev server: `cd frontend && npm run dev`
- Build: `cd frontend && npm run build`
- Lint: `cd frontend && npm run lint`
- Storybook: `cd frontend && npm run storybook`


## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- The project draws inspiration from established CAS projects like SymPy, Mathematica, and Maxima
- Thanks to the Rust, WebAssembly, and React communities for the excellent tools that make this project possible