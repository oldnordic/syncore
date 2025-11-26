use std::io::{self, BufRead, BufReader, Write};
use anyhow::Result;

mod dlr_entry;
mod c_cpp_indexer;
mod c_cpp_diagnostics;
mod c_cpp_include_graph;
mod c_cpp_macro_extractor;
mod plugin_api;

fn main() -> Result<()> {
    // Set up the plugin with line-based stdin/stdout communication
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let reader = BufReader::new(stdin.lock());

    // Initialize the DLR entry point
    let mut entry = dlr_entry::DlrEntry::new();

    // Process each line from stdin
    for line in reader.lines() {
        let line = match line {
            Ok(line) => line,
            Err(_) => break, // EOF
        };

        // Process the command and get response
        let response = entry.process_command(&line);

        // Write response to stdout
        if let Err(e) = writeln!(stdout, "{}", response) {
            eprintln!("Error writing response: {}", e);
            break;
        }

        // Flush output to ensure immediate response
        if let Err(e) = stdout.flush() {
            eprintln!("Error flushing output: {}", e);
            break;
        }

        // If this was a shutdown command, exit. loop
        if entry.should_shutdown() {
            break;
        }
    }

    Ok(())
}
