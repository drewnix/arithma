#![allow(unexpected_cfgs)]

pub mod exact;
pub use crate::exact::ExactNum;

pub mod integer;
pub use crate::integer::{extract_square_factors, prime_factorize, prime_factorize_latex};

mod node;
pub use crate::node::Node;

pub mod assumptions;
pub use crate::assumptions::Assumptions;

// Declare the environment module and make Environment public
mod environment;
pub use crate::environment::Environment;

// Declare the evaluator module
mod evaluator; // Declare evaluator module
pub use crate::evaluator::Evaluator; // Re-export Evaluator so it can be used elsewhere

pub mod tokenizer;
pub use crate::tokenizer::Tokenizer;

mod parser; // Add this to lib.rs
pub use crate::parser::{build_expression_tree, parse_latex, shunting_yard};

pub mod expression;
pub use crate::expression::{
    solve_for_variable, solve_for_variable_exact, solve_for_variable_nodes, solve_full, SolveResult,
};

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
pub use crate::derivative::{
    differentiate, differentiate_and_evaluate, differentiate_latex, partial_derivative,
};

// Declare the composition module
pub mod composition;
pub use crate::composition::{compose, compose_latex, compose_multiple};

// Declare the integration module
pub mod integration;
pub use crate::integration::{
    definite_integral, definite_integral_exact, definite_integral_exact_latex,
    definite_integral_latex, integrate, integrate_latex,
};

// Declare the matrix module
pub mod matrix;
pub use crate::matrix::{parse_latex_matrix, Matrix};

pub mod polynomial;
pub use crate::polynomial::Polynomial;

pub mod rational_function;
pub use crate::rational_function::RationalFunction;

pub mod ext_poly;
pub use crate::ext_poly::ExtPoly;

pub mod mod_poly;
pub use crate::mod_poly::{factor_mod_p, factor_over_q, ModPoly};

pub mod partial_fractions;
pub use crate::partial_fractions::{partial_fraction_decomposition, partial_fractions_latex};

pub mod multipoly;
pub use crate::multipoly::MultiPoly;

pub mod fps;
pub use crate::fps::FormalPowerSeries;

pub mod series;
pub use crate::series::{
    taylor_series, taylor_series_latex, taylor_series_latex_symbolic, taylor_series_multivar_latex,
    taylor_series_symbolic, taylor_to_fps,
};

pub mod limits;
pub use crate::limits::{
    compute_limit, compute_limit_directed, compute_limit_general, limit_latex, limit_latex_str,
    LimitDirection, LimitPoint, LimitResult,
};

pub mod ode;
pub use crate::ode::{
    solve_constant_coeff, solve_constant_coeff_latex, solve_ode_latex, solve_series,
    solve_series_ivp,
};

pub mod risch;

pub mod algebraic;

pub mod systems;
pub use crate::systems::{solve_linear_system, solve_system, SystemSolution};

pub mod inequality;
pub use crate::inequality::solve_inequality;

pub mod verify;
pub use crate::verify::verify_identity;

pub use crate::risch::{
    build_tower, hermite_reduce, try_risch_tower, DifferentialExtension, HermiteResult, RischResult,
};
