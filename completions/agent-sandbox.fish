# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_agent_sandbox_global_optspecs
    string join \n v h/help V/version
end

function __fish_agent_sandbox_needs_command
    # Figure out if the current invocation already has a command.
    set -l cmd (commandline -opc)
    set -e cmd[1]
    argparse -s (__fish_agent_sandbox_global_optspecs) -- $cmd 2>/dev/null
    or return
    if set -q argv[1]
        # Also print the command, so this can be used to figure out what it is.
        echo $argv[1]
        return 1
    end
    return 0
end

function __fish_agent_sandbox_using_subcommand
    set -l cmd (__fish_agent_sandbox_needs_command)
    test -z "$cmd"
    and return 1
    contains -- $cmd[1] $argv
end

complete -c agent-sandbox -n "__fish_agent_sandbox_needs_command" -s v -d 'Print version information (-v)'
complete -c agent-sandbox -n "__fish_agent_sandbox_needs_command" -s h -l help -d 'Print help'
complete -c agent-sandbox -n "__fish_agent_sandbox_needs_command" -s V -l version -d 'Print version'
complete -c agent-sandbox -n "__fish_agent_sandbox_needs_command" -f -a "run" -d 'Execute an isolated command inside the Apple Seatbelt kernel sandbox'
complete -c agent-sandbox -n "__fish_agent_sandbox_needs_command" -f -a "shell" -d 'Launch an interactive sandboxed shell (zsh/bash) with ephemeral session storage'
complete -c agent-sandbox -n "__fish_agent_sandbox_needs_command" -f -a "doctor" -d 'Audit system security controls and in-memory kernel sandbox'
complete -c agent-sandbox -n "__fish_agent_sandbox_needs_command" -f -a "test" -d 'Execute automated security isolation regression test suite (12 invariants)'
complete -c agent-sandbox -n "__fish_agent_sandbox_needs_command" -f -a "broker" -d 'Hardware and macOS capability broker interface'
complete -c agent-sandbox -n "__fish_agent_sandbox_needs_command" -f -a "mcp" -d 'Run Model Context Protocol (MCP) stdio server'
complete -c agent-sandbox -n "__fish_agent_sandbox_needs_command" -f -a "version" -d 'Print version information'
complete -c agent-sandbox -n "__fish_agent_sandbox_needs_command" -f -a "completions" -d 'Generate shell autocompletion script (bash, zsh, fish, powershell)'
complete -c agent-sandbox -n "__fish_agent_sandbox_needs_command" -f -a "man" -d 'Render Unix roff man page documentation to stdout'
complete -c agent-sandbox -n "__fish_agent_sandbox_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand run" -s w -l workspace -d 'Path to workspace directory (default: current working directory)' -r -F
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand run" -s n -l net -d 'Outbound network policy: allow, proxy, or deny' -r
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand run" -s d -l devices -d 'Hardware and serial device policy: serial, allow, or deny' -r
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand run" -l writable -d 'Additional directories permitted for write operations' -r -F
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand run" -s e -l env -d 'Set an environment variable inside the sandbox (KEY=VAL)' -r
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand run" -l pass-env -d 'Explicitly pass through a sensitive host environment variable' -r
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand run" -l allow-home -d 'Explicitly permit using $HOME directory as workspace'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand run" -s h -l help -d 'Print help'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand shell" -s w -l workspace -d 'Path to workspace directory (default: current working directory)' -r -F
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand shell" -s n -l net -d 'Outbound network policy: allow, proxy, or deny' -r
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand shell" -s d -l devices -d 'Hardware and serial device policy: serial, allow, or deny' -r
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand shell" -l writable -d 'Additional directories permitted for write operations' -r -F
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand shell" -s e -l env -d 'Set an environment variable inside the sandbox (KEY=VAL)' -r
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand shell" -l pass-env -d 'Explicitly pass through a sensitive host environment variable' -r
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand shell" -l allow-home -d 'Explicitly permit using $HOME directory as workspace'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand shell" -s h -l help -d 'Print help'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand doctor" -s h -l help -d 'Print help'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand test" -s h -l help -d 'Print help'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand broker; and not __fish_seen_subcommand_from serial-list usb-list notify status help" -s h -l help -d 'Print help'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand broker; and not __fish_seen_subcommand_from serial-list usb-list notify status help" -f -a "serial-list" -d 'List active serial ports (/dev/cu.*)'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand broker; and not __fish_seen_subcommand_from serial-list usb-list notify status help" -f -a "usb-list" -d 'Query USB device topology'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand broker; and not __fish_seen_subcommand_from serial-list usb-list notify status help" -f -a "notify" -d 'Post a safe native macOS user notification'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand broker; and not __fish_seen_subcommand_from serial-list usb-list notify status help" -f -a "status" -d 'Query macOS battery and uptime telemetry'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand broker; and not __fish_seen_subcommand_from serial-list usb-list notify status help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand broker; and __fish_seen_subcommand_from serial-list" -s h -l help -d 'Print help'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand broker; and __fish_seen_subcommand_from usb-list" -s h -l help -d 'Print help'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand broker; and __fish_seen_subcommand_from notify" -s t -l title -r
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand broker; and __fish_seen_subcommand_from notify" -s m -l message -r
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand broker; and __fish_seen_subcommand_from notify" -s s -l subtitle -r
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand broker; and __fish_seen_subcommand_from notify" -s h -l help -d 'Print help'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand broker; and __fish_seen_subcommand_from status" -s h -l help -d 'Print help'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand broker; and __fish_seen_subcommand_from help" -f -a "serial-list" -d 'List active serial ports (/dev/cu.*)'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand broker; and __fish_seen_subcommand_from help" -f -a "usb-list" -d 'Query USB device topology'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand broker; and __fish_seen_subcommand_from help" -f -a "notify" -d 'Post a safe native macOS user notification'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand broker; and __fish_seen_subcommand_from help" -f -a "status" -d 'Query macOS battery and uptime telemetry'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand broker; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand mcp" -s h -l help -d 'Print help'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand version" -s h -l help -d 'Print help'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand completions" -s h -l help -d 'Print help'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand man" -s h -l help -d 'Print help'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand help; and not __fish_seen_subcommand_from run shell doctor test broker mcp version completions man help" -f -a "run" -d 'Execute an isolated command inside the Apple Seatbelt kernel sandbox'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand help; and not __fish_seen_subcommand_from run shell doctor test broker mcp version completions man help" -f -a "shell" -d 'Launch an interactive sandboxed shell (zsh/bash) with ephemeral session storage'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand help; and not __fish_seen_subcommand_from run shell doctor test broker mcp version completions man help" -f -a "doctor" -d 'Audit system security controls and in-memory kernel sandbox'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand help; and not __fish_seen_subcommand_from run shell doctor test broker mcp version completions man help" -f -a "test" -d 'Execute automated security isolation regression test suite (12 invariants)'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand help; and not __fish_seen_subcommand_from run shell doctor test broker mcp version completions man help" -f -a "broker" -d 'Hardware and macOS capability broker interface'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand help; and not __fish_seen_subcommand_from run shell doctor test broker mcp version completions man help" -f -a "mcp" -d 'Run Model Context Protocol (MCP) stdio server'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand help; and not __fish_seen_subcommand_from run shell doctor test broker mcp version completions man help" -f -a "version" -d 'Print version information'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand help; and not __fish_seen_subcommand_from run shell doctor test broker mcp version completions man help" -f -a "completions" -d 'Generate shell autocompletion script (bash, zsh, fish, powershell)'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand help; and not __fish_seen_subcommand_from run shell doctor test broker mcp version completions man help" -f -a "man" -d 'Render Unix roff man page documentation to stdout'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand help; and not __fish_seen_subcommand_from run shell doctor test broker mcp version completions man help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand help; and __fish_seen_subcommand_from broker" -f -a "serial-list" -d 'List active serial ports (/dev/cu.*)'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand help; and __fish_seen_subcommand_from broker" -f -a "usb-list" -d 'Query USB device topology'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand help; and __fish_seen_subcommand_from broker" -f -a "notify" -d 'Post a safe native macOS user notification'
complete -c agent-sandbox -n "__fish_agent_sandbox_using_subcommand help; and __fish_seen_subcommand_from broker" -f -a "status" -d 'Query macOS battery and uptime telemetry'
