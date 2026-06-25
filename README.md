# Atrium (atriumd)
The zero-token-waste, structural memory and verification layer for AI coding agents.

Current AI coding tools (like standard IDE wrappers or terminal agents) have a severe short-term memory problem. Every time you ask them to make a surgical edit, they re-scan or dump giant blocks of your repository into a text blob. This wastes your token budget, floods context windows with noise, and breaks down entirely on production-scale codebases.

Atrium fixes this. It is a headless, lightning-fast background daemon written in Rust that acts as a durable, code-aware brain for tools like Claude Code and AI orchestration frameworks. It maintains an incremental, language-agnostic code graph and pairs it with localized, persistent memory tables.

Instead of reading your whole repo, AI agents simply ask Atrium for a precision context slice.

---

## 🚀 Track 1: Quick Start (For Vibe Coders)

### 1. Install Atrium Instantly
Run the automated shell installer to download the pre-compiled native binary of `atriumd` matching your OS and architecture, and automatically initialize Python requirements:
```bash
curl -fsSL https://raw.githubusercontent.com/MrDawell/atrium/main/install.sh | bash
```

### 2. Configure Claude Code as an MCP Server
To integrate Atrium's deep context engine directly into your daily terminal workflow, register the bridge script inside your local Claude MCP configuration file (typically at `~/.config/claude/mcp.json` or your system equivalent):

```json
{
  "mcpServers": {
    "atrium": {
      "command": "python",
      "args": ["/absolute/path/to/atrium/api/bridge.py", "--mcp"]
    }
  }
}
```

### 3. Basic Terminal Loop
Start the background daemon once inside any repository root folder:
```bash
$ atriumd
# [Atrium] Daemon active and listening on 127.0.0.1:50051
# [Atrium] Initialized localized database file: atrium_memory.db
```

Interact with your code using the bridge CLI or let your connected MCP tool handle requests natively:
```bash
# Build the initial structural AST graph and file hashes
$ python api/bridge.py --action index --path .

# Inject a durable, long-term project-scoped architectural rule
$ python api/bridge.py --action add-fact --fact "Always use standard library errors, do not use the anyhow crate" --scope "global"

# Request a precision prompt context bundle for a task
$ python api/bridge.py --action get-context --task "Fix routing handlers inside src/auth.rs"
```

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
│                                     atriumd DAEMON                                      │
│                                                                                         │
│  ┌─────────────────────────┐   ┌─────────────────────────┐   ┌────────────────────────┐  │
│  │   atrium-core-graph     │   │   atrium-core-memory    │   │   atrium-core-verify   │  │
│  │                         │   │                         │   │                        │  │
│  │ • Tree-sitter Parser    │──>│ • SQLite Database       │──>│ • Asynchronous Process │  │
│  │ • Fast AST Extractor    │   │ • Durable Fact Store    │   │   Runner               │  │
│  │ • Language Grammar Map  │   │ • Unique Symbol Indices │   │ • cargo test/check/lint│  │
│  └─────────────────────────┘   └─────────────────────────┘   └────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

### Workspace Layout
Atrium is organized as a Cargo workspace:
- [api/daemon.proto](file:///C:/Users/LENOVO/Documents/antigravity/modest-lovelace/api/daemon.proto): Protocol Buffers interface contract.
- [crates/atrium-core-graph](file:///C:/Users/LENOVO/Documents/antigravity/modest-lovelace/crates/atrium-core-graph/): Handles Tree-sitter AST parsing and symbol graph construction.
- [crates/atrium-core-memory](file:///C:/Users/LENOVO/Documents/antigravity/modest-lovelace/crates/atrium-core-memory/): Manages persistent SQLite storage (`atrium_memory.db`) for files, hashes, symbols, and durable facts/rules.
- [crates/atrium-core-verify](file:///C:/Users/LENOVO/Documents/antigravity/modest-lovelace/crates/atrium-core-verify/): Runs tests, linters, and checkers asynchronously in sub-processes.
- [crates/atrium-daemon](file:///C:/Users/LENOVO/Documents/antigravity/modest-lovelace/crates/atrium-daemon/): The central gRPC IPC server (`atriumd`) coordinating memory and verifier layers.
- [api/bridge.py](file:///C:/Users/LENOVO/Documents/antigravity/modest-lovelace/api/bridge.py): Python client bridge supporting CLI utilities and Model Context Protocol (MCP) server execution.

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

## ⚖️ License
Distributed under the MIT License. See `LICENSE` for more information.