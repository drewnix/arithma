/**
 * Arithma Tool Definitions
 *
 * Each tool maps to a WASM function and defines its parameters.
 * The UI renders parameter inputs based on these definitions.
 */

export type ToolParam = {
  name: string;
  label: string;
  type: 'variable' | 'expression' | 'number';
  default?: string;
  placeholder?: string;
};

export type Tool = {
  id: string;
  name: string;
  icon: string; // lucide icon name
  description: string;
  category: 'calculus' | 'algebra' | 'matrix' | 'evaluate';
  params: ToolParam[];
  /** The WASM function to call. Expression input is always the first argument. */
  wasmFn: string;
};

export const tools: Tool[] = [
  // === Evaluate (catch-all) ===
  {
    id: 'evaluate',
    name: 'Evaluate',
    icon: 'Calculator',
    description: 'Evaluate or simplify an expression',
    category: 'evaluate',
    params: [],
    wasmFn: 'evaluate_latex_expression_js',
  },

  // === Calculus ===
  {
    id: 'differentiate',
    name: 'Differentiate',
    icon: 'TrendingUp',
    description: 'Compute the derivative of an expression',
    category: 'calculus',
    params: [
      { name: 'variable', label: 'Variable', type: 'variable', default: 'x', placeholder: 'x' },
    ],
    wasmFn: 'differentiate_js',
  },
  {
    id: 'integrate',
    name: 'Integrate',
    icon: 'Activity',
    description: 'Compute the indefinite integral',
    category: 'calculus',
    params: [
      { name: 'variable', label: 'Variable', type: 'variable', default: 'x', placeholder: 'x' },
    ],
    wasmFn: 'integrate_expression_js',
  },
  {
    id: 'limit',
    name: 'Limit',
    icon: 'ArrowRight',
    description: 'Compute a limit',
    category: 'calculus',
    params: [
      { name: 'variable', label: 'Variable', type: 'variable', default: 'x', placeholder: 'x' },
      { name: 'point', label: 'Approaches', type: 'number', default: '0', placeholder: '0' },
    ],
    wasmFn: 'limit_js',
  },
  {
    id: 'taylor',
    name: 'Taylor Series',
    icon: 'Waves',
    description: 'Compute a Taylor series expansion',
    category: 'calculus',
    params: [
      { name: 'variable', label: 'Variable', type: 'variable', default: 'x', placeholder: 'x' },
      { name: 'center', label: 'Center', type: 'number', default: '0', placeholder: '0' },
      { name: 'order', label: 'Order', type: 'number', default: '5', placeholder: '5' },
    ],
    wasmFn: 'taylor_series_js',
  },
  {
    id: 'ode',
    name: 'Solve ODE',
    icon: 'GitBranch',
    description: 'Solve an ordinary differential equation',
    category: 'calculus',
    params: [
      { name: 'indep', label: 'Independent var', type: 'variable', default: 'x', placeholder: 'x' },
      { name: 'dep', label: 'Dependent var', type: 'variable', default: 'y', placeholder: 'y' },
    ],
    wasmFn: 'solve_ode_js',
  },

  // === Algebra ===
  {
    id: 'simplify',
    name: 'Simplify',
    icon: 'Minimize2',
    description: 'Simplify an expression',
    category: 'algebra',
    params: [],
    wasmFn: 'simplify_latex_js',
  },
  {
    id: 'solve',
    name: 'Solve',
    icon: 'Target',
    description: 'Solve an equation for a variable',
    category: 'algebra',
    params: [
      { name: 'variable', label: 'Solve for', type: 'variable', default: 'x', placeholder: 'x' },
    ],
    wasmFn: 'solve_js',
  },
  {
    id: 'factor',
    name: 'Factor',
    icon: 'Split',
    description: 'Factor a polynomial',
    category: 'algebra',
    params: [
      { name: 'variable', label: 'Variable', type: 'variable', default: 'x', placeholder: 'x' },
    ],
    wasmFn: 'polynomial_factor_js',
  },
  {
    id: 'partial_fractions',
    name: 'Partial Fractions',
    icon: 'Layers',
    description: 'Decompose into partial fractions',
    category: 'algebra',
    params: [
      { name: 'variable', label: 'Variable', type: 'variable', default: 'x', placeholder: 'x' },
    ],
    wasmFn: 'partial_fractions_js',
  },
  {
    id: 'substitute',
    name: 'Substitute',
    icon: 'Replace',
    description: 'Substitute a value for a variable',
    category: 'algebra',
    params: [
      { name: 'variable', label: 'Variable', type: 'variable', default: 'x', placeholder: 'x' },
      { name: 'value', label: 'Value (LaTeX)', type: 'expression', placeholder: '2' },
    ],
    wasmFn: 'substitute_js',
  },

  // === Matrix ===
  {
    id: 'matrix_det',
    name: 'Determinant',
    icon: 'Grid3x3',
    description: 'Compute matrix determinant',
    category: 'matrix',
    params: [],
    wasmFn: 'matrix_determinant_js',
  },
  {
    id: 'matrix_inv',
    name: 'Inverse',
    icon: 'RotateCcw',
    description: 'Compute matrix inverse',
    category: 'matrix',
    params: [],
    wasmFn: 'matrix_inverse_js',
  },
  {
    id: 'matrix_eigen',
    name: 'Eigenvalues',
    icon: 'Sparkles',
    description: 'Compute eigenvalues',
    category: 'matrix',
    params: [],
    wasmFn: 'matrix_eigenvalues_js',
  },
];

export const categories = [
  { id: 'evaluate', name: 'Evaluate', icon: 'Calculator' },
  { id: 'calculus', name: 'Calculus', icon: 'TrendingUp' },
  { id: 'algebra', name: 'Algebra', icon: 'Sigma' },
  { id: 'matrix', name: 'Matrix', icon: 'Grid3x3' },
] as const;

export type Category = typeof categories[number]['id'];

export function getToolsByCategory(category: Category): Tool[] {
  return tools.filter(t => t.category === category);
}

export function getToolById(id: string): Tool | undefined {
  return tools.find(t => t.id === id);
}
