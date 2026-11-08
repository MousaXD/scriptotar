fn main() {
    let result = if std::env::args().any(|arg| arg == "--mcp-http") {
        scriptotar_mcp::server::run_http_from_env()
    } else {
        scriptotar_mcp::server::run_stdio_from_env()
    };
    if let Err(error) = result {
        eprintln!("[scriptotar-mcp] startup failed: {error}");
        std::process::exit(1);
    }
}
