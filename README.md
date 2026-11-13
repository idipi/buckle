# buckle

> **Buckle up your AI agents on macOS.**

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
│          1. Apple Seatbelt Kernel Sandbox              │   │         2. Audited Hardware & OS Capability Broker      │
│     Direct Rust FFI -> libSystem sandbox_init()        │   │             Native CLI & MCP JSON-RPC Stdio             │
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

## The Killer Feature: `buckle up`

One simple, intuitive command to safely run anything:

```bash
# 1. Execute a single isolated command inside the sandbox:
buckle up cargo build
buckle up -- git status
buckle up --devices serial -- probe-rs run firmware.elf

# 2. Drop directly into an isolated interactive subshell (no args needed):
buckle up
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
   - Allocates dedicated ephemeral session storage under `/private/tmp/buckle/session_*` with RAII automatic cleanup on exit.
   - Tracks child process groups (`setpgid`) and reaps orphaned background daemons on exit.

---

## Installation

### Via Homebrew Tap
```bash
brew tap idipi/tap
brew install buckle
```

### Build from Source
```bash
git clone https://github.com/idipi/buckle.git
cd buckle
cargo build --release
```
Compiled binaries will be in `target/release/buckle` and `target/release/buckle-broker`.

---

## Quick Start

### 1. Run System Health Audit
Verify that your macOS version, Seatbelt FFI engine, credential lock, and serial devices are active:

```bash
buckle doctor
```

Output:
```
==================================================
   Buckle (macOS Agent Sandbox) - Health Audit    
==================================================
[+] macOS Version:    15.8.3 (arm64)
[+] Kernel Sandbox:   Direct FFI libSystem.B.dylib (sandbox_init)
[+] Credential Lock:  VERIFIED (~/.ssh blocked in-memory by kernel)
[+] Serial Devices:   3 available in /dev/cu.*
==================================================
Audit status: All primary security controls operational.
```

### 2. Buckle Up!
Execute an isolated command or launch a sandboxed shell:

```bash
# Execute isolated command
buckle up cargo test

# Launch interactive sandboxed shell
buckle up
```

Inside this shell:
* Try `ls ~/.ssh` $\rightarrow$ `Operation not permitted`
* Try `touch ~/.test` $\rightarrow$ `Operation not permitted`
* Try `touch file.txt` (in workspace) $\rightarrow$ `Allowed`

### 3. Run the Automated Security Test Suite
Verify that all 14 kernel isolation invariants pass:

```bash
buckle test
# or: cargo test --test integration_tests
```

---

## Command Reference

### `buckle`

```bash
buckle [COMMAND]

Commands:
  up           Buckle up: execute command in sandbox, or launch interactive shell if empty
  run          Execute an isolated command inside the Apple Seatbelt kernel sandbox
  shell        Launch an interactive sandboxed shell (zsh/bash) with ephemeral session storage
  doctor       Audit system security controls and in-memory kernel sandbox
  test         Execute automated security isolation regression test suite (14 invariants)
  broker       Hardware and macOS capability broker interface
  mcp          Run Model Context Protocol (MCP) stdio server
  version      Print version information (-v, -V, --version)
  completions  Generate shell autocompletion script (bash, zsh, fish, powershell)
  man          Render Unix roff man page documentation to stdout
```

#### `buckle up` Options
* `-w, --workspace <PATH>`: Target workspace directory (default: current directory). Guarded against running directly in `$HOME`.
* `-n, --net <allow|proxy|deny>`: Outbound network policy (default: `allow`).
* `-d, --devices <serial|allow|deny>`: Hardware device access policy (default: `serial`).
* `--writable <PATH>`: Additional directory permitted for write operations.
* `-e, --env <KEY=VAL>`: Set custom environment variables.
* `--pass-env <NAME>`: Explicitly pass through sensitive host environment variables.
* `--allow-home`: Explicitly override safety guard and permit workspace in `$HOME`.

---

### `buckle-broker` (Hardware & macOS Interface)

```bash
# Enumerate serial devices (/dev/cu.*)
buckle-broker serial list

# Inspect connected USB devices
buckle-broker usb list

# Display safe native notification
buckle-broker notify --title "Build Complete" --message "All tests passed."

# Read or write clipboard safely
buckle-broker clip get
buckle-broker clip set "Cleaned output"

# Query system telemetry
buckle-broker status

# Start as Model Context Protocol (MCP) server over stdio
buckle-broker mcp
```

---

## AI Agent Integration Guide

### Integration with Antigravity / Claude / Cursor via MCP
Add `buckle-broker` to your agent's MCP configuration (`mcp_config.json`):

```json
{
  "mcpServers": {
    "buckle-broker": {
      "command": "buckle-broker",
      "args": ["mcp"]
    }
  }
}
```

This gives the agent structured tools (`device_list_serial`, `device_list_usb`, `macos_notification`, `macos_clipboard_read`, `macos_clipboard_write`, `macos_system_telemetry`) while ensuring all terminal commands run wrapped inside `buckle up`.

### Shell Alias Wrapper for Everyday Use
Add to your `~/.zshrc`:

```bash
alias buckle-up="buckle up"
alias safe="buckle up"
```

---

## Directory Structure

* [src/main.rs](src/main.rs) - Primary CLI runner (`buckle` and `buckle up`).
* [src/bin/broker_main.rs](src/bin/broker_main.rs) - Standalone `buckle-broker` binary CLI.
* [src/ffi.rs](src/ffi.rs) - Direct C FFI bindings to Apple Seatbelt (`sandbox_init`).
* [src/process.rs](src/process.rs) - Process manager (`fork`, `execve`, TTY `tcsetpgrp`, `rlimit`, session cleanup).
* [src/profile.rs](src/profile.rs) - In-memory SBPL profile compiler and template engine.
* [src/env.rs](src/env.rs) - Environment variable secret sanitizer.
* [src/broker.rs](src/broker.rs) - Audited hardware and macOS capability implementations.
* [src/mcp.rs](src/mcp.rs) - Model Context Protocol JSON-RPC stdio engine (`buckle-broker`).
* [src/cli.rs](src/cli.rs) - Terminal UI styling, completions, and man page rendering.
* [share/man/man1/](share/man/man1/) - Unix roff man pages (`buckle.1`, `buckle-broker.1`).
* [completions/](completions/) - Pre-generated shell completions for zsh, bash, fish.
* [profiles/default.sb.template](profiles/default.sb.template) - Master Apple Seatbelt profile template (SBPL).
* [tests/integration_tests.rs](tests/integration_tests.rs) - Automated security regression test suite (14 invariants).
