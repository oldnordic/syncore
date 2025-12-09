use crate::mcp_server::server;

macro_rules! assert_dry_run_forwarded {
    ($fn:ident, $($args:expr),+) => {
        let payload = server::$fn($($args),+);
        assert!(payload.get("dry_run").is_some());
    };
}

#[test]
fn parser_analyze_forwards_dry_run() {
    assert_dry_run_forwarded!(parser_analyze_payload, "src/lib.rs", true, false);
}

#[test]
fn parser_search_forwards_dry_run() {
    assert_dry_run_forwarded!(parser_search_payload, "foo", Some("src"), Some(3), true);
}

#[test]
fn code_index_forwards_dry_run() {
    assert_dry_run_forwarded!(code_index_payload, "src/lib.rs", true);
}

#[test]
fn code_search_forwards_dry_run() {
    assert_dry_run_forwarded!(code_search_payload, "query", 5, true);
}

#[test]
fn code_index_directory_forwards_dry_run() {
    assert_dry_run_forwarded!(code_index_directory_payload, "src", "*.rs", true);
}

#[test]
fn document_index_forwards_dry_run() {
    assert_dry_run_forwarded!(document_index_payload, "docs", true);
}

#[test]
fn document_search_forwards_dry_run() {
    assert_dry_run_forwarded!(document_search_payload, "query", 10, true);
}
