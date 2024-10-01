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
