# rs_terminal

A Rust-based terminal backend for Waylon Terminal, supporting multiple PTY implementations and WebSocket communication.

## Features

- RESTful API for session management
- WebSocket support for terminal communication
- Multiple PTY implementations support
- Cross-platform compatibility
- Async programming with Tokio
- Configurable via TOML file

## PTY Implementations

rs_terminal supports multiple PTY implementations, which can be selected via the `config.toml` file:

1. **tokio_process** - Default implementation using standard process I/O, cross-platform compatible
2. **portable_pty** - Cross-platform PTY support using `portable-pty` library

## Getting Started

### Prerequisites

- Rust 1.65 or newer
- Cargo package manager

### Installation

1. Clone the repository:

```bash
git clone https://github.com/waylondev/terminal.git
cd terminal/rs_terminal
```

2. Build with default features:

```bash
cargo build
```

3. Build with specific PTY implementation features:

```bash
# Build with portable-pty support
cargo build --features portable-pty
```

### Configuration

Edit the `config.toml` file to configure the terminal:

```toml
# PTY implementation to use (options: "tokio_process", "tokio_pty_process", "portable_pty")
pty_implementation = "tokio_pty_process"

# Default shell type to use
default_shell_type = "bash"

# HTTP server port
http_port = 8080

# WebTransport server port
webtransport_port = 8082
```

### Running

```bash
# Run with default features
cargo run

# Run with portable-pty implementation
cargo run --features portable-pty
cargo run --features expectrl-pty
```

## API Endpoints

### Sessions

- `POST /api/sessions` - Create a new terminal session
- `GET /api/sessions` - Get all terminal sessions
- `GET /api/sessions/:session_id` - Get a specific terminal session
- `POST /api/sessions/:session_id/resize` - Resize a terminal session
- `DELETE /api/sessions/:session_id` - Terminate a terminal session

### WebSocket

- `GET /ws` - Connect to a new terminal session via WebSocket
- `GET /ws/:session_id` - Connect to an existing terminal session via WebSocket

## Project Structure

```
rs_terminal/
├── src/
│   ├── api/            # API DTO definitions
│   ├── app_state/      # Application state management
│   ├── config/         # Configuration handling
│   ├── handlers/       # HTTP and WebSocket handlers
│   ├── protocol/       # Terminal connection protocols
│   ├── pty/            # PTY implementations
│   │   ├── mod.rs              # PTY factory and trait definitions
│   │   ├── portable_pty_impl.rs # portable-pty implementation
│   │   ├── pty_trait.rs        # AsyncPty trait definition
│   │   └── tokio_process_pty_impl.rs  # tokio-process implementation
│   ├── server/         # HTTP server setup
│   ├── service/        # Business logic services
│   └── main.rs         # Application entry point
├── config.toml         # Configuration file
└── Cargo.toml          # Rust package configuration
```

## Feature Flags

- `portable-pty` - Enable portable-pty PTY implementation

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT
