use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::broadcast;
use tonic::{transport::Server, Request, Response, Status};
use axum::response::IntoResponse;

pub mod daemon {
    tonic::include_proto!("atrium.daemon");
}

use daemon::daemon_service_server::{DaemonService, DaemonServiceServer};
use daemon::{
    AddFactRequest, AddFactResponse, ContextFile, ContextRequest, ContextResponse, IndexRequest,
    IndexResponse, VerifyRequest, VerifyResponse,
};

// Import workspace crates
use atrium_core_graph::CodeParser;
use atrium_core_memory::MemoryStore;
use atrium_core_verify::PatchVerifier;

// Web assets compiled directly into the binary
#[derive(rust_embed::RustEmbed)]
#[folder = "../../ui/"]
struct Assets;

pub struct AtriumDaemon {
    store: Arc<MemoryStore>,
    tx: broadcast::Sender<String>,
}

fn visit_dirs(dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                // Avoid traversing target build directories or hidden version control folders
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name == "target" || name.starts_with('.') {
                        continue;
                    }
                }
                visit_dirs(&path, files)?;
            } else if path.extension().map_or(false, |ext| ext == "rs") {
                files.push(path);
            }
        }
    }
    Ok(())
}

fn calculate_hash(content: &str) -> String {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[tonic::async_trait]
impl DaemonService for AtriumDaemon {
    async fn index_repository(
        &self,
        request: Request<IndexRequest>,
    ) -> Result<Response<IndexResponse>, Status> {
        let req = request.into_inner();
        let repo_path = Path::new(&req.repo_path);

        if !repo_path.exists() {
            return Err(Status::invalid_argument("Repository path does not exist"));
        }

        println!("Atriumd: Indexing repository at {:?}", repo_path);

        let mut files = Vec::new();
        if let Err(e) = visit_dirs(repo_path, &mut files) {
            return Err(Status::internal(format!(
                "Failed to scan repository directory: {}",
                e
            )));
        }

        let mut total_symbols = 0;
        for file in &files {
            let content = match std::fs::read_to_string(file) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Warning: Failed to read file {:?}: {}", file, e);
                    continue;
                }
            };

            let hash = calculate_hash(&content);
            let symbols = match CodeParser::parse_source(&content) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Warning: Failed to parse AST for file {:?}: {}", file, e);
                    continue;
                }
            };

            total_symbols += symbols.len() as u64;

            // Make the path relative to the repo root to store clean keys
            let relative_path = file
                .strip_prefix(repo_path)
                .unwrap_or(file)
                .to_string_lossy()
                .to_string();

            if let Err(e) = self.store.store_symbols(&relative_path, &hash, &symbols) {
                return Err(Status::internal(format!("Database write failure: {}", e)));
            }
        }

        // Add a global fact indicating when this repository was indexed
        let repo_name = repo_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        let fact_msg = format!(
            "Repository {} indexed. Found {} files containing {} symbols.",
            repo_name,
            files.len(),
            total_symbols
        );
        if let Err(e) = self.store.add_durable_fact(&fact_msg, "global", 1.0) {
            eprintln!("Warning: Failed to add index run fact: {}", e);
        }

        // Emit real-time DevUI dashboard metric event
        let event = serde_json::json!({
            "event": "index",
            "files": files.len(),
            "symbols": total_symbols
        }).to_string();
        let _ = self.tx.send(event);

        Ok(Response::new(IndexResponse {
            success: true,
            message: format!("Successfully indexed {} Rust source files.", files.len()),
            indexed_symbols: total_symbols,
        }))
    }

    async fn get_context_for_task(
        &self,
        request: Request<ContextRequest>,
    ) -> Result<Response<ContextResponse>, Status> {
        let req = request.into_inner();

        // 1. Determine focus files. If none specified, pull all indexed files in the store.
        let files_to_query = if req.focus_files.is_empty() {
            match self.store.get_indexed_files() {
                Ok(f) => f,
                Err(e) => return Err(Status::internal(format!("Database query error: {}", e))),
            }
        } else {
            req.focus_files.clone()
        };

        // 2. Fetch relevant facts (global ones first)
        let mut facts = self.store.get_relevant_facts("global").unwrap_or_default();

        let mut context_files = Vec::new();

        // 3. For each file, fetch file-specific facts and symbol signatures to build high-signal context
        for file_path in &files_to_query {
            // Fetch any rules/invariants associated with this file
            if let Ok(file_facts) = self.store.get_relevant_facts(file_path) {
                facts.extend(file_facts);
            }

            // Fetch parsed symbols to construct a structural outline snippet
            let symbols = match self.store.get_symbols_for_file(file_path) {
                Ok(syms) => syms,
                Err(_) => vec![],
            };

            let mut snippet = format!("// Outline of file: {}\n", file_path);
            if symbols.is_empty() {
                snippet.push_str("// (No symbols indexed in database)\n");
            } else {
                for (name, kind) in symbols {
                    snippet.push_str(&format!("//   - {} ({})\n", name, kind));
                }
            }

            context_files.push(ContextFile {
                path: file_path.clone(),
                snippet,
                relevance_score: 1.0, // Default relevance
            });
        }

        // 4. Aggregate facts into a reasoning summary
        let reasoning = if facts.is_empty() {
            "No matching durable facts or guidelines found in MemoryStore.".to_string()
        } else {
            let mut summary = "Relevant Project Rules & Architectural Invariants:\n".to_string();
            for (idx, fact) in facts.iter().enumerate() {
                summary.push_str(&format!("{}. {}\n", idx + 1, fact));
            }
            summary
        };

        // Emit context event for DevUI dashboard
        let event = serde_json::json!({
            "event": "context",
            "task": req.task_description,
            "files": files_to_query
        }).to_string();
        let _ = self.tx.send(event);

        Ok(Response::new(ContextResponse {
            files: context_files,
            reasoning,
        }))
    }

    async fn verify_patch(
        &self,
        request: Request<VerifyRequest>,
    ) -> Result<Response<VerifyResponse>, Status> {
        let req = request.into_inner();
        println!("Atriumd: Triggering async verification pipeline for patch...");

        let verifier = PatchVerifier::new();
        let ver_res = verifier.verify_patch(&req.patch_content).await.map_err(|e| {
            Status::internal(format!("Failed to run verification framework: {}", e))
        })?;

        // Emit verification outcome to DevUI
        let event = serde_json::json!({
            "event": "verify",
            "is_valid": ver_res.is_valid
        }).to_string();
        let _ = self.tx.send(event);

        Ok(Response::new(VerifyResponse {
            is_valid: ver_res.is_valid,
            linter_output: ver_res.linter_output,
            test_output: ver_res.test_output,
            compile_errors: ver_res.compile_errors,
        }))
    }

    async fn add_durable_fact(
        &self,
        request: Request<AddFactRequest>,
    ) -> Result<Response<AddFactResponse>, Status> {
        let req = request.into_inner();
        println!(
            "Atriumd: Adding durable fact: '{}' for scope: '{}'",
            req.fact, req.scope
        );

        if let Err(e) = self
            .store
            .add_durable_fact(&req.fact, &req.scope, req.confidence)
        {
            return Err(Status::internal(format!(
                "Database fact insertion failure: {}",
                e
            )));
        }

        // Emit fact registration event to DevUI
        let event = serde_json::json!({
            "event": "fact",
            "fact": req.fact,
            "scope": req.scope
        }).to_string();
        let _ = self.tx.send(event);

        Ok(Response::new(AddFactResponse {
            success: true,
            message: "Fact successfully registered in localized MemoryStore.".into(),
        }))
    }
}

// -------------------------------------------------------------
// IDE MCP Auto-Registration Engine
// -------------------------------------------------------------
fn register_mcp_to_file(
    path: &Path,
    bridge_path: &str,
    service_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if !path.parent().map_or(false, |p| p.exists()) {
        println!("  - {} folder not detected, skipping.", service_name);
        return Ok(());
    }

    let mut json_data = if path.exists() {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str::<serde_json::Value>(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if !json_data.is_object() {
        json_data = serde_json::json!({});
    }

    if json_data.get("mcpServers").is_none() {
        if let Some(map) = json_data.as_object_mut() {
            map.insert("mcpServers".to_string(), serde_json::json!({}));
        }
    }

    if let Some(mcp_servers) = json_data
        .get_mut("mcpServers")
        .and_then(|m| m.as_object_mut())
    {
        mcp_servers.insert(
            "atrium".to_string(),
            serde_json::json!({
                "command": "python",
                "args": [bridge_path, "--mcp"]
            }),
        );
    }

    let formatted = serde_json::to_string_pretty(&json_data)?;
    std::fs::write(path, formatted)?;
    println!(
        "  - Successfully registered Atrium in {} configuration.",
        service_name
    );
    Ok(())
}

fn register_all(bridge_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Registering Atrium bridge at: {}", bridge_path);

    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_default();

    let appdata = std::env::var("APPDATA").unwrap_or_default();

    // 1. Claude Desktop config path
    let claude_desktop_path = if !appdata.is_empty() {
        PathBuf::from(&appdata)
            .join("Claude")
            .join("claude_desktop_config.json")
    } else {
        PathBuf::from(&home).join("Library/Application Support/Claude/claude_desktop_config.json")
    };
    register_mcp_to_file(&claude_desktop_path, bridge_path, "Claude Desktop")?;

    // 2. Claude Code CLI config path
    let claude_code_path = PathBuf::from(&home).join(".config/claude/mcp.json");
    register_mcp_to_file(&claude_code_path, bridge_path, "Claude Code")?;

    // 3. Cursor config path
    let cursor_path = if !appdata.is_empty() {
        PathBuf::from(&appdata).join("Cursor/User/globalStorage/cursor-vip/config.json")
    } else {
        PathBuf::from(&home)
            .join("Library/Application Support/Cursor/User/globalStorage/cursor-vip/config.json")
    };
    register_mcp_to_file(&cursor_path, bridge_path, "Cursor")?;

    // 4. Cline & Roo Code settings
    let cline_path = if !appdata.is_empty() {
        PathBuf::from(&appdata).join("Code/User/globalStorage/saoudrizwan.claude-dev/settings/cline_mcp_settings.json")
    } else {
        PathBuf::from(&home).join("Library/Application Support/Code/User/globalStorage/saoudrizwan.claude-dev/settings/cline_mcp_settings.json")
    };
    register_mcp_to_file(&cline_path, bridge_path, "Cline / Roo-Code")?;

    // 5. Continue.dev configuration
    let continue_path = PathBuf::from(&home).join(".continue/config.json");
    register_mcp_to_file(&continue_path, bridge_path, "Continue.dev")?;

    Ok(())
}

// -------------------------------------------------------------
// Asynchronous Remote Self-Updater Engine
// -------------------------------------------------------------
const CURRENT_VERSION: &str = "0.1.0";

fn get_latest_github_release() -> Result<String, Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        let output = std::process::Command::new("powershell")
            .args(["-Command", "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; (Invoke-RestMethod -Uri 'https://api.github.com/repos/MrDawell/atrium/releases/latest') | ConvertTo-Json -Depth 5"])
            .output()?;
        if !output.status.success() {
            return Err("Failed to query github releases via PowerShell".into());
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
    #[cfg(not(windows))]
    {
        let output = std::process::Command::new("curl")
            .args(["-fsSL", "https://api.github.com/repos/MrDawell/atrium/releases/latest"])
            .output()?;
        if !output.status.success() {
            return Err("Failed to query github releases via curl".into());
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

fn perform_update(force: bool) -> Result<(), Box<dyn std::error::Error>> {
    if force {
        println!("Checking for updates on GitHub Releases...");
    }

    let json_str = match get_latest_github_release() {
        Ok(s) => s,
        Err(e) => {
            if force {
                return Err(format!("Update check failed: {}", e).into());
            }
            return Ok(()); // Background checks fail silently
        }
    };

    let release: serde_json::Value = serde_json::from_str(&json_str)?;
    let tag_name = release
        .get("tag_name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();

    if tag_name.is_empty() {
        if force {
            println!("Could not read remote tag version from GitHub Releases.");
        }
        return Ok(());
    }

    let remote_version = tag_name.strip_prefix('v').unwrap_or(tag_name);

    if remote_version != CURRENT_VERSION {
        println!(
            "Atrium: New release found: {} (Current: {}). Upgrading...",
            tag_name, CURRENT_VERSION
        );

        let os = if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "macos") {
            "macos"
        } else {
            "linux"
        };
        let arch = if cfg!(target_arch = "x86_64") {
            "x64"
        } else {
            "arm64"
        };
        let ext = if cfg!(target_os = "windows") {
            ".exe"
        } else {
            ""
        };
        let binary_filename = format!("atriumd-{}-{}{}", os, arch, ext);
        let download_url = format!(
            "https://github.com/MrDawell/atrium/releases/download/{}/{}",
            tag_name, binary_filename
        );

        let current_exe = std::env::current_exe()?;
        let temp_dir = std::env::temp_dir();
        let download_target = temp_dir.join(format!("atriumd_upgrade_tmp{}", ext));

        println!("Downloading upgrade asset from {}...", download_url);

        #[cfg(windows)]
        {
            let download_cmd = format!(
                "Invoke-WebRequest -Uri '{}' -OutFile '{}' -UseBasicParsing",
                download_url,
                download_target.to_string_lossy()
            );
            let status = std::process::Command::new("powershell")
                .args(["-Command", &download_cmd])
                .status()?;
            if !status.success() {
                return Err("Upgrade download failed".into());
            }
        }
        #[cfg(not(windows))]
        {
            let status = std::process::Command::new("curl")
                .args([
                    "-fsSL",
                    "-o",
                    &download_target.to_string_lossy().to_string(),
                    &download_url,
                ])
                .status()?;
            if !status.success() {
                return Err("Upgrade download failed".into());
            }
        }

        // Apply executable rename trick to replace running binary
        println!("Applying binary updates to {:?}...", current_exe);
        let old_backup = current_exe.with_extension(format!("exe.old_{}", remote_version));

        // Rename current to backup, copy new download target to current
        std::fs::rename(&current_exe, &old_backup)?;
        std::fs::copy(&download_target, &current_exe)?;

        // Ensure new executable is executable on Unix
        #[cfg(not(windows))]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&current_exe)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&current_exe, perms)?;
        }

        println!("Successfully updated binary to version {}. Changes will apply on next restart.", tag_name);
    } else {
        if force {
            println!(
                "Atrium is already up to date (Version {}).",
                CURRENT_VERSION
            );
        }
    }

    Ok(())
}

// -------------------------------------------------------------
// Axum Web Server & DevUI SSE Service
// -------------------------------------------------------------
async fn static_file_handler(uri: axum::http::Uri) -> impl axum::response::IntoResponse {
    let mut path = uri.path().trim_start_matches('/').to_string();
    if path.is_empty() {
        path = "index.html".to_string();
    }

    match Assets::get(&path) {
        Some(content) => {
            let mime = mime_guess::from_path(&path).first_or_octet_stream();
            (
                [(axum::http::header::CONTENT_TYPE, mime.as_ref())],
                content.data.to_vec(),
            )
                .into_response()
        }
        None => (axum::http::StatusCode::NOT_FOUND, "Not Found").into_response(),
    }
}

async fn sse_handler(
    axum::Extension(tx): axum::Extension<broadcast::Sender<String>>,
) -> axum::response::sse::Sse<impl futures_util::stream::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>>
{
    let mut rx = tx.subscribe();
    let stream = async_stream::stream! {
        while let Ok(msg) = rx.recv().await {
            yield Ok(axum::response::sse::Event::default().data(msg));
        }
    };
    axum::response::sse::Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}

// -------------------------------------------------------------
// Main Entrypoint
// -------------------------------------------------------------
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    // 1. Process --register Hook Action
    if args.iter().any(|arg| arg == "--register") {
        let bridge_path = args
            .iter()
            .position(|arg| arg == "--bridge-path")
            .and_then(|idx| args.get(idx + 1))
            .map(|s| s.as_str())
            .unwrap_or("api/bridge.py");

        let abs_path = std::fs::canonicalize(bridge_path)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| bridge_path.to_string());

        if let Err(e) = register_all(&abs_path) {
            eprintln!("Error: IDE registry integration failed: {}", e);
            std::process::exit(1);
        }
        println!("IDE and CLI registrations completed successfully.");
        std::process::exit(0);
    }

    // 2. Process --update Manual Upgrade Action
    if args.iter().any(|arg| arg == "--update") {
        if let Err(e) = perform_update(true) {
            eprintln!("Error: Update check failed: {}", e);
            std::process::exit(1);
        }
        std::process::exit(0);
    }

    // Spawn startup silent background upgrade check
    tokio::spawn(async {
        if let Err(e) = perform_update(false) {
            eprintln!("Background upgrade check warning: {}", e);
        }
    });

    // 3. Start Atrium Server Engine
    let grpc_addr = "127.0.0.1:50051".parse()?;
    let http_addr = "127.0.0.1:4040".parse()?;

    // Initialize standard single-file SQLite database
    let db_path = "atrium_memory.db";
    let memory_store = Arc::new(MemoryStore::new(db_path)?);
    println!("Atriumd: Durable memory layer initialized at '{}'", db_path);

    // Setup SSE Broadcast Channel
    let (tx, _rx) = broadcast::channel::<String>(100);

    let daemon_service = AtriumDaemon {
        store: memory_store,
        tx: tx.clone(),
    };

    // Spin up Axum HTTP Background Server
    let app = axum::Router::new()
        .route("/api/stream", axum::routing::get(sse_handler))
        .fallback(axum::routing::get(static_file_handler))
        .layer(axum::Extension(tx));

    tokio::spawn(async move {
        println!("Atriumd: Visual DevUI dashboard starting on http://localhost:4040");
        if let Err(e) = axum::Server::bind(&http_addr)
            .serve(app.into_make_service())
            .await
        {
            eprintln!("Atriumd: Axum HTTP Server failed: {}", e);
        }
    });

    // Start tonic gRPC Server
    println!("Atriumd: gRPC Daemon starting on {}", grpc_addr);

    Server::builder()
        .add_service(DaemonServiceServer::new(daemon_service))
        .serve(grpc_addr)
        .await?;

    Ok(())
}
