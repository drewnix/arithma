/// <reference types="vite/client" />

// Add types for mathlive math-field element
import { MathfieldElement } from "mathlive";

declare global {
  namespace JSX {
    interface IntrinsicElements {
      'math-field': React.DetailedHTMLProps<React.HTMLAttributes<MathfieldElement>, MathfieldElement>;
    }
  }
}

// Add module declaration for the arithma WASM module
declare module 'arithma' {
  export function init(options?: { path?: string }): Promise<void>;
  export function evaluate_latex_expression_js(latex: string, environment: string): string;
}
