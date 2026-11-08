pub fn execute(cmd: &str) -> String {
    let output = std::process::Command::new("sh").arg("-c").arg(cmd).output();
    match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(e) => e.to_string(),
    }
}
