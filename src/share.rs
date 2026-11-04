// Kirim file ke HP: server HTTP kecil di jaringan lokal (Wi-Fi yang sama).
// Alamatnya memuat token acak; hanya file di shelf yang bisa diunduh, dan hanya saat sakelar aktif.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn local_ip() -> Option<String> {
    // Tidak mengirim paket apa pun; hanya menanyakan antarmuka mana yang akan dipakai.
    let s = UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("8.8.8.8:80").ok()?;
    Some(s.local_addr().ok()?.ip().to_string())
}

fn token() -> String {
    let mut h = RandomState::new().build_hasher();
    h.write_u64(std::process::id() as u64);
    format!("{:016x}", h.finish())
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn respond(s: &mut TcpStream, status: &str, ctype: &str, body: &[u8]) {
    let head = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        status,
        ctype,
        body.len()
    );
    let _ = s.write_all(head.as_bytes());
    let _ = s.write_all(body);
}

fn handle(mut s: TcpStream, tok: &str, files: &Arc<Mutex<Vec<PathBuf>>>) {
    let _ = s.set_read_timeout(Some(Duration::from_secs(5)));
    let mut line = String::new();
    {
        let mut r = BufReader::new(&s);
        if r.read_line(&mut line).is_err() {
            return;
        }
    }
    let path = line.split_whitespace().nth(1).unwrap_or("/").to_string();
    let prefix = format!("/{}", tok);
    if !path.starts_with(&prefix) {
        respond(&mut s, "404 Not Found", "text/plain", b"Not found");
        return;
    }
    let rest = &path[prefix.len()..];
    let list: Vec<PathBuf> = files.lock().map(|g| g.clone()).unwrap_or_default();

    if rest.is_empty() || rest == "/" {
        let mut html = String::from(
            "<!doctype html><html><head><meta charset=utf-8><meta name=viewport content='width=device-width,initial-scale=1'>\
             <title>RafLand</title><style>body{font-family:system-ui;background:#14141c;color:#eee;padding:20px}\
             a{display:block;padding:14px 16px;margin:8px 0;background:#26263a;border-radius:14px;color:#cdb8ff;text-decoration:none}\
             </style></head><body><h2>RafLand Shelf</h2>",
        );
        if list.is_empty() {
            html.push_str("<p>Belum ada file di shelf.</p>");
        }
        for (i, p) in list.iter().enumerate() {
            let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            html.push_str(&format!("<a href='{}/f/{}'>{}</a>", prefix, i, html_escape(&name)));
        }
        html.push_str("</body></html>");
        respond(&mut s, "200 OK", "text/html; charset=utf-8", html.as_bytes());
        return;
    }

    if let Some(idx) = rest.strip_prefix("/f/").and_then(|x| x.parse::<usize>().ok()) {
        if let Some(p) = list.get(idx) {
            if let Ok(mut f) = std::fs::File::open(p) {
                let len = f.metadata().map(|m| m.len()).unwrap_or(0);
                let name: String = p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "file".into())
                    .chars()
                    .map(|c| if c.is_ascii() && c != '"' && !c.is_control() { c } else { '_' })
                    .collect();
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\
                     Content-Disposition: attachment; filename=\"{}\"\r\nConnection: close\r\n\r\n",
                    len, name
                );
                let _ = s.write_all(head.as_bytes());
                let _ = std::io::copy(&mut f, &mut s);
                return;
            }
        }
    }
    respond(&mut s, "404 Not Found", "text/plain", b"Not found");
}

/// Mulai server (sekali). Mengembalikan alamat yang dibuka dari HP.
pub fn start(files: Arc<Mutex<Vec<PathBuf>>>, on: Arc<AtomicBool>) -> Option<String> {
    let listener = TcpListener::bind("0.0.0.0:0").ok()?;
    let port = listener.local_addr().ok()?.port();
    let ip = local_ip()?;
    let tok = token();
    let url = format!("http://{}:{}/{}/", ip, port, tok);
    thread::spawn(move || {
        for conn in listener.incoming() {
            let Ok(stream) = conn else { continue };
            if !on.load(Ordering::Relaxed) {
                continue; // sakelar mati: tutup koneksi
            }
            let files = files.clone();
            let tok = tok.clone();
            thread::spawn(move || handle(stream, &tok, &files));
        }
    });
    Some(url)
}
