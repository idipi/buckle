//! buckle: High-performance kernel-level sandbox orchestrator on macOS.
//! Written in Rust with direct libSystem C FFI, in-memory SBPL compilation, and POSIX process control.

use anyhow::{bail, Result};
use clap::{Args, CommandFactory, Parser, Subcommand};
use colored::Colorize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use buckle::broker::{DeviceBroker, MacOSBroker};
use buckle::cli::{clap_styles, generate_completions, render_man_page, ShellCompletion};
use buckle::doctor::run_doctor;
use buckle::mcp::run_mcp_stdio_server;
use buckle::process::{execute_sandboxed_process, RunOptions};
use buckle::profile::{render_profile, DeviceMode, NetworkMode, ProfileConfig};

#[derive(Parser, Debug)]
#[command(
    name = "buckle",
    about = "Buckle: Kernel-Level Seatbelt Sandbox for AI Agents on macOS",
    version,
    styles = clap_styles()
)]
struct Cli {
    /// Print version information (-v)
    #[arg(short = 'v', action = clap::ArgAction::Version)]
    _version: Option<bool>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Buckle up: execute command in sandbox, or launch interactive shell if empty
    Up(UpArgs),

    /// Execute an isolated command inside the Apple Seatbelt kernel sandbox
    Run(RunArgs),

    /// Launch an interactive sandboxed shell (zsh/bash) with ephemeral session storage
    Shell(ShellArgs),

    /// Audit system security controls and in-memory kernel sandbox
    Doctor,

    /// Execute automated security isolation regression test suite (12 invariants)
    Test,

    /// Hardware and macOS capability broker interface
    Broker(BrokerArgs),

    /// Run Model Context Protocol (MCP) stdio server
    Mcp,

    /// Print version information
    Version,

    /// Generate shell autocompletion script (bash, zsh, fish, powershell)
    Completions {
        #[arg(value_enum)]
        shell: ShellCompletion,
    },

    /// Render Unix roff man page documentation to stdout
    Man,
}

#[derive(Args, Debug)]
pub struct CommonSandboxArgs {
    /// Path to workspace directory (default: current working directory)
    #[arg(short, long)]
    pub workspace: Option<PathBuf>,

    /// Outbound network policy: allow, proxy, or deny
    #[arg(short, long, default_value = "allow")]
    pub net: NetworkMode,

    /// Hardware and serial device policy: serial, allow, or deny
    #[arg(short, long, default_value = "serial")]
    pub devices: DeviceMode,

    /// Additional directories permitted for write operations
    #[arg(long = "writable")]
    pub writable: Vec<PathBuf>,

    /// Set an environment variable inside the sandbox (KEY=VAL)
    #[arg(short = 'e', long = "env")]
    pub env: Vec<String>,

    /// Explicitly pass through a sensitive host environment variable
    #[arg(long = "pass-env")]
    pub pass_env: Vec<String>,

    /// Explicitly permit using $HOME directory as workspace
    #[arg(long = "allow-home")]
    pub allow_home: bool,
}

#[derive(Args, Debug)]
struct UpArgs {
    #[command(flatten)]
    common: CommonSandboxArgs,

    /// Command and arguments to execute inside the sandbox (launches interactive shell if omitted)
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    command: Vec<String>,
}

#[derive(Args, Debug)]
struct RunArgs {
    #[command(flatten)]
    common: CommonSandboxArgs,

    /// Command and arguments to execute inside the sandbox
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
    command: Vec<String>,
}

#[derive(Args, Debug)]
struct ShellArgs {
    #[command(flatten)]
    common: CommonSandboxArgs,
}

#[derive(Args, Debug)]
struct BrokerArgs {
    #[command(subcommand)]
    subcommand: BrokerSubcommands,
}

#[derive(Subcommand, Debug)]
enum BrokerSubcommands {
    /// List active serial ports (/dev/cu.*)
    SerialList,
    /// Query USB device topology
    UsbList,
    /// Post a safe native macOS user notification
    Notify {
        #[arg(short, long, default_value = "Buckle")]
        title: String,
        #[arg(short, long)]
        message: String,
        #[arg(short, long, default_value = "")]
        subtitle: String,
    },
    /// Query macOS battery and uptime telemetry
    Status,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let command = match cli.command {
        Some(c) => c,
        None => {
            let mut help_cmd = Cli::command();
            let _ = help_cmd.print_help();
            println!();
            return Ok(());
        }
    };

    match command {
        Commands::Up(args) => {
            if args.command.is_empty() {
                let code = run_shell(&args.common)?;
                std::process::exit(code);
            } else {
                let code = run_command(&args.common, &args.command, false)?;
                std::process::exit(code);
            }
        }
        Commands::Run(args) => {
            let code = run_command(&args.common, &args.command, false)?;
            std::process::exit(code);
        }
        Commands::Shell(args) => {
            let code = run_shell(&args.common)?;
            std::process::exit(code);
        }
        Commands::Doctor => run_doctor(),
        Commands::Test => run_tests(),
        Commands::Broker(args) => match args.subcommand {
            BrokerSubcommands::SerialList => {
                let ports = DeviceBroker::list_serial_ports()?;
                println!("{}", serde_json::to_string_pretty(&ports)?);
                Ok(())
            }
            BrokerSubcommands::UsbList => {
                let tree = DeviceBroker::list_usb_tree()?;
                println!("{}", serde_json::to_string_pretty(&tree)?);
                Ok(())
            }
            BrokerSubcommands::Notify {
                title,
                message,
                subtitle,
            } => {
                let ok = MacOSBroker::show_notification(&title, &message, &subtitle)?;
                println!("{}", serde_json::json!({ "success": ok }));
                Ok(())
            }
            BrokerSubcommands::Status => {
                let status = MacOSBroker::get_system_telemetry()?;
                println!("{}", serde_json::to_string_pretty(&status)?);
                Ok(())
            }
        },
        Commands::Mcp => run_mcp_stdio_server(),
        Commands::Version => {
            println!("buckle {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            generate_completions(shell, &mut cmd, "buckle");
            Ok(())
        }
        Commands::Man => {
            let cmd = Cli::command();
            render_man_page(cmd)?;
            Ok(())
        }
    }
}

fn run_shell(common: &CommonSandboxArgs) -> Result<i32> {
    let shell_bin = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    println!("{}", "[*] Buckling up: Spawning sandboxed interactive shell...".cyan());
    println!("{}", "[*] Press Ctrl+D or type 'exit' to leave.".dimmed());
    run_command(common, &[shell_bin, "-i".to_string(), "-l".to_string()], true)
}

fn run_command(common: &CommonSandboxArgs, command: &[String], is_shell: bool) -> Result<i32> {
    let cwd = std::env::current_dir()?;
    let workspace = common.workspace.clone().unwrap_or(cwd).canonicalize().unwrap_or_else(|_| common.workspace.clone().unwrap_or_default());
    let home = std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("/var/empty"));

    // Safety Guard: Prohibit accidental workspace execution in $HOME without explicit flag
    if workspace == home && !common.allow_home {
        eprintln!(
            "{} Safety Guard: Running buckle directly in $HOME ({}) is blocked by default.",
            "[-]".red().bold(),
            workspace.display()
        );
        eprintln!("[-] An agent should operate inside a dedicated project directory (e.g. ~/projects/my-repo).");
        eprintln!("[-] Please 'cd' into your project, or pass --allow-home if you explicitly want to allow writing in $HOME.");
        return Ok(1);
    }

    if !is_shell {
        println!("{}", "[*] Buckling up: Initializing Native macOS Kernel Sandbox (Seatbelt/FFI)...".cyan().bold());
        println!("{} Workspace: {}", "[*]".blue(), workspace.display());
        println!("{} Network Mode: {:?}", "[*]".blue(), common.net);
        println!("{} Device Mode:  {:?}", "[*]".blue(), common.devices);
    }

    // Parse custom environment variables
    let mut custom_env = HashMap::new();
    for env_str in &common.env {
        if let Some((k, v)) = env_str.split_once('=') {
            custom_env.insert(k.to_string(), v.to_string());
        }
    }

    // Compile in-memory Seatbelt profile
    let profile_cfg = ProfileConfig {
        workspace_dir: &workspace,
        home_dir: &home,
        extra_writable: &common.writable,
        network_mode: common.net,
        device_mode: common.devices,
        proxy_port: None,
    };
    let profile_content = render_profile(&profile_cfg);

    // Execute native fork + sandbox_init + execve
    let run_opts = RunOptions {
        command,
        workspace: &workspace,
        profile_content: &profile_content,
        pass_env: &common.pass_env,
        custom_env: &custom_env,
        is_interactive_shell: is_shell,
    };

    execute_sandboxed_process(&run_opts)
}

fn run_tests() -> Result<()> {
    println!("{}", "Running automated security isolation regression tests (Native Rust)...".bold().cyan());

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cargo_toml = manifest_dir.join("Cargo.toml");

    if !cargo_toml.exists() {
        bail!(
            "Source repository not found at {:?}. Cannot run cargo test without project source.",
            manifest_dir
        );
    }

    let status = std::process::Command::new("cargo")
        .arg("test")
        .arg("--test")
        .arg("integration_tests")
        .arg("--")
        .arg("--nocapture")
        .current_dir(manifest_dir)
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("{}", "\n[+] All native Rust security tests PASSED.".green().bold());
            Ok(())
        }
        Ok(s) => bail!("Integration tests failed with exit code: {:?}", s.code()),
        Err(e) => bail!("Failed to execute 'cargo test': {}. Please ensure cargo is installed and in PATH.", e),
    }
}
