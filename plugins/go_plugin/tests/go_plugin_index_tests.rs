use anyhow::Result;
use tempfile::TempDir;
use syncore_go_plugin::{GoIndexer};
use syncore_go_plugin::plugin_api::{EntityKind, EdgeKind};

#[test]
fn test_index_package_declaration() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.go");
    std::fs::write(&file_path, "package main\n")?;

    let mut indexer = GoIndexer::new()?;
    let result = indexer.index_file(file_path.to_str().unwrap())?;

    println!("=== Result: {:?}", result);
    
    assert!(result.entities.is_some());
    let entities = result.entities.unwrap();
    
    println!("Found {} entities:", entities.len());
    for entity in &entities {
        println!("  - {:?}: {}", entity.kind, entity.name);
    }
    
    let package_entity = entities.iter().find(|e| e.kind == EntityKind::Package);
    assert!(package_entity.is_some(), "Should find package entity");
    
    let pkg = package_entity.unwrap();
    assert_eq!(pkg.name, "main");
    assert_eq!(pkg.file_path, file_path.to_str().unwrap());

    Ok(())
}

#[test]
fn test_index_imports() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.go");
    std::fs::write(&file_path, r#"
package main

import "fmt"
import "os"
import (
    "net/http"
    "encoding/json"
)
"#)?;

    let mut indexer = GoIndexer::new()?;
    let result = indexer.index_file(file_path.to_str().unwrap())?;

    assert!(result.entities.is_some());
    let entities = result.entities.unwrap();
    
    println!("Found {} entities:", entities.len());
    for entity in &entities {
        println!("  - {:?}: {}", entity.kind, entity.name);
    }
    
    let import_entities: Vec<_> = entities.iter()
        .filter(|e| e.kind == EntityKind::Import)
        .collect();
    
    assert_eq!(import_entities.len(), 4, "Should find 4 imports");
    
    let import_names: Vec<_> = import_entities.iter().map(|e| &e.name).collect();
    assert!(import_names.contains(&&"fmt".to_string()));
    assert!(import_names.contains(&&"os".to_string()));
    assert!(import_names.contains(&&"net/http".to_string()));
    assert!(import_names.contains(&&"encoding/json".to_string()));

    Ok(())
}

#[test]
fn test_index_functions() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.go");
    std::fs::write(&file_path, r#"
package main

func simpleFunction() {
    fmt.Println("hello")
}

func withParams(a int, b string) (int, error) {
    return a + 1, nil
}
"#)?;

    let mut indexer = GoIndexer::new()?;
    let result = indexer.index_file(file_path.to_str().unwrap())?;

    assert!(result.entities.is_some());
    let entities = result.entities.unwrap();
    
    let func_entities: Vec<_> = entities.iter()
        .filter(|e| e.kind == EntityKind::Function)
        .collect();
    
    assert_eq!(func_entities.len(), 2, "Should find 2 functions");
    
    let func_names: Vec<_> = func_entities.iter().map(|e| &e.name).collect();
    assert!(func_names.contains(&&"simpleFunction".to_string()));
    assert!(func_names.contains(&&"withParams".to_string()));

    Ok(())
}

#[test]
fn test_index_methods_with_receivers() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.go");
    std::fs::write(&file_path, r#"
package main

type MyStruct struct {
    field int
}

func (m *MyStruct) pointerMethod() string {
    return "pointer"
}

func (m MyStruct) valueMethod() string {
    return "value"
}
"#)?;

    let mut indexer = GoIndexer::new()?;
    let result = indexer.index_file(file_path.to_str().unwrap())?;

    assert!(result.entities.is_some());
    let entities = result.entities.unwrap();
    
    let method_entities: Vec<_> = entities.iter()
        .filter(|e| e.kind == EntityKind::Method)
        .collect();
    
    assert_eq!(method_entities.len(), 2, "Should find 2 methods");
    
    let method_names: Vec<_> = method_entities.iter().map(|e| &e.name).collect();
    assert!(method_names.contains(&&"pointerMethod".to_string()));
    assert!(method_names.contains(&&"valueMethod".to_string()));

    Ok(())
}

#[test]
fn test_index_structs_and_interfaces() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.go");
    std::fs::write(&file_path, r#"
package main

type User struct {
    ID    int
    Name  string
    Email string
}

type Reader interface {
    Read() ([]byte, error)
    Close() error
}

type Writer interface {
    Write(data []byte) (int, error)
}
"#)?;

    let mut indexer = GoIndexer::new()?;
    let result = indexer.index_file(file_path.to_str().unwrap())?;

    assert!(result.entities.is_some());
    let entities = result.entities.unwrap();
    
    let struct_entities: Vec<_> = entities.iter()
        .filter(|e| e.kind == EntityKind::Struct)
        .collect();
    
    assert_eq!(struct_entities.len(), 1, "Should find 1 struct");
    assert_eq!(struct_entities[0].name, "User");

    let interface_entities: Vec<_> = entities.iter()
        .filter(|e| e.kind == EntityKind::Interface)
        .collect();
    
    assert_eq!(interface_entities.len(), 2, "Should find 2 interfaces");
    
    let interface_names: Vec<_> = interface_entities.iter().map(|e| &e.name).collect();
    assert!(interface_names.contains(&&"Reader".to_string()));
    assert!(interface_names.contains(&&"Writer".to_string()));

    Ok(())
}

#[test]
fn test_calls_edges_detection() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.go");
    std::fs::write(&file_path, r#"
package main

func caller() {
    callee1()
    someStruct.callee2()
    fmt.Println("test")
}

func callee1() {}
"#)?;

    let mut indexer = GoIndexer::new()?;
    let result = indexer.index_file(file_path.to_str().unwrap())?;

    assert!(result.edges.is_some());
    let edges = result.edges.unwrap();
    
    let call_edges: Vec<_> = edges.iter()
        .filter(|e| e.kind == EdgeKind::Calls)
        .collect();
    
    assert!(!call_edges.is_empty(), "Should find call edges");

    Ok(())
}

#[test]
fn test_vendor_folder_ignored() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let vendor_dir = temp_dir.path().join("vendor");
    std::fs::create_dir_all(&vendor_dir)?;
    
    let vendor_file = vendor_dir.join("vendor.go");
    std::fs::write(&vendor_file, "package vendor\nfunc vendorFunc() {}\n")?;
    
    let normal_file = temp_dir.path().join("normal.go");
    std::fs::write(&normal_file, "package main\nfunc normalFunc() {}\n")?;

    let mut indexer = GoIndexer::new()?;
    let result = indexer.index_directory(temp_dir.path().to_str().unwrap())?;

    assert!(result.entities.is_some());
    let entities = result.entities.unwrap();
    
    let entity_names: Vec<_> = entities.iter().map(|e| &e.name).collect();
    assert!(!entity_names.contains(&&"vendorFunc".to_string()), "Should ignore vendor files");
    assert!(entity_names.contains(&&"normalFunc".to_string()), "Should include normal files");

    Ok(())
}

#[test]
fn test_index_const_and_var() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.go");
    std::fs::write(&file_path, r#"
package main

const (
    MaxRetries = 3
    Timeout    = 30
)

var (
    globalCounter int
    config        map[string]string
)

const Version = "1.0.0"
var debug = false
"#)?;

    let mut indexer = GoIndexer::new()?;
    let result = indexer.index_file(file_path.to_str().unwrap())?;

    assert!(result.entities.is_some());
    let entities = result.entities.unwrap();
    
    let const_entities: Vec<_> = entities.iter()
        .filter(|e| e.kind == EntityKind::Const)
        .collect();
    
    assert_eq!(const_entities.len(), 3, "Should find 3 constants");
    
    let var_entities: Vec<_> = entities.iter()
        .filter(|e| e.kind == EntityKind::Var)
        .collect();
    
    assert_eq!(var_entities.len(), 3, "Should find 3 variables");

    Ok(())
}