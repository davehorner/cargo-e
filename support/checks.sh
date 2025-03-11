#!/bin/sh
# Enable command echoing
set -x

# Change directory to the current working directory (invocation directory)
CURRENT_DIR="$(pwd)"

# Run the initial cargo commands in the current shell
cargo doc
cargo build
cargo test
cargo hack check --each-feature --no-dev-deps
cargo fix --allow-dirty
cargo fmt
cargo audit

CLIPPY_ARGS=''

# Define the command to run in the new terminal window for bacon clippy
BACON_COMMAND='cd "'"$CURRENT_DIR"'" && bacon clippy'"$CLIPPY_ARGS"'; echo "Press Enter to exit..."; read'

# Function to launch a command in a new terminal window
launch_in_terminal() {
    local cmd="$1"
    case "$(uname)" in
      Darwin)
        # macOS: Check if Alacritty exists in /Applications
        if [ -d "/Applications/Alacritty.app" ]; then
          /Applications/Alacritty.app/Contents/MacOS/alacritty -e bash -c "$cmd" &
        else
          # Fallback: use Terminal with AppleScript
          osascript <<EOF
tell application "Terminal"
    do script "$cmd"
    activate
end tell
EOF
        fi
        ;;
      Linux)
        # Linux: Try launching with Alacritty if available, otherwise check common terminal emulators
        if command -v alacritty >/dev/null 2>&1; then
          alacritty -e bash -c "$cmd" &
        elif command -v gnome-terminal >/dev/null 2>&1; then
          gnome-terminal -- bash -c "$cmd; exec bash"
        elif command -v konsole >/dev/null 2>&1; then
          konsole -e bash -c "$cmd; exec bash"
        elif command -v xterm >/dev/null 2>&1; then
          xterm -e bash -c "$cmd; exec bash" &
        else
          echo "No supported terminal emulator found. Please install alacritty, gnome-terminal, konsole, or xterm."
        fi
        ;;
      *)
        echo "Unsupported OS"
        ;;
    esac
}

# Launch bacon clippy in a new terminal
launch_in_terminal "$BACON_COMMAND"

# Now, similarly launch cbacon in a new terminal window
# Define the command for cbacon (adjust as needed)
CBACON_COMMAND='cd "'"$CURRENT_DIR"'" && cbacon; echo "Press Enter to exit..."; read'
launch_in_terminal "$CBACON_COMMAND"

