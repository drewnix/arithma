import { useState, useCallback } from 'react';
import type { Tool } from '../tools';

// WASM module functions, populated after init
let wasmModule: Record<string, (...args: unknown[]) => unknown> | null = null;

export async function initWasm(): Promise<void> {
  const module = await import('../../public/pkg/arithma');
  await module.default({ path: '/pkg/arithma_bg.wasm' });
  // Spread the module into a plain object so bracket-notation lookup works
  // (ES module namespace objects aren't always indexable in all bundlers)
  const fns: Record<string, unknown> = {};
  for (const key of Object.keys(module)) {
    if (key !== 'default' && typeof module[key as keyof typeof module] === 'function') {
      fns[key] = module[key as keyof typeof module];
    }
  }
  wasmModule = fns as Record<string, (...args: unknown[]) => unknown>;
}

export interface ToolResult {
  input: string;
  toolId: string;
  toolName: string;
  result: string;
  error?: string;
  params: Record<string, string>;
}

export function useArithma() {
  const [activeTool, setActiveTool] = useState<Tool | null>(null);
  const [history, setHistory] = useState<ToolResult[]>([]);
  const [params, setParams] = useState<Record<string, string>>({});

  const execute = useCallback(async (tool: Tool, expression: string, toolParams: Record<string, string>) => {
    if (!wasmModule) {
      throw new Error('WASM not initialized');
    }

    const fn = wasmModule[tool.wasmFn];
    if (!fn) {
      throw new Error(`WASM function ${tool.wasmFn} not found`);
    }

    // Build argument list based on tool definition
    const args: string[] = [expression];

    for (const param of tool.params) {
      const value = toolParams[param.name] || param.default || '';
      args.push(value);
    }

    // Special case: evaluate needs env JSON as second arg
    if (tool.id === 'evaluate') {
      args.push(JSON.stringify({ vars: {} }));
    }
    // Matrix tools need env JSON
    if (tool.category === 'matrix') {
      args.push(JSON.stringify({ vars: {} }));
    }

    try {
      const result = fn(...args);
      const entry: ToolResult = {
        input: expression,
        toolId: tool.id,
        toolName: tool.name,
        result,
        params: toolParams,
      };
      setHistory(prev => [...prev, entry]);
      return entry;
    } catch (err: unknown) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      const entry: ToolResult = {
        input: expression,
        toolId: tool.id,
        toolName: tool.name,
        result: '',
        error: errorMsg,
        params: toolParams,
      };
      setHistory(prev => [...prev, entry]);
      return entry;
    }
  }, []);

  return {
    activeTool,
    setActiveTool,
    history,
    params,
    setParams,
    execute,
  };
}
