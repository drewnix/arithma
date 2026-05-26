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
    const mf = document.querySelector("math-field:not([read-only])") as MathfieldElement;
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
      <div style={{ maxWidth: 960, margin: "0 auto", padding: "0 32px" }}>

        {/* Header */}
        <header style={{
          display: "flex", alignItems: "baseline", justifyContent: "space-between",
          padding: "32px 0 28px",
        }}>
          <h1 style={{ fontSize: "1.5rem", fontWeight: 600, letterSpacing: "-0.02em", color: "#E8E6E3" }}>
            arithma
          </h1>
          <span style={{ fontSize: "0.7rem", color: "#706D68", fontWeight: 400 }}>
            symbolic math engine
          </span>
        </header>

        {/* Navigation bar: categories + tools in one row */}
        <div style={{
          display: "flex", alignItems: "center", gap: 8,
          padding: "8px 0", marginBottom: 20,
          minHeight: 44,
        }}>
          {/* Category tabs */}
          {categories.map(cat => {
            const Icon = categoryIcons[cat.id];
            const active = activeCategory === cat.id;
            return (
              <button
                key={cat.id}
                onClick={() => setActiveCategory(cat.id as Category)}
                style={{
                  display: "flex", alignItems: "center", gap: 8,
                  padding: "8px 14px", borderRadius: 8, fontSize: "0.78rem",
                  fontWeight: 500, border: active ? "1px solid #2A2A2E" : "1px solid transparent",
                  background: active ? "#1A1A1E" : "transparent",
                  color: active ? "#E8B84C" : "#706D68",
                  cursor: "pointer", transition: "all 0.15s", whiteSpace: "nowrap", ...mono,
                }}
              >
                {Icon && <Icon size={14} />}
                {cat.name}
              </button>
            );
          })}

          {/* Divider */}
          <div style={{
            width: 1, height: 20, background: "#2A2A2E", margin: "0 6px", flexShrink: 0,
          }} />

          {/* Tool pills for selected category */}
          {categoryTools.map(tool => {
            const active = activeTool?.id === tool.id;
            const Icon = iconMap[tool.icon];
            return (
              <button
                key={tool.id}
                onClick={() => handleToolSelect(tool)}
                style={{
                  display: "flex", alignItems: "center", gap: 5,
                  padding: "5px 10px", borderRadius: 5, fontSize: "0.7rem",
                  border: active ? "1px solid rgba(232,184,76,0.3)" : "1px solid transparent",
                  background: active ? "rgba(232,184,76,0.1)" : "transparent",
                  color: active ? "#E8B84C" : "#585550",
                  cursor: "pointer", transition: "all 0.15s", whiteSpace: "nowrap", ...mono,
                }}
              >
                {Icon && <Icon size={11} />}
                {tool.name}
              </button>
            );
          })}
        </div>

        {/* Input area */}
        <div style={{
          background: "#141416", border: "1px solid #2A2A2E", borderRadius: 12,
          padding: "16px 20px", marginBottom: 8,
        }}>
          {/* Math input row */}
          <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
            {/* Input container — the visible "type here" area */}
            <div style={{
              flex: 1, background: "#0C0C0E", border: "1px solid #2A2A2E",
              borderRadius: 8, padding: "4px 12px",
              display: "flex", alignItems: "center",
            }}>
              <math-field
                style={{
                  flex: 1, background: "transparent", color: "#E8E6E3",
                  border: "none", fontSize: "1.25rem", padding: "8px 0",
                  caretColor: "#E8B84C", outline: "none", minHeight: "40px",
                  fontFamily: "'JetBrains Mono', monospace",
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
            </div>

            {/* Action button */}
            <button
              onClick={handleExecute}
              disabled={!wasmReady}
              style={{
                display: "flex", alignItems: "center", gap: 8,
                padding: "12px 20px", borderRadius: 8, fontSize: "0.78rem",
                fontWeight: 500, border: "none", flexShrink: 0,
                background: "#E8B84C", color: "#0C0C0E",
                cursor: wasmReady ? "pointer" : "not-allowed",
                opacity: wasmReady ? 1 : 0.4,
                transition: "all 0.15s", ...mono,
              }}
            >
              <Play size={13} />
              {activeTool?.name || "Evaluate"}
            </button>
          </div>

          {/* Dynamic params */}
          {activeTool && activeTool.params.length > 0 && (
            <div style={{
              display: "flex", gap: 16, marginTop: 12, paddingTop: 12,
              borderTop: "1px solid #1E1E22",
            }}>
              {activeTool.params.map(param => (
                <div key={param.name} style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <label style={{ fontSize: "0.68rem", color: "#706D68" }}>
                    {param.label}
                  </label>
                  <input
                    type={param.type === "number" ? "number" : "text"}
                    value={params[param.name] || ""}
                    onChange={e => setParams({ ...params, [param.name]: e.target.value })}
                    placeholder={param.placeholder}
                    style={{
                      background: "#0C0C0E", border: "1px solid #2A2A2E", borderRadius: 4,
                      padding: "5px 8px", fontSize: "0.75rem", width: 56,
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
            <div style={{ textAlign: "center", color: "#504D48", fontSize: "0.78rem", marginTop: 48 }}>
              Type an expression above and press Enter
            </div>
          )}

          <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
            {[...history].reverse().map((entry, i) => (
              <div key={i} style={{
                background: "#141416", border: "1px solid #2A2A2E",
                borderRadius: 10, padding: "14px 18px",
              }}>
                {/* Tool badge + params */}
                <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
                  <span style={{
                    fontSize: "0.58rem", padding: "2px 7px", borderRadius: 3,
                    background: "rgba(232,184,76,0.08)", color: "#C9A040",
                    letterSpacing: "0.02em",
                  }}>
                    {entry.toolName}
                  </span>
                  {Object.keys(entry.params).length > 0 && (
                    <span style={{ fontSize: "0.58rem", color: "#504D48" }}>
                      {Object.entries(entry.params).map(([k, v]) => `${k}=${v}`).join(", ")}
                    </span>
                  )}
                </div>

                {/* Input */}
                <div style={{
                  fontSize: "0.82rem", color: "#A8A5A0", marginBottom: 4,
                  overflow: "hidden", textOverflow: "ellipsis",
                }}>
                  {entry.input}
                </div>

                {/* Result */}
                {entry.error ? (
                  <div style={{ fontSize: "0.82rem", color: "#C75B5B" }}>
                    {entry.error}
                  </div>
                ) : (
                  <div style={{ display: "flex", alignItems: "baseline", gap: 8 }}>
                    <span style={{ color: "#504D48", fontSize: "1rem" }}>=</span>
                    <math-field
                      read-only=""
                      style={{
                        background: "transparent", border: "none", color: "#E8E6E3",
                        fontSize: "1.1rem", padding: 0, pointerEvents: "none",
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
