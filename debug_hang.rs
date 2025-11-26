use std::sync::Arc;
use std::time::Instant;
use syncore::project_analysis::{ProjectAnalysisEngine, deps::DepsRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let test_dir = setup_test_env().await?;
    
    println!("Testing individual PAE methods...");
    
    // Test 1: Dependencies
    println!("\n1. Testing dependencies method...");
    let start = Instant::now();
    let deps_result = test_dependencies_method(&test_dir).await;
    let duration = start.elapsed();
    
    match deps_result {
        Ok(_) => println!("✓ Dependencies completed in {:?}", duration),
        Err(e) => println!("✗ Dependencies failed: {:?}", e),
    }
    
    // Test 2: Architecture Overview
    println!("\n2. Testing architecture overview method...");
    let start = Instant::now();
    let arch_result = test_architecture_method(&test_dir).await;
    let duration = start.elapsed();
    
    match arch_result {
        Ok(_) => println!("✓ Architecture overview completed in {:?}", duration),
        Err(e) => println!("✗ Architecture overview failed: {:?}", e),
    }
    
    // Test 3: Complexity Dashboard
    println!("\n3. Testing complexity dashboard method...");
    let start = Instant::now();
    let complexity_result = test_complexity_method(&test_dir).await;
    let duration = start.elapsed();
    
    match complexity_result {
        Ok(_) => println!("✓ Complexity dashboard completed in {:?}", duration),
        Err(e) => println!("✗ Complexity dashboard failed: {:?}", e),
    }
    
    cleanup_test_env(&test_dir)?;
    Ok(())
}

async fn setup_test_env() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let temp_dir = std::fs::create_dir_all("/tmp/syncore_test_project")?;
    let test_dir = std::path::PathBuf::from("/tmp/syncore_test_project");
    
    // Create a simple Rust project structure
    std::fs::write(test_dir.join("Cargo.toml"), r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"

[dependencies]
"#)?;
    
    std::fs::create_dir_all(test_dir.join("src"))?;
    std::fs::write(test_dir.join("src/main.rs"), r#"
fn main() {
    println!("Hello, world!");
}

mod utils;
use utils::helper;

fn process_data() {
    let data = vec![1, 2, 3];
    helper(data);
}
"#)?;
    
    std::fs::write(test_dir.join("src/utils.rs"), r#"
pub fn helper<T>(data: Vec<T>) {
    println!("Processing {} items", data.len());
}

pub fn calculate_sum(numbers: &[i32]) -> i32 {
    numbers.iter().sum()
}
"#)?;
    
    Ok(test_dir)
}

async fn test_dependencies_method(test_dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let pae = ProjectAnalysisEngine::new(test_dir.to_str().unwrap(), ":memory:")?;
    let request = DepsRequest {
        root: None,
        max_depth: Some(5),
        include_external: Some(false),
    };
    
    let result = pae.build_unified_dependency_summary(request)?;
    println!("Dependencies found: {} modules, {} dependencies", 
             result.modules.len(), result.dependencies.len());
    
    Ok(())
}

async fn test_architecture_method(test_dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let pae = ProjectAnalysisEngine::new(test_dir.to_str().unwrap(), ":memory:")?;
    let request = syncore::project_analysis::architecture_overview::ArchitectureOverviewRequest::default();
    
    let result = pae.architecture_overview(request)?;
    println!("Architecture overview: {} modules, {} layers", 
             result.modules.len(), result.layers.len());
    
    Ok(())
}

async fn test_complexity_method(test_dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let pae = ProjectAnalysisEngine::new(test_dir.to_str().unwrap(), ":memory:")?;
    let request = syncore::project_analysis::complexity_dashboard::ComplexityDashboardRequest::default();
    
    let result = pae.complexity_dashboard(request)?;
    println!("Complexity dashboard: {} files analyzed", result.files.len());
    
    Ok(())
}

fn cleanup_test_env(test_dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::remove_dir_all(test_dir)?;
    Ok(())
}