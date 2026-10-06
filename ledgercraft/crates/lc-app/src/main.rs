//! LedgerCraft desktop app. Double-click: starts the local app and opens it in
//! the default browser. Keep the small window open while you work; close it to
//! quit (or use "Quit" in the app).

use lc_app::server;

use std::path::PathBuf;
use std::sync::Arc;

fn default_data_dir() -> PathBuf {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let docs = home.join("Documents");
    if docs.exists() {
        docs.join("LedgerCraft Data")
    } else {
        home.join("LedgerCraft Data")
    }
}

fn open_browser(url: &str) {
    #[cfg(target_os = "windows")]
    let r = std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .spawn();
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(url).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let r = std::process::Command::new("xdg-open").arg(url).spawn();
    if r.is_err() {
        println!("Open this address in your browser: {url}");
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let get = |k: &str| {
        args.iter()
            .position(|a| a == k)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let data = get("--data")
        .map(PathBuf::from)
        .unwrap_or_else(default_data_dir);
    let ollama = get("--ollama").unwrap_or_else(|| "http://127.0.0.1:11434".into());
    // Client data must not leave this computer unless the user says so.
    if !server::is_local_url(&ollama) && !args.iter().any(|a| a == "--allow-remote-ai") {
        eprintln!("The AI address {ollama} is not on this computer. Client data would be sent there. Start with --allow-remote-ai only if you accept that.");
        std::process::exit(2);
    }
    let port: u16 = get("--port").and_then(|p| p.parse().ok()).unwrap_or(7878);
    let browser = !args.iter().any(|a| a == "--no-browser");
    let app = match server::App::new(data.clone(), &ollama) {
        Ok(a) => Arc::new(a),
        Err(e) => {
            eprintln!("LedgerCraft could not start: {e}");
            std::process::exit(1);
        }
    };
    println!(
        "LedgerCraft {} – offline financial statements",
        env!("CARGO_PKG_VERSION")
    );
    println!("Data folder: {}", data.display());
    let token = app.token.clone();
    let ready = move |addr: String| {
        let url = format!("{addr}/");
        println!("Running at {url}  (keep this window open; close it to quit)");
        let _ = &token;
        if browser {
            open_browser(&url);
        }
    };
    let result = server::serve(app.clone(), port, ready).or_else(|_| {
        server::serve(app, 0, |addr| {
            let url = format!("{addr}/");
            println!("Running at {url}  (keep this window open; close it to quit)");
            if browser {
                open_browser(&url);
            }
        })
    });
    if let Err(e) = result {
        eprintln!("LedgerCraft stopped: {e}");
        std::process::exit(1);
    }
}
