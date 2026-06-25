# Atrium (`atriumd`)

> The zero-token-waste, structural memory and verification layer for AI coding agents.

Current AI coding tools (like standard IDE wrappers or terminal agents) have a severe short-term memory problem. Every time you ask them to make a surgical edit, they re-scan or dump giant blocks of your repository into a text blob. This wastes your token budget, floods context windows with noise, and breaks down entirely on production-scale codebases.

**Atrium fixes this.** It is a headless, lightning-fast background daemon written in Rust that acts as a durable, code-aware brain for tools like Claude Code and AI orchestration frameworks. It maintains an incremental, language-agnostic code graph, tracks active architectural guidelines, and pairs them with localized, persistent memory tables.

Instead of reading your whole repo, AI agents simply ask Atrium for a precision context slice.

---

## ✨ Core Features

### 📊 DevUI Dashboard (`http://localhost:4040`)
When `atriumd` boots up, it automatically spins up a lightweight embedded web server serving a gorgeous, glassmorphic visual dashboard.
* **Code Graph Topology:** Dynamic canvas rendering of parsed source files, modules, and their relational links.
* **Live Memory Stream:** High-performance Server-Sent Events (SSE) log stream showing real-time indexing, facts querying, and patch verification events.
* **Token Savings Metrics:** Dynamic calculation of token efficiency and reduction over time.
* **Active Invariants Board:** Live display of registered architectural guidelines and rule scopes.

### 🔌 Universal IDE/CLI integration (`atriumd --register`)
A single CLI hook automatically scans and registers Atrium as an MCP server or contextual data provider in your favorite environments:
* **Anthropic Claude Code CLI:** Integrates Atrium stdio MCP server directly.
* **Cursor IDE:** Automates active MCP provider registration.
* **Cline & Roo Code:** Initializes `cline_mcp_settings.json` integrations.
* **Continue.dev:** Registers Atrium as an active contextual data provider in `~/.continue/config.json`.
* **Claude Desktop:** Registers Atrium into the desktop client workspace settings.

### 🔄 Asynchronous Self-Updater (`atriumd --update`)
* **Silent Background Check:** On daemon startup, a background worker queries the GitHub Releases API. If a new production tag is available, it downloads and performs a safe, in-place binary overwrite.
* **Manual Upgrade Override:** Forces verification against the remote release repository with real-time update stream progress.
* **Locked Binary Overwrite:** Uses an executable rename trick on Windows to successfully replace the running program without stopping operations.

---

## 🚀 Track 1: Quick Start (For Vibe Coders)

### 1. Install Atrium Instantly
Run the automated installer script matching your operating system. This automatically detects your system architecture (macOS x86_64/arm64, Linux x86_64/aarch64, Windows x64/ARM64), downloads the precompiled release binary, registers it to your PATH, installs Python bridge dependencies (`grpcio`, `grpcio-tools`), and triggers editor auto-registration.

#### 🍎 macOS & 🐧 Linux (Bash)
```bash
curl -fsSL https://raw.githubusercontent.com/MrDawell/atrium/main/install.sh | bash
```

#### ❖ Windows (PowerShell)
```powershell
irm https://raw.githubusercontent.com/MrDawell/atrium/main/install.ps1 | iex
```

### 2. Run the Background Daemon
Start the background daemon inside any repository root folder:
```bash
$ atriumd
# [Atrium] Daemon active and listening on 127.0.0.1:50051
# [Atrium] Durable memory layer initialized at 'atrium_memory.db'
# [Atrium] Visual DevUI dashboard starting on http://localhost:4040
```

### 3. Open the DevUI Visual Dashboard
Simply open [http://localhost:4040](http://localhost:4040) in your web browser to monitor memory pools, active outlines, and live metrics stream visually.

---

## 🛠️ Track 2: Building From Source (For Core Developers)

### Architecture Overview
```
               ┌────────────────────────────────────────────────────────┐
               │                    TERMINAL INTERACTION                │
               │          (Claude Code CLI / AI Agent Frameworks)       │
               └───────────────────────────┬────────────────────────────┘
                                            │
                                   MCP / Python Hooks
                                            │
                                            ▼
               ┌────────────────────────────────────────────────────────┐
               │                       api/bridge.py                    │
               │           (Dual-Mode CLI & Stdio MCP Server)           │
               └───────────────────────────┬────────────────────────────┘
                                            │
                                       gRPC Loopback
                                            │
                                            ▼
 ┌─────────────────────────────────────────────────────────────────────────────────────────┐
 │                                     atriumd DAEMON (Port 50051)                         │
 │                                                                                         │
 │  ┌─────────────────────────┐   ┌─────────────────────────┐   ┌────────────────────────┐  │
 │  │   atrium-core-graph     │   │   atrium-core-memory    │   │   atrium-core-verify   │  │
 │  │                         │   │                         │   │                        │  │
 │  │ • Tree-sitter Parser    │──>│ • SQLite Database       │──>│ • Asynchronous Process │  │
 │  │ • Fast AST Extractor    │   │ • Durable Fact Store    │   │   Runner               │  │
 │  │ • Language Grammar Map  │   │ • Unique Symbol Indices │   │ • cargo test/check/lint│  │
 │  └─────────────────────────┘   └─────────────────────────┘   └────────────────────────┘  │
 └───────────────────────────────────────┬─────────────────────────────────────────────────┘
                                         │  (Port 4040)
                                         ▼
                        ┌─────────────────────────────────┐
                        │      Embedded DevUI Web App     │
                        │   (rust-embed + Axum SSE/HTML)  │
                        └─────────────────────────────────┘
```

### Workspace Layout
Atrium is organized as a Cargo workspace:
* [api/daemon.proto](file:///C:/Users/LENOVO/Documents/antigravity/modest-lovelace/api/daemon.proto): Protocol Buffers interface contract.
* [crates/atrium-core-graph](file:///C:/Users/LENOVO/Documents/antigravity/modest-lovelace/crates/atrium-core-graph/): Handles Tree-sitter AST parsing and symbol graph construction.
* [crates/atrium-core-memory](file:///C:/Users/LENOVO/Documents/antigravity/modest-lovelace/crates/atrium-core-memory/): Manages persistent SQLite storage (`atrium_memory.db`) for files, hashes, symbols, and durable facts/rules.
* [crates/atrium-core-verify](file:///C:/Users/LENOVO/Documents/antigravity/modest-lovelace/crates/atrium-core-verify/): Runs tests, linters, and checkers asynchronously in sub-processes.
* [crates/atrium-daemon](file:///C:/Users/LENOVO/Documents/antigravity/modest-lovelace/crates/atrium-daemon/): The central gRPC IPC server (`atriumd`) coordinating memory, verifier, and DevUI layers.
* [api/bridge.py](file:///C:/Users/LENOVO/Documents/antigravity/modest-lovelace/api/bridge.py): Python client bridge supporting CLI utilities and Model Context Protocol (MCP) server execution.

### Building manually from source
Ensure you have the Rust toolchain installed (via [rustup](https://rustup.rs/)) and a Protocol Buffers compiler (`protoc`) in your system path.

1. **Clone the repository:**
   ```bash
   git clone https://github.com/MrDawell/atrium.git
   cd atrium
   ```
2. **Build the Rust workspace binaries:**
   ```bash
   cargo build --release --bin atriumd
   ```
3. **Move the executable to your binary path:**
   ```bash
   sudo cp target/release/atriumd /usr/local/bin/
   ```
4. **Compile the Python bridge protobuf files manually:**
   ```bash
   python -m grpc_tools.protoc -Iapi --python_out=api --grpc_python_out=api api/daemon.proto
   ```

---

## 🎛️ Command Line Interface

`atriumd` accepts flags to manage registrations and software updates directly:

```bash
# Start the background service daemon (default)
atriumd

# Automatically scan IDE folders and register the Atrium MCP server
atriumd --register

# Register with a custom location for the python bridge
atriumd --register --bridge-path /path/to/atrium/api/bridge.py

# Force check GitHub Releases and upgrade the binary immediately
atriumd --update
```

---

## ⚖️ License
Distributed under the MIT License. See `LICENSE` for more information.