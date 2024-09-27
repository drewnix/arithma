// Declare the node module
mod node;
pub use crate::node::Node;

// Declare the environment module and make Environment public
mod environment;
pub use crate::environment::Environment;

// Declare the evaluator module
mod evaluator; // Declare evaluator module
pub use crate::evaluator::Evaluator; // Re-export Evaluator so it can be used elsewhere

mod parser; // Add this to lib.rs
pub use crate::parser::{build_expression_tree, shunting_yard, tokenize};

pub mod expression;
pub use crate::expression::{evaluate_rpn, solve_for_variable};

mod wasm_bindings;
pub use crate::wasm_bindings::evaluate_latex_expression_js;

mod functions;
pub use crate::functions::LATEX_FUNCTIONS;

pub mod simplify;
