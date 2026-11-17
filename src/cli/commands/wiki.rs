//! CLI Command Handlers for Living Wiki (`xavier wiki ...`).

use anyhow::{Context, Result};
use colored::Colorize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::info;

use crate::cli::commands::enums::WikiCommand;
use crate::wiki::{
    EntityCompiler, LintReport, WikiCatalogBuilder, WikiLinter, WikiLogEntry, WikiLogManager,
    WikiStorageEngine, WIKI_SQLITE_FILE,
};

/// Entry point for `xavier wiki ...` subcommands.
pub async fn handle_wiki_command(cmd: WikiCommand) -> Result<()> {
    match cmd {
        WikiCommand::Compile { path, force } => run_compile(path, force).await,
        WikiCommand::Lint { path, json } => run_lint(path, json).await,
        WikiCommand::Status { path, json } => run_status(path, json).await,
        WikiCommand::Export { out } => run_export(out).await,
    }
}

async fn run_compile(target_path: Option<PathBuf>, _force: bool) -> Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let root = target_path.unwrap_or(cwd);
    let wiki_dir = root.join(".xavier").join("wiki");
    std::fs::create_dir_all(&wiki_dir)?;

    println!(
        "{} Compiling Living Knowledge Wiki for {}",
        "⚙".cyan().bold(),
        root.display().to_string().yellow()
    );

    // Open CodeGraphDB
    let code_db_path = root.join(".xavier").join("code_graph.db");
    let (symbols, edges, stats, hubs) = if code_db_path.exists() {
        let db = code_graph::db::CodeGraphDB::new(&code_db_path)?;
        let syms = db.get_all_symbols().unwrap_or_default();
        let edgs = db.get_all_edges().unwrap_or_default();
        let st = db.stats().ok();
        let hb = db.hub_nodes(1, 10).unwrap_or_default();
        (syms, edgs, st, hb)
    } else {
        println!(
            "{} No persistent CodeGraphDB found at {}. Compiling empty catalog.",
            "⚠".yellow().bold(),
            code_db_path.display()
        );
        (Vec::new(), Vec::new(), None, Vec::new())
    };

    // 1. Compile AST symbols into entities
    let compiler = EntityCompiler::new(&wiki_dir);
    let compile_stats = compiler.compile(&symbols, &edges)?;

    // 2. Generate master MOC index.md
    let repo_name = root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project");
    let catalog_builder = WikiCatalogBuilder::new(&wiki_dir);
    catalog_builder.generate_index(repo_name, stats.as_ref(), &hubs)?;

    // 3. Log chronicle event
    let log_mgr = WikiLogManager::new(&wiki_dir);
    let log_entry = WikiLogEntry::new(
        "xavier-cli",
        "compile",
        format!(
            "Compiled {} entity dossiers",
            compile_stats.entities_written
        ),
    );
    log_mgr.append_entry(&log_entry).await?;

    // 4. Also synchronize into .xavier/wiki.sqlite container
    let sqlite_path = root.join(".xavier").join(WIKI_SQLITE_FILE);
    if let Ok(storage) = WikiStorageEngine::open(&sqlite_path) {
        let _ = storage.import_from_markdown_dir(&wiki_dir);
    }

    println!(
        "{} Compiled {} entity dossiers in {} ms",
        "✓".green().bold(),
        compile_stats.entities_written.to_string().green(),
        compile_stats.duration_ms
    );
    println!("   Catalog: {}", catalog_builder.index_path().display());
    println!("   Container: {}", sqlite_path.display());

    Ok(())
}

async fn run_lint(target_path: Option<PathBuf>, json_output: bool) -> Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let root = target_path.unwrap_or(cwd);
    let wiki_dir = if root.join(".xavier/wiki").exists() {
        root.join(".xavier/wiki")
    } else {
        root.clone()
    };

    let code_db_path = root.join(".xavier").join("code_graph.db");
    let code_db = if code_db_path.exists() {
        code_graph::db::CodeGraphDB::new(&code_db_path).ok()
    } else {
        None
    };

    let report = WikiLinter::lint_wiki(&wiki_dir, code_db.as_ref())?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    println!(
        "{} Living Wiki Lint Report for {}",
        "🔍".cyan().bold(),
        wiki_dir.display().to_string().yellow()
    );
    println!("   Total documents checked: {}", report.total_checked);

    if report.is_clean() {
        println!(
            "{} Wiki is 100% consistent! (0 broken links, 0 orphans, 0 code drift)",
            "✓".green().bold()
        );
    } else {
        if !report.broken_links.is_empty() {
            println!(
                "\n{} Broken Wikilinks ({}):",
                "✗".red().bold(),
                report.broken_links.len()
            );
            for b in &report.broken_links {
                println!(
                    "   - In `{}`: link to `[[{}]]` not found",
                    b.source_file,
                    b.target.red()
                );
            }
        }

        if !report.orphans.is_empty() {
            println!(
                "\n{} Orphan Notes ({}):",
                "⚠".yellow().bold(),
                report.orphans.len()
            );
            for orphan in &report.orphans {
                println!("   - `{}` has 0 incoming links", orphan.yellow());
            }
        }

        if !report.drift_warnings.is_empty() {
            println!(
                "\n{} Code-Doc Drift Warnings ({}):",
                "⚠".yellow().bold(),
                report.drift_warnings.len()
            );
            for drift in &report.drift_warnings {
                println!(
                    "   - `{}` references code symbol `{}` no longer in code_graph.db",
                    drift.entity_file,
                    drift.symbol_name.red()
                );
            }
        }
    }

    Ok(())
}

async fn run_status(target_path: Option<PathBuf>, json_output: bool) -> Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let root = target_path.unwrap_or(cwd);
    let wiki_dir = root.join(".xavier").join("wiki");
    let sqlite_path = root.join(".xavier").join(WIKI_SQLITE_FILE);

    let has_sqlite = sqlite_path.exists();
    let has_md_dir = wiki_dir.exists();

    let node_count = if has_sqlite {
        WikiStorageEngine::open(&sqlite_path)
            .and_then(|s| s.list_nodes())
            .map(|n| n.len())
            .unwrap_or(0)
    } else {
        0
    };

    if json_output {
        let status_json = serde_json::json!({
            "wiki_dir": wiki_dir.display().to_string(),
            "sqlite_container": sqlite_path.display().to_string(),
            "has_sqlite": has_sqlite,
            "has_markdown_vault": has_md_dir,
            "total_nodes": node_count,
        });
        println!("{}", serde_json::to_string_pretty(&status_json)?);
    } else {
        println!("{} Living Wiki Status:", "📊".cyan().bold());
        println!(
            "   Markdown Vault: {}",
            if has_md_dir {
                "Active".green()
            } else {
                "Not generated".yellow()
            }
        );
        println!(
            "   SQLite Container: {}",
            if has_sqlite {
                "Operational".green()
            } else {
                "Not generated".yellow()
            }
        );
        println!("   Total Living Nodes: {}", node_count.to_string().green());
    }

    Ok(())
}

async fn run_export(out_path: Option<PathBuf>) -> Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let sqlite_path = cwd.join(".xavier").join(WIKI_SQLITE_FILE);
    let target_dir = out_path.unwrap_or_else(|| cwd.join(".xavier").join("wiki"));

    let storage = WikiStorageEngine::open(&sqlite_path)
        .with_context(|| format!("Cannot open wiki container at {}", sqlite_path.display()))?;

    let count = storage.export_to_markdown_dir(&target_dir)?;
    println!(
        "{} Exported {} living wiki nodes from SQLite to {}",
        "✓".green().bold(),
        count.to_string().green(),
        target_dir.display().to_string().yellow()
    );

    Ok(())
}
