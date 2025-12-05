# Waylon Terminal - Web-Based Terminal Application

A modern, multi-protocol web terminal application with support for real-time communication, built with cutting-edge technologies across multiple platforms.

## 🚀 Core Business Features

### Terminal Session Management
- **Create and manage** terminal sessions through a web interface
- **Multi-session support** - Run multiple terminals simultaneously
- **Session persistence** - Resume sessions across browser refreshes
- **User-based session isolation** - Secure session management per user

### Real-time Communication
- **WebSocket support** for reliable real-time communication
- **WebTransport support** for low-latency communication (future-proof)
- **Automatic protocol fallback** - Uses best available protocol

### Terminal Features
- **Full terminal emulation** using xterm.js
- **Dynamic resizing** - Resize terminals in real-time
- **Multiple shell support** - Configure different shell types
- **Customizable working directories** - Start terminals in any directory
- **Environment variable support** - Configure shell environments

### User Experience
- **Responsive design** - Works on desktop and mobile devices
- **Fullscreen mode** - Immersive terminal experience
- **Modern UI** - Clean, intuitive interface built with React
- **Session management panel** - Easy session switching and management

## 🛠️ Technical Architecture

### Frontend Implementation
**Location**: `clients/web-terminal`

**Technology Stack**:
- **React 19** - Modern UI framework
- **TypeScript** - Type-safe development
- **Vite** - Fast build tool and dev server
- **xterm.js** - Terminal emulation
- **Tailwind CSS** - Utility-first CSS framework
- **Radix UI** - Accessible UI components
- **Kotlinx Serialization** - JSON serialization
- **WebSocket/WebTransport** - Real-time communication

**Key Features**:
- Component-based architecture with clean separation of concerns
- Type-safe API communication
- Real-time terminal rendering
- Responsive design with mobile support
- Session management UI
- Fullscreen mode

### Backend Implementations

#### Kotlin Implementation (Primary)
**Location**: `kt-terminal`

**Technology Stack**:
- **Kotlin** - Modern JVM language
- **Ktor** - Asynchronous web framework
- **Koin** - Dependency injection
- **Coroutines** - Asynchronous programming
- **PTY4J** - Pseudo-terminal implementation
- **DDD Architecture** - Domain-driven design
- **SOLID Principles** - Clean code design

**Key Features**:
- RESTful API for session management
- WebSocket and WebTransport support
- PTY process management
- Cross-platform terminal support
- Session timeout management
- Comprehensive logging

#### Rust Implementation
**Location**: `rs_terminal`

**Technology Stack**:
- **Rust** - Systems programming language
- **Tokio** - Asynchronous runtime
- **async-std** - Async utilities
- **nix** - Unix system calls
- **PTY** - Pseudo-terminal implementation

**Key Features**:
- High-performance terminal processing
- Low-memory footprint
- Direct system call access
- Unix-like systems optimization

## 🌟 Technology Highlights

### Multi-Protocol Support
- WebSocket for reliable communication
- WebTransport for next-generation low-latency communication
- Automatic protocol negotiation and fallback

### Domain-Driven Design
- Clear bounded contexts
- Rich domain models
- Repository pattern
- Use case-driven architecture
- Clean separation of concerns

### SOLID Design Principles
- Single Responsibility Principle
- Open/Closed Principle
- Liskov Substitution Principle
- Interface Segregation Principle
- Dependency Inversion Principle

### Modern Language Features
- Kotlin coroutines for async programming
- Rust's safety and performance
- TypeScript for type safety
- React hooks for component logic

### Cross-Platform Support
- Works on Windows, macOS, and Linux
- Multiple backend implementations for different use cases
- Responsive design for mobile and desktop

## 📁 Project Structure

```
┌───────────────────────────────────────────────────────────────────────────────────┐
│                               Waylon Terminal Project                            │
├───────────────────────────────────────────────────────────────────────────────────┤
│                                                                                   │
│  ┌───────────────────────────────────────────────────────────────────────────┐     │
│  │                             Frontend Layer                              │     │
│  └───────────────────────────────────────────────────────────────────────────┘     │
│  │  clients/web-terminal/                                                  │     │
│  │  ├── public/                # Static assets                            │     │
│  │  ├── src/                   # Source code                               │     │
│  │  │   ├── components/        # React components                         │     │
│  │  │   ├── config/            # Application configuration               │     │
│  │  │   ├── services/          # API and communication services          │     │
│  │  │   ├── App.tsx            # Main application component              │     │
│  │  │   └── main.tsx           # Application entry point                 │     │
│  │  └── package.json           # Dependencies and scripts                │     │
│  └───────────────────────────────────────────────────────────────────────────┘     │
│                                                                                   │
│  ┌───────────────────────────────────────────────────────────────────────────┐     │
│  │                            Backend Layer                               │     │
│  └───────────────────────────────────────────────────────────────────────────┘     │
│  │  kt-terminal/               # Kotlin backend implementation             │     │
│  │  ├── src/                   # Source code                               │     │
│  │  │   ├── main/kotlin/        # Kotlin source                            │     │
│  │  │   └── resources/          # Configuration files                     │     │
│  │  └── build.gradle.kts        # Gradle build file                       │     │
│  │                                                                       │     │
│  │  rs_terminal/                # Rust terminal implementation            │     │
│  │  ├── src/                   # Source code                               │     │
│  │  └── Cargo.toml              # Cargo configuration                     │     │
│  └───────────────────────────────────────────────────────────────────────────┘     │
│                                                                                   │
└───────────────────────────────────────────────────────────────────────────────────┘
```

## 🚀 Getting Started

### Prerequisites
- **Node.js 18+** - For frontend development
- **pnpm** - Package manager for frontend
- **Java 17+** - For Kotlin backend
- **Gradle 7.5+** - Build tool for Kotlin backend
- **Rust 1.70+** - For Rust backend (optional)

### Frontend Development
```bash
# Install dependencies
cd clients/web-terminal
pnpm install

# Start development server
pnpm run dev
```

### Kotlin Backend Development
```bash
# Build the project
cd kt-terminal
./gradlew build

# Run the application
./gradlew run

# Run tests
./gradlew test
```

### Rust Backend Development (Optional)
```bash
# Build the project
cd rs_terminal
cargo build

# Run the application
cargo run

# Run tests
cargo test
```

## 📱 Usage

1. **Start the backend server**
   ```bash
   cd kt-terminal
   ./gradlew run
   ```

2. **Start the frontend development server**
   ```bash
   cd clients/web-terminal
   pnpm run dev
   ```

3. **Open your browser**
   - Navigate to `http://localhost:5173`
   - Create a new terminal session
   - Start using the terminal!

## 🔧 Configuration

### Frontend Configuration
- Located in `clients/web-terminal/src/config/appConfig.ts`
- Configures API endpoints, WebSocket URLs, and application settings

### Backend Configuration
- Located in `kt-terminal/src/main/resources/application.conf`
- Configures server port, shell settings, session timeout, and more

## 📋 Core API Endpoints

### Session Management
- `POST /api/sessions` - Create a new terminal session
- `GET /api/sessions` - Get all terminal sessions
- `GET /api/sessions/{id}` - Get a specific terminal session
- `DELETE /api/sessions/{id}` - Delete a terminal session
- `POST /api/sessions/{id}/resize` - Resize a terminal session

### Real-time Communication
- `ws://localhost:8080/ws` - WebSocket endpoint
- `https://localhost:8082` - WebTransport endpoint

## 🎯 Key Benefits

### For Developers
- Modern, type-safe development environment
- Clear separation of concerns
- Comprehensive documentation
- Test-driven development support
- Easy to extend and modify

### For Users
- Responsive, intuitive interface
- Low-latency terminal experience
- Secure session management
- Multi-session support
- Cross-platform compatibility

### For Organizations
- Scalable architecture
- Multiple backend options
- Easy deployment
- Comprehensive logging and monitoring
- Secure by design

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

### Contribution Guidelines
1. Follow the existing code style
2. Write comprehensive tests
3. Update documentation as needed
4. Create small, focused pull requests
5. Follow the project's architecture

## 📞 Contact

For questions or feedback, please open an issue on the GitHub repository.

## 📚 Additional Resources

- [React Documentation](https://react.dev/)
- [Kotlin Documentation](https://kotlinlang.org/docs/home.html)
- [Ktor Documentation](https://ktor.io/docs/welcome.html)
- [Rust Documentation](https://www.rust-lang.org/learn)
- [xterm.js Documentation](https://xtermjs.org/docs/)
- [WebSocket API](https://developer.mozilla.org/en-US/docs/Web/API/WebSockets_API)
- [WebTransport API](https://developer.mozilla.org/en-US/docs/Web/API/WebTransport_API)

---

**Waylon Terminal** - Empowering developers with a modern, high-performance web terminal experience.

⭐ If you find this project useful, please give it a star!