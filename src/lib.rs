// Declare the node module
mod node;
pub use crate::node::Node;

// Declare the environment module and make Environment public
mod environment;
pub use crate::environment::Environment;

// Declare the evaluator module
mod evaluator; // Declare evaluator module
pub use crate::evaluator::Evaluator; // Re-export Evaluator so it can be used elsewhere

mod tokenizer;
pub use crate::tokenizer::Tokenizer;

mod parser; // Add this to lib.rs
pub use crate::parser::{build_expression_tree, shunting_yard};

pub mod expression;
pub use crate::expression::solve_for_variable;

mod wasm_bindings;
pub use crate::wasm_bindings::evaluate_latex_expression_js;

mod functions;
pub use crate::functions::FUNCTION_REGISTRY;

pub mod simplify;

// Declare the substitution module
pub mod substitute;
pub use crate::substitute::{substitute, substitute_latex};

// Declare the derivative module
pub mod derivative;
pub use crate::derivative::{differentiate, differentiate_latex, partial_derivative};

// Declare the composition module
pub mod composition;
pub use crate::composition::{compose, compose_latex, compose_multiple};

// Declare the integration module
pub mod integration;
pub use crate::integration::{
    definite_integral, definite_integral_latex, integrate, integrate_latex,
};
