# macos-agent-sandbox

A high-performance, kernel-level security sandbox and hardware capability broker for running AI coding agents, autonomous LLM workflows, and untrusted scripts on **macOS (Apple Silicon & Intel)**.

Built in **100% pure native Rust** with direct FFI bindings to Apple's `libSystem.B.dylib` (`sandbox_init`). Operates **completely in-memory with zero temporary profile files on disk**, sub-millisecond startup overhead, and zero external runtime dependencies.

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
│     1. Apple Seatbelt (In-Memory Kernel Sandbox)       │   │        2. Audited Capability Broker (CLI & MCP)         │
│   Native Rust FFI -> libSystem sandbox_init() in RAM   │   │     Safely bridges to macOS APIs & Hardware Devices     │
├────────────────────────────────────────────────────────┤   ├─────────────────────────────────────────────────────────┤
│ [✓] Read/Write: Current project workspace & tmpfs      │   │ [✓] Serial: /dev/cu.* microcontrollers (UART)           │
│ [✓] Read-only:  System binaries, toolchains, libs      │   │ [✓] USB:    Query USB hardware topology via profiler    │
│ [✓] Flashing:   IOKit USB access (openocd, qdl, dfu)   │   │ [✓] Flashing: Direct libusb & JTAG / EDL endpoint access│
│ [X] BLOCKED:    ~/.ssh, ~/.gnupg, Keychains (write)    │   │ [✓] macOS:  Sanitized desktop notifications             │
│ [X] BLOCKED:    Browser sessions, cookies, history     │   │ [✓] macOS:  Controlled clipboard & system telemetry    │
│ [X] BLOCKED:    Raw disk access (/dev/disk*, rdisk*)   │   │ [✓] MCP:    High-speed JSON-RPC 2.0 stdio server        │
│ [X] SCRUBBED:   Host secrets scrubbed from environment │   │ [✓] Clean:  Automatic orphan process tree reaper       │
└────────────────────────────────────────────────────────┘   └─────────────────────────────────────────────────────────┘
```

---

## Key Features

1. **Kernel-Enforced Filesystem Containment**:
   - Uses Apple's native **Seatbelt (SBPL)** engine directly via C FFI (`sandbox_init`).
   - Even if an agent is tricked by prompt injection into running `cat ~/.ssh/id_rsa` or `rm -rf /`, the kernel halts the operation with `Operation not permitted (EPERM)`.
   - Your home directory root, cloud keys (`~/.config/gcloud`), GPG keys, macOS Keychains, browser profiles (Chrome, Arc, Safari), and shell history files are completely protected.

2. **Environment Variable Secret Scrubbing**:
   - Automatically sanitizes the environment before spawning untrusted processes.
   - Strips any variables matching sensitive patterns (`*TOKEN*`, `*KEY*`, `*SECRET*`, `*PASSWORD*`, `*AUTH*`, `*CREDENTIAL*`, `*PRIVATE*`) unless explicitly allowed via `--pass-env`.

3. **Controlled Hardware & Microcontroller Access**:
   - Supports serial adapters and microcontrollers (`/dev/cu.*`, `/dev/tty.*`) for embedded and hardware projects.
   - Enables `IOKit` USB device client classes (`IOUSBDeviceUserClientV2`) for firmware flashers (`probe-rs`, `openocd`, `dfu-util`, `qdl`, `st-flash`).
   - Denies raw storage block devices (`/dev/disk*`, `/dev/rdisk*`) to eliminate disk tampering risks.

4. **Audited macOS System Integration (Broker & MCP)**:
   - Exposes safe system calls (notifications, clipboard, battery status) without granting unrestricted Accessibility or AppleScript execution permissions.
   - High-performance native **Model Context Protocol (MCP)** server over stdio for plug-and-play integration with Antigravity, Claude, and Cursor.

5. **Process Group & Ephemeral Session Management**:
   - Allocates dedicated ephemeral session storage under `/private/tmp/agent_sandbox/session_*` with RAII automatic cleanup on exit.
   - Tracks child process groups (`setpgid`) and reaps orphaned background daemons on exit.

---

## Installation

### Via Homebrew Tap
```bash
brew tap idipi/tap
brew install agent-sandbox
```

### Build from Source
```bash
git clone https://github.com/idipi/macos-agent-sandbox.git
cd macos-agent-sandbox
cargo build --release
```
Compiled binaries will be in `target/release/agent-sandbox` and `target/release/agent-broker`.

---

## Quick Start

### 1. Run System Health Audit
Verify that your macOS version, Seatbelt FFI engine, credential lock, and serial devices are active:

```bash
agent-sandbox doctor
```

Output:
```
==================================================
   macOS Agent Sandbox - System Health Audit (Rust)
==================================================
[+] macOS Version:    15.8.3 (arm64)
[+] Kernel Sandbox:   Direct FFI libSystem.B.dylib (sandbox_init)
[+] Credential Lock:  VERIFIED (~/.ssh blocked in-memory by kernel)
[+] Serial Devices:   3 available in /dev/cu.*
==================================================
Audit status: All primary security controls operational.
```

### 2. Run an Isolated Command
Execute any command within the current workspace:

```bash
# Sandboxed compilation
agent-sandbox run cargo build

# Sandboxed Git status
agent-sandbox run git status
```

### 3. Launch an Interactive Sandboxed Shell
Spawn an isolated subshell for prolonged development sessions:

```bash
agent-sandbox shell
```

Inside this shell:
* Try `ls ~/.ssh` $\rightarrow$ `Operation not permitted`
* Try `touch ~/.test` $\rightarrow$ `Operation not permitted`
* Try `touch file.txt` (in workspace) $\rightarrow$ `Allowed`

### 4. Run the Automated Security Test Suite
Verify that all 12 kernel isolation invariants pass:

```bash
agent-sandbox test
# or: cargo test --test integration_tests
```

---

## Command Reference

### `agent-sandbox`

```bash
agent-sandbox [COMMAND]

Commands:
  run          Execute an isolated command inside the Apple Seatbelt kernel sandbox
  shell        Launch an interactive sandboxed shell (zsh/bash) with ephemeral session storage
  doctor       Audit system security controls and in-memory kernel sandbox
  test         Execute automated security isolation regression test suite (12 invariants)
  broker       Hardware and macOS capability broker interface
  mcp          Run Model Context Protocol (MCP) stdio server
  completions  Generate shell autocompletion script (bash, zsh, fish, powershell)
  man          Render Unix roff man page documentation to stdout
```

#### `agent-sandbox run` Options
* `-w, --workspace <PATH>`: Target workspace directory (default: current directory). Guarded against running directly in `$HOME`.
* `-n, --net <allow|proxy|deny>`: Outbound network policy (default: `allow`).
* `-d, --devices <serial|allow|deny>`: Hardware device access policy (default: `serial`).
* `--writable <PATH>`: Additional directory permitted for write operations.
* `-e, --env <KEY=VAL>`: Set custom environment variables.
* `--pass-env <NAME>`: Explicitly pass through sensitive host environment variables.
* `--allow-home`: Explicitly override safety guard and permit workspace in `$HOME`.

---

### `agent-broker` (Hardware & macOS Interface)

```bash
# Enumerate serial devices (/dev/cu.*)
agent-broker serial list

# Inspect connected USB devices
agent-broker usb list

# Display safe native notification
agent-broker notify --title "Build Complete" --message "All tests passed."

# Read or write clipboard safely
agent-broker clip get
agent-broker clip set "Cleaned output"

# Query system telemetry
agent-broker status

# Start as Model Context Protocol (MCP) server over stdio
agent-broker mcp

# Shell completions and man pages
agent-broker completions zsh
agent-broker man
```

---

## AI Agent Integration Guide

### Integration with Antigravity / Claude / Cursor via MCP
Add the broker to your agent's MCP configuration (`mcp_config.json`):

```json
{
  "mcpServers": {
    "macos-broker": {
      "command": "/Users/ench/projects/macos-agent-sandbox/bin/agent-broker",
      "args": ["mcp"]
    }
  }
}
```

This gives the agent structured tools (`device_list_serial`, `device_list_usb`, `macos_notification`, `macos_clipboard_read`, `macos_clipboard_write`, `macos_system_telemetry`) while ensuring all terminal commands run wrapped inside `agent-sandbox run`.

### Shell Alias Wrapper for Everyday Use
Add to your `~/.zshrc`:

```bash
alias safe="agent-sandbox run"
alias safe-shell="agent-sandbox shell"
```

---

## Directory Structure

* [src/main.rs](file:///Users/ench/projects/macos-agent-sandbox/src/main.rs) - CLI entry point with styled clap interface.
* [src/bin/broker_main.rs](file:///Users/ench/projects/macos-agent-sandbox/src/bin/broker_main.rs) - Standalone `agent-broker` binary CLI.
* [src/ffi.rs](file:///Users/ench/projects/macos-agent-sandbox/src/ffi.rs) - Direct C FFI bindings to Apple Seatbelt (`sandbox_init`).
* [src/process.rs](file:///Users/ench/projects/macos-agent-sandbox/src/process.rs) - Process manager (`fork`, `execve`, TTY `tcsetpgrp`, `rlimit`, session cleanup).
* [src/profile.rs](file:///Users/ench/projects/macos-agent-sandbox/src/profile.rs) - In-memory SBPL profile compiler and template engine.
* [src/env.rs](file:///Users/ench/projects/macos-agent-sandbox/src/env.rs) - Environment variable secret sanitizer.
* [src/broker.rs](file:///Users/ench/projects/macos-agent-sandbox/src/broker.rs) - Audited hardware and macOS capability implementations.
* [src/mcp.rs](file:///Users/ench/projects/macos-agent-sandbox/src/mcp.rs) - Model Context Protocol JSON-RPC stdio engine.
* [src/cli.rs](file:///Users/ench/projects/macos-agent-sandbox/src/cli.rs) - Terminal UI styling, completions, and man page rendering.
* [share/man/man1/](file:///Users/ench/projects/macos-agent-sandbox/share/man/man1) - Unix roff man pages (`agent-sandbox.1`, `agent-broker.1`).
* [completions/](file:///Users/ench/projects/macos-agent-sandbox/completions) - Pre-generated shell completions for zsh, bash, fish.
* [profiles/default.sb.template](file:///Users/ench/projects/macos-agent-sandbox/profiles/default.sb.template) - Master Apple Seatbelt profile template (SBPL).
* [tests/integration_tests.rs](file:///Users/ench/projects/macos-agent-sandbox/tests/integration_tests.rs) - Automated security regression test suite (12 invariants).
