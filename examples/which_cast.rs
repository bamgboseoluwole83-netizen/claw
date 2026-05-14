use std::process::Command;
fn main() {
    // Test with tokio's spawn
    let out = Command::new("cast").arg("--version").output();
    match out {
        Ok(o) => println!("cast --version: status={}, stdout={}", o.status, String::from_utf8_lossy(&o.stdout).trim()),
        Err(e) => println!("cast --version error: {}", e),
    }
    let out2 = Command::new("which").arg("cast").output();
    match out2 {
        Ok(o) => println!("which cast: status={}, stdout={}", o.status, String::from_utf8_lossy(&o.stdout).trim()),
        Err(e) => println!("which cast error: {}", e),
    }
    let path = std::env::var("PATH").unwrap_or_default();
    println!("PATH: {}", path);
}
