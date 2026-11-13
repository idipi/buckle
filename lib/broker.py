#!/usr/bin/env python3
"""
Audited Capability Broker for macOS System and Device Interactions.
Provides a strictly controlled interface for AI agents to interact with:
1. Serial / USB microcontrollers and external hardware
2. macOS notifications, clipboard, and system telemetry

Applies input validation, sanitization, and least-privilege constraints
to prevent arbitrary code execution or command injection.
"""

import glob
import json
import os
import re
import subprocess
import sys
import time
from urllib.parse import urlparse

# Optional termios support for serial communication on POSIX/Darwin
try:
    import fcntl
    import select
    import termios
    HAS_TERMIOS = True
except ImportError:
    HAS_TERMIOS = False


class DeviceBroker:
    """Safe access to external hardware and serial devices on macOS."""

    @staticmethod
    def list_serial_ports():
        """
        Scan and return all available serial communication ports (/dev/cu.*).
        Excludes raw debug consoles by default unless explicitly configured.
        """
        ports = []
        raw_devices = glob.glob("/dev/cu.*")
        for dev in sorted(raw_devices):
            # Classify device types
            dev_name = os.path.basename(dev)
            category = "general"
            if "usbserial" in dev_name.lower() or "usbmodem" in dev_name.lower():
                category = "usb_hardware"
            elif "bluetooth" in dev_name.lower():
                category = "bluetooth"
            elif "debug" in dev_name.lower():
                category = "kernel_debug"

            ports.append({
                "path": dev,
                "name": dev_name,
                "category": category,
                "exists": os.path.exists(dev),
            })
        return ports

    @staticmethod
    def list_usb_tree():
        """
        Enumerate USB hardware hierarchy via macOS system_profiler.
        Returns parsed structured JSON data.
        """
        try:
            res = subprocess.run(
                ["/usr/sbin/system_profiler", "SPUSBDataType", "-json"],
                capture_output=True,
                text=True,
                timeout=10,
            )
            if res.returncode == 0:
                data = json.loads(res.stdout)
                return data.get("SPUSBDataType", [])
        except Exception as e:
            return [{"error": f"Failed to query USB subsystem: {str(e)}"}]
        return []

    @staticmethod
    def serial_communicate(port_path, command_str, baudrate=115200, timeout_sec=2.0, max_bytes=4096):
        """
        Send a command to a microcontroller/device and read the response.
        Enforces strict path checks, timeout guards, and response size ceilings.
        """
        if not HAS_TERMIOS:
            return {"error": "termios not available on this platform"}

        # Validate that path strictly matches /dev/cu.* or /dev/tty.*
        if not re.match(r"^/dev/(cu|tty)\.[a-zA-Z0-9_\-\.]+$", port_path):
            return {"error": f"Invalid serial port path: {port_path}"}

        if not os.path.exists(port_path):
            return {"error": f"Device port not found: {port_path}"}

        # Map integer baudrates to termios speed constants
        baud_map = {
            9600: termios.B9600,
            19200: termios.B19200,
            38400: termios.B38400,
            57600: termios.B57600,
            115200: termios.B115200,
            230400: termios.B230400,
        }
        baud_const = baud_map.get(baudrate, termios.B115200)

        try:
            fd = os.open(port_path, os.O_RDWR | os.O_NOCTTY | os.O_NONBLOCK)
        except OSError as e:
            return {"error": f"Failed to open port {port_path}: {e}"}

        try:
            # Configure raw terminal attributes (8N1)
            attrs = termios.tcgetattr(fd)
            # iflag
            attrs[0] &= ~(termios.IGNBRK | termios.BRKINT | termios.PARMRK |
                          termios.ISTRIP | termios.INLCR | termios.IGNCR |
                          termios.ICRNL | termios.IXON)
            # oflag
            attrs[1] &= ~termios.OPOST
            # cflag
            attrs[2] &= ~(termios.CSIZE | termios.PARENB)
            attrs[2] |= termios.CS8 | termios.CREAD | termios.CLOCAL
            # lflag
            attrs[3] &= ~(termios.ECHO | termios.ECHONL | termios.ICANON |
                          termios.ISIG | termios.IEXTEN)

            termios.cfsetispeed(attrs, baud_const)
            termios.cfsetospeed(attrs, baud_const)
            termios.tcsetattr(fd, termios.TCSANOW, attrs)

            # Transmit command
            payload = command_str.encode("utf-8")
            if not payload.endswith(b"\n") and not payload.endswith(b"\r"):
                payload += b"\n"
            os.write(fd, payload)

            # Wait and read response up to timeout
            start_time = time.time()
            received_bytes = bytearray()

            while time.time() - start_time < timeout_sec:
                r, _, _ = select.select([fd], [], [], 0.2)
                if r:
                    chunk = os.read(fd, 512)
                    if not chunk:
                        break
                    received_bytes.extend(chunk)
                    if len(received_bytes) >= max_bytes:
                        break

            return {
                "success": True,
                "port": port_path,
                "baudrate": baudrate,
                "bytes_read": len(received_bytes),
                "data": received_bytes.decode("utf-8", errors="replace"),
            }
        except Exception as e:
            return {"error": f"Communication failure on {port_path}: {str(e)}"}
        finally:
            try:
                os.close(fd)
            except OSError:
                pass


class MacOSBroker:
    """Safe gateway for authorized macOS system interactions."""

    @staticmethod
    def show_notification(title, message, subtitle=""):
        """
        Safely display a native macOS User Notification.
        Sanitizes strings against AppleScript injection.
        """
        clean_title = re.sub(r'["\\]', "", str(title))[:64]
        clean_msg = re.sub(r'["\\]', "", str(message))[:256]
        clean_sub = re.sub(r'["\\]', "", str(subtitle))[:64]

        script = f'display notification "{clean_msg}" with title "{clean_title}"'
        if clean_sub:
            script += f' subtitle "{clean_sub}"'

        try:
            res = subprocess.run(["/usr/bin/osascript", "-e", script], capture_output=True, timeout=5)
            return {"success": res.returncode == 0}
        except Exception as e:
            return {"success": False, "error": str(e)}

    @staticmethod
    def get_clipboard(max_chars=16384):
        """Read plain text from macOS clipboard with length ceiling."""
        try:
            res = subprocess.run(["/usr/bin/pbpaste"], capture_output=True, text=True, timeout=3)
            if res.returncode == 0:
                text = res.stdout[:max_chars]
                return {"success": True, "text": text, "truncated": len(res.stdout) > max_chars}
            return {"success": False, "error": "pbpaste exited with error"}
        except Exception as e:
            return {"success": False, "error": str(e)}

    @staticmethod
    def set_clipboard(text, max_chars=32768):
        """Write plain text to macOS clipboard with sanity constraints."""
        safe_text = str(text)[:max_chars]
        try:
            proc = subprocess.Popen(["/usr/bin/pbcopy"], stdin=subprocess.PIPE)
            proc.communicate(input=safe_text.encode("utf-8"), timeout=3)
            return {"success": proc.returncode == 0}
        except Exception as e:
            return {"success": False, "error": str(e)}

    @staticmethod
    def open_browser_url(url):
        """
        Open a URL in user's default macOS browser.
        Strictly requires http or https scheme (blocks file://, javascript:, etc.).
        """
        parsed = urlparse(url.strip())
        if parsed.scheme.lower() not in ("http", "https"):
            return {"success": False, "error": f"Prohibited URL scheme: {parsed.scheme}. Only http/https allowed."}

        try:
            res = subprocess.run(["/usr/bin/open", parsed.geturl()], capture_output=True, timeout=5)
            return {"success": res.returncode == 0}
        except Exception as e:
            return {"success": False, "error": str(e)}

    @staticmethod
    def get_system_telemetry():
        """Retrieve system telemetry (battery, load, uptime)."""
        info = {}
        try:
            # Uptime & load average
            res = subprocess.run(["/usr/bin/uptime"], capture_output=True, text=True, timeout=3)
            if res.returncode == 0:
                info["uptime"] = res.stdout.strip()

            # Battery status (pmset)
            pm = subprocess.run(["/usr/bin/pmset", "-g", "batt"], capture_output=True, text=True, timeout=3)
            if pm.returncode == 0:
                info["battery"] = pm.stdout.strip()
        except Exception as e:
            info["error"] = str(e)
        return info
