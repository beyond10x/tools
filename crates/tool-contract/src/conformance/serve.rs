//! The loopback HTTP/1.1 server a scenario's `fixture.serve` routes run on: `std::net` only, one
//! request per connection, every thread scoped to the scenario so none outlives it.

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, Scope, ScopedJoinHandle};
use std::time::{Duration, Instant};

use serde_json::{Map, Value, json};

use super::Route;

/// The most a request's head may take before the connection is answered `431`.
const MAX_HEAD: usize = 64 * 1024;
/// The most of a request's body that is read and logged.
const MAX_BODY: usize = 8 * 1024 * 1024;
/// How long a connection may sit idle before it is dropped.
const IDLE: Duration = Duration::from_secs(5);
/// How often a delayed answer looks whether its scenario has ended.
const TICK: Duration = Duration::from_millis(10);

/// The checks a route's path must pass at suite parse: `None` when it is servable.
pub(super) fn path_problem(path: &str) -> Option<&'static str> {
    if !path.starts_with('/') {
        Some("must start with /")
    } else if path.contains(|c: char| c.is_whitespace() || c == '?' || c == '#') {
        Some("must not hold whitespace, ? or #")
    } else {
        None
    }
}

/// The request log a route's path writes to, under the workspace's `.serve/`: the path with every
/// `/` as `_`, then `.jsonl`.
pub(super) fn log_name(path: &str) -> String {
    format!("{}.jsonl", path.replace('/', "_"))
}

/// Two routes whose paths give one request log, named with that log: `None` when every route
/// has its own.
pub(super) fn shared_log(routes: &BTreeMap<String, Route>) -> Option<String> {
    let mut owners: BTreeMap<String, &str> = BTreeMap::new();
    for path in routes.keys() {
        let name = log_name(path);
        if let Some(first) = owners.get(&name) {
            return Some(format!(
                "routes {first} and {path} both log to .serve/{name}"
            ));
        }
        owners.insert(name, path);
    }
    None
}

/// The checks a route's answer must pass at suite parse: `None` when it is servable.
pub(super) fn route_problem(route: &Route) -> Option<&'static str> {
    if route.body.is_some() && route.body_file.is_some() {
        Some("names both body and body_file")
    } else if route
        .status
        .is_some_and(|status| !(100..=599).contains(&status))
    {
        Some("status must be 100 to 599")
    } else {
        None
    }
}

/// One route, ready to answer.
struct Answer {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    delay: Duration,
}

/// What the server's threads share with the scenario: the routes, where requests are logged,
/// the stop flag and the problems a route met that the scenario must fail on.
pub(super) struct Shared {
    answers: BTreeMap<String, Answer>,
    log_dir: PathBuf,
    log: Mutex<()>,
    stop: AtomicBool,
    problems: Mutex<Vec<String>>,
}

impl Shared {
    /// Resolves `routes` for `workspace`: a `body_file` is read against `base`, the contract
    /// directory.
    pub(super) fn new(
        routes: &BTreeMap<String, Route>,
        base: Option<&Path>,
        workspace: &Path,
    ) -> Result<Self, String> {
        let mut answers = BTreeMap::new();
        for (path, route) in routes {
            let body = match (&route.body, &route.body_file) {
                (_, Some(file)) => {
                    let Some(base) = base else {
                        return Err(format!(
                            "serve {path}: body_file {} needs a suite read from a directory",
                            file.display()
                        ));
                    };
                    fs::read(base.join(file))
                        .map_err(|e| format!("serve {path}: body_file {}: {e}", file.display()))?
                }
                (Some(text), None) => text.clone().into_bytes(),
                (None, None) => Vec::new(),
            };
            let mut headers: Vec<(String, String)> = route
                .headers
                .iter()
                .map(|(name, value)| (name.clone(), value.clone()))
                .collect();
            if let Some(location) = &route.redirect {
                headers.push(("Location".to_string(), location.clone()));
            }
            let fallback = if route.redirect.is_some() { 302 } else { 200 };
            answers.insert(
                path.clone(),
                Answer {
                    status: route.status.unwrap_or(fallback),
                    headers,
                    body,
                    delay: Duration::from_millis(route.delay_ms.unwrap_or(0)),
                },
            );
        }
        Ok(Self {
            answers,
            log_dir: workspace.join(".serve"),
            log: Mutex::new(()),
            stop: AtomicBool::new(false),
            problems: Mutex::new(Vec::new()),
        })
    }

    /// The first problem a route met while the scenario ran.
    pub(super) fn problem(&self) -> Option<String> {
        self.problems
            .lock()
            .ok()
            .and_then(|problems| problems.first().cloned())
    }

    fn note(&self, problem: String) {
        if let Ok(mut problems) = self.problems.lock() {
            problems.push(problem);
        }
    }
}

/// A running server. Dropping it stops the acceptor; the scope it runs in joins every thread.
pub(super) struct Server<'scope, 'env> {
    shared: &'env Shared,
    address: SocketAddr,
    base: String,
    acceptor: Option<ScopedJoinHandle<'scope, ()>>,
}

impl<'scope, 'env> Server<'scope, 'env> {
    /// Binds `127.0.0.1:0` and accepts on a thread of `scope`.
    pub(super) fn start(
        scope: &'scope Scope<'scope, 'env>,
        shared: &'env Shared,
    ) -> Result<Self, String> {
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .map_err(|e| format!("serve: cannot bind 127.0.0.1: {e}"))?;
        let address = listener
            .local_addr()
            .map_err(|e| format!("serve: no local address: {e}"))?;
        let acceptor = scope.spawn(move || {
            for stream in listener.incoming() {
                if shared.stop.load(Ordering::SeqCst) {
                    break;
                }
                if let Ok(stream) = stream {
                    scope.spawn(move || answer(stream, shared));
                }
            }
        });
        Ok(Self {
            shared,
            address,
            base: format!("http://{address}"),
            acceptor: Some(acceptor),
        })
    }

    /// The base URL `{{serve}}` stands for: `http://127.0.0.1:<port>`, no trailing slash.
    pub(super) fn base(&self) -> &str {
        &self.base
    }
}

impl Drop for Server<'_, '_> {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.address);
        if let Some(acceptor) = self.acceptor.take() {
            let _ = acceptor.join();
        }
    }
}

struct Request {
    method: String,
    target: String,
    headers: Map<String, Value>,
    /// `None` when the request carries `Transfer-Encoding`: its body is not read.
    body: Option<Vec<u8>>,
}

fn answer(mut stream: TcpStream, shared: &Shared) {
    let _ = stream.set_read_timeout(Some(IDLE));
    let _ = stream.set_write_timeout(Some(IDLE));
    let request = match read_request(&mut stream) {
        Ok(request) => request,
        Err(status) => {
            let _ = respond(&mut stream, status, &[], b"", false);
            if status != 408 {
                drain(&mut stream);
            }
            return;
        }
    };
    let path = request
        .target
        .split_once('?')
        .map_or(request.target.as_str(), |(path, _)| path);
    let Some(answer) = shared.answers.get(path) else {
        let _ = respond(&mut stream, 404, &[], b"", request.method == "HEAD");
        drain(&mut stream);
        return;
    };
    log(shared, path, &request);
    if request.body.is_none() {
        let _ = respond(&mut stream, 501, &[], b"", request.method == "HEAD");
        drain(&mut stream);
        return;
    }
    let started = Instant::now();
    while started.elapsed() < answer.delay {
        if shared.stop.load(Ordering::SeqCst) {
            return;
        }
        thread::sleep(TICK.min(answer.delay.saturating_sub(started.elapsed())));
    }
    let _ = respond(
        &mut stream,
        answer.status,
        &answer.headers,
        &answer.body,
        request.method == "HEAD",
    );
}

fn log(shared: &Shared, path: &str, request: &Request) {
    let line = json!({
        "method": request.method,
        "path": request.target,
        "headers": request.headers,
        "body": request.body.as_deref().map(String::from_utf8_lossy),
    });
    let file = shared.log_dir.join(log_name(path));
    let Ok(_guard) = shared.log.lock() else {
        shared.note(format!("serve {path}: the request log lock is poisoned"));
        return;
    };
    let written = fs::create_dir_all(&shared.log_dir).and_then(|()| {
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file)?
            .write_all(format!("{line}\n").as_bytes())
    });
    if let Err(err) = written {
        shared.note(format!(
            "serve {path}: cannot log to {}: {err}",
            file.display()
        ));
    }
}

/// Reads one request; `Err` is the status to answer a malformed or oversized one with.
fn read_request(stream: &mut TcpStream) -> Result<Request, u16> {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 8192];
    let head_end = loop {
        if let Some(at) = find(&buffer, b"\r\n\r\n") {
            break at;
        }
        if buffer.len() > MAX_HEAD {
            return Err(431);
        }
        let read = stream.read(&mut chunk).map_err(|_| 408_u16)?;
        if read == 0 {
            return Err(400);
        }
        buffer.extend_from_slice(&chunk[..read]);
    };
    let head = String::from_utf8_lossy(&buffer[..head_end]).into_owned();
    let mut lines = head.split("\r\n");
    let mut request_line = lines.next().unwrap_or_default().split(' ');
    let (Some(method), Some(target), Some(_version)) = (
        request_line.next(),
        request_line.next(),
        request_line.next(),
    ) else {
        return Err(400);
    };
    let mut headers = Map::new();
    let mut length = 0_usize;
    let mut encoded = false;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            return Err(400);
        };
        let value = value.trim();
        if name.eq_ignore_ascii_case("content-length") {
            length = value.parse().map_err(|_| 400_u16)?;
        } else if name.eq_ignore_ascii_case("transfer-encoding") {
            encoded = true;
        }
        match headers.get_mut(name) {
            Some(Value::String(joined)) => {
                joined.push_str(", ");
                joined.push_str(value);
            }
            _ => {
                headers.insert(name.to_string(), Value::String(value.to_string()));
            }
        }
    }
    if encoded {
        return Ok(Request {
            method: method.to_string(),
            target: target.to_string(),
            headers,
            body: None,
        });
    }
    if length > MAX_BODY {
        return Err(413);
    }
    let mut body = buffer[head_end + 4..].to_vec();
    while body.len() < length {
        let read = stream.read(&mut chunk).map_err(|_| 408_u16)?;
        if read == 0 {
            return Err(400);
        }
        body.extend_from_slice(&chunk[..read]);
    }
    body.truncate(length);
    Ok(Request {
        method: method.to_string(),
        target: target.to_string(),
        headers,
        body: Some(body),
    })
}

/// Ends a connection answered before its request was used up (`400`, `404`, `413`, `431`,
/// `501`): the answer is sent and the write side shut, then what the client still sends (up to
/// [`MAX_BODY`], until it closes or idles) is read and dropped, so the close does not reset the
/// connection before the client reads the answer.
fn drain(stream: &mut TcpStream) {
    let _ = stream.shutdown(Shutdown::Write);
    let mut chunk = [0_u8; 8192];
    let mut left = MAX_BODY;
    while left > 0 {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(read) => left = left.saturating_sub(read),
        }
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn respond(
    stream: &mut TcpStream,
    status: u16,
    headers: &[(String, String)],
    body: &[u8],
    head_only: bool,
) -> std::io::Result<()> {
    let mut lines = String::new();
    for (name, value) in headers {
        lines.push_str(name);
        lines.push_str(": ");
        lines.push_str(value);
        lines.push_str("\r\n");
    }
    let response = format!(
        "HTTP/1.1 {status} {}\r\n{lines}Content-Length: {}\r\nConnection: close\r\n\r\n",
        reason(status),
        body.len()
    );
    let mut bytes = response.into_bytes();
    if !head_only {
        bytes.extend_from_slice(body);
    }
    stream.write_all(&bytes)?;
    stream.flush()
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        303 => "See Other",
        304 => "Not Modified",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        408 => "Request Timeout",
        413 => "Content Too Large",
        429 => "Too Many Requests",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "Status",
    }
}
