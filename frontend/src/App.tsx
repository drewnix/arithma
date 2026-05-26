import { useEffect, useState, useCallback } from "react";
import type { MathfieldElement } from "mathlive";
import "mathlive";
import { initWasm, useArithma } from "./hooks/useArithma";
import { categories, getToolsByCategory, type Tool, type Category } from "./tools";
import {
  Calculator, TrendingUp, Sigma, Grid3x3, Activity, ArrowRight,
  Waves, GitBranch, Minimize2, Target, Split, Layers, Replace,
  RotateCcw, Sparkles, Play, type LucideIcon,
} from "lucide-react";

const iconMap: Record<string, LucideIcon> = {
  Calculator, TrendingUp, Sigma, Grid3x3, Activity, ArrowRight,
  Waves, GitBranch, Minimize2, Target, Split, Layers, Replace,
  RotateCcw, Sparkles,
};

const categoryIcons: Record<string, LucideIcon> = {
  evaluate: Calculator,
  calculus: TrendingUp,
  algebra: Sigma,
  matrix: Grid3x3,
};

const mono = { fontFamily: "'JetBrains Mono', monospace" };

export default function App() {
  const { activeTool, setActiveTool, history, params, setParams, execute } = useArithma();
  const [activeCategory, setActiveCategory] = useState<Category>("evaluate");
  const [wasmReady, setWasmReady] = useState(false);
  const [input, setInput] = useState("");

  useEffect(() => {
    initWasm().then(() => setWasmReady(true)).catch(console.error);
  }, []);

  // Set default tool when category changes
  useEffect(() => {
    const ct = getToolsByCategory(activeCategory);
    if (ct.length > 0) {
      setActiveTool(ct[0]);
      const d: Record<string, string> = {};
      ct[0].params.forEach(p => { d[p.name] = p.default || ""; });
      setParams(d);
    }
  }, [activeCategory, setActiveTool, setParams]);

  const handleToolSelect = useCallback((tool: Tool) => {
    setActiveTool(tool);
    const d: Record<string, string> = {};
    tool.params.forEach(p => { d[p.name] = p.default || ""; });
    setParams(d);
  }, [setActiveTool, setParams]);

  const handleExecute = useCallback(async () => {
    if (!activeTool || !wasmReady) return;
    const mf = document.querySelector("math-field") as MathfieldElement;
    const latex = mf?.getValue("latex-expanded") || input;
    if (!latex.trim()) return;
    await execute(activeTool, latex, params);
  }, [activeTool, wasmReady, input, params, execute]);

  const handleKeyDown = useCallback((evt: React.KeyboardEvent) => {
    if (evt.key === "Enter") { evt.preventDefault(); handleExecute(); }
  }, [handleExecute]);

  const categoryTools = getToolsByCategory(activeCategory);

  return (
    <div style={{ minHeight: "100vh", background: "#0C0C0E", color: "#E8E6E3", ...mono }}>
      <div style={{ maxWidth: 760, margin: "0 auto", padding: "0 24px" }}>

        {/* Header */}
        <header style={{ display: "flex", alignItems: "baseline", justifyContent: "space-between", padding: "32px 0 24px" }}>
          <h1 style={{ fontSize: "1.5rem", fontWeight: 600, letterSpacing: "-0.02em", color: "#E8E6E3" }}>
            arithma
          </h1>
          <span style={{ fontSize: "0.7rem", color: "#706D68", fontWeight: 400 }}>
            symbolic math engine
          </span>
        </header>

        {/* Category tabs */}
        <div style={{ display: "flex", gap: 6, marginBottom: 12 }}>
          {categories.map(cat => {
            const Icon = categoryIcons[cat.id];
            const active = activeCategory === cat.id;
            return (
              <button
                key={cat.id}
                onClick={() => setActiveCategory(cat.id as Category)}
                style={{
                  display: "flex", alignItems: "center", gap: 8,
                  padding: "8px 16px", borderRadius: 8, fontSize: "0.8rem",
                  fontWeight: 500, border: active ? "1px solid #2A2A2E" : "1px solid transparent",
                  background: active ? "#1A1A1E" : "transparent",
                  color: active ? "#E8B84C" : "#706D68",
                  cursor: "pointer", transition: "all 0.15s", ...mono,
                }}
              >
                {Icon && <Icon size={15} />}
                {cat.name}
              </button>
            );
          })}
        </div>

        {/* Tool pills */}
        <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginBottom: 24 }}>
          {categoryTools.map(tool => {
            const active = activeTool?.id === tool.id;
            const Icon = iconMap[tool.icon];
            return (
              <button
                key={tool.id}
                onClick={() => handleToolSelect(tool)}
                style={{
                  display: "flex", alignItems: "center", gap: 6,
                  padding: "6px 12px", borderRadius: 6, fontSize: "0.72rem",
                  border: active ? "1px solid rgba(232,184,76,0.3)" : "1px solid transparent",
                  background: active ? "rgba(232,184,76,0.1)" : "transparent",
                  color: active ? "#E8B84C" : "#706D68",
                  cursor: "pointer", transition: "all 0.15s", ...mono,
                }}
              >
                {Icon && <Icon size={12} />}
                {tool.name}
              </button>
            );
          })}
        </div>

        {/* Input card */}
        <div style={{
          background: "#141416", border: "1px solid #2A2A2E", borderRadius: 12,
          padding: 20, marginBottom: 8, transition: "border-color 0.2s",
        }}>
          {/* Tool description */}
          {activeTool && (
            <div style={{ fontSize: "0.72rem", color: "#706D68", marginBottom: 12 }}>
              {activeTool.description}
            </div>
          )}

          <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
            <math-field
              style={{
                flex: 1, background: "transparent", color: "#E8E6E3",
                border: "none", fontSize: "1.3rem", padding: "8px 4px",
                caretColor: "#E8B84C", outline: "none",
                fontFamily: "'JetBrains Mono', monospace",
                // MathLive custom properties
                '--selection-background-color': 'rgba(232,184,76,0.25)',
                '--contains-highlight-background-color': 'transparent',
              } as React.CSSProperties}
              onInput={(evt: React.FormEvent<MathfieldElement>) => {
                setInput((evt.target as MathfieldElement).getValue("latex-expanded"));
              }}
              onKeyDown={handleKeyDown}
            >
              {input}
            </math-field>

            <button
              onClick={handleExecute}
              disabled={!wasmReady}
              style={{
                display: "flex", alignItems: "center", gap: 8,
                padding: "10px 20px", borderRadius: 8, fontSize: "0.8rem",
                fontWeight: 500, border: "none",
                background: "#E8B84C", color: "#0C0C0E",
                cursor: wasmReady ? "pointer" : "not-allowed",
                opacity: wasmReady ? 1 : 0.4,
                transition: "all 0.15s", ...mono,
              }}
            >
              <Play size={14} />
              {activeTool?.name || "Evaluate"}
            </button>
          </div>

          {/* Dynamic params */}
          {activeTool && activeTool.params.length > 0 && (
            <div style={{
              display: "flex", gap: 16, marginTop: 14, paddingTop: 14,
              borderTop: "1px solid #2A2A2E",
            }}>
              {activeTool.params.map(param => (
                <div key={param.name} style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <label style={{ fontSize: "0.7rem", color: "#706D68" }}>
                    {param.label}
                  </label>
                  <input
                    type={param.type === "number" ? "number" : "text"}
                    value={params[param.name] || ""}
                    onChange={e => setParams({ ...params, [param.name]: e.target.value })}
                    placeholder={param.placeholder}
                    style={{
                      background: "#0C0C0E", border: "1px solid #2A2A2E", borderRadius: 4,
                      padding: "4px 8px", fontSize: "0.75rem", width: 56,
                      color: "#E8E6E3", outline: "none", ...mono,
                    }}
                  />
                </div>
              ))}
            </div>
          )}
        </div>

        {/* History */}
        <div style={{ paddingTop: 16, paddingBottom: 48 }}>
          {history.length === 0 && (
            <div style={{ textAlign: "center", color: "#706D68", fontSize: "0.8rem", marginTop: 48 }}>
              Type an expression and press Enter
            </div>
          )}

          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            {[...history].reverse().map((entry, i) => (
              <div key={i} style={{
                background: "#141416", border: "1px solid #2A2A2E",
                borderRadius: 10, padding: 16,
              }}>
                {/* Tool badge + params */}
                <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
                  <span style={{
                    fontSize: "0.6rem", padding: "2px 8px", borderRadius: 4,
                    background: "rgba(232,184,76,0.1)", color: "#E8B84C",
                  }}>
                    {entry.toolName}
                  </span>
                  {Object.keys(entry.params).length > 0 && (
                    <span style={{ fontSize: "0.6rem", color: "#706D68" }}>
                      {Object.entries(entry.params).map(([k, v]) => `${k}=${v}`).join(", ")}
                    </span>
                  )}
                </div>

                {/* Input (raw LaTeX) */}
                <div style={{ fontSize: "0.85rem", color: "#A8A5A0", marginBottom: 4 }}>
                  {entry.input}
                </div>

                {/* Result */}
                {entry.error ? (
                  <div style={{ fontSize: "0.85rem", color: "#C75B5B" }}>
                    Error: {entry.error}
                  </div>
                ) : (
                  <div style={{ display: "flex", alignItems: "baseline", gap: 8 }}>
                    <span style={{ color: "#706D68", fontSize: "1rem" }}>=</span>
                    <math-field
                      read-only
                      style={{
                        background: "transparent", border: "none", color: "#E8E6E3",
                        fontSize: "1.15rem", padding: 0, pointerEvents: "none",
                        fontFamily: "'JetBrains Mono', monospace",
                      }}
                    >
                      {entry.result}
                    </math-field>
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
