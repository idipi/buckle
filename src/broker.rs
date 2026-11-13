//! Audited Capability Gateway for hardware devices and macOS system integrations.
//! Provides safe APIs for serial microcontrollers, USB topology, notifications, and clipboard.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Debug, Serialize, Deserialize)]
pub struct SerialPortInfo {
    pub path: String,
    pub name: String,
    pub category: String,
    pub exists: bool,
}

pub struct DeviceBroker;

impl DeviceBroker {
    /// Enumerate active serial communication ports (/dev/cu.*)
    pub fn list_serial_ports() -> Result<Vec<SerialPortInfo>> {
        let mut ports = Vec::new();
        let dev_dir = Path::new("/dev");

        if dev_dir.is_dir() {
            for entry in fs::read_dir(dev_dir)? {
                let entry = entry?;
                let file_name = entry.file_name().to_string_lossy().to_string();

                if file_name.starts_with("cu.") {
                    let path_str = format!("/dev/{}", file_name);
                    let category = if file_name.to_lowercase().contains("usbserial")
                        || file_name.to_lowercase().contains("usbmodem")
                    {
                        "usb_hardware".to_string()
                    } else if file_name.to_lowercase().contains("bluetooth") {
                        "bluetooth".to_string()
                    } else if file_name.to_lowercase().contains("debug") {
                        "kernel_debug".to_string()
                    } else {
                        "general".to_string()
                    };

                    ports.push(SerialPortInfo {
                        path: path_str,
                        name: file_name,
                        category,
                        exists: true,
                    });
                }
            }
        }

        ports.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(ports)
    }

    /// Query connected USB hardware hierarchy via macOS system_profiler
    pub fn list_usb_tree() -> Result<serde_json::Value> {
        let output = Command::new("/usr/sbin/system_profiler")
            .args(["SPUSBDataType", "-json"])
            .output()
            .context("Failed to run system_profiler")?;

        if !output.status.success() {
            bail!("system_profiler failed with status: {}", output.status);
        }

        let parsed: serde_json::Value = serde_json::from_slice(&output.stdout)
            .context("Failed to parse system_profiler JSON output")?;

        Ok(parsed.get("SPUSBDataType").cloned().unwrap_or(serde_json::Value::Array(vec![])))
    }
}

pub struct MacOSBroker;

impl MacOSBroker {
    /// Post a safe native macOS user notification dialog
    pub fn show_notification(title: &str, message: &str, subtitle: &str) -> Result<bool> {
        let clean_title: String = title.chars().filter(|&c| c != '"' && c != '\\').take(64).collect();
        let clean_msg: String = message.chars().filter(|&c| c != '"' && c != '\\').take(256).collect();
        let clean_sub: String = subtitle.chars().filter(|&c| c != '"' && c != '\\').take(64).collect();

        let mut script = format!("display notification \"{}\" with title \"{}\"", clean_msg, clean_title);
        if !clean_sub.is_empty() {
            script.push_str(&format!(" subtitle \"{}\"", clean_sub));
        }

        let status = Command::new("/usr/bin/osascript")
            .args(["-e", &script])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .context("Failed to invoke osascript")?;

        Ok(status.success())
    }

    /// Read plain text from macOS system clipboard with length cap
    pub fn get_clipboard(max_chars: usize) -> Result<String> {
        let output = Command::new("/usr/bin/pbpaste")
            .output()
            .context("Failed to invoke pbpaste")?;

        let text = String::from_utf8_lossy(&output.stdout);
        Ok(text.chars().take(max_chars).collect())
    }

    /// Write plain text to macOS system clipboard with size constraints
    pub fn set_clipboard(text: &str, max_chars: usize) -> Result<bool> {
        let safe_text: String = text.chars().take(max_chars).collect();

        let mut child = Command::new("/usr/bin/pbcopy")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Failed to invoke pbcopy")?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(safe_text.as_bytes())?;
        }

        let status = child.wait()?;
        Ok(status.success())
    }

    /// Query current system telemetry (battery and uptime)
    pub fn get_system_telemetry() -> Result<serde_json::Value> {
        let mut telemetry = serde_json::Map::new();

        if let Ok(uptime_out) = Command::new("/usr/bin/uptime").output() {
            let s = String::from_utf8_lossy(&uptime_out.stdout).trim().to_string();
            telemetry.insert("uptime".to_string(), serde_json::Value::String(s));
        }

        if let Ok(batt_out) = Command::new("/usr/bin/pmset").args(["-g", "batt"]).output() {
            let s = String::from_utf8_lossy(&batt_out.stdout).trim().to_string();
            telemetry.insert("battery".to_string(), serde_json::Value::String(s));
        }

        Ok(serde_json::Value::Object(telemetry))
    }
}
