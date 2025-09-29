use std::path::Path;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backup_dir = "./data/backups";
    println!("Scanning directory: {}", backup_dir);
    
    if !Path::new(backup_dir).exists() {
        println!("Directory does not exist!");
        return Ok(());
    }
    
    let entries = fs::read_dir(backup_dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        println!("Found entry: {:?}", path);
        
        if path.is_dir() {
            let meta_file = path.join("rdumper.backup.json");
            println!("  Checking for metadata: {:?}", meta_file);
            if meta_file.exists() {
                println!("  ✓ Found metadata file!");
            } else {
                println!("  ✗ No metadata file");
            }
        }
    }
    
    Ok(())
}