// Stub router.rs - placeholder to unblock compilation
// Real MCP router will be re-implemented later

use anyhow::Result;

#[cfg(not(test))]
pub fn route_tool(_name: &str, _args: &[u8]) -> Result<Vec<u8>> {
    Err(anyhow::anyhow!("router not yet wired"))
}

#[cfg(test)]
pub fn route_tool(_name: &str, _args: &[u8]) -> Result<Vec<u8>> {
    // Test stub - return empty response
    Ok(vec![])
}
