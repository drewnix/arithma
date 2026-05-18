use std::io::{self, BufRead, Write};

use serde_json::{json, Value};

use arithma::derivative::differentiate_latex;
use arithma::exact::ExactNum;
use arithma::integration::integrate_latex;
use arithma::limits::limit_latex;
use arithma::series::taylor_series_latex;
use arithma::simplify::Simplifiable;
use arithma::{
    build_expression_tree, solve_for_variable_exact, Environment, Evaluator, Polynomial, Tokenizer,
};

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        let request: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                write_response(&mut out, json_rpc_error(None, -32700, &format!("Parse error: {}", e)));
                continue;
            }
        };

        let id = request.get("id").cloned();
        let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = request.get("params").cloned().unwrap_or(json!({}));

        let response = match method {
            "initialize" => handle_initialize(id, &params),
            "notifications/initialized" => continue, // no response needed
            "tools/list" => handle_tools_list(id),
            "tools/call" => handle_tools_call(id, &params),
            "ping" => json!({ "jsonrpc": "2.0", "id": id, "result": {} }),
            _ => json_rpc_error(id, -32601, &format!("Method not found: {}", method)),
        };

        write_response(&mut out, response);
    }
}

fn write_response(out: &mut impl Write, response: Value) {
    let s = serde_json::to_string(&response).unwrap();
    let _ = writeln!(out, "{}", s);
    let _ = out.flush();
}

fn json_rpc_error(id: Option<Value>, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })
}

fn handle_initialize(id: Option<Value>, _params: &Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "arithma",
                "version": "0.1.0"
            }
        }
    })
}

fn handle_tools_list(id: Option<Value>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "tools": tools_schema()
        }
    })
}

fn tools_schema() -> Value {
    json!([
        {
            "name": "simplify",
            "description": "Simplify a mathematical expression. Returns the simplified form in LaTeX. Handles polynomial normalization, trigonometric identities, logarithmic properties, and multivariate GCD cancellation.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "expr": {
                        "type": "string",
                        "description": "LaTeX expression to simplify, e.g. \"x^2 + 2x + 1\" or \"\\frac{x^2 - 1}{x - 1}\""
                    }
                },
                "required": ["expr"]
            }
        },
        {
            "name": "differentiate",
            "description": "Compute the derivative of an expression with respect to a variable. Supports polynomials, trigonometric, exponential, logarithmic, and composed functions via chain rule.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "expr": {
                        "type": "string",
                        "description": "LaTeX expression to differentiate, e.g. \"x^3 + \\sin(x)\""
                    },
                    "variable": {
                        "type": "string",
                        "description": "Variable to differentiate with respect to",
                        "default": "x"
                    }
                },
                "required": ["expr"]
            }
        },
        {
            "name": "integrate",
            "description": "Compute the indefinite integral of an expression. Returns the antiderivative in LaTeX (without the +C constant).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "expr": {
                        "type": "string",
                        "description": "LaTeX expression to integrate, e.g. \"3x^2\" or \"\\sin(x)\""
                    },
                    "variable": {
                        "type": "string",
                        "description": "Variable of integration",
                        "default": "x"
                    }
                },
                "required": ["expr"]
            }
        },
        {
            "name": "solve",
            "description": "Solve an equation for a variable. Input should contain '=' sign. Returns exact solutions when possible (rational roots, quadratic formula, Cardano's formula for cubics, Ferrari's method for quartics).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "equation": {
                        "type": "string",
                        "description": "LaTeX equation to solve, e.g. \"x^2 - 5x + 6 = 0\" or \"2x + 1 = 7\""
                    },
                    "variable": {
                        "type": "string",
                        "description": "Variable to solve for",
                        "default": "x"
                    }
                },
                "required": ["equation"]
            }
        },
        {
            "name": "factor",
            "description": "Factor a polynomial expression. Returns the square-free factorization showing irreducible factors and their multiplicities.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "expr": {
                        "type": "string",
                        "description": "LaTeX polynomial to factor, e.g. \"x^4 - 1\" or \"x^3 - 3x^2 + 3x - 1\""
                    },
                    "variable": {
                        "type": "string",
                        "description": "Main variable of the polynomial",
                        "default": "x"
                    }
                },
                "required": ["expr"]
            }
        },
        {
            "name": "limit",
            "description": "Compute the limit of an expression as a variable approaches a point. Handles direct substitution, polynomial GCD cancellation for 0/0 forms, and L'Hôpital's rule for transcendental indeterminate forms.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "expr": {
                        "type": "string",
                        "description": "LaTeX expression, e.g. \"\\frac{\\sin(x)}{x}\" or \"\\frac{x^2-1}{x-1}\""
                    },
                    "variable": {
                        "type": "string",
                        "description": "Variable approaching the limit point",
                        "default": "x"
                    },
                    "point": {
                        "type": "number",
                        "description": "The point the variable approaches",
                        "default": 0
                    }
                },
                "required": ["expr"]
            }
        },
        {
            "name": "taylor_series",
            "description": "Compute the Taylor (or Maclaurin) series expansion of an expression around a center point. Returns exact rational coefficients when possible (e.g. 1/24 instead of 0.041666...).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "expr": {
                        "type": "string",
                        "description": "LaTeX expression to expand, e.g. \"\\sin(x)\" or \"e^x\" or \"\\frac{1}{1-x}\""
                    },
                    "variable": {
                        "type": "string",
                        "description": "Variable to expand in",
                        "default": "x"
                    },
                    "center": {
                        "type": "number",
                        "description": "Center point of the expansion (0 for Maclaurin)",
                        "default": 0
                    },
                    "order": {
                        "type": "integer",
                        "description": "Maximum degree of the expansion",
                        "default": 5
                    }
                },
                "required": ["expr"]
            }
        },
        {
            "name": "evaluate",
            "description": "Evaluate a mathematical expression numerically with given variable values. Returns an exact rational result when possible, otherwise a decimal approximation.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "expr": {
                        "type": "string",
                        "description": "LaTeX expression to evaluate, e.g. \"x^2 + 2x + 1\" or \"\\sin(\\pi/6)\""
                    },
                    "variables": {
                        "type": "object",
                        "description": "Variable assignments as key-value pairs, e.g. {\"x\": 3, \"y\": 4}",
                        "additionalProperties": { "type": "number" }
                    }
                },
                "required": ["expr"]
            }
        }
    ])
}

fn handle_tools_call(id: Option<Value>, params: &Value) -> Value {
    let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    let result = match tool_name {
        "simplify" => tool_simplify(&args),
        "differentiate" => tool_differentiate(&args),
        "integrate" => tool_integrate(&args),
        "solve" => tool_solve(&args),
        "factor" => tool_factor(&args),
        "limit" => tool_limit(&args),
        "taylor_series" => tool_taylor_series(&args),
        "evaluate" => tool_evaluate(&args),
        _ => Err(format!("Unknown tool: {}", tool_name)),
    };

    match result {
        Ok(text) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "content": [{ "type": "text", "text": text }]
            }
        }),
        Err(e) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "content": [{ "type": "text", "text": format!("Error: {}", e) }],
                "isError": true
            }
        }),
    }
}

fn get_str<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(|v| v.as_str())
}

fn get_str_or<'a>(args: &'a Value, key: &str, default: &'a str) -> &'a str {
    get_str(args, key).unwrap_or(default)
}

fn parse_and_simplify(expr_str: &str) -> Result<String, String> {
    let mut tokenizer = Tokenizer::new(expr_str);
    let tokens = tokenizer.tokenize();
    let expr = build_expression_tree(tokens)?;
    let env = Environment::new();
    let simplified = expr.simplify(&env).unwrap_or(expr);
    Ok(format!("{}", simplified))
}

fn tool_simplify(args: &Value) -> Result<String, String> {
    let expr = get_str(args, "expr").ok_or("Missing required parameter: expr")?;
    parse_and_simplify(expr)
}

fn tool_differentiate(args: &Value) -> Result<String, String> {
    let expr = get_str(args, "expr").ok_or("Missing required parameter: expr")?;
    let var = get_str_or(args, "variable", "x");
    differentiate_latex(expr, var)
}

fn tool_integrate(args: &Value) -> Result<String, String> {
    let expr = get_str(args, "expr").ok_or("Missing required parameter: expr")?;
    let var = get_str_or(args, "variable", "x");
    integrate_latex(expr, var)
}

fn tool_solve(args: &Value) -> Result<String, String> {
    let equation = get_str(args, "equation").ok_or("Missing required parameter: equation")?;
    let var = get_str_or(args, "variable", "x");

    let mut tokenizer = Tokenizer::new(equation);
    let tokens = tokenizer.tokenize();
    let expr = build_expression_tree(tokens)?;

    let solutions = solve_for_variable_exact(&expr, var)?;
    if solutions.is_empty() {
        Ok("No solutions found".to_string())
    } else {
        let parts: Vec<String> = solutions
            .iter()
            .map(|s| format!("{} = {}", var, s))
            .collect();
        Ok(parts.join(", "))
    }
}

fn tool_factor(args: &Value) -> Result<String, String> {
    let expr = get_str(args, "expr").ok_or("Missing required parameter: expr")?;
    let var = get_str_or(args, "variable", "x");

    let mut tokenizer = Tokenizer::new(expr);
    let tokens = tokenizer.tokenize();
    let node = build_expression_tree(tokens)?;
    let poly = Polynomial::from_node(&node, var)
        .map_err(|e| format!("Not a polynomial: {}", e))?;

    let factors = poly.square_free_decomposition();
    let parts: Vec<String> = factors
        .iter()
        .map(|(f, m)| {
            if *m == 1 {
                format!("({})", f)
            } else {
                format!("({})^{}", f, m)
            }
        })
        .collect();
    Ok(parts.join(" \\cdot "))
}

fn tool_limit(args: &Value) -> Result<String, String> {
    let expr = get_str(args, "expr").ok_or("Missing required parameter: expr")?;
    let var = get_str_or(args, "variable", "x");
    let point = args
        .get("point")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    limit_latex(expr, var, point)
}

fn tool_taylor_series(args: &Value) -> Result<String, String> {
    let expr = get_str(args, "expr").ok_or("Missing required parameter: expr")?;
    let var = get_str_or(args, "variable", "x");
    let center = args
        .get("center")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let order = args
        .get("order")
        .and_then(|v| v.as_u64())
        .unwrap_or(5) as usize;
    taylor_series_latex(expr, var, center, order)
}

fn tool_evaluate(args: &Value) -> Result<String, String> {
    let expr_str = get_str(args, "expr").ok_or("Missing required parameter: expr")?;

    let mut tokenizer = Tokenizer::new(expr_str);
    let tokens = tokenizer.tokenize();
    let expr = build_expression_tree(tokens)?;

    let env_simplified = Environment::new();
    let simplified = expr
        .simplify(&env_simplified)
        .unwrap_or_else(|_| expr.clone());

    let mut env = Environment::new();
    if let Some(vars) = args.get("variables").and_then(|v| v.as_object()) {
        for (k, v) in vars {
            if let Some(f) = v.as_f64() {
                if f == f.floor() && f.abs() < 1e15 {
                    env.set_exact(k, ExactNum::integer(f as i64));
                } else {
                    env.set(k, f);
                }
            }
        }
    }

    match Evaluator::evaluate_exact(&simplified, &env) {
        Ok(val) => Ok(format!("{}", arithma::Node::Num(val))),
        Err(_) => {
            match Evaluator::evaluate(&simplified, &env) {
                Ok(val) => Ok(val.to_string()),
                Err(_) => Ok(format!("{}", simplified)),
            }
        }
    }
}
