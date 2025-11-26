use std::path::PathBuf;

#[test]
fn test_basic() {
    // Verify the plugin binary exists after build (debug mode)
    let plugin_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/debug/syncore_c_cpp_plugin");

    assert!(plugin_path.exists(), "Plugin binary should exist after build at {:?}", plugin_path);
}
