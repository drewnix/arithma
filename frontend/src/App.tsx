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

/* ── Design tokens ─────────────────────────────────── */
const t = {
  ground:    "#0C0C0E",
  elevated:  "#141416",
  surface:   "#1C1C20",
  border:    "#2A2A2E",
  borderLt:  "#222226",
  text1:     "#F0EEEB",
  text2:     "#C0BDB8",
  text3:     "#8A8780",
  text4:     "#605D58",
  accent:    "#6E9EF5",
  accentDim: "rgba(110,158,245,0.10)",
  accentMid: "rgba(110,158,245,0.25)",
  accentBrd: "rgba(110,158,245,0.30)",
  error:     "#C75B5B",
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
    <div style={{ minHeight: "100vh", background: t.ground, color: t.text1, ...mono }}>
      <div style={{ width: "100%", maxWidth: 1100, margin: "0 auto", padding: "0 40px" }}>

        {/* Header */}
        <header style={{
          display: "flex", alignItems: "baseline", justifyContent: "space-between",
          padding: "32px 0 24px",
        }}>
          <h1 style={{ fontSize: "1.5rem", fontWeight: 600, letterSpacing: "-0.02em", color: t.accent }}>
            Arithma
          </h1>
          <span style={{ fontSize: "0.7rem", color: t.text3, fontWeight: 400 }}>
            Symbolic Math Engine
          </span>
        </header>

        {/* Category tabs — row 1 */}
        <div style={{
          display: "flex", gap: 4, marginBottom: 8,
          borderBottom: `1px solid ${t.border}`, paddingBottom: 8,
        }}>
          {categories.map(cat => {
            const Icon = categoryIcons[cat.id];
            const active = activeCategory === cat.id;
            return (
              <button
                key={cat.id}
                onClick={() => setActiveCategory(cat.id as Category)}
                style={{
                  display: "flex", alignItems: "center", gap: 8,
                  padding: "8px 16px", borderRadius: 6, fontSize: "0.78rem",
                  fontWeight: 500,
                  background: active ? t.surface : "transparent",
                  color: active ? t.accent : t.text3,
                  transition: "all 0.15s", whiteSpace: "nowrap", ...mono,
                }}
              >
                {Icon && <Icon size={14} />}
                {cat.name}
              </button>
            );
          })}
        </div>

        {/* Tool pills — row 2, fixed height */}
        <div style={{
          display: "flex", gap: 4, flexWrap: "wrap",
          minHeight: 36, alignItems: "center",
          marginBottom: 20,
        }}>
          {categoryTools.map(tool => {
            const active = activeTool?.id === tool.id;
            const Icon = iconMap[tool.icon];
            return (
              <button
                key={tool.id}
                onClick={() => handleToolSelect(tool)}
                style={{
                  display: "flex", alignItems: "center", gap: 5,
                  padding: "5px 10px", borderRadius: 5, fontSize: "0.68rem",
                  border: active ? `1px solid ${t.accentBrd}` : "1px solid transparent",
                  background: active ? t.accentDim : "transparent",
                  color: active ? t.accent : t.text4,
                  transition: "all 0.15s", whiteSpace: "nowrap", ...mono,
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
          background: t.elevated, border: `1px solid ${t.border}`, borderRadius: 12,
          padding: "16px 20px", marginBottom: 10,
        }}>
          <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
            {/* Input container — minWidth 0 allows flex shrink below content size */}
            <div style={{
              flex: 1, minWidth: 0,
              background: t.ground, border: `1px solid ${t.border}`,
              borderRadius: 8, padding: "4px 12px",
              display: "flex", alignItems: "center",
            }}>
              <math-field
                style={{
                  flex: 1, minWidth: 0,
                  background: "transparent", color: t.text1,
                  border: "none", fontSize: "1.25rem", padding: "8px 0",
                  caretColor: t.accent, outline: "none", minHeight: "40px",
                  fontFamily: "'JetBrains Mono', monospace",
                  '--selection-background-color': t.accentMid,
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
                fontWeight: 500, flexShrink: 0,
                background: t.accent, color: t.ground,
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
              borderTop: `1px solid ${t.borderLt}`,
            }}>
              {activeTool.params.map(param => (
                <div key={param.name} style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <label style={{ fontSize: "0.68rem", color: t.text3 }}>
                    {param.label}
                  </label>
                  <input
                    type={param.type === "number" ? "number" : "text"}
                    value={params[param.name] || ""}
                    onChange={e => setParams({ ...params, [param.name]: e.target.value })}
                    placeholder={param.placeholder}
                    style={{
                      background: t.ground, border: `1px solid ${t.border}`, borderRadius: 4,
                      padding: "5px 8px", fontSize: "0.75rem", width: 56,
                      color: t.text1, outline: "none", ...mono,
                    }}
                  />
                </div>
              ))}
            </div>
          )}
        </div>

        {/* History */}
        <div style={{ paddingBottom: 48 }}>
          {history.length === 0 && (
            <div style={{ textAlign: "center", color: t.text4, fontSize: "0.78rem", marginTop: 32 }}>
              Type an expression above and press Enter
            </div>
          )}

          <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
            {[...history].reverse().map((entry, i) => (
              <div key={i} style={{
                background: t.elevated, border: `1px solid ${t.border}`,
                borderRadius: 10, padding: "14px 18px",
              }}>
                <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
                  <span style={{
                    fontSize: "0.58rem", padding: "2px 7px", borderRadius: 3,
                    background: t.accentDim, color: t.accent, letterSpacing: "0.02em",
                  }}>
                    {entry.toolName}
                  </span>
                  {Object.keys(entry.params).length > 0 && (
                    <span style={{ fontSize: "0.58rem", color: t.text4 }}>
                      {Object.entries(entry.params).map(([k, v]) => `${k}=${v}`).join(", ")}
                    </span>
                  )}
                </div>

                {/* Input — rendered as math */}
                <div style={{ marginBottom: 4 }}>
                  <math-field
                    read-only=""
                    style={{
                      background: "transparent", border: "none", color: t.text2,
                      fontSize: "0.9rem", padding: 0, pointerEvents: "none",
                      fontFamily: "'JetBrains Mono', monospace",
                    }}
                  >
                    {entry.input}
                  </math-field>
                </div>

                {/* Result */}
                {entry.error ? (
                  <div style={{ fontSize: "0.82rem", color: t.error }}>
                    {entry.error}
                  </div>
                ) : (
                  <div style={{ display: "flex", alignItems: "baseline", gap: 8 }}>
                    <span style={{ color: t.text4, fontSize: "1rem" }}>=</span>
                    <math-field
                      read-only=""
                      style={{
                        background: "transparent", border: "none", color: t.text1,
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
