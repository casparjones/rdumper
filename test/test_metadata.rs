use std::path::Path;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let meta_file = "/home/frank/projects/rdumper/backend/data/backups/5b191863-55ca-4338-bbad-2f3548dc31de/rdumper.backup.json";
    println!("Testing metadata loading from: {}", meta_file);
    
    if !Path::new(meta_file).exists() {
        println!("❌ Metadata file does not exist!");
        return Ok(());
    }
    
    println!("✅ Metadata file exists");
    
    let content = fs::read_to_string(meta_file)?;
    println!("📄 Metadata content: {}", content);
    
    // Try to parse as JSON
    let json: serde_json::Value = serde_json::from_str(&content)?;
    println!("✅ Successfully parsed JSON:");
    println!("  - ID: {}", json["id"]);
    println!("  - Database: {}", json["database_name"]);
    println!("  - File size: {}", json["file_size"]);
    println!("  - Created at: {}", json["created_at"]);
    
    Ok(())
}