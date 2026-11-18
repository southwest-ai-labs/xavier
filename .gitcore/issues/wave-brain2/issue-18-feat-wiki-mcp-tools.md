# [Ola 33.18] feat-wiki-mcp-tools — Expose Living Wiki Tools via MCP

> Ola 33 — MCP. Labels: `ola33`, `wave-33`, `mcp`, `wiki`, `tools`

## Current State (measurable)
- Current MCP tools provide raw vector searches (`mem_search`) or raw AST lookups (`codegraph_explore`).
- There are no MCP tools allowing LLM agents (Claude Code, Antigravity, Hermes) to inspect living wiki pages, traverse entity backlinks, or check wiki lint status.

## Desired State (delta)
- Create `src/server/mcp/tools_wiki.rs`:
  - Implement 4 new MCP tools:
    1. `wiki_read_page`: Read a compiled entity or concept markdown page (`page_path: String`).
    2. `wiki_get_backlinks`: Return all documents linking to a specific entity or concept (`target_id: String`).
    3. `wiki_search_graph`: Graph traversal querying entity neighbors up to `depth: usize`.
    4. `wiki_lint_status`: Run `WikiLinter` and return broken links, orphans, and code drift warnings.
- In `src/server/mcp/session.rs`:
  - Register wiki tools in the unified MCP dispatcher.

## Acceptance Criteria (command-verifiable)
- [ ] `cargo test --test mcp_tool_parity` — all tests pass, including new wiki tools
- [ ] `cargo test -p xavier --lib server::mcp::tools_wiki` — passes
- [ ] `cargo clippy --all-targets -- -D warnings` — 0 warnings
- [ ] `cargo fmt --check` — clean

## Files to Modify
| File | Current state | Change | Risk |
|---|---|---|---|
| `src/server/mcp/tools_wiki.rs` | Does not exist | Implement 4 wiki MCP tools with typed inputs and structured JSON | LOW |
| `src/server/mcp/session.rs` | Missing wiki tools routing | Add wiki tool dispatch arms | LOW |

## DO NOT touch
- `src/server/mcp/tools_core.rs` — core tools remain untouched
- `src/server/mcp/tools_memory.rs` — memory tools remain untouched

## Anti-Hallucination Guard
1. READ `src/server/mcp/types.rs` for `MCPToolResult` structure and error wrapping.
2. Tools must respect workspace boundaries (never read arbitrary host filesystem paths).

## Merge order
- Within wave: 8/9 · Effort: Small (<45m) · Parallel with: all in Ola 33
