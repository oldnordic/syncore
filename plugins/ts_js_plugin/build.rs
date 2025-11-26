fn main() {
    // The tree-sitter grammars are handled by the tree-sitter-typescript and tree-sitter-javascript crates
    // No additional build steps needed
    println!("cargo:rerun-if-changed=src/");
}