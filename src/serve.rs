//! A tiny static server for the fixture site, so the benchmark runs on any machine without
//! network access. It understands a Netlify-style `_redirects` file ("from to 301") and replaces
//! the fixture's home origin with its own address in HTML, XML and text files.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};

/// The origin written in the fixture files. `.example` is reserved, so it never resolves.
pub const FIXTURE_ORIGIN: &str = "https://lune-et-lin.example";

pub struct Server {
    pub origin: String,
    listener: TcpListener,
    root: PathBuf,
}

impl Server {
    /// Bind on 127.0.0.1 (port 0 picks a free port).
    pub fn bind(root: &Path, port: u16) -> std::io::Result<Server> {
        let listener = TcpListener::bind(("127.0.0.1", port))?;
        let origin = format!("http://{}", listener.local_addr()?);
        Ok(Server { origin, listener, root: root.to_path_buf() })
    }

    /// Serve forever.
    pub fn run(self) {
        for stream in self.listener.incoming().flatten() {
            let root = self.root.clone();
            let origin = self.origin.clone();
            std::thread::spawn(move || {
                let _ = handle(stream, &root, &origin);
            });
        }
    }

    /// Serve in a background thread and return the origin.
    pub fn spawn(self) -> String {
        let origin = self.origin.clone();
        std::thread::spawn(move || self.run());
        origin
    }
}

fn redirect_for(root: &Path, path: &str) -> Option<(String, u16)> {
    let rules = std::fs::read_to_string(root.join("_redirects")).ok()?;
    rules.lines().find_map(|l| {
        let parts: Vec<&str> = l.split_whitespace().collect();
        match parts.as_slice() {
            [from, to, code] if *from == path => Some((to.to_string(), code.parse().unwrap_or(301))),
            [from, to] if *from == path => Some((to.to_string(), 301)),
            _ => None,
        }
    })
}

fn handle(stream: TcpStream, root: &Path, origin: &str) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request = String::new();
    reader.read_line(&mut request)?;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 || line == "\r\n" || line == "\n" {
            break;
        }
    }
    let path = request.split_whitespace().nth(1).unwrap_or("/").split('?').next().unwrap_or("/").to_string();
    let mut out = stream;
    if let Some((to, code)) = redirect_for(root, &path) {
        let head = format!("HTTP/1.1 {code} Moved\r\nLocation: {to}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        return out.write_all(head.as_bytes());
    }
    let mut file = root.join(path.trim_start_matches('/'));
    if path.ends_with('/') {
        file = file.join("index.html");
    }
    let inside = file.canonicalize().ok().zip(root.canonicalize().ok()).map(|(f, r)| f.starts_with(r)).unwrap_or(false);
    let body = if inside && file.is_file() { std::fs::read(&file).ok() } else { None };
    let (status, ctype, body) = match body {
        Some(bytes) => {
            let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
            let ctype = match ext {
                "html" => "text/html; charset=utf-8",
                "xml" => "application/xml; charset=utf-8",
                "txt" => "text/plain; charset=utf-8",
                "json" => "application/json",
                "svg" => "image/svg+xml",
                _ => "application/octet-stream",
            };
            let bytes = if matches!(ext, "html" | "xml" | "txt") {
                String::from_utf8_lossy(&bytes).replace(FIXTURE_ORIGIN, origin).into_bytes()
            } else {
                bytes
            };
            ("200 OK", ctype, bytes)
        }
        None => (
            "404 Not Found",
            "text/html; charset=utf-8",
            b"<!doctype html><title>404</title><h1>Not found</h1>".to_vec(),
        ),
    };
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    out.write_all(head.as_bytes())?;
    out.write_all(&body)
}
