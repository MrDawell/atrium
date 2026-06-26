use petgraph::graph::DiGraph;
use std::path::Path;
use tree_sitter::{Node, Parser};

#[derive(Debug, Clone)]
pub struct SymbolNode {
    pub name: String,
    pub kind: String, // e.g., "function", "struct", "class", "import"
    pub start_line: usize,
    pub end_line: usize,
    pub content: String,
}

pub struct CodeParser;

fn get_tree_sitter_lang_name(file_ext: &str) -> Option<&'static str> {
    match file_ext.to_lowercase().as_str() {
        "rs" => Some("rust"),
        "py" => Some("python"),
        "java" => Some("java"),
        "cs" => Some("c_sharp"),
        "kt" => Some("kotlin"),
        "swift" => Some("swift"),
        "php" => Some("php"),
        "rb" => Some("ruby"),
        "lua" => Some("lua"),
        "pl" => Some("perl"),
        "r" => Some("r"),
        "sol" => Some("solidity"),
        "zig" => Some("zig"),
        "ex" | "exs" => Some("elixir"),
        "hs" => Some("haskell"),
        "clj" => Some("clojure"),
        "sh" | "bash" => Some("bash"),
        _ => None,
    }
}

fn load_dynamic_grammar(lang: &str) -> Result<tree_sitter::Language, String> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_default();

    let search_dirs = vec![
        std::path::PathBuf::from("grammars"),
        std::path::PathBuf::from(home).join(".gemini").join("antigravity-cli").join("grammars"),
    ];

    let lib_name = if cfg!(target_os = "windows") {
        format!("tree-sitter-{}.dll", lang)
    } else if cfg!(target_os = "macos") {
        format!("libtree-sitter-{}.dylib", lang)
    } else {
        format!("libtree-sitter-{}.so", lang)
    };

    let mut found_path = None;
    for dir in search_dirs {
        let p = dir.join(&lib_name);
        if p.exists() {
            found_path = Some(p);
            break;
        }
    }

    let library_path = found_path.ok_or_else(|| format!("Dynamic grammar library {} not found in search paths", lib_name))?;

    unsafe {
        let lib = libloading::Library::new(library_path)
            .map_err(|e| format!("Failed to load dynamic library: {:?}", e))?;

        let symbol_name = format!("tree_sitter_{}", lang);

        // Leak dynamic library to keep memory pointers valid for the program duration
        let leaked_lib = Box::leak(Box::new(lib));

        let constructor: libloading::Symbol<unsafe extern "C" fn() -> tree_sitter::Language> = leaked_lib
            .get(symbol_name.as_bytes())
            .map_err(|e| format!("Failed to get symbol {}: {:?}", symbol_name, e))?;

        let language = constructor();
        Ok(language)
    }
}

impl CodeParser {
    pub fn parse_source(content: &str, file_ext: &str) -> Result<Vec<SymbolNode>, String> {
        let ext = file_ext.trim_start_matches('.').to_lowercase();

        if ext == "rs" {
            let mut parser = Parser::new();
            parser
                .set_language(tree_sitter_rust::language())
                .map_err(|e| format!("Failed to set Rust language: {:?}", e))?;

            let tree = parser
                .parse(content, None)
                .ok_or_else(|| "Failed to parse content".to_string())?;

            let mut symbols = Vec::new();
            Self::traverse_node(tree.root_node(), content, &mut symbols);
            return Ok(symbols);
        }

        // Try dynamic grammar registry
        if let Some(lang_name) = get_tree_sitter_lang_name(&ext) {
            if let Ok(language) = load_dynamic_grammar(lang_name) {
                let mut parser = Parser::new();
                if parser.set_language(language).is_ok() {
                    if let Some(tree) = parser.parse(content, None) {
                        let mut symbols = Vec::new();
                        Self::traverse_node(tree.root_node(), content, &mut symbols);
                        return Ok(symbols);
                    }
                }
            }
        }

        // Fallback to Universal Structural Fallback Engine
        Ok(Self::fallback_parse(content))
    }

    fn get_symbol_kind_and_name(node: Node, content: &str) -> Option<(String, String)> {
        let kind = node.kind();

        let is_func = kind.contains("function") || kind.contains("method") || kind.contains("procedure") || kind == "fn";
        let is_type = kind.contains("class") || kind.contains("struct") || kind.contains("interface") || kind.contains("enum") || kind == "record" || kind == "contract" || kind == "impl_item";
        let is_import = kind.contains("import") || kind.contains("use_declaration") || kind.contains("require") || kind.contains("include");

        if is_func || is_type || is_import {
            let kind_str = if is_func {
                "function".to_string()
            } else if is_import {
                "import".to_string()
            } else if kind == "impl_item" {
                "impl".to_string()
            } else {
                kind.replace("_declaration", "").replace("_definition", "").replace("_item", "")
            };

            let name = Self::extract_name(node, content).unwrap_or_else(|| "anonymous".to_string());
            Some((kind_str, name))
        } else {
            None
        }
    }

    fn traverse_node(node: Node, content: &str, symbols: &mut Vec<SymbolNode>) {
        if let Some((kind_str, name)) = Self::get_symbol_kind_and_name(node, content) {
            let start_line = node.start_position().row;
            let end_line = node.end_position().row;
            let node_content = node
                .utf8_text(content.as_bytes())
                .unwrap_or("")
                .to_string();

            symbols.push(SymbolNode {
                name,
                kind: kind_str,
                start_line,
                end_line,
                content: node_content,
            });
        }

        // Recursively traverse children
        let count = node.child_count();
        for i in 0..count {
            if let Some(child) = node.child(i) {
                Self::traverse_node(child, content, symbols);
            }
        }
    }

    fn extract_name(node: Node, content: &str) -> Option<String> {
        if let Some(name_node) = node.child_by_field_name("name") {
            if let Ok(text) = name_node.utf8_text(content.as_bytes()) {
                if !text.is_empty() {
                    return Some(text.to_string());
                }
            }
        }

        // Fallback identifier search
        let count = node.child_count();
        for i in 0..count {
            if let Some(child) = node.child(i) {
                let ck = child.kind();
                if ck == "identifier" || ck == "type_identifier" {
                    if let Ok(text) = child.utf8_text(content.as_bytes()) {
                        return Some(text.to_string());
                    }
                }
            }
        }

        if node.kind() == "impl_item" {
            if let Some(type_node) = node.child_by_field_name("type") {
                if let Ok(text) = type_node.utf8_text(content.as_bytes()) {
                    return Some(text.to_string());
                }
            }
        }

        if node.kind() == "use_declaration" {
            if let Ok(text) = node.utf8_text(content.as_bytes()) {
                let text = text.trim();
                let clean = text
                    .strip_prefix("use ")
                    .unwrap_or(text)
                    .strip_suffix(";")
                    .unwrap_or(text);
                return Some(clean.to_string());
            }
        }

        None
    }

    fn fallback_parse(content: &str) -> Vec<SymbolNode> {
        let lines: Vec<&str> = content.lines().collect();
        let mut symbols = Vec::new();

        let import_keywords = ["import", "include", "use", "require", "package"];
        let type_keywords = ["class", "struct", "interface", "enum", "type", "record", "contract", "impl", "module"];
        let func_keywords = ["fn", "func", "function", "def", "method", "procedure"];

        let mut i = 0;
        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();

            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("#") || trimmed.starts_with("/*") || trimmed.starts_with("*") {
                i += 1;
                continue;
            }

            // 1. Check for imports
            let mut is_import = false;
            for kw in &import_keywords {
                let prefix = format!("{} ", kw);
                if trimmed.starts_with(&prefix) || trimmed.starts_with(&format!("{}('", kw)) || trimmed.starts_with(&format!("{}(\"", kw)) {
                    let name = trimmed
                        .strip_prefix(kw)
                        .unwrap_or(trimmed)
                        .trim_matches(|c: char| c == '(' || c == ')' || c == ';' || c == '\'' || c == '"' || c.is_whitespace())
                        .to_string();

                    if !name.is_empty() {
                        symbols.push(SymbolNode {
                            name,
                            kind: "import".to_string(),
                            start_line: i,
                            end_line: i,
                            content: line.to_string(),
                        });
                        is_import = true;
                        break;
                    }
                }
            }
            if is_import {
                i += 1;
                continue;
            }

            // 2. Check for declarations
            let mut matched_decl = None;

            let tokens: Vec<&str> = trimmed
                .split(|c: char| c.is_whitespace() || c == '(' || c == '{' || c == ':' || c == ';')
                .filter(|s| !s.is_empty())
                .collect();

            if tokens.len() >= 2 {
                for (idx, token) in tokens.iter().enumerate() {
                    if type_keywords.contains(token) {
                        if idx + 1 < tokens.len() {
                            let name = tokens[idx + 1].trim_matches(|c: char| c == ':' || c == '{' || c == '(').to_string();
                            if !name.is_empty() {
                                matched_decl = Some((name, token.to_string(), i));
                                break;
                            }
                        }
                    } else if func_keywords.contains(token) {
                        if idx + 1 < tokens.len() {
                            let name = tokens[idx + 1].trim_matches(|c: char| c == ':' || c == '{' || c == '(').to_string();
                            if !name.is_empty() {
                                matched_decl = Some((name, "function".to_string(), i));
                                break;
                            }
                        }
                    }
                }
            }

            if let Some((name, kind, start_idx)) = matched_decl {
                let mut end_idx = start_idx;
                let mut has_braces = false;
                let mut brace_count = 0;
                let mut found_first_brace = false;

                for j in start_idx..std::cmp::min(start_idx + 5, lines.len()) {
                    let l = lines[j];
                    if l.contains('{') {
                        has_braces = true;
                        break;
                    }
                }

                if has_braces {
                    for j in start_idx..lines.len() {
                        let l = lines[j];
                        for c in l.chars() {
                            if c == '{' {
                                found_first_brace = true;
                                brace_count += 1;
                            } else if c == '}' {
                                if found_first_brace {
                                    brace_count -= 1;
                                    if brace_count == 0 {
                                        end_idx = j;
                                        break;
                                    }
                                }
                            }
                        }
                        if found_first_brace && brace_count == 0 {
                            break;
                        }
                    }
                } else {
                    let start_indent = line.len() - line.trim_start().len();
                    let mut last_non_empty = start_idx;
                    for j in (start_idx + 1)..lines.len() {
                        let l = lines[j];
                        let t = l.trim();
                        if t.is_empty() {
                            continue;
                        }
                        if t.starts_with("//") || t.starts_with("#") {
                            last_non_empty = j;
                            continue;
                        }
                        let indent = l.len() - l.trim_start().len();
                        if indent <= start_indent {
                            break;
                        }
                        last_non_empty = j;
                    }
                    end_idx = last_non_empty;
                }

                let content_slice = lines[start_idx..=end_idx].join("\n");
                symbols.push(SymbolNode {
                    name,
                    kind,
                    start_line: start_idx,
                    end_line: end_idx,
                    content: content_slice,
                });
                i = end_idx + 1;
            } else {
                i += 1;
            }
        }

        symbols
    }
}

pub struct SymbolGraph {
    pub graph: DiGraph<SymbolNode, ()>,
}

impl SymbolGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
        }
    }

    pub fn parse_file(&mut self, _path: &Path, content: &str) -> Result<(), String> {
        let ext = _path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
        let symbols = CodeParser::parse_source(content, ext)?;
        for symbol in symbols {
            self.graph.add_node(symbol);
        }
        Ok(())
    }
}

impl Default for SymbolGraph {
    fn default() -> Self {
        Self::new()
    }
}
