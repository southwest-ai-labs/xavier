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

#[test]
fn test_parser_per_project_isolation() {
    use code_graph::parser::parse_native_with_root;
    use code_graph::parser::rust::RustParser;

    let proj_a = "/workspace/projA";
    let proj_b = "/workspace/projB";

    let src_a = r#"
        pub fn process_order(id: u64) -> bool {
            true
        }
        pub struct Order {
            pub id: u64,
        }
    "#;

    let src_b = r#"
        pub fn process_payment(amount: f64) -> bool {
            true
        }
        pub struct Payment {
            pub amount: f64,
        }
    "#;

    // 1. Parse using RustParser builder with_project_root
    let mut parser_a = RustParser::new().unwrap().with_project_root(Some(proj_a));
    let syms_a = parser_a.parse(src_a, "src/order.rs").unwrap();
    assert!(!syms_a.is_empty());
    for s in &syms_a {
        assert_eq!(
            s.project_root.as_deref(),
            Some(proj_a),
            "symbol {} must retain projA project_root",
            s.name
        );
    }

    // 2. Parse using parse_native_with_root
    let syms_b =
        parse_native_with_root(src_b, &Language::Rust, "src/payment.rs", Some(proj_b)).unwrap();
    assert!(!syms_b.is_empty());
    for s in &syms_b {
        assert_eq!(
            s.project_root.as_deref(),
            Some(proj_b),
            "symbol {} must retain projB project_root",
            s.name
        );
    }

    // 3. Insert both symbol sets into the database
    let db = CodeGraphDB::in_memory().expect("in-memory db");
    db.insert_symbols(&syms_a).unwrap();
    db.insert_symbols(&syms_b).unwrap();

    let engine = QueryEngine::new(Arc::new(db.clone()));

    // 4. Query independently by project root
    let query_a = engine
        .search_with_root("process", 10, Some(proj_a))
        .unwrap();
    assert_eq!(query_a.symbols.len(), 1);
    assert_eq!(query_a.symbols[0].name, "process_order");
    assert_eq!(query_a.symbols[0].project_root.as_deref(), Some(proj_a));

    let query_b = engine
        .search_with_root("process", 10, Some(proj_b))
        .unwrap();
    assert_eq!(query_b.symbols.len(), 1);
    assert_eq!(query_b.symbols[0].name, "process_payment");
    assert_eq!(query_b.symbols[0].project_root.as_deref(), Some(proj_b));

    // 5. Test stats_for_project
    let stats_a = db.stats_for_project(Some(proj_a)).unwrap();
    assert_eq!(stats_a.total_symbols, syms_a.len() as u64);

    let stats_b = db.stats_for_project(Some(proj_b)).unwrap();
    assert_eq!(stats_b.total_symbols, syms_b.len() as u64);

    let engine_stats_a = engine.stats_for_project(Some(proj_a)).unwrap();
    assert_eq!(engine_stats_a.total_symbols, syms_a.len() as u64);

    // 6. Test find_by_name_with_root
    let found_a = db.find_by_name_with_root("Order", 5, Some(proj_a)).unwrap();
    assert_eq!(found_a.len(), 1);
    assert_eq!(found_a[0].project_root.as_deref(), Some(proj_a));

    let found_a_in_b = db.find_by_name_with_root("Order", 5, Some(proj_b)).unwrap();
    assert_eq!(found_a_in_b.len(), 0);
}
