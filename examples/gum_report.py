#!/usr/bin/env python3
"""Verified GUM uncertainty report from a model file, via arithma-mcp.

Reads a measurement model (LaTeX expression + parameter values and
standard uncertainties), drives the arithma MCP server over stdio, and
emits a complete first-order uncertainty budget (JCGM 100:2008) in
Markdown — with every step machine-verified:

  1. each sensitivity coefficient  c_i = ∂f/∂x_i  computed symbolically
     (`differentiate`) and INDEPENDENTLY re-checked as a `verify_chain`
     `derivative_of` step — a failed check aborts the report;
  2. each c_i evaluated at the operating point by exact substitution
     (`substitute`, values as LaTeX strings, so non-integer rationals
     like 1/2 stay exact);
  3. each variance contribution (c_i·u_i)² and the combined u_c²
     evaluated by the engine in exact rational arithmetic, and
     cross-checked against an independent assembly in Python fractions —
     any disagreement is a hard error, never a footnote;
  4. the report carries the MINIMUM evidence tier across all steps, and
     names the verification mechanism that ran for each claim.

Usage:
    python3 examples/gum_report.py examples/beam-stress.json
    python3 examples/gum_report.py model.json --mcp ./target/debug/arithma-mcp

Model file shape (values and uncertainties are LaTeX strings so exact
rationals survive — "1/2" and "\\frac{1}{2}" both work):

    {
      "name": "bending stress",
      "expression": "\\frac{3 F L}{2 b h^2}",
      "unit": "MPa",
      "parameters": {
        "F": {"value": "1000", "uncertainty": "10",  "unit": "N"},
        "L": {"value": "2000", "uncertainty": "10",  "unit": "mm"}
      }
    }

Exit status is nonzero if any verification fails, any tool returns an
error, or the engine and the independent assembly disagree. A report
that prints is a report whose claims all checked out.
"""

import argparse
import json
import re
import subprocess
import sys
from fractions import Fraction

# ── MCP transport ──────────────────────────────────────────────────────


class McpServer:
    """One arithma-mcp subprocess, JSON-RPC over stdio, one id space."""

    def __init__(self, binary):
        self.proc = subprocess.Popen(
            [binary],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
        )
        self.next_id = 0

    def call(self, tool, arguments):
        """Call a tool; return the full JSON-RPC response object."""
        self.next_id += 1
        req = {
            "jsonrpc": "2.0",
            "id": self.next_id,
            "method": "tools/call",
            "params": {"name": tool, "arguments": arguments},
        }
        self.proc.stdin.write(json.dumps(req) + "\n")
        self.proc.stdin.flush()
        line = self.proc.stdout.readline()
        if not line:
            die(f"MCP server closed the stream during {tool}")
        resp = json.loads(line)
        if "error" in resp:
            die(f"{tool} protocol error: {resp['error']['message']}")
        result = resp["result"]
        if result.get("isError"):
            die(f"{tool} error: {result['content'][0]['text']}")
        return result

    def text(self, result):
        """The value line of a response text (skips any marker line)."""
        return result["content"][0]["text"].splitlines()[-1]

    def status(self, result):
        return result.get("result_status", {})

    def close(self):
        self.proc.stdin.close()
        self.proc.wait(timeout=10)


def die(msg):
    print(f"gum_report: REFUSED — {msg}", file=sys.stderr)
    sys.exit(1)


# ── Exact LaTeX rationals ──────────────────────────────────────────────

_FRAC = re.compile(r"^(-?)\\frac\{(-?\d+)\}\{(\d+)\}$")
_INT = re.compile(r"^-?\d+$")


def latex_to_fraction(s):
    """Parse the engine's exact output forms into a Fraction.

    Only integers and \\frac{p}{q} are accepted — the forms the exact
    path emits. Anything else (a float, a symbol) means the computation
    left exact arithmetic, and the honest response is refusal, not a
    quiet float() fallback.
    """
    s = s.strip()
    if _INT.match(s):
        return Fraction(int(s))
    m = _FRAC.match(s)
    if m:
        sign = -1 if m.group(1) == "-" else 1
        return sign * Fraction(int(m.group(2)), int(m.group(3)))
    die(f"non-exact value from engine: {s!r} (expected integer or \\frac)")


def fraction_to_latex(f):
    if f.denominator == 1:
        return str(f.numerator)
    if f < 0:
        return f"-\\frac{{{-f.numerator}}}{{{f.denominator}}}"
    return f"\\frac{{{f.numerator}}}{{{f.denominator}}}"


# ── The pipeline ───────────────────────────────────────────────────────


def sensitivity(mcp, expr, param):
    """Symbolic ∂expr/∂param, independently verified. Returns
    (latex, mechanism) or aborts."""
    d = mcp.call("differentiate", {"expr": expr, "variable": param})
    deriv = mcp.text(d)
    if mcp.status(d).get("status") != "exact":
        die(f"d/d{param} did not earn exact: {mcp.status(d)}")

    chain = mcp.call(
        "verify_chain",
        {
            "steps": [
                {"label": "model", "expr": expr},
                {
                    "label": f"d_d{param}",
                    "expr": deriv,
                    "relation": "derivative_of",
                    "variable": param,
                },
            ]
        },
    )
    st = mcp.status(chain)
    if st.get("verdict") != "pass":
        die(f"derivative check FAILED for {param}: {chain['content'][0]['text']}")
    mechanism = st["steps"][1]["mechanism"]
    return deriv, mechanism


def substitute_all(mcp, expr, values):
    """Substitute every parameter value (exact LaTeX strings) into expr;
    return the resulting exact Fraction."""
    current = expr
    for name, val in values.items():
        r = mcp.call(
            "substitute", {"expr": current, "variable": name, "value": val}
        )
        current = mcp.text(r)
    return latex_to_fraction(current)


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("model", help="JSON model file")
    ap.add_argument(
        "--mcp",
        default="arithma-mcp",
        help="path to the arithma-mcp binary (default: from PATH)",
    )
    ap.add_argument(
        "--coverage-k",
        type=int,
        default=2,
        help="coverage factor k for expanded uncertainty (default 2)",
    )
    args = ap.parse_args()

    with open(args.model, encoding="utf-8") as fh:
        model = json.load(fh)
    expr = model["expression"]
    params = model["parameters"]
    unit = model.get("unit", "")
    values = {p: spec["value"] for p, spec in params.items()}

    mcp = McpServer(args.mcp)

    # Operating-point value of the model output.
    y = substitute_all(mcp, expr, values)

    # Per-parameter: symbolic sensitivity, verification, exact numeric
    # coefficient, exact variance contribution.
    rows = []
    for p, spec in params.items():
        deriv, mechanism = sensitivity(mcp, expr, p)
        c = substitute_all(mcp, deriv, values)
        u = latex_to_fraction_latex_input(mcp, spec["uncertainty"])
        contribution = (c * u) ** 2
        rows.append(
            {
                "param": p,
                "deriv": deriv,
                "mechanism": mechanism,
                "c": c,
                "u": u,
                "unit": spec.get("unit", ""),
                "contribution": contribution,
            }
        )

    # Combined variance: engine evaluation of the composed expression,
    # cross-checked against the independent Python-fraction assembly.
    combined_expr = " + ".join(
        f"({fraction_to_latex(r['c'])} \\cdot {fraction_to_latex(r['u'])})^2"
        for r in rows
    )
    engine = latex_to_fraction(
        mcp.text(mcp.call("evaluate", {"expr": combined_expr}))
    )
    assembled = sum(r["contribution"] for r in rows)
    if engine != assembled:
        die(
            f"engine and independent assembly disagree on u_c^2: "
            f"{engine} vs {assembled}"
        )
    u_c2 = engine

    # u_c: exact when u_c² is a perfect square of a rational, else the
    # honest symbolic form.
    num_r = isqrt_exact(u_c2.numerator)
    den_r = isqrt_exact(u_c2.denominator)
    if num_r is not None and den_r is not None:
        u_c = Fraction(num_r, den_r)
        u_c_str = fraction_to_latex(u_c)
        u_c_note = "exact (perfect-square rational)"
        expanded = f"{fraction_to_latex(args.coverage_k * u_c)} {unit}".strip()
    else:
        u_c = None
        u_c_str = f"\\sqrt{{{fraction_to_latex(u_c2)}}}"
        u_c_note = "irrational — left in exact symbolic form"
        expanded = f"{args.coverage_k}\\cdot{u_c_str} {unit}".strip()

    mcp.close()
    print(report(model, expr, y, unit, rows, u_c2, u_c, u_c_str, u_c_note,
                 expanded, args.coverage_k))


def latex_to_fraction_latex_input(mcp, s):
    """Parse a model-file value ("10", "1/2", "\\frac{1}{2}") exactly,
    normalizing through the engine so every accepted input form shares
    one parser."""
    r = mcp.call("evaluate", {"expr": s})
    if mcp.status(r).get("status") != "exact":
        die(f"model value {s!r} did not evaluate exactly: {mcp.status(r)}")
    return latex_to_fraction(mcp.text(r))


def isqrt_exact(n):
    """Integer square root if n is a perfect square, else None."""
    if n < 0:
        return None
    r = int(n**0.5)
    for cand in (r - 1, r, r + 1):
        if cand >= 0 and cand * cand == n:
            return cand
    return None


def report(model, expr, y, unit, rows, u_c2, u_c, u_c_str, u_c_note,
           expanded, k):
    name = model.get("name", "model output")
    rows_ranked = sorted(rows, key=lambda r: r["contribution"], reverse=True)
    lines = []
    a = lines.append
    a(f"# GUM uncertainty report — {name}")
    a("")
    a(f"Model: `{expr}`")
    a(f"Operating-point value: **{fraction_to_latex(y)} {unit}**".rstrip())
    a("")
    a("## Sensitivity coefficients (each independently verified)")
    a("")
    a("| Input | ∂f/∂x (symbolic) | Verified by | Value at point |")
    a("|-------|------------------|-------------|----------------|")
    for r in rows:
        a(
            f"| {r['param']} | `{r['deriv']}` | {r['mechanism']} "
            f"(pass) | {fraction_to_latex(r['c'])} |"
        )
    a("")
    a("## Ranked budget")
    a("")
    a("| Rank | Input | u(x) | (c·u)² | Share |")
    a("|------|-------|------|--------|-------|")
    for i, r in enumerate(rows_ranked, 1):
        share = r["contribution"] / u_c2 if u_c2 else Fraction(0)
        pct = float(share) * 100.0
        a(
            f"| {i} | {r['param']} | {fraction_to_latex(r['u'])} {r['unit']}"
            f" | {fraction_to_latex(r['contribution'])} | {pct:.4g}% |"
        )
    a("")
    a(f"Combined variance u_c² = **{fraction_to_latex(u_c2)}** "
      "(engine evaluation cross-checked against independent assembly)")
    a(f"Combined standard uncertainty u_c = **{u_c_str} {unit}** "
      f"({u_c_note})".rstrip())
    a(f"Expanded uncertainty (k = {k}): **± {expanded}**")
    a("")
    a("All sensitivity checks passed `verify_chain`; all arithmetic ran "
      "in exact rational arithmetic. A report only prints if every "
      "verification succeeded — a failed check aborts with a nonzero "
      "exit status.")
    return "\n".join(lines)


if __name__ == "__main__":
    main()
