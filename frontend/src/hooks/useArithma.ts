import { useState, useCallback } from 'react';
import type { Tool } from '../tools';

// These will be dynamically imported from the WASM module
let wasmModule: Record<string, (...args: string[]) => string> | null = null;

export async function initWasm(): Promise<void> {
  const module = await import('../../public/pkg/arithma');
  await module.default({ path: '/pkg/arithma_bg.wasm' });
  wasmModule = module as unknown as Record<string, (...args: string[]) => string>;
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
