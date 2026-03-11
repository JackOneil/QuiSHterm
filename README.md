# QuiSHterm

QuiSHterm is a modern, lightweight, and customizable SSH terminal built with Rust, Tauri, and SvelteKit.

## Features
- **Modern UI**: Clean, intuitive interface for managing multiple SSH sessions.
- **QuickConnect Sidebar**: Right sidebar panel for easy access to saved connections with folder categorization and right-click context menu (Edit/Connect/Remove).
- **Agent/Pageant Authentication**: Connections automatically try Pageant or the system SSH agent first, then configured keys, then default keys from `~/.ssh`, before falling back to password authentication.
- **Password Prompt Fallback**: If no stored password is available, the terminal prompts for it at connect time with a masked input and reveal toggle.
- **Per-Connection Terminal Type**: Each saved connection can choose its PTY terminal type such as `xterm`, `xterm-color`, `xterm-256color`, `screen`, or `vt100`.
- **Config Directory Override**: Choose a custom directory for `settings.json`, `profiles.json`, and `autocomplete.json` directly from Settings.
- **Customizable Highlights**: Color-code terminal output with user-defined regex rules (e.g., green for IP addresses, cyan for timestamps).
- **Live Status Tracking**: Bottom status bar tracking sent/received (TX/RX) network data over SSH along with connection targets.
- **Persistent Storage**: Safely manage SSH profiles and settings.
- **PTY Resizing**: Terminal resize dynamics accurately reflected on the remote server.

## Building from source

To easily compile QuiSHterm on any machine (especially Linux hosts building for Windows) without manually installing all the cross-compilation toolchains, we use Docker. 

A `Dockerfile.builder` is included in the root of the project.

### Prerequisites
- [Docker](https://docs.docker.com/get-docker/) installed and running.

### Build Instructions

1. **Clone the repository**:
   ```bash
   git clone https://github.com/yourusername/ssh_terminal.git
   cd ssh_terminal
   ```

2. **Build the Docker builder image**:
   ```bash
   docker build -t tauri-builder -f Dockerfile.builder .
   ```

3. **Install npm dependencies** (Using the builder container):
   ```bash
   docker run --rm -v $(pwd):/app -w /app tauri-builder npm install
   ```

4. **Build the application**:
   To build the Windows executable (`.exe`) via cross-compilation from the Docker container:
   ```bash
   docker run --rm -v $(pwd):/app -e PKG_CONFIG_ALLOW_CROSS=1 -w /app tauri-builder npm run tauri build -- --target x86_64-pc-windows-gnu
   ```

The compiled installer will be located in: `src-tauri/target/x86_64-pc-windows-gnu/release/bundle/nsis/QuiSHterm_0.1.0_x64-setup.exe`

## Technologies Used
- Frontend: Svelte, Vite, xterm.js, lucide-svelte
- Backend: Rust, Tauri, russh/ssh2
