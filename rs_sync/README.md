# rs_sync - Rust File Synchronization Tool

A simple Rust application that allows a client to periodically fetch a file from a server and copy its content to the clipboard.

## Project Structure

```
rs_sync/
├── Cargo.toml          # Workspace configuration
├── server/             # Server implementation
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
└── client/             # Client implementation
    ├── Cargo.toml
    └── src/
        └── main.rs
```

## Features

### Server
- Accepts command line arguments for host, port, and file path
- Sets up an HTTP server using Axum
- Provides an endpoint to serve the file content
- Streams file content efficiently

### Client
- Accepts command line arguments for server address, endpoint, and update interval
- Periodically fetches file content from the server
- Copies the content to the system clipboard
- Provides status updates to the console

## Installation

### Prerequisites
- Rust 1.70.0 or later

### Build

```bash
# Build both server and client
cargo build --workspace

# Build only server
cargo build -p server

# Build only client
cargo build -p client
```

## Usage

### Server

```bash
# Run server with default host (127.0.0.1) and port (3000)
cargo run -p server -- -f <file_path>

# Run server with custom host and port
cargo run -p server -- --host 0.0.0.0 --port 8080 -f <file_path>

# Help
cargo run -p server -- --help
```

#### Command Line Arguments
- `-h, --host <HOST>` - Server host address (default: 127.0.0.1)
- `-p, --port <PORT>` - Server port (default: 3000)
- `-f, --file-path <FILE_PATH>` - Path to the file to serve (required)

### Client

```bash
# Run client with default endpoint (/file) and interval (5 seconds)
cargo run -p client -- -a http://localhost:3000

# Run client with custom endpoint and interval
cargo run -p client -- -a http://localhost:3000 -e /api/file -i 10

# Help
cargo run -p client -- --help
```

#### Command Line Arguments
- `-a, --http-address <HTTP_ADDRESS>` - Server HTTP address (required)
- `-e, --endpoint <ENDPOINT>` - API endpoint path (default: /file)
- `-i, --interval <INTERVAL>` - Update interval in seconds (default: 5)

## Example Usage

### Terminal 1: Start the Server

```bash
# Serve the content of example.txt on localhost:3000
cargo run -p server -- -f ./example.txt
```

### Terminal 2: Start the Client

```bash
# Fetch from http://localhost:3000/file every 5 seconds
cargo run -p client -- -a http://localhost:3000
```

## Technologies Used

- **Server**: Axum, Tokio, Clap
- **Client**: Reqwest, Arboard, Tokio, Clap
- **Build System**: Cargo

## License

MIT
