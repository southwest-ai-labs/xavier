//! Living Wiki MCP Tools.
//!
//! Exposes `wiki_read_page`, `wiki_get_backlinks`, `wiki_search_graph`, and `wiki_lint_status`
//! to AI agents via the Model Context Protocol.

use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::types::{MCPContent, MCPTextContent, MCPTool, MCPToolResult};
use crate::wiki::{WikiGraph, WikiLinter, WikiStorageEngine, WIKI_SQLITE_FILE};
use crate::workspace::WorkspaceContext;

/// Returns tool definitions for all living wiki MCP tools.
pub fn get_xavier_wiki_tools() -> Vec<MCPTool> {
    vec![
        MCPTool {
            name: "wiki_read_page".to_string(),
            description: "Read a compiled living wiki markdown entity or concept dossier"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "page_id": {
                        "type": "string",
                        "description": "Identifier or filename of the entity or concept (e.g. 'calculate_tax' or 'entities/rust/calculate_tax.md')"
                    }
                },
                "required": ["page_id"]
            }),
        },
        MCPTool {
            name: "wiki_get_backlinks".to_string(),
            description:
                "Retrieve all wiki pages that reference or call this entity (incoming backlinks)"
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "target_id": {
                        "type": "string",
                        "description": "Target entity or concept name to find backlinks for"
                    }
                },
                "required": ["target_id"]
            }),
        },
        MCPTool {
            name: "wiki_search_graph".to_string(),
            description: "Perform N-hop neighborhood graph exploration around a wiki entity"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "root_id": {
                        "type": "string",
                        "description": "Central root entity identifier"
                    },
                    "depth": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 5,
                        "default": 2,
                        "description": "Maximum traversal depth hops"
                    }
                },
                "required": ["root_id"]
            }),
        },
        MCPTool {
            name: "wiki_lint_status".to_string(),
            description:
                "Run living wiki integrity linter to detect broken links, orphans, and code drift"
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        MCPTool {
            name: "wiki_vram_status".to_string(),
            description:
                "Check status, node count, and resident byte size of the Living Wiki GPU/VRAM engine"
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        MCPTool {
            name: "wiki_vram_search".to_string(),
            description:
                "Perform ultra-low-latency (<0.15ms) Graph-RAG neural and CSR graph traversal in VRAM"
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "root_id": {
                        "type": "string",
                        "description": "Root entity identifier to anchor graph traversal"
                    },
                    "max_hops": {
                        "type": "integer",
                        "default": 2,
                        "description": "Maximum graph traversal hops"
                    }
                },
                "required": ["root_id"]
            }),
        },
    ]
}

/// Checks if a tool name belongs to the living wiki group.
pub fn is_wiki_tool(name: &str) -> bool {
    matches!(
        name,
        "wiki_read_page"
            | "wiki_get_backlinks"
            | "wiki_search_graph"
            | "wiki_lint_status"
            | "wiki_vram_status"
            | "wiki_vram_search"
    )
}

/// Dispatches wiki tool execution.
pub async fn handle_wiki_tool(
    workspace: &WorkspaceContext,
    name: &str,
    arguments: Value,
) -> Result<Value> {
    let ws_path = {
        let p = PathBuf::from(&workspace.workspace_id);
        if p.exists() {
            p
        } else {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        }
    };

    let wiki_dir = ws_path.join(".xavier").join("wiki");
    let sqlite_path = ws_path.join(".xavier").join(WIKI_SQLITE_FILE);

    match name {
        "wiki_read_page" => {
            let page_id = arguments
                .get("page_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing required 'page_id' argument"))?;

            // Try SQLite container first
            if sqlite_path.exists() {
                if let Ok(storage) = WikiStorageEngine::open(&sqlite_path) {
                    if let Ok(Some(node)) = storage.get_node(page_id) {
                        return format_mcp_result(json!({
                            "found": true,
                            "source": "sqlite",
                            "stable_id": node.stable_id,
                            "title": node.title,
                            "kind": node.kind,
                            "content": node.markdown_content
                        }));
                    }
                }
            }

            // Fallback to disk .md file
            let candidates = [
                wiki_dir.join(page_id),
                wiki_dir.join(format!("{page_id}.md")),
                wiki_dir.join("entities").join(page_id),
                wiki_dir.join("entities").join(format!("{page_id}.md")),
                wiki_dir.join("concepts").join(page_id),
                wiki_dir.join("concepts").join(format!("{page_id}.md")),
            ];

            for cand in &candidates {
                if cand.is_file() {
                    let content = std::fs::read_to_string(cand)?;
                    return format_mcp_result(json!({
                        "found": true,
                        "source": "filesystem",
                        "file_path": cand.display().to_string(),
                        "content": content
                    }));
                }
            }

            format_mcp_result(json!({
                "found": false,
                "message": format!("Wiki page '{}' not found in {}", page_id, wiki_dir.display())
            }))
        }

        "wiki_get_backlinks" => {
            let target_id = arguments
                .get("target_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing required 'target_id' argument"))?;

            if sqlite_path.exists() {
                let storage = Arc::new(WikiStorageEngine::open(&sqlite_path)?);
                let graph = WikiGraph::new(storage);
                let backlinks = graph.get_backlinks(target_id)?;
                format_mcp_result(json!({
                    "target_id": target_id,
                    "backlinks_count": backlinks.len(),
                    "backlinks": backlinks
                }))
            } else {
                format_mcp_result(json!({
                    "target_id": target_id,
                    "backlinks_count": 0,
                    "backlinks": [],
                    "message": "Wiki SQLite container not yet generated. Run 'xavier wiki compile'"
                }))
            }
        }

        "wiki_search_graph" => {
            let root_id = arguments
                .get("root_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing required 'root_id' argument"))?;

            let depth = arguments.get("depth").and_then(|v| v.as_u64()).unwrap_or(2) as usize;

            // 1. Check resident VRAM engine first for sub-millisecond CSR traversal
            {
                let vram = workspace.workspace.wiki_vram.read().await;
                if vram.is_active() {
                    if let Some(&idx) = vram.node_to_idx().get(root_id) {
                        let start = std::time::Instant::now();
                        let traversed = vram.traverse_hops(&[idx], depth);
                        let duration_us = start.elapsed().as_micros();
                        let nodes: Vec<_> = traversed
                            .into_iter()
                            .filter_map(|(n_idx, hop)| {
                                vram.idx_to_node_name(n_idx).map(|name| {
                                    json!({
                                        "node_id": name,
                                        "hop": hop
                                    })
                                })
                            })
                            .collect();

                        return format_mcp_result(json!({
                            "source": "vram_csr_resident",
                            "root_id": root_id,
                            "traversal_time_us": duration_us,
                            "depth": depth,
                            "nodes_count": nodes.len(),
                            "nodes": nodes
                        }));
                    }
                }
            }

            // 2. Fallback to SQLite container
            if sqlite_path.exists() {
                let storage = Arc::new(WikiStorageEngine::open(&sqlite_path)?);
                let graph = WikiGraph::new(storage);
                let subgraph = graph.get_neighborhood(root_id, depth)?;
                format_mcp_result(json!(subgraph))
            } else {
                format_mcp_result(json!({
                    "root_id": root_id,
                    "nodes": [],
                    "edges": [],
                    "message": "Wiki SQLite container not found. Run 'xavier wiki compile'"
                }))
            }
        }

        "wiki_lint_status" => {
            let code_db_path = ws_path.join(".xavier").join("code_graph.db");
            let code_db = if code_db_path.exists() {
                code_graph::db::CodeGraphDB::new(&code_db_path).ok()
            } else {
                None
            };

            let report = WikiLinter::lint_wiki(&wiki_dir, code_db.as_ref())?;
            format_mcp_result(json!(report))
        }

        "wiki_vram_status" => {
            let vram = workspace.workspace.wiki_vram.read().await;
            format_mcp_result(json!({
                "active": vram.is_active(),
                "node_count": vram.node_count(),
                "estimated_vram_bytes": vram.vram_bytes(),
                "execution_plane": if vram.is_active() { "VRAM / In-Memory Resident" } else { "Unloaded (SQLite fallback)" }
            }))
        }

        "wiki_vram_search" => {
            let vram = workspace.workspace.wiki_vram.read().await;
            let root_id = arguments
                .get("root_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing required 'root_id' argument"))?;
            let max_hops = arguments
                .get("max_hops")
                .and_then(|v| v.as_u64())
                .unwrap_or(2) as usize;

            if vram.is_active() {
                if let Some(&idx) = vram.node_to_idx().get(root_id) {
                    let start = std::time::Instant::now();
                    let traversed = vram.traverse_hops(&[idx], max_hops);
                    let duration_us = start.elapsed().as_micros();
                    let nodes: Vec<_> = traversed
                        .into_iter()
                        .filter_map(|(n_idx, hop)| {
                            vram.idx_to_node_name(n_idx).map(|name| {
                                json!({
                                    "node_id": name,
                                    "hop": hop
                                })
                            })
                        })
                        .collect();

                    return format_mcp_result(json!({
                        "source": "vram_csr_resident",
                        "root_id": root_id,
                        "traversal_time_us": duration_us,
                        "hops_applied": max_hops,
                        "nodes_count": nodes.len(),
                        "nodes": nodes
                    }));
                }
            }

            format_mcp_result(json!({
                "source": "vram_empty_fallback",
                "root_id": root_id,
                "message": "VRAM engine has 0 nodes loaded or root_id not found. Run 'xavier wiki compile' to populate."
            }))
        }

        _ => Err(anyhow!("Unknown wiki tool: {name}")),
    }
}

fn format_mcp_result(val: Value) -> Result<Value> {
    let pretty_str = serde_json::to_string_pretty(&val)?;
    Ok(serde_json::to_value(MCPToolResult {
        content: vec![MCPContent::Text(MCPTextContent {
            content_type: "text".to_string(),
            text: pretty_str,
        })],
        structured_content: Some(val),
        is_error: Some(false),
    })?)
}
