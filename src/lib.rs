#![allow(unexpected_cfgs)]

pub mod foundation {
    pub mod assumptions;
    pub mod environment;
    pub mod exact;
    pub mod integer;
    pub mod node;
}

pub mod language {
    pub(crate) mod function_meta;
    pub mod functions;
    pub mod parser;
    pub mod tokenizer;
}

pub mod math {
    pub mod algebra {
        pub mod algebraic;
        pub mod ext_poly;
        pub mod matrix;
        pub mod mod_poly;
        pub mod multipoly;
        pub mod partial_fractions;
        pub mod polynomial;
        pub mod rational_function;
    }

    pub mod transform {
        pub mod composition;
        pub mod error_eval;
        pub mod evaluator;
        pub mod simplify;
        pub(crate) mod simplify_literal;
        pub mod substitute;
    }

    pub mod calculus {
        pub mod derivative;
        pub mod fps;
        pub mod integration;
        pub mod limits;
        pub mod risch;
        pub mod series;
        pub mod special_functions;
    }

    pub mod solving {
        pub mod expression;
        pub mod inequality;
        pub mod ode;
        pub mod systems;
    }
}

pub mod validation {
    pub mod chain;
    pub mod status;
    pub mod verify;
}

pub mod interface {
    pub mod wasm_bindings;
}

// Flat re-exports — preserve existing `crate::` / `arithma::` paths.

pub(crate) use language::function_meta;
pub(crate) use math::transform::simplify_literal;

pub use foundation::assumptions;
pub use foundation::assumptions::Assumptions;
pub use foundation::environment;
pub use foundation::environment::Environment;
pub use foundation::exact;
pub use foundation::exact::ExactNum;
pub use foundation::integer;
pub use foundation::integer::{
    as_non_negative_integer, binom, extract_square_factors, factorial, gcd, lcm,
    parse_non_negative_integer, prime_factorize, prime_factorize_latex,
};
pub use foundation::node;
pub use foundation::node::Node;

pub use language::functions;
pub use language::functions::FUNCTION_REGISTRY;
pub use language::parser;
pub use language::parser::{build_expression_tree, parse_latex, parse_latex_raw, shunting_yard};
pub use language::tokenizer;
pub use language::tokenizer::Tokenizer;

pub use math::transform::composition;
pub use math::transform::composition::{compose, compose_latex, compose_multiple};
pub use math::transform::error_eval;
pub use math::transform::error_eval::{
    evaluate_with_error, evaluate_with_error_traced, significant_digits,
};
pub use math::transform::evaluator;
pub use math::transform::evaluator::Evaluator;
pub use math::transform::simplify;
pub use math::transform::substitute;
pub use math::transform::substitute::{substitute, substitute_latex};

pub use math::algebra::algebraic;
pub use math::algebra::ext_poly;
pub use math::algebra::ext_poly::ExtPoly;
pub use math::algebra::matrix;
pub use math::algebra::matrix::{parse_latex_matrix, Matrix};
pub use math::algebra::mod_poly;
pub use math::algebra::mod_poly::{factor_mod_p, factor_over_q, ModPoly};
pub use math::algebra::multipoly;
pub use math::algebra::multipoly::MultiPoly;
pub use math::algebra::partial_fractions;
pub use math::algebra::partial_fractions::{
    partial_fraction_decomposition, partial_fractions_latex,
};
pub use math::algebra::polynomial;
pub use math::algebra::polynomial::Polynomial;
pub use math::algebra::rational_function;
pub use math::algebra::rational_function::RationalFunction;

pub use math::calculus::derivative;
pub use math::calculus::derivative::{
    differentiate, differentiate_and_evaluate, differentiate_latex, partial_derivative,
};
pub use math::calculus::fps;
pub use math::calculus::fps::FormalPowerSeries;
pub use math::calculus::integration;
pub use math::calculus::integration::{
    definite_integral, definite_integral_exact, definite_integral_exact_latex,
    definite_integral_latex, integrate, integrate_latex, integrate_outcome, IntegralOutcome,
};
pub use math::calculus::limits;
pub use math::calculus::limits::{
    compute_limit, compute_limit_directed, compute_limit_general, limit_latex, limit_latex_str,
    LimitDirection, LimitPoint, LimitResult,
};
pub use math::calculus::risch;
pub use math::calculus::risch::{
    build_tower, hermite_reduce, try_risch_tower, DifferentialExtension, HermiteResult, RischResult,
};
pub use math::calculus::series;
pub use math::calculus::series::{
    taylor_series, taylor_series_latex, taylor_series_latex_symbolic, taylor_series_multivar_latex,
    taylor_series_symbolic, taylor_to_fps,
};
pub use math::calculus::special_functions;
pub use math::calculus::special_functions::SpecialAntiderivative;

pub use math::solving::expression;
pub use math::solving::expression::{
    solve_for_variable, solve_for_variable_exact, solve_for_variable_nodes, solve_full, SolveResult,
};
pub use math::solving::inequality;
pub use math::solving::inequality::solve_inequality;
pub use math::solving::ode;
pub use math::solving::ode::{
    solve_constant_coeff, solve_constant_coeff_latex, solve_ode_latex, solve_series,
    solve_series_ivp,
};
pub use math::solving::systems;
pub use math::solving::systems::{solve_linear_system, solve_system, SystemSolution};

pub use validation::chain;
pub use validation::status;
pub use validation::verify;
pub use validation::verify::verify_identity;

pub use interface::wasm_bindings;
pub use interface::wasm_bindings::evaluate_latex_expression_js;
