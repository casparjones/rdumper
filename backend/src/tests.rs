use std::fs;
use tempfile::TempDir;
use crate::services::FilesystemBackupService;
use crate::models::{DatabaseConfig, Task, BackupMetadata};

#[tokio::test]
async fn test_backup_process_creates_single_folder() {
    // Setup: Create temporary directory for testing
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let backup_base_dir = temp_dir.path().join("backups");
    fs::create_dir_all(&backup_base_dir).expect("Failed to create backup dir");
    
    let service = FilesystemBackupService::new(backup_base_dir.to_string_lossy().to_string());
    
    // Create test database config
    let db_config = DatabaseConfig {
        id: "test-db-1".to_string(),
        name: "Test Database".to_string(),
        host: "localhost".to_string(),
        port: 3306,
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        database_name: "testdb".to_string(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    
    // Create test task
    let task = Task {
        id: "test-task-1".to_string(),
        name: "Test Task".to_string(),
        database_config_id: "test-db-1".to_string(),
        cron_schedule: "0 0 * * *".to_string(),
        compression_type: "gzip".to_string(),
        use_non_transactional: false,
        cleanup_days: 30,
        is_active: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    
    // Test: Create backup process
    let backup_id = "test-backup-123";
    let backup_process = service.create_backup_process(backup_id, &db_config, Some(&task)).await
        .expect("Failed to create backup process");
    
    // Verify: Only one folder should be created
    let backup_dirs: Vec<_> = fs::read_dir(&backup_base_dir)
        .expect("Failed to read backup dir")
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .collect();
    
    assert_eq!(backup_dirs.len(), 1, "Should create exactly one backup folder");
    
    // Verify: The folder should be named with the backup_id
    let backup_folder = backup_dirs[0].path();
    assert_eq!(backup_folder.file_name().unwrap(), backup_id);
    
    // Verify: tmp folder should exist
    let tmp_folder = backup_folder.join("tmp");
    assert!(tmp_folder.exists(), "tmp folder should exist");
    assert!(tmp_folder.is_dir(), "tmp should be a directory");
    
    // Verify: rdumper.backup.json should exist
    let meta_file = backup_folder.join("rdumper.backup.json");
    assert!(meta_file.exists(), "rdumper.backup.json should exist");
    
    // Verify: No backup archive should exist yet
    let backup_files: Vec<_> = fs::read_dir(&backup_folder)
        .expect("Failed to read backup folder")
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .filter(|entry| {
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();
            name.ends_with(".tar.gz") || name.ends_with(".tar.zst") || name.ends_with(".tar")
        })
        .collect();
    
    assert_eq!(backup_files.len(), 0, "No backup archive should exist yet");
}

#[tokio::test]
async fn test_backup_process_completes_successfully() {
    // Setup: Create temporary directory for testing
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let backup_base_dir = temp_dir.path().join("backups");
    fs::create_dir_all(&backup_base_dir).expect("Failed to create backup dir");
    
    let service = FilesystemBackupService::new(backup_base_dir.to_string_lossy().to_string());
    
    // Create test database config
    let db_config = DatabaseConfig {
        id: "test-db-1".to_string(),
        name: "Test Database".to_string(),
        host: "localhost".to_string(),
        port: 3306,
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        database_name: "testdb".to_string(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    
    // Create test task
    let task = Task {
        id: "test-task-1".to_string(),
        name: "Test Task".to_string(),
        database_config_id: "test-db-1".to_string(),
        cron_schedule: "0 0 * * *".to_string(),
        compression_type: "gzip".to_string(),
        use_non_transactional: false,
        cleanup_days: 30,
        is_active: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    
    // Test: Create and complete backup process
    let backup_id = "test-backup-456";
    let mut backup_process = service.create_backup_process(backup_id, &db_config, Some(&task)).await
        .expect("Failed to create backup process");
    
    // Simulate mydumper output by creating some test files in tmp/
    let tmp_dir = backup_process.tmp_dir();
    fs::write(tmp_dir.join("database.sql"), "CREATE DATABASE testdb;").expect("Failed to write test file");
    fs::write(tmp_dir.join("table1.sql"), "CREATE TABLE table1 (id INT);").expect("Failed to write test file");
    fs::write(tmp_dir.join("table2.sql"), "CREATE TABLE table2 (name VARCHAR(100));").expect("Failed to write test file");
    
    // Complete the backup process
    backup_process.complete().await.expect("Failed to complete backup");
    
    // Verify: Backup archive should exist with correct naming
    let backup_folder = backup_base_dir.join(backup_id);
    let backup_files: Vec<_> = fs::read_dir(&backup_folder)
        .expect("Failed to read backup folder")
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .filter(|entry| {
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();
            name.ends_with(".tar.gz") || name.ends_with(".tar.zst") || name.ends_with(".tar")
        })
        .collect();
    
    assert_eq!(backup_files.len(), 1, "Should create exactly one backup archive");
    
    let backup_file = &backup_files[0];
    let file_name_os = backup_file.file_name();
    let file_name = file_name_os.to_string_lossy();
    assert!(file_name.starts_with("testdb-"), "Backup file should start with database name");
    assert!(file_name.ends_with(".tar.gz"), "Backup file should end with .tar.gz for gzip compression");
    
    // Verify: tmp folder should be deleted
    let tmp_folder = backup_folder.join("tmp");
    assert!(!tmp_folder.exists(), "tmp folder should be deleted after completion");
    
    // Verify: rdumper.backup.json should be updated with correct data
    let meta_file = backup_folder.join("rdumper.backup.json");
    let meta_content = fs::read_to_string(&meta_file).expect("Failed to read metadata file");
    let metadata: BackupMetadata = serde_json::from_str(&meta_content).expect("Failed to parse metadata");
    
    assert_eq!(metadata.database_name, "testdb");
    assert_eq!(metadata.compression_type, "gzip");
    assert!(metadata.sha256_hash.is_some(), "SHA-256 hash should be calculated");
    assert!(metadata.file_size > 0, "File size should be greater than 0");
    assert_eq!(metadata.file_path, backup_file.path().to_string_lossy());
}

#[tokio::test]
async fn test_backup_process_handles_different_compression_types() {
    // Setup: Create temporary directory for testing
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let backup_base_dir = temp_dir.path().join("backups");
    fs::create_dir_all(&backup_base_dir).expect("Failed to create backup dir");
    
    let service = FilesystemBackupService::new(backup_base_dir.to_string_lossy().to_string());
    
    // Create test database config
    let db_config = DatabaseConfig {
        id: "test-db-1".to_string(),
        name: "Test Database".to_string(),
        host: "localhost".to_string(),
        port: 3306,
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        database_name: "testdb".to_string(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    
    // Test different compression types
    let compression_types = vec!["gzip", "zstd", "none"];
    
    for (i, compression_type) in compression_types.iter().enumerate() {
        // Create test task with different compression
        let task = Task {
            id: format!("test-task-{}", i),
            name: format!("Test Task {}", i),
            database_config_id: "test-db-1".to_string(),
            cron_schedule: "0 0 * * *".to_string(),
            compression_type: compression_type.to_string(),
            use_non_transactional: false,
            cleanup_days: 30,
            is_active: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        
        // Test: Create and complete backup process
        let backup_id = format!("test-backup-{}", i);
        let mut backup_process = service.create_backup_process(&backup_id, &db_config, Some(&task)).await
            .expect("Failed to create backup process");
        
        // Simulate mydumper output
        let tmp_dir = backup_process.tmp_dir();
        fs::write(tmp_dir.join("test.sql"), "SELECT 1;").expect("Failed to write test file");
        
        // Complete the backup process
        backup_process.complete().await.expect("Failed to complete backup");
        
        // Verify: Backup archive should exist with correct extension
        let backup_folder = backup_base_dir.join(&backup_id);
        let backup_files: Vec<_> = fs::read_dir(&backup_folder)
            .expect("Failed to read backup folder")
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().is_file())
            .filter(|entry| {
                let file_name = entry.file_name();
                let name = file_name.to_string_lossy();
                name.ends_with(".tar.gz") || name.ends_with(".tar.zst") || name.ends_with(".tar")
            })
            .collect();
        
        assert_eq!(backup_files.len(), 1, "Should create exactly one backup archive");
        
        let backup_file = &backup_files[0];
        let file_name_os = backup_file.file_name();
        let file_name = file_name_os.to_string_lossy();
        
        match *compression_type {
            "gzip" => assert!(file_name.ends_with(".tar.gz"), "Should create .tar.gz for gzip"),
            "zstd" => assert!(file_name.ends_with(".tar.zst"), "Should create .tar.zst for zstd"),
            "none" => assert!(file_name.ends_with(".tar"), "Should create .tar for none"),
            _ => panic!("Unknown compression type"),
        }
    }
}
