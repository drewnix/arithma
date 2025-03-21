/**
 * TypeScript declarations for the Arithma WASM module
 */
declare module 'arithma' {
  /**
   * Initialize the WASM module
   * @param options Configuration options for initialization
   * @returns A promise that resolves when initialization is complete
   */
  export default function init(options?: { path?: string }): Promise<void>;

  /**
   * Evaluate a LaTeX expression with the given environment
   * @param latex The LaTeX expression to evaluate
   * @param environment JSON string representing the environment with variables
   * @returns The result of evaluation as a string
   */
  export function evaluate_latex_expression_js(latex: string, environment: string): string;

  /**
   * Compose two functions represented in LaTeX
   * @param f_latex The outer function in LaTeX format
   * @param f_var The variable in the outer function to substitute
   * @param g_latex The inner function in LaTeX format
   * @returns The composed function in LaTeX format
   */
  export function compose_functions_js(f_latex: string, f_var: string, g_latex: string): string;

  /**
   * Integrate a LaTeX expression with respect to a variable
   * @param latex_expr The LaTeX expression to integrate
   * @param var_name The variable to integrate with respect to
   * @returns The integrated expression in LaTeX format
   */
  export function integrate_expression_js(latex_expr: string, var_name: string): string;

  /**
   * Calculate a definite integral of a LaTeX expression
   * @param latex_expr The LaTeX expression to integrate
   * @param var_name The variable to integrate with respect to
   * @param lower The lower bound of integration
   * @param upper The upper bound of integration
   * @returns The result of the definite integral
   */
  export function definite_integral_js(
    latex_expr: string,
    var_name: string,
    lower: number,
    upper: number
  ): string;
}