fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let status = std::process::Command::new("pulsarkey")
        .args(&args)
        .status();
    if let Ok(s) = status {
        std::process::exit(s.code().unwrap_or(0));
    }
}
