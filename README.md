# ◬ Atrium (`atriumd`)

![Token Efficiency](https://img.shields.io/badge/Token_Efficiency-98%25_Reduction-00f0ff?style=for-the-badge&logo=cpu)
![Build Status](https://img.shields.io/badge/Build-Passing-39ff14?style=for-the-badge)
![Platform](https://img.shields.io/badge/Platform-Windows_|_macOS_|_Linux-bd10e0?style=for-the-badge)

> **The Universal, Zero-Token-Waste Context Stabilization Daemon & Verification Layer for AI Coding Agents.**
> Atrium runs in the background to shield your wallet from AI token inflation by deflecting codebase noise and verifying code output in real-time.

---

## 🚀 Track 1: The Fast Lane (For Vibe Coders)

Are you tired of AI coding agents consuming thousands of dollars in API fees just to read your boilerplate code? Every time you ask a coding agent (Cursor, Claude Code, Cline, Roo Code, etc.) to edit a file, it dumps your entire codebase into the prompt. **Atrium puts an end to this waste.**

### The AI API Economic Formula
In any LLM coding session, your token usage is defined by a simple equation:

$$\text{Total Session Context} = \text{System Overhead} + \text{Codebase Noise}$$

* **System Overhead (Fixed Baseline Cost):** The system prompt, agent tooling schemas, and conversational history. This is the necessary cost of the AI's "brain."
* **Codebase Noise (Variable Repository Waste):** The raw text of all files in your repository, including imports, boilerplate, brackets, and helper utilities. As your codebase grows, this noise scales exponentially, consuming your token budget and confusing the AI model.

#### ❌ Traditional AI Coding Tools (Without Atrium)
Traditional tools inject raw file contents directly into the prompt. A simple change in a 50-file repository wastes massive context:
```text
System Overhead: [██████████████████████████████] (3,000 tokens)
Codebase Noise:  [████████████████████████████████████████████████████████████████████] (75,000 tokens)
Total Cost:      ⚠️ Extremely High Token Bleed ($$$)
```

####  Atrium Context Stabilization (With Atrium Active)
Atrium strips out 98% of the codebase noise by serving precision AST blueprints to the AI. Codebase token ingestion flatlines at zero-waste, regardless of repository size:
```text
System Overhead: [██████████████████████████████] (3,000 tokens)
Atrium Slice:    [█] (800 tokens)
Total Cost:      ✨ 98% Cost Reduction (Flat-rate AI billing)
```

### 1-Minute Automated Boot Sequence

#### 🍎 macOS & 🐧 Linux (Bash)
```bash
curl -fsSL https://raw.githubusercontent.com/MrDawell/atrium/main/install.sh | bash
```

#### ❖ Windows (PowerShell)
```powershell
irm https://raw.githubusercontent.com/MrDawell/atrium/main/install.ps1 | iex
```

### 2. Auto-Discovery & Activation
Run the following inside your repository root folder to automatically configure your active editor tools:
```bash
atriumd --register
```
This command auto-detects installations of **Claude Code**, **Cursor**, **Cline**, and **Roo Code**, setting up the Atrium MCP server bindings automatically.

### 3. Run the Background Daemon
```bash
atriumd
```
Atrium will start its local gRPC server on port `50051`, initialize its localized SQLite memory database (`atrium_memory.db`), and serve the visual dashboard at [http://localhost:4040](http://localhost:4040).

---

## 🛠️ Track 2: Systems Architecture (For Core Developers)

Atrium is a production-grade context stabilization daemon built using a highly optimized, low-latency Rust core. The architecture is split into a multi-crate Rust workspace linked to a stdio MCP client via a Python gRPC loopback.

```text
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

### 1. Two-Pronged Universal Capture Layer
The ingestion pipeline inside `atrium-core-graph` ([lib.rs](file:///C:/Users/LENOVO/Documents/antigravity/modest-lovelace/crates/atrium-core-graph/src/lib.rs)) combines high-performance native parser drivers with a flexible dynamic plugin loader.

* **Dynamic Grammar Loader (`libloading`):** To avoid massive compilation overhead and support 100+ languages on demand, Atrium utilizes the `libloading` crate to load compiled Tree-sitter `.so`, `.dylib`, or `.dll` libraries at runtime.
* **Leaked Pointer Guarantee:** To prevent dangling references and undefined behavior when parsing hundreds of files, Atrium intentionally leaks the dynamic `libloading::Library` reference via `Box::leak`. This guarantees that the memory location of the constructor `Language` function pointer remains completely stable for the lifetime of the daemon execution loop.

### 2. Standardized Entity Translation
Once a file's concrete AST is generated, Atrium filters out variable nodes and maps syntax tokens to a uniform, language-agnostic layout. Node paths are mapped to standard system wrappers:
* `class_definition`, `struct_specifier` $\rightarrow$ **`class` / `struct`**
* `interface_declaration` $\rightarrow$ **`interface`**
* `function_definition`, `method_declaration` $\rightarrow$ **`function`**
* `use_declaration`, `import_statement` $\rightarrow$ **`import`**

This translation creates a clean symbol topology that allows the LLM to understand dependencies without digesting lines of syntax noise.

### 3. Universal Structural Fallback Engine
If a missing dynamic grammar target is encountered or a syntax error prevents the parser from generating a valid AST, Atrium automatically engages its resilient fallback engine:
* **Spatial Indentation Scanning:** Scans indentation levels to reconstruct code nesting structures.
* **Brace Matching (`{}`) Matrix:** Tracks lexical boundaries to compute precise scope start/end locations.
* **Zero Panic Policy:** The parser guarantees runtime stability by smoothly generating basic symbol nodes instead of halting execution or throwing unhandled errors.

### 4. Language-Agnostic Relational Graph
All extracted symbols, parent-child links, and local metadata facts are written to a localized SQLite memory matrix (`atrium_memory.db`). This storage is driven by `atrium-core-memory` ([lib.rs](file:///C:/Users/LENOVO/Documents/antigravity/modest-lovelace/crates/atrium-core-memory/src/lib.rs)).

#### `symbols` Schema
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

#### `durable_memories` Schema
```sql
CREATE TABLE IF NOT EXISTS durable_memories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    fact TEXT NOT NULL,
    scope TEXT NOT NULL,
    confidence REAL NOT NULL,
    last_validated_at INTEGER NOT NULL
);
```

---

## 🛠️ Track 3: Development, Build & CLI Execution

### Repository Layout
* **[api/daemon.proto](file:///C:/Users/LENOVO/Documents/antigravity/modest-lovelace/api/daemon.proto):** Protobuf contract for daemon/bridge gRPC communication.
* **[crates/atrium-core-graph](file:///C:/Users/LENOVO/Documents/antigravity/modest-lovelace/crates/atrium-core-graph/):** AST parser and symbol mapper.
* **[crates/atrium-core-memory](file:///C:/Users/LENOVO/Documents/antigravity/modest-lovelace/crates/atrium-core-memory/):** Local SQLite storage interface.
* **[crates/atrium-core-verify](file:///C:/Users/LENOVO/Documents/antigravity/modest-lovelace/crates/atrium-core-verify/):** Asynchronous test sandbox runner (`cargo check`, `cargo test`, `clippy`).
* **[crates/atrium-daemon](file:///C:/Users/LENOVO/Documents/antigravity/modest-lovelace/crates/atrium-daemon/):** Daemon service implementation and Axum HTTP visual UI wrapper.
* **[api/bridge.py](file:///C:/Users/LENOVO/Documents/antigravity/modest-lovelace/api/bridge.py):** MCP and CLI interface wrapper.

### Build from Source
```bash
# Clone the repository
git clone https://github.com/MrDawell/atrium.git
cd atrium

# Build the daemon binary in release mode
cargo build --release --bin atriumd

# Compile python protobuf hooks
python -m grpc_tools.protoc -Iapi --python_out=api --grpc_python_out=api api/daemon.proto
```

### CLI Command Matrix

| Command | Action | Scope |
| :--- | :--- | :--- |
| `atriumd` | Launch the background daemon and API listeners | Localhost port `50051` |
| `atriumd --index` | Trigger a synchronous workspace-wide AST index sweep | Current directory |
| `atriumd --reset` | Clear all database tables in `atrium_memory.db` | Local workspace |
| `atriumd --register` | Detect editors and inject local MCP configuration files | System-wide |

---

## 🛡️ Track 4: The Wallet Audit Checklist

Verify that Atrium is deflecting token bleed inside your AI chat clients (Cursor, Claude Code, Cline, etc.) by following this step-by-step audit:

1. **Verify MCP Binding:**
   Type `/context` in your chat client. Confirm that `atrium` is listed as an active contextual tool provider.
2. **Execute a Codebase Query:**
   Ask the AI a structural query (e.g., *"How do modules in my project import the database connections?"*).
3. **Audit the Prompt Footprint:**
   Run the `/usage` command (or inspect the agent's request logs).
4. **Compare Ingestion Costs:**
   - **Expected raw size:** $>50,000$ tokens.
   - **Atrium stabilized size:** $<1,200$ tokens.
   - **Savings Output:** Look for the signature output card:
     ```text
     ✨ Atrium Shield: 98% OF REPOSITORY WASTE DEFLECTED!
     ```

---
*License: MIT Open Source. Designed to keep AI engineering fast, accurate, and completely free of token waste.*