//! Development bridge: `POST /api` with a Request JSON → Response JSON.
//! In the desktop app the same `Engine::dispatch_json` is called through Tauri IPC.
//! Optionally serves the built frontend (`app/dist`) for a single-process demo.

use aic_api::Engine;
use std::path::{Path, PathBuf};
use tiny_http::{Header, Method, Response, Server};

fn header(k: &str, v: &str) -> Header {
    Header::from_bytes(k.as_bytes(), v.as_bytes()).unwrap()
}

fn content_type(p: &Path) -> &'static str {
    match p.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript",
        Some("css") => "text/css",
        Some("svg") => "image/svg+xml",
        Some("json") => "application/json",
        Some("png") => "image/png",
        _ => "application/octet-stream",
    }
}

fn main() {
    // Cổng API: AIC_ADDR (địa chỉ đầy đủ) hoặc AIC_PORT, mặc định 127.0.0.1:8790.
    let port = std::env::var("AIC_PORT").unwrap_or_else(|_| "8790".into());
    let addr = std::env::var("AIC_ADDR").unwrap_or_else(|_| format!("127.0.0.1:{port}"));
    // Serve the built UI when available: AIC_STATIC, else <repo>/app/dist.
    let static_dir: Option<PathBuf> = std::env::var("AIC_STATIC")
        .ok()
        .map(PathBuf::from)
        .or_else(|| Some(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app/dist")))
        .and_then(|p| p.canonicalize().ok())
        .filter(|p| p.join("index.html").is_file());
    let server = Server::http(&addr).expect("bind");
    eprintln!("AIC CAD core: http://{addr}/api");
    match &static_dir {
        Some(d) => eprintln!("Giao diện: http://{addr}  (phục vụ từ {})", d.display()),
        None => eprintln!("Giao diện chưa build. Chạy `cd app && npm run dev` rồi mở http://localhost:5173"),
    }
    let mut engine = Engine::new();
    // Thư viện mẫu dùng chung mọi dự án (template, preset, mẫu vùng, mẫu kết cấu).
    let lib = aic_api::library::default_library_path();
    if let Some(p) = &lib {
        eprintln!("Thư viện mẫu: {}", p.display());
    }
    engine.set_library_path(lib);
    for mut req in server.incoming_requests() {
        let cors = [
            header("Access-Control-Allow-Origin", "*"),
            header("Access-Control-Allow-Headers", "content-type"),
            header("Access-Control-Allow-Methods", "POST, GET, OPTIONS"),
        ];
        let url = req.url().to_string();
        let resp = match (req.method(), url.as_str()) {
            (Method::Options, _) => Response::from_string(""),
            (Method::Post, "/api") => {
                let mut body = String::new();
                if req.as_reader().read_to_string(&mut body).is_err() {
                    Response::from_string("bad body").with_status_code(400)
                } else {
                    let out = engine.dispatch_json(&body);
                    Response::from_string(out).with_header(header("Content-Type", "application/json"))
                }
            }
            (Method::Get, path) if static_dir.is_some() => {
                let dir = static_dir.as_ref().unwrap();
                let rel = path.split('?').next().unwrap_or("/").trim_start_matches('/');
                let mut p = dir.join(if rel.is_empty() { "index.html" } else { rel });
                if !p.starts_with(dir) || !p.is_file() {
                    p = dir.join("index.html");
                }
                match std::fs::read(&p) {
                    Ok(bytes) => Response::from_data(bytes).with_header(header("Content-Type", content_type(&p))),
                    Err(_) => Response::from_data(b"not found".to_vec()).with_status_code(404),
                }
            }
            (Method::Get, _) => Response::from_string(HELP_PAGE).with_header(header("Content-Type", "text/html; charset=utf-8")),
            _ => Response::from_data(b"AIC dev server: POST /api".to_vec()).with_status_code(404),
        };
        let mut resp = resp;
        for h in cors {
            resp.add_header(h);
        }
        let _ = req.respond(resp);
    }
}

const HELP_PAGE: &str = r#"<!doctype html><html lang="vi"><meta charset="utf-8"><title>AIC CAD core</title>
<body style="font-family:system-ui,sans-serif;max-width:640px;margin:60px auto;color:#212529;line-height:1.6">
<h2 style="color:#e8590c">AIC CAD – lõi CAD đang chạy</h2>
<p>Cổng này là <b>API của lõi CAD</b> (<code>POST /api</code>), không phải giao diện.</p>
<p><b>Cách 1 – chế độ phát triển:</b> mở terminal khác và chạy</p>
<pre style="background:#f1f3f5;padding:12px;border-radius:6px">cd app
npm install
npm run dev</pre>
<p>rồi mở <a href="http://localhost:5173">http://localhost:5173</a>.</p>
<p><b>Cách 2 – một cổng duy nhất:</b> build giao diện (<code>cd app &amp;&amp; npm run build</code>), khởi động lại lõi,
rồi tải lại trang này.</p>
</body></html>"#;
