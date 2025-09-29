use axum::{
    routing::get,
    Router,
};
use serde_json::json;
use std::process::Command;

use super::{ApiResult, success_response};

pub fn routes() -> Router {
    Router::new()
        .route("/info", get(get_system_info))
        .route("/version", get(get_version_info))
        .route("/health", get(get_health_status))
        .route("/mydumper/version", get(get_mydumper_version))
        .route("/myloader/version", get(get_myloader_version))
}

async fn get_system_info() -> ApiResult<impl axum::response::IntoResponse> {
    let os_info = get_os_info();
    let kernel_version = get_kernel_version();
    let uptime = get_system_uptime();
    let memory_info = get_memory_info();

    Ok(success_response(json!({
        "os": os_info,
        "kernel": kernel_version,
        "uptime": uptime,
        "memory": memory_info,
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

async fn get_version_info() -> ApiResult<impl axum::response::IntoResponse> {
    let app_version = env!("CARGO_PKG_VERSION");
    let git_commit = get_git_commit();
    let build_date = get_build_date();

    Ok(success_response(json!({
        "app_version": app_version,
        "git_commit": git_commit,
        "build_date": build_date,
        "rust_version": get_rust_version(),
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

async fn get_health_status() -> ApiResult<impl axum::response::IntoResponse> {
    let mydumper_available = check_mydumper_available();
    let myloader_available = check_myloader_available();
    let disk_space = get_disk_space();

    let overall_status = if mydumper_available && myloader_available {
        "healthy"
    } else {
        "degraded"
    };

    Ok(success_response(json!({
        "status": overall_status,
        "checks": {
            "mydumper": mydumper_available,
            "myloader": myloader_available,
            "disk_space": disk_space
        },
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

async fn get_mydumper_version() -> ApiResult<impl axum::response::IntoResponse> {
    let version = get_tool_version("mydumper");
    
    Ok(success_response(json!({
        "tool": "mydumper",
        "version": version,
        "available": version.is_some(),
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

async fn get_myloader_version() -> ApiResult<impl axum::response::IntoResponse> {
    let version = get_tool_version("myloader");
    
    Ok(success_response(json!({
        "tool": "myloader",
        "version": version,
        "available": version.is_some(),
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

// Helper functions

fn get_os_info() -> serde_json::Value {
    let output = Command::new("cat")
        .arg("/etc/os-release")
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let content = String::from_utf8_lossy(&output.stdout);
            let mut info = serde_json::Map::new();
            
            for line in content.lines() {
                if let Some((key, value)) = line.split_once('=') {
                    let value = value.trim_matches('"');
                    info.insert(key.to_lowercase(), json!(value));
                }
            }
            
            json!(info)
        }
        _ => {
            json!({
                "name": "Unknown",
                "version": "Unknown"
            })
        }
    }
}

fn get_kernel_version() -> String {
    let output = Command::new("uname")
        .arg("-r")
        .output();

    match output {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        _ => "Unknown".to_string()
    }
}

fn get_system_uptime() -> Option<String> {
    let output = Command::new("uptime")
        .arg("-p")
        .output();

    match output {
        Ok(output) if output.status.success() => {
            Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
        }
        _ => None
    }
}

fn get_memory_info() -> serde_json::Value {
    let output = Command::new("cat")
        .arg("/proc/meminfo")
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let content = String::from_utf8_lossy(&output.stdout);
            let mut info = serde_json::Map::new();
            
            for line in content.lines() {
                if let Some((key, value)) = line.split_once(':') {
                    let value = value.trim().split_whitespace().next().unwrap_or("0");
                    if let Ok(kb) = value.parse::<u64>() {
                        info.insert(key.to_lowercase().replace("(", "").replace(")", ""), json!(kb * 1024)); // Convert to bytes
                    }
                }
            }
            
            json!(info)
        }
        _ => json!({})
    }
}

fn get_git_commit() -> Option<String> {
    // Try environment variable first (set during build)
    if let Some(commit) = option_env!("GIT_COMMIT") {
        return Some(commit.to_string());
    }
    
    // Fallback to git command (for development)
    let output = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
        }
        _ => None
    }
}

fn get_build_date() -> Option<String> {
    // This would typically be set during build time
    // For now, we'll return the compile time
    option_env!("BUILD_DATE").map(|s| s.to_string())
}

fn get_rust_version() -> String {
    // Try environment variable first (set during build)
    if let Some(version) = option_env!("RUSTC_VERSION") {
        return version.to_string();
    }
    
    // Fallback to runtime detection
    std::env::var("RUSTC_VERSION").unwrap_or_else(|_| "Unknown".to_string())
}

fn check_mydumper_available() -> bool {
    Command::new("mydumper")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn check_myloader_available() -> bool {
    Command::new("myloader")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn get_tool_version(tool: &str) -> Option<String> {
    let output = Command::new(tool)
        .arg("--version")
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let version_output = String::from_utf8_lossy(&output.stdout);
            // Parse version from output (this might need adjustment based on actual mydumper/myloader output format)
            version_output
                .lines()
                .next()
                .map(|line| line.trim().to_string())
        }
        _ => None
    }
}

fn get_disk_space() -> serde_json::Value {
    let output = Command::new("df")
        .args(&["-h", "/"])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let content = String::from_utf8_lossy(&output.stdout);
            if let Some(line) = content.lines().nth(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    return json!({
                        "filesystem": parts[0],
                        "size": parts[1],
                        "used": parts[2],
                        "available": parts[3],
                        "use_percentage": parts[4]
                    });
                }
            }
            json!({})
        }
        _ => json!({})
    }
}