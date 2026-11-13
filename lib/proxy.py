#!/usr/bin/env python3
"""
Lightweight Egress Filtering HTTP/HTTPS CONNECT Proxy for macOS Agent Sandbox.
Pure Python standard library (no third-party dependencies required).

Enforces strict domain-level whitelisting for agent outbound network traffic.
Blocks data exfiltration attempts to unapproved IP addresses and hosts.
"""

import fnmatch
import logging
import os
import select
import socket
import sys
import threading
from urllib.parse import urlparse

logger = logging.getLogger("agent_proxy")


class DomainFilter:
    """Manages permitted outbound hostnames with wildcard matching support."""

    def __init__(self, allowed_domains=None, rules_file=None):
        self.rules = set()
        if rules_file and os.path.exists(rules_file):
            self.load_from_file(rules_file)
        if allowed_domains:
            for domain in allowed_domains:
                cleaned = domain.strip().lower()
                if cleaned and not cleaned.startswith("#"):
                    self.rules.add(cleaned)

    def load_from_file(self, file_path):
        """Load domain whitelist rules from a text file."""
        with open(file_path, "r", encoding="utf-8") as f:
            for line in f:
                cleaned = line.strip().lower()
                if cleaned and not cleaned.startswith("#"):
                    self.rules.add(cleaned)

    def is_allowed(self, host):
        """
        Check if host matches any allowed rule.
        Supports exact match, wildcard (*.domain.com), and root domain match.
        """
        host = host.lower().split(":")[0].strip()
        for rule in self.rules:
            if rule == host:
                return True
            if rule.startswith("*.") and (host.endswith(rule[1:]) or host == rule[2:]):
                return True
            if fnmatch.fnmatch(host, rule):
                return True
        return False


class EgressProxyServer:
    """Multi-threaded filtering HTTP/HTTPS proxy server."""

    def __init__(self, host="127.0.0.1", port=0, domain_filter=None):
        self.host = host
        self.port = port
        self.domain_filter = domain_filter or DomainFilter()
        self.server_socket = None
        self.is_running = False
        self._thread = None

    def start(self):
        """Bind server socket and begin listening for client connections."""
        self.server_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.server_socket.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        self.server_socket.bind((self.host, self.port))
        self.port = self.server_socket.getsockname()[1]
        self.server_socket.listen(64)
        self.is_running = True

        self._thread = threading.Thread(target=self._serve_loop, daemon=True)
        self._thread.start()
        logger.info(f"Egress filtering proxy active at http://{self.host}:{self.port}")
        return self.port

    def stop(self):
        """Shutdown proxy server cleanly."""
        self.is_running = False
        if self.server_socket:
            try:
                self.server_socket.close()
            except Exception:
                pass
        if self._thread and self._thread.is_alive():
            self._thread.join(timeout=2.0)
        logger.info("Egress proxy stopped")

    def _serve_loop(self):
        while self.is_running:
            try:
                client_sock, client_addr = self.server_socket.accept()
                handler_thread = threading.Thread(
                    target=self._handle_client,
                    args=(client_sock, client_addr),
                    daemon=True,
                )
                handler_thread.start()
            except (socket.error, OSError):
                break

    def _handle_client(self, client_sock, client_addr):
        client_sock.settimeout(15.0)
        try:
            request_data = b""
            while b"\r\n\r\n" not in request_data and len(request_data) < 8192:
                chunk = client_sock.recv(4096)
                if not chunk:
                    break
                request_data += chunk

            if not request_data:
                client_sock.close()
                return

            header_lines = request_data.split(b"\r\n")
            request_line = header_lines[0].decode("utf-8", errors="replace")
            parts = request_line.split()

            if len(parts) < 2:
                client_sock.close()
                return

            method, target = parts[0], parts[1]

            if method.upper() == "CONNECT":
                # HTTPS Tunneling
                self._handle_connect(client_sock, target)
            else:
                # Plain HTTP Proxying
                self._handle_plain_http(client_sock, method, target, request_data)

        except Exception as e:
            logger.debug(f"Client handler error from {client_addr}: {e}")
        finally:
            try:
                client_sock.close()
            except Exception:
                pass

    def _handle_connect(self, client_sock, target):
        """Handle HTTPS CONNECT request with domain allowlist inspection."""
        if ":" in target:
            dest_host, dest_port_str = target.split(":", 1)
            try:
                dest_port = int(dest_port_str)
            except ValueError:
                dest_port = 443
        else:
            dest_host, dest_port = target, 443

        if not self.domain_filter.is_allowed(dest_host):
            logger.warning(f"[EGRESS BLOCKED] CONNECT request to forbidden host: {dest_host}:{dest_port}")
            msg = (
                b"HTTP/1.1 403 Forbidden\r\n"
                b"Content-Type: text/plain; charset=utf-8\r\n"
                b"Connection: close\r\n\r\n"
                b"[BLOCKED by macos-agent-sandbox] Outbound traffic to host '"
                + dest_host.encode("utf-8")
                + b"' is prohibited by policy.\r\n"
            )
            client_sock.sendall(msg)
            return

        # Allowed host: establish upstream TCP tunnel
        try:
            remote_sock = socket.create_connection((dest_host, dest_port), timeout=10.0)
            client_sock.sendall(b"HTTP/1.1 200 Connection Established\r\n\r\n")
            logger.info(f"[EGRESS ALLOWED] CONNECT tunnel established: {dest_host}:{dest_port}")
            self._pipe_sockets(client_sock, remote_sock)
        except Exception as e:
            logger.error(f"Failed to connect to {dest_host}:{dest_port}: {e}")
            client_sock.sendall(b"HTTP/1.1 502 Bad Gateway\r\nConnection: close\r\n\r\n")

    def _handle_plain_http(self, client_sock, method, target, raw_request):
        """Handle standard HTTP request with domain inspection."""
        parsed = urlparse(target)
        dest_host = parsed.hostname or ""
        dest_port = parsed.port or 80

        if not dest_host:
            # Fallback to Host: header
            for line in raw_request.split(b"\r\n"):
                if line.lower().startswith(b"host:"):
                    dest_host = line.split(b":", 1)[1].strip().decode("utf-8", errors="replace")
                    if ":" in dest_host:
                        dest_host, p = dest_host.split(":", 1)
                        dest_port = int(p)
                    break

        if not self.domain_filter.is_allowed(dest_host):
            logger.warning(f"[EGRESS BLOCKED] HTTP {method} request to forbidden host: {dest_host}")
            msg = (
                b"HTTP/1.1 403 Forbidden\r\n"
                b"Content-Type: text/plain; charset=utf-8\r\n"
                b"Connection: close\r\n\r\n"
                b"[BLOCKED by macos-agent-sandbox] Outbound HTTP to '"
                + dest_host.encode("utf-8")
                + b"' is prohibited.\r\n"
            )
            client_sock.sendall(msg)
            return

        try:
            remote_sock = socket.create_connection((dest_host, dest_port), timeout=10.0)
            remote_sock.sendall(raw_request)
            logger.info(f"[EGRESS ALLOWED] HTTP {method} -> {dest_host}:{dest_port}")
            self._pipe_sockets(client_sock, remote_sock)
        except Exception as e:
            logger.error(f"Failed HTTP forward to {dest_host}:{dest_port}: {e}")
            client_sock.sendall(b"HTTP/1.1 502 Bad Gateway\r\nConnection: close\r\n\r\n")

    def _pipe_sockets(self, sock1, sock2):
        """Bidirectional streaming between client and remote sockets."""
        sockets = [sock1, sock2]
        while True:
            try:
                readable, _, exceptional = select.select(sockets, [], sockets, 30.0)
                if exceptional or not readable:
                    break
                for s in readable:
                    other = sock2 if s is sock1 else sock1
                    data = s.recv(16384)
                    if not data:
                        return
                    other.sendall(data)
            except (socket.error, OSError):
                break
        try:
            sock1.close()
        except Exception:
            pass
        try:
            sock2.close()
        except Exception:
            pass


if __name__ == "__main__":
    import argparse

    logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(levelname)s] %(message)s")
    parser = argparse.ArgumentParser(description="Standalone Agent Egress Filtering Proxy")
    parser.add_argument("--port", type=int, default=18080, help="Listening port (default: 18080)")
    parser.add_argument("--host", default="127.0.0.1", help="Listening host (default: 127.0.0.1)")
    parser.add_argument("--rules", default=None, help="Path to domain whitelist rules file")
    args = parser.parse_args()

    domain_filter = DomainFilter(rules_file=args.rules)
    proxy = EgressProxyServer(host=args.host, port=args.port, domain_filter=domain_filter)
    assigned_port = proxy.start()

    print(f"Proxy listening on {args.host}:{assigned_port}")
    print(f"Configured with {len(domain_filter.rules)} allowed domain rules.")
    print("Press Ctrl+C to terminate.")

    try:
        while True:
            threading.Event().wait(1.0)
    except KeyboardInterrupt:
        print("\nShutting down proxy...")
        proxy.stop()
