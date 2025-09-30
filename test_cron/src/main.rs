use cron::Schedule;
use std::str::FromStr;

fn main() {
    let test_expressions = vec![
        "* * * * *",
        "0 * * * *",
        "*/1 * * * *",
        "0 */1 * * *",
        "0 0 * * *",
        "0 0 1 * *",
        "0 0 * * 1",
    ];
    
    for cron_expr in test_expressions {
        println!("\nTesting cron expression: '{}'", cron_expr);
        
        match Schedule::from_str(cron_expr) {
            Ok(schedule) => {
                println!("✅ Cron expression is valid!");
                
                // Test next run
                if let Some(next_run) = schedule.upcoming(chrono::Utc).next() {
                    println!("Next run: {}", next_run);
                } else {
                    println!("❌ Could not calculate next run");
                }
            }
            Err(e) => {
                println!("❌ Cron expression is invalid: {}", e);
            }
        }
    }
}
