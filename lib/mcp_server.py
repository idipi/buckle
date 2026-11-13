#!/usr/bin/env python3
"""
Standard JSON-RPC 2.0 / MCP Server exposing audited macOS and Device Capabilities.
Runs over stdio to integrate seamlessly with Antigravity, Claude Desktop, Cursor, etc.
Zero external pip dependencies required.
"""

import json
import logging
import sys
from .broker import DeviceBroker, MacOSBroker

logger = logging.getLogger("mcp_server")

# Define MCP Tool Schemas
TOOLS_METADATA = [
    {
        "name": "device_list_serial",
        "description": "List all active serial communication ports (/dev/cu.*) connected to macOS.",
        "inputSchema": {
            "type": "object",
            "properties": {},
            "required": []
        }
    },
    {
        "name": "device_serial_communicate",
        "description": "Send a command string to a serial microcontroller/device and read the response.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "port": {"type": "string", "description": "Serial port device path (e.g. /dev/cu.usbserial-0001)"},
                "command": {"type": "string", "description": "Command string or payload to transmit"},
                "baudrate": {"type": "integer", "description": "Baud rate (default: 115200)"},
                "timeout_sec": {"type": "number", "description": "Read timeout in seconds (default: 2.0)"}
            },
            "required": ["port", "command"]
        }
    },
    {
        "name": "device_list_usb",
        "description": "Enumerate connected USB hardware tree via macOS system profiler.",
        "inputSchema": {
            "type": "object",
            "properties": {},
            "required": []
        }
    },
    {
        "name": "macos_notification",
        "description": "Display a safe native macOS user notification dialog.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "title": {"type": "string", "description": "Notification title"},
                "message": {"type": "string", "description": "Notification body message"},
                "subtitle": {"type": "string", "description": "Optional subtitle"}
            },
            "required": ["title", "message"]
        }
    },
    {
        "name": "macos_clipboard_read",
        "description": "Read plain text currently stored on the macOS system clipboard.",
        "inputSchema": {
            "type": "object",
            "properties": {},
            "required": []
        }
    },
    {
        "name": "macos_clipboard_write",
        "description": "Write plain text to the macOS system clipboard.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "text": {"type": "string", "description": "Text to place on clipboard"}
            },
            "required": ["text"]
        }
    },
    {
        "name": "macos_system_telemetry",
        "description": "Retrieve current battery, load average, and system uptime from macOS.",
        "inputSchema": {
            "type": "object",
            "properties": {},
            "required": []
        }
    }
]


def execute_tool(name, arguments):
    """Dispatch tool execution to the appropriate broker capability."""
    args = arguments or {}
    if name == "device_list_serial":
        return DeviceBroker.list_serial_ports()
    elif name == "device_serial_communicate":
        return DeviceBroker.serial_communicate(
            port_path=args.get("port", ""),
            command_str=args.get("command", ""),
            baudrate=args.get("baudrate", 115200),
            timeout_sec=args.get("timeout_sec", 2.0)
        )
    elif name == "device_list_usb":
        return DeviceBroker.list_usb_tree()
    elif name == "macos_notification":
        return MacOSBroker.show_notification(
            title=args.get("title", "Agent Notification"),
            message=args.get("message", ""),
            subtitle=args.get("subtitle", "")
        )
    elif name == "macos_clipboard_read":
        return MacOSBroker.get_clipboard()
    elif name == "macos_clipboard_write":
        return MacOSBroker.set_clipboard(text=args.get("text", ""))
    elif name == "macos_system_telemetry":
        return MacOSBroker.get_system_telemetry()
    else:
        return {"error": f"Unknown tool: {name}"}


def serve_mcp_stdio():
    """Main stdio loop for Model Context Protocol JSON-RPC."""
    while True:
        line = sys.stdin.readline()
        if not line:
            break
        line = line.strip()
        if not line:
            continue

        try:
            req = json.loads(line)
        except json.JSONDecodeError:
            continue

        req_id = req.get("id")
        method = req.get("method")
        params = req.get("params", {})

        # MCP Protocol Handlers
        if method == "initialize":
            res = {
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "serverInfo": {
                        "name": "macos-agent-sandbox-broker",
                        "version": "1.0.0"
                    },
                    "capabilities": {
                        "tools": {}
                    }
                }
            }
        elif method == "notifications/initialized":
            continue
        elif method == "tools/list":
            res = {
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {
                    "tools": TOOLS_METADATA
                }
            }
        elif method == "tools/call":
            tool_name = params.get("name")
            tool_args = params.get("arguments", {})
            tool_result = execute_tool(tool_name, tool_args)
            res = {
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {
                    "content": [
                        {
                            "type": "text",
                            "text": json.dumps(tool_result, indent=2, ensure_ascii=False)
                        }
                    ]
                }
            }
        elif method == "ping":
            res = {"jsonrpc": "2.0", "id": req_id, "result": {}}
        else:
            res = {
                "jsonrpc": "2.0",
                "id": req_id,
                "error": {
                    "code": -32601,
                    "message": f"Method not supported: {method}"
                }
            }

        sys.stdout.write(json.dumps(res) + "\n")
        sys.stdout.flush()


if __name__ == "__main__":
    serve_mcp_stdio()
