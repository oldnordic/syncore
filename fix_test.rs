use std::fs;
use std::path::PathBuf;

fn main() {
    let content = fs::read_to_string("tests/live_indexer_unit_tests.rs").unwrap();
    
    // Replace all FsEvent struct patterns with IngestionJob
    let lines: Vec<String> = content.lines().map(|line| {
        if line.contains("mpsc::channel::<FsEvent>") {
            line.replace("mpsc::channel::<FsEvent>", "mpsc::channel::<IngestionJob>")
        } else if line.contains("tx.send(FsEvent {") && line.contains("kind: FsEventKind::Created") {
            // Replace Created event
            line.to_string()
        } else if line.contains("path:") && line.contains("kind: FsEventKind::") {
            // Remove these lines
            String::new()
        } else if line.contains("});") && line.trim() == "})" {
            // Remove closing brace lines
            String::new()
        } else if line.contains(".await?;") && line.trim().ends_with(".await?;") {
            // Replace await lines
            line.to_string()
        } else {
            line.to_string()
        }
    }).collect();
    
    // Filter out empty lines
    let filtered_lines: Vec<String> = lines.into_iter().filter(|line| !line.trim().is_empty()).collect();
    
    fs::write("tests/live_indexer_unit_tests_fixed.rs", filtered_lines.join("\n")).unwrap();
}