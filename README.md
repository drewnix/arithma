# Arithma 

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://github.com/drewnix/arithma/actions/workflows/ci.yml/badge.svg)](https://github.com/drewnix/arithma/actions/workflows/ci.yml)

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
- Docker (for containerized deployment)
- Kubernetes CLI (kubectl) and Helm (for Kubernetes deployment)

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

- **Using Docker**:
  ```
  make docker-build             # Build the Docker image
  make docker-build-multiarch   # Build for both ARM64 and AMD64
  make docker-run               # Run the container locally
  make docker-publish           # Build, tag and push to DockerHub
  make docker-publish-multiarch # Build and push multi-arch image to DockerHub
  ```

### Deploying with Helm

Arithma can be deployed to a Kubernetes cluster using the provided Helm chart:

1. **Build the Docker image**:
   ```
   make docker-build
   ```

2. **Verify the Helm chart**:
   ```
   make helm-lint
   ```

3. **Deploy to Kubernetes**:
   ```
   make k8s-deploy
   ```

   This will:
   - Build the Docker image locally
   - Save the image to a tar file and copy it directly to your Kubernetes nodes
   - Set image pull policy to "Never" so Kubernetes uses the local image
   - Install the Helm chart with LoadBalancer service type


4. **Alternative deployment methods**:

   - Deploy using a registry (if you have one configured):
     ```
     make k8s-deploy-registry
     ```

   - Deploy using DockerHub (requires DockerHub credentials):
     ```
     make k8s-deploy-dockerhub
     ```

5. **Advanced deployment options**:

   - Preview the Kubernetes manifests:
     ```
     make helm-template
     ```

   - Upgrade an existing deployment:
     ```
     make helm-upgrade
     ```

   - Uninstall the application:
     ```
     make helm-uninstall
     ```

5. **Customizing the deployment**:

   Edit the `charts/arithma/values.yaml` file to customize:
   - Image repository and tag
   - Number of replicas
   - Resource limits
   - Ingress configuration
   - Service type (ClusterIP, NodePort, LoadBalancer)

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

### CI/CD Pipeline

Arithma uses GitHub Actions for continuous integration and deployment:

- **CI Workflow**: Automatically builds, lints, and tests both the Rust backend 
and TypeScript frontend on every push and pull request.
- **Nightly Builds**: Runs comprehensive tests, security audits, and 
cross-platform checks daily.
- **Release Pipeline**: Automatically creates GitHub releases with assets when 
version tags are pushed.

All workflows can be found in the `.github/workflows` directory.

## Roadmap

For detailed development plans, see the [ROADMAP.md](ROADMAP.md) file.

## Contributing

Contributions to Arithma are welcome, please feel free to submit pull requests, create issues, or suggest enhancements.

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/amazing-feature`
3. Commit your changes: `git commit -m 'Add some amazing feature'`
4. Push to the branch: `git push origin feature/amazing-feature`
5. Open a pull request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- The project draws inspiration from established CAS projects like SymPy, Mathematica, and Maxima
- Thanks to the Rust, WebAssembly, and React communities for the excellent tools that make this project possible