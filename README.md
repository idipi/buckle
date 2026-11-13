# macos-agent-sandbox

A production-grade, zero-overhead security toolkit for running AI coding agents, autonomous LLM workflows, and untrusted code on **macOS (Apple Silicon & Intel)**.

Operates **a level below application prompts** by enforcing kernel-level Mandatory Access Control (MAC via Apple Seatbelt), strict domain-level network egress filtering, and an audited capability broker for interacting with macOS APIs and hardware peripherals (USB/Serial).

---

## Architecture Overview

```
                                      ┌────────────────────────────────────────────────┐
                                      │             AI Agent / LLM Shell               │
                                      │   (Antigravity, Claude, Cursor, Aider, etc.)   │
                                      └───────────────────────┬────────────────────────┘
                                                              │
                                     ┌────────────────────────┴────────────────────────┐
                                     ▼                                                 ▼
┌────────────────────────────────────────────────────────┐   ┌─────────────────────────────────────────────────────────┐
│        1. Apple Seatbelt (Kernel Sandbox Engine)       │   │        2. Audited Capability Broker (CLI & MCP)         │
│   Enforced directly via /usr/bin/sandbox-exec (SBPL)   │   │     Safely bridges to macOS APIs & Hardware Devices     │
├────────────────────────────────────────────────────────┤   ├─────────────────────────────────────────────────────────┤
│ [✓] Read/Write: Current project workspace & /tmp       │   │ [✓] Serial: /dev/cu.* microcontrollers (8N1, timeout)   │
│ [✓] Read-only:  System binaries, toolchains, libs      │   │ [✓] USB:    Query USB hardware topology via profiler    │
│ [X] BLOCKED:    ~/.ssh, ~/.aws, ~/.gnupg, Keychains    │   │ [✓] macOS:  Sanitized desktop notifications             │
│ [X] BLOCKED:    Browser sessions, cookies, history     │   │ [✓] macOS:  Controlled clipboard & system telemetry    │
│ [X] BLOCKED:    Raw disk access (/dev/disk*, rdisk*)   │   │ [✓] MCP:    Standard JSON-RPC 2.0 stdio server          │
└────────────────────────────┬───────────────────────────┘   └─────────────────────────────────────────────────────────┘
                             │
                             ▼
┌────────────────────────────────────────────────────────┐
│           3. Egress-Filtering Local Proxy              │
│       Enforces Whitelist for Outbound Connections      │
├────────────────────────────────────────────────────────┤
│ [✓] Allowed: LLM APIs (Anthropic, Gemini, OpenAI)      │
│ [✓] Allowed: Package registries (GitHub, PyPI, npm)    │
│ [X] BLOCKED: Untrusted external domains (HTTP 403)     │
│ [X] BLOCKED: Direct IP connects blocked by kernel EPERM│
└────────────────────────────────────────────────────────┘
```

---

## Key Features

1. **Kernel-Enforced Filesystem Containment**:
   - Uses Apple's native **Seatbelt (SBPL)** engine (`sandbox-exec`).
   - Even if an agent is tricked by a Prompt Injection into running `cat ~/.ssh/id_rsa` or `rm -rf /`, the kernel halts the operation with `Operation not permitted (EPERM)`.
   - Your home directory root, cloud keys (`~/.aws`, `~/.config/gcloud`), GPG keys, macOS Keychains, browser profiles (Chrome, Arc, Safari), and shell history files are completely invisible.

2. **Unbypassable Network Egress Filtering**:
   - In `proxy` mode (default), the kernel **denies all outbound network sockets** except loopback to `localhost`.
   - Outbound connections must traverse the built-in filtering proxy, which inspects HTTP/HTTPS `CONNECT` handshakes against [allowed_domains.txt](file:///Users/ench/projects/macos-agent-sandbox/rules/allowed_domains.txt).
   - Prevents exfiltration of project tokens, environment variables, or private source code.

3. **Controlled Hardware & Device Access**:
   - Supports serial adapters and microcontrollers (`/dev/cu.*`, `/dev/tty.*`) for embedded and hardware projects.
   - Denies raw storage block devices (`/dev/disk*`, `/dev/rdisk*`) to eliminate catastrophic disk format risks.

4. **Audited macOS System Integration (Broker & MCP)**:
   - Exposes safe system calls (notifications, clipboard, battery status) without granting unrestricted Accessibility or AppleScript execution permissions.
   - Includes a built-in **Model Context Protocol (MCP)** server for plug-and-play integration with modern AI assistants.

5. **Zero External Dependencies**:
   - Built entirely on macOS system binaries and Python 3 standard library.
   - No Docker daemon, no VM spin-up delay, no performance loss.

---

## Quick Start

### 1. Run System Health Audit
Verify that your macOS version, Seatbelt compiler, credential lock, and rules are active:

```bash
./bin/agent-sandbox doctor
```

Output:
```
==================================================
   macOS Agent Sandbox - System Health Audit      
==================================================
[+] macOS Version:    15.8.3 (arm64)
[+] Seatbelt Engine:  Available (/usr/bin/sandbox-exec)
[+] Credential Lock:  VERIFIED (~/.ssh is blocked with EPERM)
[+] Serial Devices:   3 available in /dev/cu.*
[+] Egress Whitelist: Active (29 domains configured in allowed_domains.txt)
==================================================
Audit status: All primary security controls operational.
```

### 2. Run an Isolated Command
Execute any command within the current workspace:

```bash
# Sandboxed Python execution
./bin/agent-sandbox run python3 -c "print('Running inside sandbox!')"

# Sandboxed Git command
./bin/agent-sandbox run git status
```

### 3. Launch an Interactive Sandboxed Shell
Spawn an isolated subshell for prolonged development sessions:

```bash
./bin/agent-sandbox shell
```

Inside this shell:
* Try `ls ~/.ssh` $\rightarrow$ `Operation not permitted`
* Try `curl https://httpbin.org` $\rightarrow$ `HTTP 403 Forbidden`
* Try `curl https://api.github.com` $\rightarrow$ `HTTP 200 OK`
* Write files in your workspace $\rightarrow$ Allowed

### 4. Run the Automated Security Test Suite
Verify that all 10 kernel isolation invariants pass:

```bash
python3 tests/test_sandbox.py
```

---

## Command Reference

### `agent-sandbox run`
```bash
./bin/agent-sandbox run [OPTIONS] <COMMAND...>

Options:
  -w, --workspace PATH     Target workspace directory (default: current directory)
  --net {proxy,allow,deny} Outbound network policy (default: proxy)
  --devices {serial,allow,deny}
                           Hardware device access policy (default: serial)
  --writable PATH          Additional directory to permit write operations
  --rules PATH             Path to custom domain whitelist file
```

### `agent-broker` (Hardware & macOS Interface)
The capability broker allows agents to interact with devices and macOS without granting root:

```bash
# Enumerate serial devices
./bin/agent-broker serial list

# Send command to microcontroller (ESP32, Arduino, Raspberry Pi Pico)
./bin/agent-broker serial send --port /dev/cu.usbmodem1101 --cmd "STATUS" --baud 115200

# Inspect connected USB devices
./bin/agent-broker usb list

# Display safe native notification
./bin/agent-broker notify --title "Build Complete" --message "All tests passed."

# Read or write clipboard safely
./bin/agent-broker clip get
./bin/agent-broker clip set "Cleaned output"

# Query system telemetry
./bin/agent-broker status

# Start as Model Context Protocol (MCP) server over stdio
./bin/agent-broker mcp
```

---

## AI Agent Integration Guide

### Integration with Antigravity / Claude / Cursor via MCP
Add the broker to your agent's MCP configuration (`mcp_config.json` or `claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "macos-agent-broker": {
      "command": "/opt/homebrew/bin/python3",
      "args": [
        "/Users/ench/projects/macos-agent-sandbox/bin/agent-broker",
        "mcp"
      ]
    }
  }
}
```

This gives the agent structured tools (`device_list_serial`, `device_serial_communicate`, `macos_notification`, etc.) while ensuring all bash executions run wrapped inside `agent-sandbox run`.

### Shell Alias Wrapper for Everyday Use
Add to your `~/.zshrc`:

```bash
# Wrap agent CLI in the sandbox
alias safe-agy="/Users/ench/projects/macos-agent-sandbox/bin/agent-sandbox run agy"
alias safe-shell="/Users/ench/projects/macos-agent-sandbox/bin/agent-sandbox shell"
```

---

## Directory Structure

* [bin/agent-sandbox](file:///Users/ench/projects/macos-agent-sandbox/bin/agent-sandbox) - Primary CLI runner and environment supervisor.
* [bin/agent-broker](file:///Users/ench/projects/macos-agent-sandbox/bin/agent-broker) - Audited device and macOS capability gateway / MCP server.
* [lib/sandbox_generator.py](file:///Users/ench/projects/macos-agent-sandbox/lib/sandbox_generator.py) - Dynamic Apple Seatbelt profile compiler and process manager.
* [lib/proxy.py](file:///Users/ench/projects/macos-agent-sandbox/lib/proxy.py) - Zero-dependency domain-whitelisting HTTP/HTTPS CONNECT proxy.
* [lib/broker.py](file:///Users/ench/projects/macos-agent-sandbox/lib/broker.py) - Sanitized implementations for serial ports, USB tree, notifications, and clipboard.
* [lib/mcp_server.py](file:///Users/ench/projects/macos-agent-sandbox/lib/mcp_server.py) - Standard JSON-RPC 2.0 stdio engine for Model Context Protocol.
* [profiles/default.sb.template](file:///Users/ench/projects/macos-agent-sandbox/profiles/default.sb.template) - Master Apple Seatbelt profile template (Scheme / SBPL).
* [rules/allowed_domains.txt](file:///Users/ench/projects/macos-agent-sandbox/rules/allowed_domains.txt) - Default egress whitelist for LLM providers and developer registries.
* [rules/pf_rules.conf](file:///Users/ench/projects/macos-agent-sandbox/rules/pf_rules.conf) - Template for macOS native packet filter (`pfctl`).
* [tests/test_sandbox.py](file:///Users/ench/projects/macos-agent-sandbox/tests/test_sandbox.py) - Automated security regression test suite.
