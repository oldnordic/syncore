use std::fs;
use std::path::Path;
use std::process::Command;
use chrono::{Utc, Local};
use anyhow::Result;

pub fn create_daily_backup(db_path: &str, logs_dir: &str) -> Result<()> {
    let today = Local::now().format("%Y-%m-%d").to_string();
    let backup_name = format!("{}.tar.gz", today);
    let backup_path = Path::new("backups").join(&backup_name);
    
    // Create backups directory if it doesn't exist
    if !Path::new("backups").exists() {
        fs::create_dir("backups")?;
    }
    
    // Create tar.gz backup
    let output = Command::new("tar")
        .args(&[
            "-czf",
            backup_path.to_str().unwrap(),
            "--exclude=backups",
            "--exclude=target",
            "--exclude=*.lock",
            Path::new(db_path).file_name().unwrap().to_str().unwrap(),
            logs_dir,
        ])
        .output()?;
    
    if !output.status.success() {
        return Err(anyhow::anyhow!("Backup failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    
    println!("✅ Created backup: {}", backup_path.display());
    
    // Clean old backups (keep last 7 days)
    cleanup_old_backups()?;
    
    Ok(())
}

fn cleanup_old_backups() -> Result<()> {
    let backups_dir = Path::new("backups");
    if !backups_dir.exists() {
        return Ok(());
    }
    
    let cutoff_date = Utc::now() - chrono::Duration::days(7);
    
    for entry in fs::read_dir(backups_dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.extension().and_then(|s| s.to_str()) == Some("gz") {
            if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                if let Ok(date) = chrono::NaiveDate::parse_from_str(name, "%Y-%m-%d") {
                    let datetime = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
                    if datetime < cutoff_date {
                        fs::remove_file(&path)?;
                        println!("🗑️  Removed old backup: {}", path.display());
                    }
                }
            }
        }
    }
    
    Ok(())
}

pub fn verify_integrity(db_path: &str) -> Result<bool> {
    let output = Command::new("sqlite3")
        .args(&[db_path, "PRAGMA integrity_check;"])
        .output()?;
    
    Ok(output.status.success() && String::from_utf8_lossy(&output.stdout).contains("ok"))
}