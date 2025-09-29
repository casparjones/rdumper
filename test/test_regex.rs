use regex::Regex;

fn main() {
    let test_line = "[2025-09-29 12:10:22] INFO: ** Message: 14:10:22.510: Thread 1: `sbtest`.`sbtest9` [ 0% ] | Tables: 16/16";
    
    let data_pattern = Regex::new(r"Thread \d+: `[^`]+`\.`([^`]+)` \[(\d+)%\]").unwrap();
    
    if let Some(caps) = data_pattern.captures(test_line) {
        println!("Match found!");
        println!("Table name: {}", caps.get(1).unwrap().as_str());
        println!("Progress: {}", caps.get(2).unwrap().as_str());
    } else {
        println!("No match found");
    }
}
