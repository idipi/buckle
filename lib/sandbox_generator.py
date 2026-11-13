#!/usr/bin/env python3
"""
Dynamic Apple Seatbelt Sandbox Profile Generator and Process Execution Engine.
Constructs hardened macOS SBPL profiles tailored to current workspace and policies.
"""

import hashlib
import os
from pathlib import Path
import resource
import shutil
import signal
import subprocess
import tempfile
import time
import uuid

BASE_DIR = Path(__file__).resolve().parent.parent
TEMPLATE_PATH = BASE_DIR / "profiles" / "default.sb.template"

SENSITIVE_ENV_PATTERNS = [
    "TOKEN", "SECRET", "PASSWORD", "PASSWD",
    "AUTH", "CREDENTIAL", "PRIVATE", "API_KEY"
]

SAFE_EXCEPTIONS = {
    "TERM", "COLORTERM", "LANG", "LC_ALL", "LC_CTYPE",
    "AGENT_SANDBOX_ACTIVE", "AGENT_SANDBOX_WORKSPACE",
    "SSH_AUTH_SOCK", "EDITOR", "PAGER", "PATH", "HOME",
    "USER", "LOGNAME", "SHELL", "PWD", "SHLVL"
}


def sanitize_environment(base_env, pass_vars=None, set_vars=None):
    """
    Sanitize host environment variables before passing into the sandbox.
    Strips sensitive secrets, tokens, and credentials by default.
    """
    clean = {}
    pass_set = set(pass_vars or [])

    for k, v in base_env.items():
        k_upper = k.upper()
        if k in pass_set or k in SAFE_EXCEPTIONS:
            clean[k] = v
            continue
        # Strip keys matching sensitive credential patterns
        if any(pat in k_upper for pat in SENSITIVE_ENV_PATTERNS):
            continue
        clean[k] = v

    if set_vars:
        clean.update(set_vars)
    return clean


class SandboxEngine:
    """Manages profile compilation, environment preparation, and sandboxed execution."""

    def __init__(self, workspace_dir=None, network_mode="allow", device_mode="serial"):
        self.workspace_dir = Path(workspace_dir or os.getcwd()).resolve()
        self.home_dir = Path.home().resolve()
        self.network_mode = network_mode.lower()  # 'allow', 'proxy', or 'deny'
        self.device_mode = device_mode.lower()    # 'serial', 'allow', or 'deny'
        self.extra_writable = []

    def add_writable_path(self, path):
        """Add an additional directory permitted for write operations."""
        resolved = Path(path).resolve()
        if resolved not in self.extra_writable:
            self.extra_writable.append(resolved)

    def _generate_toolchain_rules(self):
        """
        Detect read-only developer toolchain directories in user home.
        Allows compilers and runtimes (Cargo, NVM, Pyenv, Go) to be read without write access.
        """
        toolchain_dirs = [
            self.home_dir / ".cargo" / "bin",
            self.home_dir / ".local" / "bin",
            self.home_dir / ".rustup",
            self.home_dir / ".nvm",
            self.home_dir / ".pyenv",
            self.home_dir / "go" / "bin",
            self.home_dir / ".bun" / "bin",
            self.home_dir / ".deno" / "bin",
        ]
        rules = []
        for p in toolchain_dirs:
            if p.exists():
                rules.append(f'    (subpath "{p}")')
        if not rules:
            return ""
        return "(allow file-read*\n" + "\n".join(rules) + ")"

    def _generate_device_rules(self):
        """Generate device access rules based on device_mode."""
        if self.device_mode == "deny":
            return ";; Device hardware access explicitly denied"

        rules = []
        if self.device_mode in ("serial", "allow"):
            # Microcontroller serial links and debugging interfaces
            rules.append('    (subpath "/dev/cu.")')
            rules.append('    (subpath "/dev/tty.")')

        if self.device_mode == "allow":
            # Broad device support for audio, IOKit peripherals
            rules.append('    (subpath "/dev/bpf")')
            rules.append('    (subpath "/dev/audioservices")')

        return "(allow file-read* file-write*\n" + "\n".join(rules) + ")"

    def _generate_network_rules(self, proxy_port=None):
        """Generate network access rules based on network_mode."""
        if self.network_mode == "deny":
            return "(deny network*)\n(deny system-socket)"

        if self.network_mode == "allow":
            return (
                ";; Unrestricted outbound network access\n"
                "(allow network-outbound)\n"
                '(allow network-inbound (local ip "localhost:*"))\n'
                "(allow system-socket)"
            )

        # network_mode == 'proxy'
        # Crucial security invariant: Direct external egress is denied by kernel; only loopback is permitted.
        # Loopback inbound, bind, and outbound are allowed so local dev servers, tests, and tool IPC (e.g. agy language server) function cleanly.
        return (
            ";; Egress restricted strictly to local proxy and loopback IPC on localhost\n"
            '(allow network-bind (local ip "localhost:*"))\n'
            '(allow network-inbound (local ip "localhost:*"))\n'
            '(allow network-outbound (remote ip "localhost:*"))\n'
            "(allow system-socket)"
        )

    def render_profile(self, proxy_port=None):
        """Render the complete Scheme SBPL profile string."""
        with open(TEMPLATE_PATH, "r", encoding="utf-8") as f:
            template = f.read()

        extra_paths_str = ""
        if self.extra_writable:
            extra_paths_str = "\n" + "\n".join(f'    (subpath "{p}")' for p in self.extra_writable)

        rendered = template.replace("{{WORKSPACE_DIR}}", str(self.workspace_dir))
        rendered = rendered.replace("{{HOME_DIR}}", str(self.home_dir))
        rendered = rendered.replace("{{EXTRA_WRITABLE_PATHS}}", extra_paths_str)
        rendered = rendered.replace("{{USER_TOOLCHAIN_READ_RULES}}", self._generate_toolchain_rules())
        rendered = rendered.replace("{{DEVICE_RULES}}", self._generate_device_rules())
        rendered = rendered.replace("{{NETWORK_RULES}}", self._generate_network_rules(proxy_port))

        return rendered

    def compile_profile_to_file(self, proxy_port=None):
        """Write compiled profile to a secure temporary file and return path."""
        rendered = self.render_profile(proxy_port=proxy_port)
        hash_digest = hashlib.sha256(rendered.encode("utf-8")).hexdigest()[:12]
        temp_dir = Path("/private/tmp/agent_sandbox")
        temp_dir.mkdir(parents=True, exist_ok=True)
        profile_path = temp_dir / f"profile_{hash_digest}.sb"
        with open(profile_path, "w", encoding="utf-8") as f:
            f.write(rendered)
        return profile_path


def run_sandboxed_command(
    command_args,
    workspace_dir=None,
    network_mode="allow",
    device_mode="serial",
    extra_writable=None,
    proxy_server=None,
    env_overrides=None,
    pass_env=None,
):
    """
    Execute a command under the generated macOS Seatbelt profile.
    Automatically manages proxy configuration, environment sanitization,
    ephemeral tmpdir lifecycle, and process group orphan reaping.
    """
    engine = SandboxEngine(
        workspace_dir=workspace_dir,
        network_mode=network_mode,
        device_mode=device_mode,
    )
    if extra_writable:
        for p in extra_writable:
            engine.add_writable_path(p)

    proxy_port = None
    if network_mode == "proxy" and proxy_server:
        proxy_port = proxy_server.port

    profile_path = engine.compile_profile_to_file(proxy_port=proxy_port)

    # 1. Ephemeral Sandbox Session Directory
    session_id = f"session_{os.getpid()}_{uuid.uuid4().hex[:6]}"
    session_tmp = Path(f"/private/tmp/agent_sandbox/{session_id}")
    session_tmp.mkdir(parents=True, exist_ok=True)

    # 2. Environment Sanitization & Preparation
    env = sanitize_environment(os.environ, pass_vars=pass_env, set_vars=env_overrides)
    env["AGENT_SANDBOX_ACTIVE"] = "1"
    env["AGENT_SANDBOX_WORKSPACE"] = str(engine.workspace_dir)
    env["TMPDIR"] = str(session_tmp)
    env["ZDOTDIR"] = str(session_tmp)
    env["HISTFILE"] = str(session_tmp / ".zsh_history")
    env["PROMPT"] = "[sandbox] %~ %# "

    if network_mode == "proxy" and proxy_port:
        proxy_url = f"http://127.0.0.1:{proxy_port}"
        env["HTTP_PROXY"] = proxy_url
        env["HTTPS_PROXY"] = proxy_url
        env["http_proxy"] = proxy_url
        env["https_proxy"] = proxy_url
        env["ALL_PROXY"] = proxy_url
        env["NO_PROXY"] = "localhost,127.0.0.1"
        env["no_proxy"] = "localhost,127.0.0.1"

    cmd = ["/usr/bin/sandbox-exec", "-f", str(profile_path)] + command_args

    def _preexec_limits():
        try:
            # Bump file descriptors to 10240 for large C++/Rust/Go builds
            soft_f, hard_f = resource.getrlimit(resource.RLIMIT_NOFILE)
            target_f = min(10240, hard_f) if hard_f > 0 else 10240
            resource.setrlimit(resource.RLIMIT_NOFILE, (target_f, hard_f))
        except (ValueError, OSError):
            pass

    returncode = 0
    try:
        is_interactive = len(command_args) > 0 and command_args[0].endswith("sh") and "-l" in command_args
        if is_interactive:
            # Interactive shell needs direct controlling terminal
            proc = subprocess.run(cmd, env=env, cwd=str(engine.workspace_dir), preexec_fn=_preexec_limits)
            returncode = proc.returncode
        else:
            # Batch command runs in dedicated process group to prevent orphan background daemons
            proc = subprocess.Popen(
                cmd,
                env=env,
                cwd=str(engine.workspace_dir),
                preexec_fn=lambda: (_preexec_limits(), os.setpgrp()),
            )
            try:
                returncode = proc.wait()
            except KeyboardInterrupt:
                try:
                    os.killpg(proc.pid, signal.SIGINT)
                    returncode = proc.wait(timeout=2.0)
                except (subprocess.TimeoutExpired, OSError):
                    try:
                        os.killpg(proc.pid, signal.SIGKILL)
                    except OSError:
                        pass
                    returncode = 130
            finally:
                # Ensure no orphaned child processes survive
                try:
                    os.killpg(proc.pid, signal.SIGTERM)
                except (ProcessLookupError, OSError):
                    pass

        return returncode
    finally:
        # 3. Clean up profile and ephemeral session storage
        try:
            if profile_path.exists():
                profile_path.unlink()
        except OSError:
            pass
        try:
            shutil.rmtree(session_tmp, ignore_errors=True)
        except OSError:
            pass
