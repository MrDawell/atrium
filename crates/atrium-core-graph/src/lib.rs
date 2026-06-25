use petgraph::graph::DiGraph;
use std::path::Path;
use tree_sitter::{Node, Parser};

#[derive(Debug, Clone)]
pub struct SymbolNode {
    pub name: String,
    pub kind: String, // e.g., "function", "struct", "impl", "import"
    pub start_line: usize,
    pub end_line: usize,
    pub content: String,
}

pub struct CodeParser;

impl CodeParser {
    pub fn parse_source(content: &str) -> Result<Vec<SymbolNode>, String> {
        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_rust::language())
            .map_err(|e| format!("Failed to set Rust language: {:?}", e))?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| "Failed to parse content".to_string())?;

        let mut symbols = Vec::new();
        Self::traverse_node(tree.root_node(), content, &mut symbols);

        Ok(symbols)
    }

    fn traverse_node(node: Node, content: &str, symbols: &mut Vec<SymbolNode>) {
        let kind = node.kind();

        match kind {
            "function_item" | "struct_item" | "impl_item" | "use_declaration"
            | "trait_item" | "enum_item" | "mod_item" => {
                let name = Self::extract_name(node, content).unwrap_or_else(|| "anonymous".to_string());
                let start_line = node.start_position().row;
                let end_line = node.end_position().row;
                let node_content = node
                    .utf8_text(content.as_bytes())
                    .unwrap_or("")
                    .to_string();

                symbols.push(SymbolNode {
                    name,
                    kind: match kind {
                        "function_item" => "function".to_string(),
                        "struct_item" => "struct".to_string(),
                        "impl_item" => "impl".to_string(),
                        "use_declaration" => "import".to_string(),
                        "trait_item" => "trait".to_string(),
                        "enum_item" => "enum".to_string(),
                        "mod_item" => "module".to_string(),
                        _ => kind.to_string(),
                    },
                    start_line,
                    end_line,
                    content: node_content,
                });
            }
            _ => {}
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
        // Try getting field named "name" (which tonic/tree-sitter can represent as str or &[u8])
        // Let's use name field if it exists
        if let Some(name_node) = node.child_by_field_name("name") {
            if let Ok(text) = name_node.utf8_text(content.as_bytes()) {
                if !text.is_empty() {
                    return Some(text.to_string());
                }
            }
        }

        // Fallback: search children for identifier tokens
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

        // Handle impl blocks where type might be under a field named "type"
        if node.kind() == "impl_item" {
            if let Some(type_node) = node.child_by_field_name("type") {
                if let Ok(text) = type_node.utf8_text(content.as_bytes()) {
                    return Some(text.to_string());
                }
            }
        }

        // Handle use declarations (imports) by formatting the import path
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
        let symbols = CodeParser::parse_source(content)?;
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
