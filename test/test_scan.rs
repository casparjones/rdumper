use std::path::Path;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backup_dir = "/home/frank/projects/rdumper/backend/data/backups";
    println!("Testing backup directory: {}", backup_dir);
    
    if !Path::new(backup_dir).exists() {
        println!("❌ Directory does not exist!");
        return Ok(());
    }
    
    println!("✅ Directory exists");
    
    let entries = fs::read_dir(backup_dir)?;
    let mut found_backups = 0;
    
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        println!("📁 Found entry: {:?}", path);
        
        if path.is_dir() {
            let meta_file = path.join("rdumper.backup.json");
            println!("  🔍 Checking for metadata: {:?}", meta_file);
            if meta_file.exists() {
                println!("  ✅ Found metadata file!");
                found_backups += 1;
            } else {
                println!("  ❌ No metadata file");
            }
        }
    }
    
    println!("📊 Total backup folders found: {}", found_backups);
    Ok(())
}
