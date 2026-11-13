//! CLI UI styling, completion generation, and man page rendering.

use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap::{Command, ValueEnum};
use std::io;

/// Return unified, high-contrast terminal styling for clap commands.
pub fn clap_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Green.on_default() | Effects::BOLD)
        .usage(AnsiColor::Green.on_default() | Effects::BOLD)
        .literal(AnsiColor::Cyan.on_default() | Effects::BOLD)
        .placeholder(AnsiColor::Cyan.on_default())
        .valid(AnsiColor::Green.on_default())
        .invalid(AnsiColor::Yellow.on_default() | Effects::BOLD)
        .error(AnsiColor::Red.on_default() | Effects::BOLD)
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum ShellCompletion {
    Bash,
    Zsh,
    Fish,
    PowerShell,
}

/// Generate shell autocompletion script to stdout.
pub fn generate_completions(shell: ShellCompletion, cmd: &mut Command, bin_name: &str) {
    let mut stdout = io::stdout();
    match shell {
        ShellCompletion::Bash => {
            clap_complete::generate(clap_complete::shells::Bash, cmd, bin_name, &mut stdout);
        }
        ShellCompletion::Zsh => {
            clap_complete::generate(clap_complete::shells::Zsh, cmd, bin_name, &mut stdout);
        }
        ShellCompletion::Fish => {
            clap_complete::generate(clap_complete::shells::Fish, cmd, bin_name, &mut stdout);
        }
        ShellCompletion::PowerShell => {
            clap_complete::generate(clap_complete::shells::PowerShell, cmd, bin_name, &mut stdout);
        }
    }
}

/// Render roff man page documentation to stdout.
pub fn render_man_page(cmd: Command) -> io::Result<()> {
    let man = clap_mangen::Man::new(cmd);
    let mut stdout = io::stdout();
    man.render(&mut stdout)
}
