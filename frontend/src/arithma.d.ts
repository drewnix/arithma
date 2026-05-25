/**
 * TypeScript declarations for the Arithma WASM module
 */
declare module 'arithma' {
  export default function init(options?: { path?: string }): Promise<void>;

  // Evaluate / Simplify
  export function evaluate_latex_expression_js(latex: string, environment: string): string;
  export function simplify_latex_js(latex: string): string;

  // Calculus
  export function differentiate_js(latex_expr: string, var_name: string): string;
  export function integrate_expression_js(latex_expr: string, var_name: string): string;
  export function definite_integral_js(latex_expr: string, var_name: string, lower: number, upper: number): string;
  export function limit_js(latex_expr: string, var_name: string, point: number): string;
  export function taylor_series_js(latex_expr: string, var_name: string, center: number, order: number): string;
  export function solve_ode_js(rhs_latex: string, indep_var: string, dep_var: string): string;

  // Algebra
  export function solve_js(latex_equation: string, var_name: string): string;
  export function polynomial_factor_js(latex_expr: string, var_name: string): string;
  export function partial_fractions_js(latex_expr: string, var_name: string): string;
  export function substitute_js(latex_expr: string, var_name: string, value_latex: string): string;
  export function equivalent_js(expr1: string, expr2: string): string;
  export function compose_functions_js(f_latex: string, f_var: string, g_latex: string): string;

  // Matrix
  export function parse_matrix_js(latex_expr: string, env_json: string): string;
  export function matrix_determinant_js(latex_expr: string, env_json: string): string;
  export function matrix_inverse_js(latex_expr: string, env_json: string): string;
  export function matrix_multiply_js(matrix_a: string, matrix_b: string, env_json: string): string;
  export function matrix_rank_js(latex_expr: string, env_json: string): number;
  export function matrix_eigenvalues_js(latex_expr: string, env_json: string): string;
  export function solve_linear_system_js(matrix_a: string, vector_b: string, env_json: string): string;
}
