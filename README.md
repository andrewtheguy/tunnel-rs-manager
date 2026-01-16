# tunnel-rs-manager

A cross-platform desktop application to manage [tunnel-rs](https://github.com/andrewtheguy/tunnel-rs) client instances. Built with Tauri, React, and TypeScript.

## Features

- Create, edit, and delete tunnel configurations
- Start and stop tunnel instances with one click
- View real-time tunnel status and logs
- System tray integration (minimize to tray)
- Persistent configuration storage
- Custom binary path support
- Cross-platform: macOS, Windows

## Prerequisites

You need to have the `tunnel-rs` binary installed on your system. The app will automatically search for it in common locations:

**macOS:**
- `~/.local/bin/tunnel-rs`
- `~/.cargo/bin/tunnel-rs`
- `~/bin/tunnel-rs`
- `/usr/local/bin/tunnel-rs`
- `/opt/homebrew/bin/tunnel-rs`
- `/usr/bin/tunnel-rs`

**Windows:**
- `%LOCALAPPDATA%\Programs\tunnel-rs\tunnel-rs.exe` (default installer location)
- `%USERPROFILE%\.local\bin\tunnel-rs.exe`
- `%USERPROFILE%\.cargo\bin\tunnel-rs.exe`
- `C:\Program Files\tunnel-rs\tunnel-rs.exe`

If `tunnel-rs` is not found automatically, you can set a custom binary path (see [Custom Binary Path](#custom-binary-path) below).

## Installation

### From Releases

Download the latest release for your platform from the [Releases](https://github.com/andrewtheguy/tunnel-rs-manager/releases) page.

### Building from Source

#### Requirements

- [Node.js](https://nodejs.org/) 18+ or [Bun](https://bun.sh/)
- [Rust](https://rustup.rs/) 1.70+
- Platform-specific dependencies for Tauri (see [Tauri Prerequisites](https://tauri.app/start/prerequisites/))

#### Build Steps

```bash
# Clone the repository
git clone https://github.com/andrewtheguy/tunnel-rs-manager.git
cd tunnel-rs-manager

# Install dependencies
bun install
# or: npm install

# Run in development mode
bun run tauri dev
# or: npm run tauri dev

# Build for production
bun run tauri build
# or: npm run tauri build
```

## Usage

1. **Add a Configuration**: Click the "+" button to create a new tunnel configuration
2. **Configure the Tunnel**: Enter the server node ID, source, target, and optional auth token
3. **Start the Tunnel**: Click the play button on a configuration card to start the tunnel
4. **View Logs**: Expand a running tunnel card to view real-time logs
5. **Stop the Tunnel**: Click the stop button to terminate the tunnel

### Configuration Options

| Field | Description |
|-------|-------------|
| Name | A friendly name for the configuration |
| Server Node ID | The [Iroh](https://iroh.computer/) node ID of the tunnel server. Iroh is a peer-to-peer networking library; the node ID is a unique identifier (public key) for the server, obtained from the tunnel-rs server output when it starts |
| Source | Source address to connect to on the server side. Can be an IP or hostname resolved by the server (e.g., `tcp://127.0.0.1:22`, `tcp://internal-host:5432`) |
| Target | Local address where the tunnel will listen for connections (e.g., `127.0.0.1:2222`) |
| Auth Token | Optional authentication token for server verification |
| Relay URLs | Optional comma-separated list of Iroh relay server URLs for NAT traversal |

### Custom Binary Path

If `tunnel-rs` is installed in a non-standard location, you can set a custom path using the Tauri `invoke` API from the browser developer console (open with F12 or Cmd+Option+I):

```javascript
// Set a custom binary path
await window.__TAURI__.core.invoke('set_binary_path', { path: '/path/to/tunnel-rs' });

// Clear the custom path (revert to auto-detection)
await window.__TAURI__.core.invoke('set_binary_path', { path: null });

// Get the current custom path
await window.__TAURI__.core.invoke('get_binary_path');
```

The setting is persisted across app restarts.

## Data Storage

Configuration and settings are stored in the platform-specific data directory:

- **macOS**: `~/Library/Application Support/tunnel-rs-manager/`
- **Windows**: `%APPDATA%\tunnel-rs-manager\`

Files:
- `configs.json` - Saved tunnel configurations
- `settings.json` - App settings (e.g., custom binary path)

## Development

### Project Structure

```
tunnel-rs-manager/
├── src/                    # React frontend
│   ├── components/         # UI components
│   ├── hooks/              # React hooks for state management
│   └── types.ts            # TypeScript type definitions
├── src-tauri/              # Tauri backend (Rust)
│   ├── src/
│   │   ├── config.rs       # Configuration storage
│   │   ├── process.rs      # Process management
│   │   └── lib.rs          # Tauri commands and setup
│   └── Cargo.toml
└── package.json
```

### Tech Stack

- **Frontend**: React 18, TypeScript, Vite
- **Backend**: Tauri 2, Rust
- **Process Management**: Tokio async runtime

## License

MIT
