use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tonic::{transport::Server, Request, Response, Status};

pub mod daemon {
    tonic::include_proto!("atrium.daemon");
}

use daemon::daemon_service_server::{DaemonService, DaemonServiceServer};
use daemon::{
    ContextFile, ContextRequest, ContextResponse, IndexRequest, IndexResponse, VerifyRequest,
    VerifyResponse,
};

// Import workspace crates
use atrium_core_graph::CodeParser;
use atrium_core_memory::MemoryStore;
use atrium_core_verify::PatchVerifier;

pub struct AtriumDaemon {
    store: Arc<MemoryStore>,
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
            req.focus_files
        };

        // 2. Fetch relevant facts (global ones first)
        let mut facts = self.store.get_relevant_facts("global").unwrap_or_default();

        let mut context_files = Vec::new();

        // 3. For each file, fetch file-specific facts and symbol signatures to build high-signal context
        for file_path in files_to_query {
            // Fetch any rules/invariants associated with this file
            if let Ok(file_facts) = self.store.get_relevant_facts(&file_path) {
                facts.extend(file_facts);
            }

            // Fetch parsed symbols to construct a structural outline snippet
            let symbols = match self.store.get_symbols_for_file(&file_path) {
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
                path: file_path,
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

        Ok(Response::new(VerifyResponse {
            is_valid: ver_res.is_valid,
            linter_output: ver_res.linter_output,
            test_output: ver_res.test_output,
            compile_errors: ver_res.compile_errors,
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:50051".parse()?;

    // Initialize standard single-file SQLite database
    let db_path = "atrium_memory.db";
    let memory_store = Arc::new(MemoryStore::new(db_path)?);

    println!("Atriumd: Durable memory layer initialized at '{}'", db_path);

    let daemon_service = AtriumDaemon {
        store: memory_store,
    };

    println!("Atriumd: gRPC Daemon starting on {}", addr);

    Server::builder()
        .add_service(DaemonServiceServer::new(daemon_service))
        .serve(addr)
        .await?;

    Ok(())
}
