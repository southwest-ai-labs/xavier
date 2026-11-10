use code_graph::db::CodeGraphDB;
use code_graph::query::QueryEngine;
use code_graph::types::{Language, Symbol, SymbolKind};
use std::sync::Arc;

#[test]
fn test_per_project_isolation() {
    let db = CodeGraphDB::in_memory().expect("in-memory db");

    let repo_a = "/app/service-a";
    let repo_b = "/app/service-b";

    db.register_project(repo_a, "Service A").unwrap();
    db.register_project(repo_b, "Service B").unwrap();

    let sym_a = Symbol {
        id: None,
        stable_id: Some("sym-a".to_string()),
        name: "FunctionA".to_string(),
        kind: SymbolKind::Function,
        lang: Language::Rust,
        file_path: "src/lib.rs".to_string(),
        start_line: 1,
        end_line: 2,
        start_col: 0,
        end_col: 10,
        signature: None,
        parent: None,
        complexity: None,
        project_root: Some(repo_a.to_string()),
    };
    db.insert_symbol(&sym_a).unwrap();

    let sym_b = Symbol {
        id: None,
        stable_id: Some("sym-b".to_string()),
        name: "FunctionB".to_string(),
        kind: SymbolKind::Function,
        lang: Language::Rust,
        file_path: "src/lib.rs".to_string(),
        start_line: 10,
        end_line: 20,
        start_col: 0,
        end_col: 10,
        signature: None,
        parent: None,
        complexity: None,
        project_root: Some(repo_b.to_string()),
    };
    db.insert_symbol(&sym_b).unwrap();

    let engine = QueryEngine::new(Arc::new(db.clone()));

    // Search with root A
    let res_a = engine
        .search_with_root("Function", 10, Some(repo_a))
        .unwrap();
    assert_eq!(res_a.symbols.len(), 1);
    assert_eq!(res_a.symbols[0].name, "FunctionA");

    // Search with root B
    let res_b = engine
        .search_with_root("Function", 10, Some(repo_b))
        .unwrap();
    assert_eq!(res_b.symbols.len(), 1);
    assert_eq!(res_b.symbols[0].name, "FunctionB");

    // Search globally (None)
    let res_global = engine.search("Function", 10).unwrap();
    assert_eq!(res_global.symbols.len(), 2);

    // Test stats per project
    let stats_a = db.stats_for_project(Some(repo_a)).unwrap();
    assert_eq!(stats_a.total_symbols, 1);

    let stats_b = db.stats_for_project(Some(repo_b)).unwrap();
    assert_eq!(stats_b.total_symbols, 1);

    let stats_global = db.stats().unwrap();
    assert_eq!(stats_global.total_symbols, 2);
}
