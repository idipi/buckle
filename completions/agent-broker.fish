# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_agent_broker_global_optspecs
    string join \n v h/help V/version
end

function __fish_agent_broker_needs_command
    # Figure out if the current invocation already has a command.
    set -l cmd (commandline -opc)
    set -e cmd[1]
    argparse -s (__fish_agent_broker_global_optspecs) -- $cmd 2>/dev/null
    or return
    if set -q argv[1]
        # Also print the command, so this can be used to figure out what it is.
        echo $argv[1]
        return 1
    end
    return 0
end

function __fish_agent_broker_using_subcommand
    set -l cmd (__fish_agent_broker_needs_command)
    test -z "$cmd"
    and return 1
    contains -- $cmd[1] $argv
end

complete -c agent-broker -n "__fish_agent_broker_needs_command" -s v -d 'Print version information (-v)'
complete -c agent-broker -n "__fish_agent_broker_needs_command" -s h -l help -d 'Print help'
complete -c agent-broker -n "__fish_agent_broker_needs_command" -s V -l version -d 'Print version'
complete -c agent-broker -n "__fish_agent_broker_needs_command" -f -a "serial" -d 'List active serial ports (/dev/cu.*)'
complete -c agent-broker -n "__fish_agent_broker_needs_command" -f -a "usb" -d 'Inspect connected USB device tree'
complete -c agent-broker -n "__fish_agent_broker_needs_command" -f -a "notify" -d 'Post a safe native macOS user notification'
complete -c agent-broker -n "__fish_agent_broker_needs_command" -f -a "status" -d 'Query system battery and load telemetry'
complete -c agent-broker -n "__fish_agent_broker_needs_command" -f -a "clip" -d 'Clipboard read/write operations'
complete -c agent-broker -n "__fish_agent_broker_needs_command" -f -a "mcp" -d 'Run Model Context Protocol (MCP) stdio server'
complete -c agent-broker -n "__fish_agent_broker_needs_command" -f -a "version" -d 'Print version information'
complete -c agent-broker -n "__fish_agent_broker_needs_command" -f -a "completions" -d 'Generate shell autocompletion script (bash, zsh, fish, powershell)'
complete -c agent-broker -n "__fish_agent_broker_needs_command" -f -a "man" -d 'Render Unix roff man page documentation to stdout'
complete -c agent-broker -n "__fish_agent_broker_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c agent-broker -n "__fish_agent_broker_using_subcommand serial; and not __fish_seen_subcommand_from list help" -s h -l help -d 'Print help'
complete -c agent-broker -n "__fish_agent_broker_using_subcommand serial; and not __fish_seen_subcommand_from list help" -f -a "list"
complete -c agent-broker -n "__fish_agent_broker_using_subcommand serial; and not __fish_seen_subcommand_from list help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c agent-broker -n "__fish_agent_broker_using_subcommand serial; and __fish_seen_subcommand_from list" -s h -l help -d 'Print help'
complete -c agent-broker -n "__fish_agent_broker_using_subcommand serial; and __fish_seen_subcommand_from help" -f -a "list"
complete -c agent-broker -n "__fish_agent_broker_using_subcommand serial; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c agent-broker -n "__fish_agent_broker_using_subcommand usb; and not __fish_seen_subcommand_from list help" -s h -l help -d 'Print help'
complete -c agent-broker -n "__fish_agent_broker_using_subcommand usb; and not __fish_seen_subcommand_from list help" -f -a "list"
complete -c agent-broker -n "__fish_agent_broker_using_subcommand usb; and not __fish_seen_subcommand_from list help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c agent-broker -n "__fish_agent_broker_using_subcommand usb; and __fish_seen_subcommand_from list" -s h -l help -d 'Print help'
complete -c agent-broker -n "__fish_agent_broker_using_subcommand usb; and __fish_seen_subcommand_from help" -f -a "list"
complete -c agent-broker -n "__fish_agent_broker_using_subcommand usb; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c agent-broker -n "__fish_agent_broker_using_subcommand notify" -s t -l title -r
complete -c agent-broker -n "__fish_agent_broker_using_subcommand notify" -s m -l message -r
complete -c agent-broker -n "__fish_agent_broker_using_subcommand notify" -s s -l subtitle -r
complete -c agent-broker -n "__fish_agent_broker_using_subcommand notify" -s h -l help -d 'Print help'
complete -c agent-broker -n "__fish_agent_broker_using_subcommand status" -s h -l help -d 'Print help'
complete -c agent-broker -n "__fish_agent_broker_using_subcommand clip; and not __fish_seen_subcommand_from get set help" -s h -l help -d 'Print help'
complete -c agent-broker -n "__fish_agent_broker_using_subcommand clip; and not __fish_seen_subcommand_from get set help" -f -a "get"
complete -c agent-broker -n "__fish_agent_broker_using_subcommand clip; and not __fish_seen_subcommand_from get set help" -f -a "set"
complete -c agent-broker -n "__fish_agent_broker_using_subcommand clip; and not __fish_seen_subcommand_from get set help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c agent-broker -n "__fish_agent_broker_using_subcommand clip; and __fish_seen_subcommand_from get" -s h -l help -d 'Print help'
complete -c agent-broker -n "__fish_agent_broker_using_subcommand clip; and __fish_seen_subcommand_from set" -s h -l help -d 'Print help'
complete -c agent-broker -n "__fish_agent_broker_using_subcommand clip; and __fish_seen_subcommand_from help" -f -a "get"
complete -c agent-broker -n "__fish_agent_broker_using_subcommand clip; and __fish_seen_subcommand_from help" -f -a "set"
complete -c agent-broker -n "__fish_agent_broker_using_subcommand clip; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c agent-broker -n "__fish_agent_broker_using_subcommand mcp" -s h -l help -d 'Print help'
complete -c agent-broker -n "__fish_agent_broker_using_subcommand version" -s h -l help -d 'Print help'
complete -c agent-broker -n "__fish_agent_broker_using_subcommand completions" -s h -l help -d 'Print help'
complete -c agent-broker -n "__fish_agent_broker_using_subcommand man" -s h -l help -d 'Print help'
complete -c agent-broker -n "__fish_agent_broker_using_subcommand help; and not __fish_seen_subcommand_from serial usb notify status clip mcp version completions man help" -f -a "serial" -d 'List active serial ports (/dev/cu.*)'
complete -c agent-broker -n "__fish_agent_broker_using_subcommand help; and not __fish_seen_subcommand_from serial usb notify status clip mcp version completions man help" -f -a "usb" -d 'Inspect connected USB device tree'
complete -c agent-broker -n "__fish_agent_broker_using_subcommand help; and not __fish_seen_subcommand_from serial usb notify status clip mcp version completions man help" -f -a "notify" -d 'Post a safe native macOS user notification'
complete -c agent-broker -n "__fish_agent_broker_using_subcommand help; and not __fish_seen_subcommand_from serial usb notify status clip mcp version completions man help" -f -a "status" -d 'Query system battery and load telemetry'
complete -c agent-broker -n "__fish_agent_broker_using_subcommand help; and not __fish_seen_subcommand_from serial usb notify status clip mcp version completions man help" -f -a "clip" -d 'Clipboard read/write operations'
complete -c agent-broker -n "__fish_agent_broker_using_subcommand help; and not __fish_seen_subcommand_from serial usb notify status clip mcp version completions man help" -f -a "mcp" -d 'Run Model Context Protocol (MCP) stdio server'
complete -c agent-broker -n "__fish_agent_broker_using_subcommand help; and not __fish_seen_subcommand_from serial usb notify status clip mcp version completions man help" -f -a "version" -d 'Print version information'
complete -c agent-broker -n "__fish_agent_broker_using_subcommand help; and not __fish_seen_subcommand_from serial usb notify status clip mcp version completions man help" -f -a "completions" -d 'Generate shell autocompletion script (bash, zsh, fish, powershell)'
complete -c agent-broker -n "__fish_agent_broker_using_subcommand help; and not __fish_seen_subcommand_from serial usb notify status clip mcp version completions man help" -f -a "man" -d 'Render Unix roff man page documentation to stdout'
complete -c agent-broker -n "__fish_agent_broker_using_subcommand help; and not __fish_seen_subcommand_from serial usb notify status clip mcp version completions man help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c agent-broker -n "__fish_agent_broker_using_subcommand help; and __fish_seen_subcommand_from serial" -f -a "list"
complete -c agent-broker -n "__fish_agent_broker_using_subcommand help; and __fish_seen_subcommand_from usb" -f -a "list"
complete -c agent-broker -n "__fish_agent_broker_using_subcommand help; and __fish_seen_subcommand_from clip" -f -a "get"
complete -c agent-broker -n "__fish_agent_broker_using_subcommand help; and __fish_seen_subcommand_from clip" -f -a "set"
