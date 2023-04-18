    fn test_check_unnecessary_braces_in_use_statement() {
        let file = SourceFileNode::parse(
            r#"
use a;
use {b};
use a::{c};
use a::{c, d::e};
use a::{c, d::{e}};
fn main() {}
"#,
        );
        let diagnostics = check_unnecessary_braces_in_use_statement(&file);
        assert_eq_dbg(
            "[Diagnostic { range: [12; 15), msg: \"Unnecessary braces in use statement\", severity: WeakWarning, fix: Some(LocalEdit { label: \"Remove unnecessary braces\", edit: TextEdit { atoms: [AtomTextEdit { delete: [12; 12), insert: \"b\" }, AtomTextEdit { delete: [12; 15), insert: \"\" }] }, cursor_position: Some(12) }) }, Diagnostic { range: [24; 27), msg: \"Unnecessary braces in use statement\", severity: WeakWarning, fix: Some(LocalEdit { label: \"Remove unnecessary braces\", edit: TextEdit { atoms: [AtomTextEdit { delete: [24; 24), insert: \"c\" }, AtomTextEdit { delete: [24; 27), insert: \"\" }] }, cursor_position: Some(24) }) }, Diagnostic { range: [61; 64), msg: \"Unnecessary braces in use statement\", severity: WeakWarning, fix: Some(LocalEdit { label: \"Remove unnecessary braces\", edit: TextEdit { atoms: [AtomTextEdit { delete: [61; 61), insert: \"e\" }, AtomTextEdit { delete: [61; 64), insert: \"\" }] }, cursor_position: Some(61) }) }]",
            &diagnostics,
        )
    }
