fn main() {
    if std::env::args().any(|argument| argument == "--mcp-stdio" || argument == "--mcp-http") {
        if std::env::var_os("SCRIPTOTAR_DB_PATH").is_none() {
            let root = std::env::var_os("SCRIPTOTAR_DATA_DIR")
                .map(std::path::PathBuf::from)
                .or_else(|| {
                    #[cfg(target_os = "windows")]
                    {
                        std::env::var_os("APPDATA").map(|p| {
                            std::path::PathBuf::from(p).join("io.github.mousaxd.scriptotar")
                        })
                    }
                    #[cfg(target_os = "linux")]
                    {
                        std::env::var_os("XDG_DATA_HOME")
                            .map(std::path::PathBuf::from)
                            .or_else(|| {
                                std::env::var_os("HOME")
                                    .map(|p| std::path::PathBuf::from(p).join(".local/share"))
                            })
                            .map(|p| p.join("io.github.mousaxd.scriptotar"))
                    }
                    #[cfg(target_os = "macos")]
                    {
                        std::env::var_os("HOME").map(|p| {
                            std::path::PathBuf::from(p)
                                .join("Library/Application Support/io.github.mousaxd.scriptotar")
                        })
                    }
                    #[cfg(not(any(
                        target_os = "windows",
                        target_os = "linux",
                        target_os = "macos"
                    )))]
                    {
                        None
                    }
                });
            let Some(root) = root else {
                eprintln!("MCP requires SCRIPTOTAR_DB_PATH or SCRIPTOTAR_DATA_DIR");
                std::process::exit(1);
            };
            std::env::set_var("SCRIPTOTAR_DB_PATH", root.join("scriptotar.sqlite3"));
        }
        if let Some(database) = std::env::var_os("SCRIPTOTAR_DB_PATH") {
            if let Some(data_dir) = std::path::Path::new(&database).parent() {
                scriptotar_desktop::configure_mcp_cli_runtime(data_dir);
            }
        }
        let result = if std::env::args().any(|arg| arg == "--mcp-http") {
            scriptotar_mcp::server::run_http_from_env()
        } else {
            scriptotar_mcp::server::run_stdio_from_env()
        };
        if let Err(error) = result {
            eprintln!("[scriptotar-mcp] startup failed: {error}");
            std::process::exit(1);
        }
        return;
    }

    if std::env::args().any(|argument| argument == "--scriptotar-installed-backend-smoke") {
        if let Err(error) = scriptotar_desktop::run_installed_backend_smoke() {
            eprintln!("installed backend smoke failed: {error}");
            std::process::exit(1);
        }
        return;
    }

    scriptotar_desktop::run();
}
