//! Dynamic Apple Seatbelt (SBPL) profile compiler.
//! Generates strict kernel confinement profiles in-memory.

use std::path::{Path, PathBuf};

const MASTER_TEMPLATE: &str = include_str!("../profiles/default.sb.template");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NetworkMode {
    #[default]
    Allow,
    Proxy,
    Deny,
}

impl std::str::FromStr for NetworkMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "allow" => Ok(NetworkMode::Allow),
            "proxy" => Ok(NetworkMode::Proxy),
            "deny" => Ok(NetworkMode::Deny),
            _ => Err(format!("Invalid network mode '{}'. Use: allow, proxy, deny", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DeviceMode {
    #[default]
    Serial,
    Allow,
    Deny,
}

impl std::str::FromStr for DeviceMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "serial" => Ok(DeviceMode::Serial),
            "allow" => Ok(DeviceMode::Allow),
            "deny" => Ok(DeviceMode::Deny),
            _ => Err(format!("Invalid device mode '{}'. Use: serial, allow, deny", s)),
        }
    }
}

pub struct ProfileConfig<'a> {
    pub workspace_dir: &'a Path,
    pub home_dir: &'a Path,
    pub extra_writable: &'a [PathBuf],
    pub network_mode: NetworkMode,
    pub device_mode: DeviceMode,
    pub proxy_port: Option<u16>,
}

pub fn render_profile(config: &ProfileConfig) -> String {
    let mut rendered = MASTER_TEMPLATE.to_string();

    let workspace_str = config.workspace_dir.to_string_lossy();
    let home_str = config.home_dir.to_string_lossy();

    let mut extra_writable_str = String::new();
    for p in config.extra_writable {
        extra_writable_str.push_str(&format!("\n    (subpath \"{}\")", p.to_string_lossy()));
    }

    // Render toolchain rules
    let toolchain_rules = generate_toolchain_rules(config.home_dir);

    // Render device rules
    let device_rules = generate_device_rules(config.device_mode);

    // Render network rules
    let network_rules = generate_network_rules(config.network_mode, config.proxy_port);

    rendered = rendered.replace("{{WORKSPACE_DIR}}", &workspace_str);
    rendered = rendered.replace("{{HOME_DIR}}", &home_str);
    rendered = rendered.replace("{{EXTRA_WRITABLE_PATHS}}", &extra_writable_str);
    rendered = rendered.replace("{{USER_TOOLCHAIN_READ_RULES}}", &toolchain_rules);
    rendered = rendered.replace("{{DEVICE_RULES}}", &device_rules);
    rendered = rendered.replace("{{NETWORK_RULES}}", &network_rules);

    rendered
}

fn generate_toolchain_rules(home_dir: &Path) -> String {
    let candidates = [
        home_dir.join(".cargo/bin"),
        home_dir.join(".local/bin"),
        home_dir.join(".rustup"),
        home_dir.join(".nvm"),
        home_dir.join(".pyenv"),
        home_dir.join("go/bin"),
        home_dir.join(".bun/bin"),
        home_dir.join(".deno/bin"),
    ];

    let mut allowed = Vec::new();
    for p in &candidates {
        if p.exists() {
            allowed.push(format!("    (subpath \"{}\")", p.to_string_lossy()));
        }
    }

    if allowed.is_empty() {
        String::new()
    } else {
        format!("(allow file-read*\n{}\n)", allowed.join("\n"))
    }
}

fn generate_device_rules(mode: DeviceMode) -> String {
    match mode {
        DeviceMode::Deny => ";; Hardware device access explicitly denied".to_string(),
        DeviceMode::Serial => {
            "(allow file-read* file-write*\n    (regex #\"^/dev/cu\\.\")\n    (regex #\"^/dev/tty\\.\"))".to_string()
        }
        DeviceMode::Allow => {
            "(allow file-read* file-write*\n    (regex #\"^/dev/cu\\.\")\n    (regex #\"^/dev/tty\\.\")\n    (regex #\"^/dev/bpf[0-9]*\")\n    (subpath \"/dev/audioservices\"))".to_string()
        }
    }
}

fn generate_network_rules(mode: NetworkMode, proxy_port: Option<u16>) -> String {
    match mode {
        NetworkMode::Deny => "(deny network*)\n(deny system-socket)".to_string(),
        NetworkMode::Allow => {
            ";; Unrestricted outbound network access\n(allow network-outbound)\n(allow network-inbound (local ip \"localhost:*\"))\n(allow network-bind (local ip \"localhost:*\"))\n(allow system-socket)".to_string()
        }
        NetworkMode::Proxy => {
            let port_spec = proxy_port.map(|p| format!(":{}", p)).unwrap_or_else(|| ":*".to_string());
            format!(
                ";; Egress restricted strictly to local proxy on localhost{}\n(allow network-bind (local ip \"localhost:*\"))\n(allow network-inbound (local ip \"localhost:*\"))\n(allow network-outbound (remote ip \"localhost{}\"))\n(allow system-socket)",
                port_spec, port_spec
            )
        }
    }
}
