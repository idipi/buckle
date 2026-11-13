//! Fast Model Context Protocol (MCP) JSON-RPC 2.0 stdio server.
//! Exposes audited device and macOS capabilities to AI agents (Antigravity, Claude, Cursor).

use anyhow::Result;
use serde_json::json;
use std::io::{BufRead, Write};

use crate::broker::{DeviceBroker, MacOSBroker};

pub fn run_mcp_stdio_server() -> Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let req: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let req_id = req.get("id").cloned();
        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = req.get("params").cloned().unwrap_or(json!({}));

        let resp = match method {
            "initialize" => json!({
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "serverInfo": {
                        "name": "macos-agent-sandbox-broker-rs",
                        "version": env!("CARGO_PKG_VERSION")
                    },
                    "capabilities": {
                        "tools": {}
                    }
                }
            }),
            "notifications/initialized" => continue,
            "tools/list" => json!({
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {
                    "tools": get_tools_metadata()
                }
            }),
            "tools/call" => {
                let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let tool_args = params.get("arguments").cloned().unwrap_or(json!({}));
                let res = execute_tool(tool_name, &tool_args);

                json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": serde_json::to_string_pretty(&res).unwrap_or_default()
                            }
                        ]
                    }
                })
            }
            "ping" => json!({
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {}
            }),
            _ => json!({
                "jsonrpc": "2.0",
                "id": req_id,
                "error": {
                    "code": -32601,
                    "message": format!("Method not found: {}", method)
                }
            }),
        };

        let _ = writeln!(stdout, "{}", serde_json::to_string(&resp)?);
        let _ = stdout.flush();
    }

    Ok(())
}

fn execute_tool(name: &str, args: &serde_json::Value) -> serde_json::Value {
    match name {
        "device_list_serial" => {
            serde_json::to_value(DeviceBroker::list_serial_ports().unwrap_or_default()).unwrap_or_default()
        }
        "device_list_usb" => DeviceBroker::list_usb_tree().unwrap_or(json!([])),
        "macos_notification" => {
            let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("Agent Notification");
            let message = args.get("message").and_then(|v| v.as_str()).unwrap_or("");
            let subtitle = args.get("subtitle").and_then(|v| v.as_str()).unwrap_or("");
            let ok = MacOSBroker::show_notification(title, message, subtitle).unwrap_or(false);
            json!({ "success": ok })
        }
        "macos_clipboard_read" => {
            let text = MacOSBroker::get_clipboard(16384).unwrap_or_default();
            json!({ "success": true, "text": text })
        }
        "macos_clipboard_write" => {
            let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
            let ok = MacOSBroker::set_clipboard(text, 32768).unwrap_or(false);
            json!({ "success": ok })
        }
        "macos_system_telemetry" => MacOSBroker::get_system_telemetry().unwrap_or(json!({})),
        _ => json!({ "error": format!("Unknown tool: {}", name) }),
    }
}

fn get_tools_metadata() -> serde_json::Value {
    json!([
        {
            "name": "device_list_serial",
            "description": "List all active serial communication ports (/dev/cu.*) connected to macOS.",
            "inputSchema": { "type": "object", "properties": {}, "required": [] }
        },
        {
            "name": "device_list_usb",
            "description": "Enumerate connected USB hardware tree via macOS system profiler.",
            "inputSchema": { "type": "object", "properties": {}, "required": [] }
        },
        {
            "name": "macos_notification",
            "description": "Display a safe native macOS user notification dialog.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Notification title" },
                    "message": { "type": "string", "description": "Notification body message" },
                    "subtitle": { "type": "string", "description": "Optional subtitle" }
                },
                "required": ["title", "message"]
            }
        },
        {
            "name": "macos_clipboard_read",
            "description": "Read plain text currently stored on the macOS system clipboard.",
            "inputSchema": { "type": "object", "properties": {}, "required": [] }
        },
        {
            "name": "macos_clipboard_write",
            "description": "Write plain text to the macOS system clipboard.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "Text to place on clipboard" }
                },
                "required": ["text"]
            }
        },
        {
            "name": "macos_system_telemetry",
            "description": "Retrieve current battery, load average, and system uptime from macOS.",
            "inputSchema": { "type": "object", "properties": {}, "required": [] }
        }
    ])
}
