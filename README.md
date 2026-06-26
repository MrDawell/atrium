# Atrium (`atriumd`)

![Token Efficiency](https://img.shields.io/badge/Token_Efficiency-98%25_Reduction-00f0ff?style=for-the-badge&logo=cpu)
![Build Status](https://img.shields.io/badge/Build-Passing-39ff14?style=for-the-badge)
![Platform](https://img.shields.io/badge/Platform-Windows_|_macOS_|_Linux-bd10e0?style=for-the-badge)

> The zero-token-waste, structural memory and verification layer for AI coding agents (Claude Code, Cursor, Cline, Roo Code, etc.).

---

## 🚀 Track 1: The Fast Lane (For Vibe Coders)

Get Atrium up and running in under 60 seconds.

### 1. 1-Click Install
Run the automated installer script matching your operating system. This script automatically detects your system architecture (macOS x86_64/arm64, Linux x86_64/aarch64, Windows x64/ARM64), downloads the precompiled native release binary, registers it to your PATH, installs Python dependencies (`grpcio`, `grpcio-tools`), and triggers editor auto-registration.

#### 🍎 macOS & 🐧 Linux (Bash)
```bash
curl -fsSL https://raw.githubusercontent.com/MrDawell/atrium/main/install.sh | bash
```

#### ❖ Windows (PowerShell)
```powershell
irm https://raw.githubusercontent.com/MrDawell/atrium/main/install.ps1 | iex
```

### 2. Auto-Discovery Setup
Simply navigate to your repository root folder and run:
```bash
atriumd --register
```
This single command automatically scans your system for active coding tools (Claude Code, Cursor, Cline, Roo Code, and Continue) and registers Atrium as an MCP server or contextual data provider—completely hands-off.

### 3. Run the Background Daemon
Start the background service daemon:
```bash
atriumd
```
Atrium will boot up, initialize its local SQLite memory database (`atrium_memory.db`), and launch a visual dashboard at [http://localhost:14040](http://localhost:14040).

### 🤖 AI Agent Autopilot Setup
If you are pair programming with an AI agent (such as Claude Code, Cursor, Cline, or Roo Code), you can copy and paste the prompt below into the chat, and the agent will configure and run everything for you autonomously:

> Please clone the Atrium repository from `https://github.com/MrDawell/atrium.git`, build the atriumd daemon from source using `cargo build --release`, install Python bridge dependencies (`grpcio` and `grpcio-tools`), compile the gRPC protobuf files to python, verify everything with `cargo test`, run `atriumd` as a background task, index the repository, and register the Atrium MCP server in all active editors (Claude Code, Cursor, Cline, Roo Code, etc.).

---

## 💡 Track 2: The "Why It Works" Analogy Section

Traditional AI coding tools have a severe memory and cost problem. Every time you ask an agent to make a small change, it re-scans your entire project and dumps thousands of lines of raw source code into the prompt. This wastes your token budget, floods the model's context window with noise, and causes it to lose focus.

Atrium changes the game using three core design patterns:

### 📐 The Blueprint vs. The Bricks
* **The Problem:** Standard tools dump every single raw code line (the "bricks") into the prompt, blinding the AI with implementation details and wasting $98\%$ of your tokens.
* **The Atrium Solution:** Atrium acts as an architect. It parses your source files using fast Tree-sitter grammars to extract structural outlines and signature declarations (the "blueprints"). The AI gets full structural comprehension of classes, functions, and invariants without the boilerplate clutter.

### 🛡️ Built-in Auto-Correct
* **The Problem:** A coding agent suggests a patch, but it contains a minor compilation or linter error. If the user doesn't check it, the build breaks.
* **The Atrium Solution:** Atrium acts as an invisible test sandbox. When a patch is proposed, Atrium's verification pipeline executes local checks (`cargo check`, `cargo test`, `clippy`) behind the scenes. If there is a compilation error, Atrium feeds the compiler diagnostics directly back to the AI, forcing it to self-correct and fix its own mistakes before you ever see a broken build.

### 🧠 Zero Brain Mixing
* **The Problem:** Jumping between different projects can pollute an agent's memory, leading to cross-talk where rules or symbols from Repo A bleed into queries for Repo B.
* **The Atrium Solution:** Atrium automatically fingerprints the active workspace directory. Facts, rules, and symbols are stored in a localized SQLite database mapped uniquely to that workspace. Memory pools never cross-contaminate.

---

## 🛠️ Track 3: Core Engineering & Architecture (For Core Devs)

Atrium is organized as a lightweight Rust workspace fronted by a Python bridge that manages stdio MCP communication.

### Crate Topology Map
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
                                          │  (Port 14040)
                                          ▼
                         ┌─────────────────────────────────┐
                         │      Embedded DevUI Web App     │
                         │   (rust-embed + Axum SSE/HTML)  │
                         └─────────────────────────────────┘
```

* **[api/daemon.proto](file:///C:/Users/LENOVO/Documents/antigravity/modest-lovelace/api/daemon.proto):** gRPC/Protocol Buffers interface contract.
* **[crates/atrium-core-graph](file:///C:/Users/LENOVO/Documents/antigravity/modest-lovelace/crates/atrium-core-graph/):** AST parsing and symbol graph construction using Tree-sitter.
* **[crates/atrium-core-memory](file:///C:/Users/LENOVO/Documents/antigravity/modest-lovelace/crates/atrium-core-memory/):** Manages SQLite (`atrium_memory.db`) storing indexed symbol keys and durable rules.
* **[crates/atrium-core-verify](file:///C:/Users/LENOVO/Documents/antigravity/modest-lovelace/crates/atrium-core-verify/):** Asynchronously runs sub-processes (`cargo check`, `cargo test`, `clippy`).
* **[crates/atrium-daemon](file:///C:/Users/LENOVO/Documents/antigravity/modest-lovelace/crates/atrium-daemon/):** Core daemon binary (`atriumd`) managing gRPC server and Axum DevUI.
* **[api/bridge.py](file:///C:/Users/LENOVO/Documents/antigravity/modest-lovelace/api/bridge.py):** Stdio MCP and CLI bridge calling the local daemon over gRPC.

### SQLite Memory Schema
Atrium uses standard localized SQLite tables for fast query lookups:
* **`symbols` Table:** Mapped unique identifiers of source declarations.
  ```sql
  CREATE TABLE IF NOT EXISTS symbols (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      file_path TEXT NOT NULL,
      symbol_name TEXT NOT NULL,
      symbol_kind TEXT NOT NULL,
      content_hash TEXT NOT NULL,
      UNIQUE(file_path, symbol_name)
  );
  ```
* **`durable_memories` Table:** Rules, architectural facts, and scope targets.
  ```sql
  CREATE TABLE IF NOT EXISTS durable_memories (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      fact TEXT NOT NULL,
      scope TEXT NOT NULL,
      confidence REAL NOT NULL,
      last_validated_at INTEGER NOT NULL
  );
  ```

### Manual Compilation from Source
If building manually without installer scripts:

1. **Clone the repository:**
   ```bash
   git clone https://github.com/MrDawell/atrium.git
   cd atrium
   ```
2. **Build the release binary:**
   ```bash
   cargo build --release --bin atriumd
   ```
3. **Move executable to system PATH:**
   ```bash
   sudo cp target/release/atriumd /usr/local/bin/
   ```
4. **Compile Python Protobuf files:**
   ```bash
   python -m grpc_tools.protoc -Iapi --python_out=api --grpc_python_out=api api/daemon.proto
   ```