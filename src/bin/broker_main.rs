//! buckle-broker: Standalone CLI and MCP Server for Hardware & macOS Capabilities.

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};

use buckle::broker::{DeviceBroker, MacOSBroker};
use buckle::cli::{clap_styles, generate_completions, render_man_page, ShellCompletion};
use buckle::mcp::run_mcp_stdio_server;

#[derive(Parser, Debug)]
#[command(
    name = "buckle-broker",
    about = "Buckle: Audited Hardware & macOS Capability Broker for AI Agents",
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
    /// List active serial ports (/dev/cu.*)
    Serial {
        #[command(subcommand)]
        action: SerialAction,
    },
    /// Inspect connected USB device tree
    Usb {
        #[command(subcommand)]
        action: UsbAction,
    },
    /// Post a safe native macOS user notification
    Notify {
        #[arg(short, long, default_value = "Agent Sandbox")]
        title: String,
        #[arg(short, long)]
        message: String,
        #[arg(short, long, default_value = "")]
        subtitle: String,
    },
    /// Query system battery and load telemetry
    Status,
    /// Clipboard read/write operations
    Clip {
        #[command(subcommand)]
        action: ClipAction,
    },
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

#[derive(Subcommand, Debug)]
enum SerialAction {
    List,
}

#[derive(Subcommand, Debug)]
enum UsbAction {
    List,
}

#[derive(Subcommand, Debug)]
enum ClipAction {
    Get,
    Set { text: String },
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
        Commands::Serial { action } => match action {
            SerialAction::List => {
                let ports = DeviceBroker::list_serial_ports()?;
                println!("{}", serde_json::to_string_pretty(&ports)?);
            }
        },
        Commands::Usb { action } => match action {
            UsbAction::List => {
                let tree = DeviceBroker::list_usb_tree()?;
                println!("{}", serde_json::to_string_pretty(&tree)?);
            }
        },
        Commands::Notify {
            title,
            message,
            subtitle,
        } => {
            let ok = MacOSBroker::show_notification(&title, &message, &subtitle)?;
            println!("{}", serde_json::json!({ "success": ok }));
        }
        Commands::Status => {
            let status = MacOSBroker::get_system_telemetry()?;
            println!("{}", serde_json::to_string_pretty(&status)?);
        }
        Commands::Clip { action } => match action {
            ClipAction::Get => {
                let text = MacOSBroker::get_clipboard(16384)?;
                println!("{}", serde_json::json!({ "success": true, "text": text }));
            }
            ClipAction::Set { text } => {
                let ok = MacOSBroker::set_clipboard(&text, 32768)?;
                println!("{}", serde_json::json!({ "success": ok }));
            }
        },
        Commands::Mcp => run_mcp_stdio_server()?,
        Commands::Version => {
            println!("buckle-broker {}", env!("CARGO_PKG_VERSION"));
        }
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            generate_completions(shell, &mut cmd, "buckle-broker");
        }
        Commands::Man => {
            let cmd = Cli::command();
            render_man_page(cmd)?;
        }
    }

    Ok(())
}
