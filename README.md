# WSH

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg?style=for-the-badge)]()

> **⚠PROTOTYPE STATUS**: WSH is currently in early development prototype phase. Features and APIs may change significantly.

A modern, fast, and extensible terminal emulator/shell written in Rust. WSH aims to provide a powerful command-line experience with modern features while maintaining excellent performance and memory safety.

##  Features

###  Currently Available (Prototype v0.1.0)

- **Interactive Shell**: Full interactive terminal with command execution
- **Command History**: Navigate through command history with arrow keys
- **Syntax Highlighting**: Colored output and error messages
- **Configuration System**: TOML-based configuration with customizable settings
- **Command Aliases**: Create custom shortcuts for frequently used commands
- **Built-in Commands**: Essential commands like `cd`, `pwd`, `help`, `history`, `alias`, `exit`
- **Smart Parsing**: Advanced command line parsing with quote handling
- **Path Expansion**: Automatic tilde (`~`) expansion to home directory
- **Rich Keyboard Support**: Arrow keys, Home/End, Ctrl+C/D shortcuts
- **Cross-platform**: Works on Linux, macOS, and Windows

###  Planned Features (Future Releases)

- **Piping & Redirection**: `command1 | command2`, `output > file.txt`
- **Job Control**: Background processes, job management
- **Environment Variables**: Full environment variable support
- **Scripting**: WSH script execution with `.wsh` files
- **Plugin System**: Extensible architecture for custom plugins
- **Themes**: Multiple color themes and customizable UI
- **Remote Features**: SSH integration, remote command execution
- **Smart Completion**: Tab completion for commands and paths
- **Debugging Tools**: Built-in debugging and profiling capabilities
- **Performance Monitoring**: Real-time performance metrics

## Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) 1.70.0 or higher
- Git

### Installation

#### From Source (Recommended for development)

```bash
# Clone the repository
git clone https://github.com/Huseynteymurzade28/wsh.git
cd wsh

# Build and install
cargo build --release

# Run
cargo run
```

#### Direct Installation (Future)

```bash
# Via cargo (when published)
cargo install wsh

# Via package managers (planned)
# brew install wsh         # macOS
# apt install wsh          # Ubuntu/Debian
# pacman -S wsh           # Arch Linux
```

## Usage

### Interactive Mode

```bash
# Start interactive shell
wsh

# Welcome message appears
Welcome to WSH - A modern shell written in Rust!
Type 'help' for available commands or 'exit' to quit.
wsh>
```

### Command Line Mode

```bash
# Execute single command
wsh -c "pwd"

# Use custom config file
wsh -f /path/to/config.toml

# Show help
wsh --help
```

### Built-in Commands

| Command                  | Description             | Example             |
| ------------------------ | ----------------------- | ------------------- |
| `cd [path]`              | Change directory        | `cd ~/Documents`    |
| `pwd`                    | Print working directory | `pwd`               |
| `help`                   | Show help message       | `help`              |
| `history`                | Show command history    | `history`           |
| `alias [name] [command]` | Create or show aliases  | `alias ll "ls -la"` |
| `exit`                   | Exit the shell          | `exit`              |

### Keyboard Shortcuts

| Shortcut           | Action                      |
| ------------------ | --------------------------- |
| `↑/↓`              | Navigate command history    |
| `←/→`              | Move cursor in current line |
| `Home/End`         | Jump to line start/end      |
| `Ctrl+C`           | Interrupt/Exit              |
| `Ctrl+D`           | Exit shell                  |
| `Backspace/Delete` | Delete characters           |

## Configuration

WSH uses TOML configuration files for customization.

### Default Config Location

- Linux/macOS: `~/.wsh.toml`
- Windows: `%USERPROFILE%\.wsh.toml`

### Example Configuration

```toml
# ~/.wsh.toml

# Customize prompt (supports {cwd} for current directory)
prompt = "wsh [{cwd}]$ "

# Command history settings
history_size = 1000

# Enable/disable colored output
enable_colors = true

# Command aliases
[aliases]
ll = "ls -la"
la = "ls -A"
l = "ls -CF"
grep = "grep --color=auto"
..  = "cd .."
... = "cd ../.."
home = "cd ~"
```

### Advanced Configuration (Future)

```toml
# Theme settings (planned)
[theme]
primary_color = "green"
error_color = "red"
prompt_color = "blue"

# Plugin settings (planned)
[plugins]
git = { enabled = true, show_branch = true }
docker = { enabled = false }

# Performance settings (planned)
[performance]
max_history_memory = "10MB"
cache_commands = true
```

##  Architecture

WSH is built with a modular architecture for extensibility and maintainability:

```
src/
├── main.rs        # Entry point and CLI argument parsing
├── config.rs      # Configuration management (TOML)
├── shell.rs       # Core shell logic and interactive mode
└── utils.rs       # Utility functions and command parsing
```

### Core Components

1. **CLI Layer** (`main.rs`): Handles command-line arguments and program initialization
2. **Configuration** (`config.rs`): Manages TOML-based configuration loading and saving
3. **Shell Engine** (`shell.rs`): Interactive shell, command execution, and user interface
4. **Utilities** (`utils.rs`): Command parsing, path handling, and helper functions

## 🛠️ Development

### Setting up Development Environment

```bash
# Clone and enter directory
git clone https://github.com/qiral/wsh.git
cd wsh

# Install dependencies
cargo build

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run

# Format code
cargo fmt

# Check for issues
cargo clippy
```

### Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run directly
cargo run

# Run with arguments
cargo run -- -c "pwd"
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```
## 🤝 Contributing

We welcome contributions! Since this is a prototype, there are many opportunities to help:

### How to Contribute

1. **Fork the repository**
2. **Create a feature branch**: `git checkout -b feature/amazing-feature`
3. **Make your changes**
4. **Add tests** if applicable
5. **Commit your changes**: `git commit -m 'Add amazing feature'`
6. **Push to the branch**: `git push origin feature/amazing-feature`
7. **Open a Pull Request**

### Areas for Contribution

- 🐛 **Bug fixes**: Help identify and fix issues
- 🚀 **New features**: Implement planned features
- 📖 **Documentation**: Improve docs and examples
- 🧪 **Testing**: Add unit tests and integration tests
- 🎨 **UI/UX**: Improve user experience and interface
- 🏗️ **Architecture**: Enhance code structure and performance

### Coding Standards

- Follow Rust naming conventions
- Use `cargo fmt` for code formatting
- Run `cargo clippy` for linting
- Write tests for new features
- Update documentation for changes

## 🔧 Dependencies

### Runtime Dependencies

- [`clap`](https://crates.io/crates/clap) - Command line argument parsing
- [`crossterm`](https://crates.io/crates/crossterm) - Cross-platform terminal manipulation
- [`anyhow`](https://crates.io/crates/anyhow) - Error handling
- [`serde`](https://crates.io/crates/serde) + [`toml`](https://crates.io/crates/toml) - Configuration serialization
- [`tokio`](https://crates.io/crates/tokio) - Async runtime (for future features)

### Development Dependencies

- [`env_logger`](https://crates.io/crates/env_logger) - Logging framework
