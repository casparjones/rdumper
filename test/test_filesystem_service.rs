use std::path::Path;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backup_dir = "/home/frank/projects/rdumper/backend/data/backups";
    println!("Testing FilesystemBackupService with directory: {}", backup_dir);
    
    if !Path::new(backup_dir).exists() {
        println!("❌ Directory does not exist!");
        return Ok(());
    }
    
    println!("✅ Directory exists");
    
    let mut backups = Vec::new();
    scan_directory_recursive(Path::new(backup_dir), &mut backups)?;
    
    println!("📊 Found {} backups:", backups.len());
    for backup in &backups {
        println!("  - {}: {}", backup.0, backup.1);
    }
    
    Ok(())
}

fn scan_directory_recursive(dir_path: &Path, backups: &mut Vec<(String, String)>) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Scanning directory: {:?}", dir_path);
    
    let entries = fs::read_dir(dir_path)?;
    
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        println!("  📁 Found entry: {:?}", path);
        
        if path.is_dir() {
            println!("    🔍 Checking for metadata: {:?}", path.join("rdumper.backup.json"));
            let meta_file = path.join("rdumper.backup.json");
            if meta_file.exists() {
                println!("    ✅ Found metadata file!");
                
                // Load metadata
                let metadata_content = fs::read_to_string(&meta_file)?;
                println!("    📄 Metadata content: {}", metadata_content);
                
                // Find backup file
                let backup_file = find_backup_file_in_folder(&path)?;
                if let Some(backup_file) = backup_file {
                    println!("    📦 Found backup file: {:?}", backup_file);
                    
                    let backup_id = path.file_name().unwrap().to_string_lossy().to_string();
                    let backup_path = backup_file.to_string_lossy().to_string();
                    backups.push((backup_id, backup_path));
                } else {
                    println!("    ❌ No backup file found in folder");
                }
            } else {
                println!("    ❌ No metadata file");
            }
        }
    }
    
    Ok(())
}

fn find_backup_file_in_folder(folder_path: &Path) -> Result<Option<std::path::PathBuf>, Box<dyn std::error::Error>> {
    let entries = fs::read_dir(folder_path)?;
    
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() {
            if let Some(extension) = path.extension() {
                if extension == "gz" || extension == "zst" || extension == "tar" {
                    println!("      🎯 Found backup file: {:?}", path);
                    return Ok(Some(path));
                }
            }
        }
    }
    
    Ok(None)
}