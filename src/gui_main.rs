fn main() {
    if let Err(e) = pulsarkey::gui::run_gui() {
        eprintln!("Failed to launch PulsarKey Settings GUI: {}", e);
        std::process::exit(1);
    }
}
