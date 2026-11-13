//! agent-broker: Standalone CLI and MCP Server for Hardware & macOS Capabilities.

use anyhow::Result;
use clap::{Parser, Subcommand};

use macos_agent_sandbox::broker::{DeviceBroker, MacOSBroker};
use macos_agent_sandbox::mcp::run_mcp_stdio_server;

#[derive(Parser, Debug)]
#[command(
    name = "agent-broker",
    about = "Audited macOS System and Device Broker for AI Agents",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
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

    match cli.command {
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
    }

    Ok(())
}
