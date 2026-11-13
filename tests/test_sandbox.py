#!/usr/bin/env python3
"""
Automated Test Suite for macOS Agent Sandbox.
Verifies kernel-level isolation, credential locking, egress filtering, and device policies.
"""

import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT_DIR))

from lib.broker import DeviceBroker, MacOSBroker
from lib.proxy import DomainFilter, EgressProxyServer
from lib.sandbox_generator import SandboxEngine, run_sandboxed_command


class TestSandboxIsolation(unittest.TestCase):
    """Rigorous verification of sandbox security boundaries."""

    def setUp(self):
        self.test_workspace = Path(tempfile.mkdtemp(prefix="agent_sandbox_test_"))
        self.home_dir = Path.home()

    def tearDown(self):
        shutil.rmtree(self.test_workspace, ignore_errors=True)

    def _run_in_sandbox(self, cmd_list, net="deny", devices="serial", extra_writable=None):
        """Helper to execute a command inside a test sandbox."""
        engine = SandboxEngine(
            workspace_dir=self.test_workspace,
            network_mode=net,
            device_mode=devices,
        )
        if extra_writable:
            for p in extra_writable:
                engine.add_writable_path(p)

        profile_path = engine.compile_profile_to_file()
        try:
            full_cmd = ["/usr/bin/sandbox-exec", "-f", str(profile_path)] + cmd_list
            res = subprocess.run(
                full_cmd,
                capture_output=True,
                text=True,
                cwd=str(self.test_workspace),
            )
            return res
        finally:
            if profile_path.exists():
                profile_path.unlink()

    def test_01_ssh_credentials_blocked(self):
        """Verify that ~/.ssh directory cannot be read or listed."""
        ssh_dir = self.home_dir / ".ssh"
        res = self._run_in_sandbox(["/bin/ls", str(ssh_dir)])
        self.assertNotEqual(res.returncode, 0, "Reading ~/.ssh should fail")
        self.assertIn("Operation not permitted", res.stderr, "Error must be kernel EPERM")

    def test_02_gnupg_and_cloud_blocked(self):
        """Verify cloud credentials and GPG keys are blocked."""
        gcloud_dir = self.home_dir / ".config" / "gcloud"
        res_gcloud = self._run_in_sandbox(["/bin/ls", str(gcloud_dir)])
        self.assertNotEqual(res_gcloud.returncode, 0, "Access to ~/.config/gcloud should be blocked")

        gnupg_dir = self.home_dir / ".gnupg"
        res_gpg = self._run_in_sandbox(["/bin/ls", str(gnupg_dir)])
        self.assertNotEqual(res_gpg.returncode, 0, "Access to ~/.gnupg should be blocked")

    def test_03_keychain_write_blocked(self):
        """Verify macOS Keychains directory cannot be written to or modified."""
        target_file = self.home_dir / "Library" / "Keychains" / "evil_probe.keychain"
        res = self._run_in_sandbox(["/usr/bin/touch", str(target_file)])
        self.assertNotEqual(res.returncode, 0, "Writing to Library/Keychains must be blocked")
        self.assertFalse(target_file.exists())

    def test_04_home_root_write_blocked(self):
        """Verify that agent cannot drop files directly into user home directory."""
        target_file = self.home_dir / "malicious_sandbox_probe.tmp"
        res = self._run_in_sandbox(["/usr/bin/touch", str(target_file)])
        self.assertNotEqual(res.returncode, 0, "Dropping files into $HOME root must be forbidden")
        self.assertFalse(target_file.exists(), "Target file must not be created")

    def test_05_workspace_read_write_succeeds(self):
        """Verify that agent has full read/write access to its assigned workspace."""
        probe_file = self.test_workspace / "workspace_test.txt"
        res = self._run_in_sandbox([
            "/bin/sh", "-c",
            f'echo "sandbox_ok" > "{probe_file}" && cat "{probe_file}"'
        ])
        self.assertEqual(res.returncode, 0, f"Writing in workspace failed: {res.stderr}")
        self.assertTrue(probe_file.exists())
        self.assertEqual(probe_file.read_text().strip(), "sandbox_ok")

    def test_06_tmp_storage_allowed(self):
        """Verify standard /tmp is writable for compiler/runtime temporary files."""
        res = self._run_in_sandbox([
            "/bin/sh", "-c",
            'F=$(mktemp /tmp/test_sb_XXXXXX) && echo "tmp_ok" > "$F" && rm "$F"'
        ])
        self.assertEqual(res.returncode, 0, f"Writing to /tmp failed: {res.stderr}")

    def test_07_raw_disk_access_blocked(self):
        """Verify that direct read on raw disk partitions (/dev/disk0) is denied."""
        res = self._run_in_sandbox(["/usr/bin/head", "-c", "8", "/dev/disk0"])
        self.assertNotEqual(res.returncode, 0, "Reading raw disk partitions must be denied")
        self.assertIn("Operation not permitted", res.stderr)

    def test_08_serial_devices_allowed(self):
        """Verify that serial/null devices in /dev are accessible."""
        res = self._run_in_sandbox(["/bin/sh", "-c", "echo 'test' > /dev/null"])
        self.assertEqual(res.returncode, 0, "Writing to /dev/null should succeed")

    def test_09_direct_egress_blocked_in_proxy_mode(self):
        """Verify that in proxy-only mode, direct external IP connection is blocked by kernel."""
        res = self._run_in_sandbox(["/usr/bin/curl", "-s", "--connect-timeout", "1", "http://8.8.8.8"], net="proxy")
        self.assertNotEqual(res.returncode, 0, "Direct egress must be blocked in proxy mode")

    def test_10_broker_capabilities(self):
        """Verify DeviceBroker and MacOSBroker API methods execute safely."""
        ports = DeviceBroker.list_serial_ports()
        self.assertIsInstance(ports, list)

        telemetry = MacOSBroker.get_system_telemetry()
        self.assertIsInstance(telemetry, dict)
        self.assertIn("uptime", telemetry)

    def test_11_sensitive_env_scrubbed(self):
        """Verify sensitive credentials in host environment variables are scrubbed."""
        test_env = os.environ.copy()
        test_env["TEST_SUPER_TOKEN"] = "leaked_secret_val"
        test_env["AWS_SECRET_KEY"] = "super_aws_secret"

        res = subprocess.run(
            ["agent-sandbox", "run", "-w", str(self.test_workspace), "--", "sh", "-c", "echo TOKEN:[${TEST_SUPER_TOKEN}] AWS:[${AWS_SECRET_KEY}]"],
            capture_output=True,
            text=True,
            env=test_env,
        )
        self.assertEqual(res.returncode, 0)
        self.assertIn("TOKEN:[] AWS:[]", res.stdout, "Sensitive environment variables must be stripped")

    def test_12_usb_iokit_accessible(self):
        """Verify that IOKit USB subsystem can be queried without kernel denial."""
        res = self._run_in_sandbox(["/usr/sbin/ioreg", "-p", "IOUSB"])
        self.assertEqual(res.returncode, 0, "IOKit USB querying should be permitted")


if __name__ == "__main__":
    suite = unittest.TestLoader().loadTestsFromTestCase(TestSandboxIsolation)
    runner = unittest.TextTestRunner(verbosity=2)
    result = runner.run(suite)
    sys.exit(0 if result.wasSuccessful() else 1)
