fn main() {
    if std::env::args().any(|argument| argument == "--scriptotar-installed-backend-smoke") {
        if let Err(error) = scriptotar_desktop::run_installed_backend_smoke() {
            eprintln!("installed backend smoke failed: {error}");
            std::process::exit(1);
        }
        return;
    }

    scriptotar_desktop::run();
}
