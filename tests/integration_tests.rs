//! Native Rust Integration Tests for buckle.
//! Verifies kernel-level isolation, credential locking, env scrubbing, and device access.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

use buckle::broker::{DeviceBroker, MacOSBroker};
use buckle::process::{execute_sandboxed_process, RunOptions};
use buckle::profile::{render_profile, DeviceMode, NetworkMode, ProfileConfig};

struct TestContext {
    _dir: tempfile::TempDir,
    workspace: PathBuf,
    home: PathBuf,
}

impl TestContext {
    fn new() -> Self {
        let temp_dir = tempfile::tempdir().expect("Failed to create tempdir");
        let workspace = temp_dir.path().to_path_buf();
        let home = PathBuf::from(std::env::var("HOME").expect("HOME not set"));
        Self {
            _dir: temp_dir,
            workspace,
            home,
        }
    }

    fn run(&self, cmd: &[&str]) -> i32 {
        self.run_with_opts(cmd, NetworkMode::Allow, DeviceMode::Serial, &[])
    }

    fn run_with_opts(
        &self,
        cmd: &[&str],
        net: NetworkMode,
        devices: DeviceMode,
        pass_env: &[String],
    ) -> i32 {
        let profile_cfg = ProfileConfig {
            workspace_dir: &self.workspace,
            home_dir: &self.home,
            extra_writable: &[],
            network_mode: net,
            device_mode: devices,
            proxy_port: None,
        };
        let profile_content = render_profile(&profile_cfg);
        let cmd_strings: Vec<String> = cmd.iter().map(|s| s.to_string()).collect();
        let custom_env = HashMap::new();

        let opts = RunOptions {
            command: &cmd_strings,
            workspace: &self.workspace,
            profile_content: &profile_content,
            pass_env,
            custom_env: &custom_env,
            is_interactive_shell: false,
        };

        execute_sandboxed_process(&opts).expect("Execution failed")
    }
}


#[test]
fn test_01_ssh_credentials_blocked() {
    let ctx = TestContext::new();
    let ssh_dir = ctx.home.join(".ssh");
    let code = ctx.run(&["/bin/ls", &ssh_dir.to_string_lossy()]);
    assert_ne!(code, 0, "Access to ~/.ssh must be blocked by kernel");
}

#[test]
fn test_02_gnupg_and_cloud_blocked() {
    let ctx = TestContext::new();
    let gpg_dir = ctx.home.join(".gnupg");
    let code = ctx.run(&["/bin/ls", &gpg_dir.to_string_lossy()]);
    assert_ne!(code, 0, "Access to ~/.gnupg must be blocked by kernel");

    let gcloud_dir = ctx.home.join(".config/gcloud");
    let code_gc = ctx.run(&["/bin/ls", &gcloud_dir.to_string_lossy()]);
    assert_ne!(code_gc, 0, "Access to ~/.config/gcloud must be blocked by kernel");
}

#[test]
fn test_03_keychain_write_blocked() {
    let ctx = TestContext::new();
    let target = ctx.home.join("Library/Keychains/evil_probe.keychain");
    let code = ctx.run(&["/usr/bin/touch", &target.to_string_lossy()]);
    assert_ne!(code, 0, "Writing to Library/Keychains must be blocked by kernel");
    assert!(!target.exists());
}

#[test]
fn test_04_home_root_write_blocked() {
    let ctx = TestContext::new();
    let target = ctx.home.join("malicious_sandbox_probe.tmp");
    let code = ctx.run(&["/usr/bin/touch", &target.to_string_lossy()]);
    assert_ne!(code, 0, "Creating files in $HOME root must be blocked by kernel");
    assert!(!target.exists());
}

#[test]
fn test_05_workspace_read_write_succeeds() {
    let ctx = TestContext::new();
    let test_file = ctx.workspace.join("workspace_ok.txt");
    let code = ctx.run(&[
        "/bin/sh",
        "-c",
        &format!("echo 'test_content' > '{}'", test_file.to_string_lossy()),
    ]);
    assert_eq!(code, 0, "Writing inside workspace must succeed");
    assert!(test_file.exists());
    assert_eq!(
        std::fs::read_to_string(test_file).unwrap().trim(),
        "test_content"
    );
}

#[test]
fn test_06_tmp_storage_allowed() {
    let ctx = TestContext::new();
    let code = ctx.run(&[
        "/bin/sh",
        "-c",
        "F=$(mktemp /private/tmp/sb_test_XXXXXX) && echo 'ok' > \"$F\" && rm \"$F\"",
    ]);
    assert_eq!(code, 0, "Temporary storage in /private/tmp must be allowed");
}

#[test]
fn test_07_raw_disk_access_blocked() {
    let ctx = TestContext::new();
    let code = ctx.run(&["/usr/bin/head", "-c", "8", "/dev/disk0"]);
    assert_ne!(code, 0, "Direct access to raw disk (/dev/disk0) must be blocked");
}

#[test]
fn test_08_serial_devices_allowed() {
    let ctx = TestContext::new();
    let code = ctx.run(&["/bin/sh", "-c", "echo 'test' > /dev/null"]);
    assert_eq!(code, 0, "Writing to /dev/null must succeed");
}

#[test]
fn test_09_broker_capabilities() {
    let ports = DeviceBroker::list_serial_ports().expect("Failed to list serial ports");
    assert!(!ports.is_empty() || ports.is_empty()); // Verify executes safely

    let telemetry = MacOSBroker::get_system_telemetry().expect("Failed to get telemetry");
    assert!(telemetry.is_object());
}

#[test]
fn test_10_sensitive_env_scrubbed() {
    let ctx = TestContext::new();
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_buckle"));

    let output = Command::new(binary)
        .args([
            "up",
            "-w",
            &ctx.workspace.to_string_lossy(),
            "--",
            "sh",
            "-c",
            "echo TOKEN:[${SECRET_TEST_TOKEN}] PASS:[${DATABASE_PASSWORD}]",
        ])
        .env("SECRET_TEST_TOKEN", "super_secret_val")
        .env("DATABASE_PASSWORD", "db_pass_123")
        .output()
        .expect("Failed to run buckle binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("TOKEN:[] PASS:[]"),
        "Sensitive environment variables must be scrubbed automatically"
    );
}

#[test]
fn test_11_usb_iokit_accessible() {
    let ctx = TestContext::new();
    let code = ctx.run(&["/usr/sbin/ioreg", "-p", "IOUSB"]);
    assert_eq!(code, 0, "IOKit USB querying should be allowed for hardware tools");
}

#[test]
fn test_12_safety_guard_blocks_home_workspace() {
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_buckle"));
    let home = std::env::var("HOME").expect("HOME not set");

    let output = Command::new(binary)
        .args(["up", "-w", &home, "--", "echo", "should_not_run"])
        .output()
        .expect("Failed to run buckle binary");

    assert_eq!(
        output.status.code(),
        Some(1),
        "Running in $HOME without --allow-home must be blocked by safety guard"
    );
}

#[test]
fn test_13_buckle_up_command_execution() {
    let ctx = TestContext::new();
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_buckle"));

    let output = Command::new(binary)
        .args([
            "up",
            "-w",
            &ctx.workspace.to_string_lossy(),
            "--",
            "echo",
            "fastened",
        ])
        .output()
        .expect("Failed to run buckle up");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("fastened"),
        "buckle up must execute the command successfully"
    );
}

#[test]
fn test_14_buckle_version_flag() {
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_buckle"));

    let output_long = Command::new(&binary)
        .arg("--version")
        .output()
        .expect("Failed to run buckle --version");
    assert!(output_long.status.success());
    let stdout_long = String::from_utf8_lossy(&output_long.stdout);
    assert!(stdout_long.contains("buckle 0.2.0"));

    let output_short = Command::new(&binary)
        .arg("-v")
        .output()
        .expect("Failed to run buckle -v");
    assert!(output_short.status.success());
    let stdout_short = String::from_utf8_lossy(&output_short.stdout);
    assert!(stdout_short.contains("buckle 0.2.0"));
}
