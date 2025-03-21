# Arithma Developer Guide

## Build, Lint & Test Commands

### Rust (Backend)
- Build: `cargo build`
- Run: `cargo run`
- Test all: `cargo test`
- Test specific: `cargo test algebra_tests::test_basic_operations`
- Format: `cargo fmt`
- Lint: `cargo clippy`
- Build WASM: `wasm-pack build --target web`

### TypeScript/React (Frontend)
- Install: `cd frontend && npm install`
- Dev server: `cd frontend && npm run dev`
- Build: `cd frontend && npm run build`
- Lint: `cd frontend && npm run lint`
- Storybook: `cd frontend && npm run storybook`

## Code Style Guidelines

### Rust
- Use snake_case for variable/function names and CamelCase for types/enums
- Prefer Result<T, String> for error handling with descriptive messages
- Implement traits for new types when appropriate
- Use docstrings (///) for public functions and structs
- Organize code with modules reflecting functionality domains
- Handle edge cases like division by zero by returning NaN, not errors

### TypeScript/React
- Use TypeScript interfaces for props and state
- Follow functional component patterns with React hooks
- Use tailwind for styling with class-variance-authority for variants
- Prefer named exports over default exports
- Keep components small and focused on a single responsibility