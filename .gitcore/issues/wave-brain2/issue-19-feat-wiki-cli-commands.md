# [Ola 33.19] feat-wiki-cli-commands — Living Wiki CLI Command Group

> Ola 33 — CLI. Labels: `ola33`, `wave-33`, `cli`, `wiki`, `commands`

## Current State (measurable)
- In `xavier --help`, there is no `wiki` subcommand.
- Developers and local agents cannot run `xavier wiki compile`, `xavier wiki lint`, or `xavier wiki show` from terminal.

## Desired State (delta)
- In `src/cli/commands/enums.rs`:
  - Add `Wiki(WikiCommand)` variant to `Commands` enum with subcommands:
    - `Compile { path: Option<PathBuf>, force: bool }`: Compiles code and docs into `.xavier/wiki/`.
    - `Lint { path: Option<PathBuf>, json: bool }`: Executes `WikiLinter` and displays formatted warnings or JSON.
    - `Status { path: Option<PathBuf> }`: Shows statistics of entities, concepts, backlinks, and health.
    - `Show { target: String }`: Displays rendered markdown or frontmatter of an entity/concept.
- Create `src/cli/commands/wiki.rs`:
  - Implement execution handlers delegating to `crate::wiki` engines.

## Acceptance Criteria (command-verifiable)
- [ ] `cargo run -p xavier --bin xavier -- wiki --help` — exits 0 and prints wiki subcommands
- [ ] `cargo test -p xavier --lib cli::commands::wiki` — passes
- [ ] `cargo clippy --all-targets -- -D warnings` — 0 warnings
- [ ] `cargo fmt --check` — clean

## Files to Modify
| File | Current state | Change | Risk |
|---|---|---|---|
| `src/cli/commands/wiki.rs` | Does not exist | Implement `wiki` CLI command handlers | LOW |
| `src/cli/commands/enums.rs` | Missing `WikiCommand` | Add `Wiki(WikiCommand)` enum variant | LOW |
| `src/cli/commands/mod.rs` | Missing `pub mod wiki;` | Export `wiki` module | LOW |

## DO NOT touch
- `src/cli/commands/code.rs` — code command group unchanged
- `src/cli/commands/memory.rs` — memory command group unchanged

## Anti-Hallucination Guard
1. READ `src/cli/commands/enums.rs` lines 1400–1460 for Clap derive conventions.
2. Ensure JSON output flag (`--json`) is supported for programmatic consumption by scripts.

## Merge order
- Within wave: 9/9 · Effort: Small (<45m) · Parallel with: all in Ola 33
