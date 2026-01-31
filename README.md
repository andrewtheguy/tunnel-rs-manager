# tunnel-rs-manager

A cross-platform desktop application to manage [tunnel-rs](https://github.com/andrewtheguy/tunnel-rs) client instances. Built with Tauri, React, and TypeScript.

## Features

- Persistent configuration storage
- Custom binary path support
- Cross-platform: macOS, Windows
- Server group
- Import/Export configurations

## Screenshot
<img width="1100" height="802" alt="Screenshot 2026-01-19 at 4 38 04 PM" src="https://github.com/user-attachments/assets/4e91fc06-f5b5-4533-b89a-8f30606a0c22" />

## Bundled Binary

This application includes a bundled `tunnel-rs` binary, so no separate installation is required. The bundled version is:

| Component | Version | Source |
|-----------|---------|--------|
| tunnel-rs | **0.1.78** | [GitHub Release](https://github.com/andrewtheguy/tunnel-rs/releases/tag/0.1.78) |

### Supported Platforms

| Platform | Architecture | Status |
|----------|--------------|--------|
| macOS | ARM64 (Apple Silicon) | Bundled |
| Windows | AMD64 (x86_64) | Bundled |

If you need to use a different version, you can specify a custom binary path (see [Custom Binary Path](#custom-binary-path) below). For unsupported platforms, you need to compile both `tunnel-rs` and this GUI yourself.

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

By default, the app uses the bundled `tunnel-rs` binary. If you need to use a different version or your platform isn't supported, you can set a custom binary path:

1. Look for the "Binary:" row in the header showing "Bundled" or the custom path
2. Click **Use Custom** to open a file browser and select your `tunnel-rs` binary
3. The path is saved and persisted across app restarts

To revert to the bundled binary, click **Use Bundled** next to the binary path.

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
│   ├── binaries/           # Bundled tunnel-rs binaries (sidecar)
│   │   ├── tunnel-rs-aarch64-apple-darwin
│   │   └── tunnel-rs-x86_64-pc-windows-msvc.exe
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
