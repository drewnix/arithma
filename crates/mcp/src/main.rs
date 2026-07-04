//! Arithma MCP server binary: newline-delimited JSON-RPC over stdio.
//! All request handling lives in the library so it can be tested directly.

use std::io::{self, BufRead, Write};

use serde_json::{json, Value};

use arithma_mcp_server::{handle_initialize, handle_tools_call, handle_tools_list, json_rpc_error};

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
                write_response(
                    &mut out,
                    json_rpc_error(None, -32700, &format!("Parse error: {}", e)),
                );
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
