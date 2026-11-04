use std::fs::File;
use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Context, Result};
use percent_encoding::percent_decode_str;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

pub fn serve(directory: &Path, port: u16, domain: Option<&str>) -> Result<()> {
    if port == 0 {
        bail!("static sites require a port between 1 and 65535");
    }
    let root = directory
        .canonicalize()
        .context("could not resolve site directory")?;
    if !root.is_dir() {
        bail!("static site root must be a directory");
    }
    let server = Server::http((std::net::Ipv4Addr::LOCALHOST, port)).map_err(|error| {
        anyhow::anyhow!("could not start static server on port {port}: {error}")
    })?;
    println!("Serving {} at http://localhost:{port}", root.display());
    for request in server.incoming_requests() {
        handle(request, &root, port, domain);
    }
    Ok(())
}

fn handle(request: Request, root: &Path, port: u16, domain: Option<&str>) {
    let host = request
        .headers()
        .iter()
        .find(|header| header.field.equiv("Host"))
        .map(|header| header.value.as_str());
    let allowed = [
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        format!("localhost:{port}"),
        format!("127.0.0.1:{port}"),
    ];
    if !host.is_some_and(|host| {
        allowed.iter().any(|value| host.eq_ignore_ascii_case(value))
            || domain.is_some_and(|domain| {
                host.eq_ignore_ascii_case(domain)
                    || host.eq_ignore_ascii_case(&format!("{domain}:{port}"))
            })
    }) {
        respond_error(request, 403);
        return;
    }
    if !matches!(request.method(), Method::Get | Method::Head) {
        let response = Response::from_string("Method not allowed\n")
            .with_status_code(StatusCode(405))
            .with_header(header("Allow", "GET, HEAD"));
        let _ = request.respond(response);
        return;
    }
    let raw_path = request.url().split('?').next().unwrap_or("/");
    if raw_path.len() > 8192 {
        respond_error(request, 414);
        return;
    }
    let path = match resolve_path(root, raw_path) {
        Ok(path) => path,
        Err(status) => {
            respond_error(request, status);
            return;
        }
    };
    if path.is_dir() && !raw_path.ends_with('/') {
        let query = request
            .url()
            .split_once('?')
            .map(|(_, query)| format!("?{query}"))
            .unwrap_or_default();
        let location = format!("{raw_path}/{query}");
        let Ok(location) = Header::from_bytes("Location", location) else {
            respond_error(request, 400);
            return;
        };
        let response = Response::empty(StatusCode(301)).with_header(location);
        let _ = request.respond(response);
        return;
    }
    let path = if path.is_dir() {
        path.join("index.html")
    } else {
        path
    };
    let path = match path.canonicalize() {
        Ok(path) if path.starts_with(root) && path.is_file() => path,
        _ => {
            respond_error(request, 404);
            return;
        }
    };
    match File::open(&path) {
        Ok(file) => {
            let mime = mime_guess::from_path(&path).first_or_octet_stream();
            let response = Response::from_file(file)
                .with_header(header("Content-Type", mime.as_ref()))
                .with_header(header("X-Content-Type-Options", "nosniff"))
                .with_header(header("Cache-Control", "no-cache"));
            let _ = request.respond(response);
        }
        Err(_) => respond_error(request, 404),
    }
}

fn resolve_path(root: &Path, raw: &str) -> std::result::Result<PathBuf, u16> {
    if !raw.starts_with('/') || raw.starts_with("//") {
        return Err(400);
    }
    let decoded = percent_decode_str(raw).decode_utf8().map_err(|_| 400u16)?;
    if decoded.contains('\0') || decoded.contains('\\') {
        return Err(400);
    }
    let relative = Path::new(decoded.strip_prefix('/').ok_or(400u16)?);
    for component in relative.components() {
        match component {
            Component::Normal(name) if !name.to_string_lossy().starts_with('.') => {}
            Component::CurDir => {}
            _ => return Err(403),
        }
    }
    let resolved = root.join(relative).canonicalize().map_err(|_| 404u16)?;
    if !resolved.starts_with(root) {
        return Err(403);
    }
    Ok(resolved)
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name, value).expect("static HTTP headers must be valid")
}

fn respond_error(request: Request, code: u16) {
    let response = Response::from_string(format!("{code}\n"))
        .with_status_code(StatusCode(code))
        .with_header(header("Content-Type", "text/plain; charset=utf-8"));
    let _ = request.respond(response);
}
