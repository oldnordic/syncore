// Stub sequential.rs - placeholder to unblock compilation
// Real sequential engine will be re-implemented later

use anyhow::Result;

#[cfg(not(test))]
pub fn run_cycle() -> Result<()> {
    println!("Sequential engine placeholder");
    Ok(())
}

#[cfg(test)]
pub fn run_cycle() -> Result<()> {
    // Test stub - no-op
    Ok(())
}

// Placeholder structs to maintain API compatibility
#[derive(Debug, Clone)]
pub struct SequentialCore {
    // Empty placeholder
}

impl SequentialCore {
    pub fn new() -> Self {
        Self {}
    }

    pub fn run(&self) -> Result<()> {
        run_cycle()
    }
}
