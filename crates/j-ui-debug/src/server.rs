use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::{Duration, Instant};

use crate::bridge::Capture;
use crate::input::{Batch, Request};

#[derive(Clone)]
pub(crate) struct Frame {
    png: Arc<Vec<u8>>,
    width: usize,
    height: usize,
    ppp: f32,
    sequence: u64,
    generation: u64,
    created: Instant,
    targets: Vec<crate::nodes::Target>,
}

pub(crate) struct Shared {
    frame: Option<Frame>,
    pub last_input: Instant,
    pub generation: u64,
}

impl Default for Shared {
    fn default() -> Self {
        Self {
            frame: None,
            last_input: Instant::now(),
            generation: 0,
        }
    }
}

pub(crate) struct Server {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    config: std::path::PathBuf,
    token: String,
    _lock: std::fs::File,
}

pub(crate) fn start(
    config: &Path,
    app: &str,
    shared: Arc<Mutex<Shared>>,
    inputs: mpsc::SyncSender<Batch>,
    images: mpsc::Receiver<Capture>,
) -> io::Result<Server> {
    if !config.is_absolute() || app.is_empty() || app.len() > 128 {
        return Err(io::Error::other(
            "an app id and absolute connection path are required",
        ));
    }
    let parent = config
        .parent()
        .ok_or_else(|| io::Error::other("invalid connection path"))?;
    std::fs::create_dir_all(parent)?;
    let lock = private_file(&config.with_extension("lock"))?;
    lock.try_lock()
        .map_err(|error| io::Error::other(format!("debug connection already in use: {error}")))?;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let mut secret = [0_u8; 32];
    getrandom::fill(&mut secret).map_err(|error| io::Error::other(error.to_string()))?;
    let token = secret
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let session = token[..16].to_owned();
    let value = serde_json::json!({"url": format!("http://{}", listener.local_addr()?), "token": token, "app": app, "pid": std::process::id(), "protocol": 1});
    let temporary = config.with_extension(format!("{session}.partial"));
    let mut file = private_file(&temporary)?;
    file.write_all(&serde_json::to_vec(&value)?)?;
    file.sync_all()?;
    std::fs::rename(temporary, config)?;
    let stop = Arc::new(AtomicBool::new(false));
    let thread_stop = stop.clone();
    let thread_token = token.clone();
    let thread = std::thread::Builder::new()
        .name("jui-display".into())
        .spawn(move || {
            while !thread_stop.load(Ordering::Relaxed) {
                if let Ok(capture) = images.try_recv()
                    && let Ok(frame) = encode(capture)
                    && let Ok(mut state) = shared.lock()
                {
                    state.frame = Some(frame);
                }
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let _ = serve(&mut stream, &thread_token, &session, &shared, &inputs);
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5))
                    }
                    Err(_) => break,
                }
            }
        })?;
    Ok(Server {
        stop,
        thread: Some(thread),
        config: config.into(),
        token,
        _lock: lock,
    })
}

fn private_file(path: &Path) -> io::Result<std::fs::File> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).read(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        let current = std::fs::read(&self.config)
            .ok()
            .and_then(|data| serde_json::from_slice::<serde_json::Value>(&data).ok());
        if current.is_some_and(|value| value["token"] == self.token) {
            let _ = std::fs::remove_file(&self.config);
        }
    }
}

fn encode(capture: Capture) -> io::Result<Frame> {
    let width = capture.image.width();
    let height = capture.image.height();
    if width == 0
        || height == 0
        || width > 8192
        || height > 8192
        || width * height > 16_777_216
        || !capture.ppp.is_finite()
        || capture.ppp <= 0.0
    {
        return Err(io::Error::other("invalid debug capture dimensions"));
    }
    let mut png = Vec::new();
    let mut encoder = png::Encoder::new(&mut png, width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Fastest);
    let mut writer = encoder.write_header()?;
    let pixels = capture
        .image
        .pixels
        .iter()
        .flat_map(|pixel| pixel.to_srgba_unmultiplied())
        .collect::<Vec<_>>();
    writer.write_image_data(&pixels)?;
    writer.finish()?;
    Ok(Frame {
        png: Arc::new(png),
        width,
        height,
        ppp: capture.ppp,
        sequence: capture.sequence,
        generation: capture.generation,
        created: Instant::now(),
        targets: capture.targets,
    })
}

fn serve(
    stream: &mut TcpStream,
    token: &str,
    session: &str,
    shared: &Mutex<Shared>,
    inputs: &mpsc::SyncSender<Batch>,
) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_millis(100)))?;
    stream.set_write_timeout(Some(Duration::from_millis(100)))?;
    let deadline = Instant::now() + Duration::from_secs(1);
    let mut data = Vec::new();
    let header_end = loop {
        check_deadline(deadline)?;
        let mut chunk = [0_u8; 2048];
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            return Ok(());
        }
        data.extend_from_slice(&chunk[..read]);
        if let Some(index) = data.windows(4).position(|part| part == b"\r\n\r\n") {
            break index + 4;
        }
        if data.len() > 8192 {
            return reply(stream, 413, &[], &[]);
        }
    };
    let mut headers = [httparse::EMPTY_HEADER; 16];
    if header_end > 8192 {
        return reply(stream, 413, &[], &[]);
    }
    let mut request = httparse::Request::new(&mut headers);
    request
        .parse(&data[..header_end])
        .map_err(io::Error::other)?;
    let header = |name: &str| {
        request
            .headers
            .iter()
            .find(|header| header.name.eq_ignore_ascii_case(name))
            .and_then(|header| std::str::from_utf8(header.value).ok())
    };
    if header("origin").is_some()
        || header("host").is_some_and(|host| {
            host != stream
                .local_addr()
                .map(|address| address.to_string())
                .unwrap_or_default()
        })
        || header("authorization") != Some(format!("Bearer {token}").as_str())
    {
        return reply(stream, 403, &[], &[]);
    }
    let length = header("content-length")
        .unwrap_or("0")
        .parse::<usize>()
        .unwrap_or(usize::MAX);
    if length > 16384 || header("transfer-encoding").is_some() {
        return reply(stream, 413, &[], &[]);
    }
    let method = request.method.unwrap_or_default().to_owned();
    let path = request.path.unwrap_or_default().to_owned();
    while data.len() < header_end + length {
        check_deadline(deadline)?;
        let mut chunk = [0_u8; 2048];
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            return Ok(());
        }
        data.extend_from_slice(&chunk[..read]);
    }
    let state = shared
        .lock()
        .map_err(|_| io::Error::other("display state unavailable"))?;
    let frame = state.frame.clone();
    let generation = state.generation;
    drop(state);
    let Some(frame) = frame else {
        return reply(stream, 503, &[], &[]);
    };
    if frame.created.elapsed() > Duration::from_secs(2) {
        return reply(stream, 503, &[], &[]);
    }
    if method == "GET" && path == "/frame" {
        let headers = format!(
            "Content-Type: image/png\r\nX-Session: {session}\r\nX-Frame: {}\r\nX-Generation: {}\r\nX-Width: {}\r\nX-Height: {}\r\nX-Ppp: {}\r\n",
            frame.sequence, frame.generation, frame.width, frame.height, frame.ppp
        );
        return reply(stream, 200, headers.as_bytes(), &frame.png);
    }
    if method == "GET" && path == "/snapshot" {
        let metadata = serde_json::to_vec(&serde_json::json!({
            "session": session, "frame": frame.sequence, "generation": frame.generation,
            "width": frame.width, "height": frame.height, "ppp": frame.ppp, "targets": frame.targets
        }))?;
        let mut body = (metadata.len() as u32).to_le_bytes().to_vec();
        body.extend_from_slice(&metadata);
        body.extend_from_slice(&frame.png);
        let headers = b"Content-Type: application/octet-stream\r\n";
        return reply(stream, 200, headers, &body);
    }
    if method != "POST" || path != "/input" {
        return reply(stream, 404, &[], &[]);
    }
    let input = serde_json::from_slice::<Request>(&data[header_end..header_end + length]);
    let Ok(input) = input else {
        return reply(stream, 400, &[], &[]);
    };
    if !input.valid() {
        return reply(stream, 400, &[], &[]);
    }
    let fresh = input.session == session
        && input.generation == frame.generation
        && input.generation == generation
        && input.frame <= frame.sequence
        && frame.sequence - input.frame <= 20;
    if !fresh {
        return reply(stream, 409, &[], &[]);
    }
    let batch = Batch {
        generation,
        accepted: Instant::now(),
        width: frame.width as f32 / frame.ppp,
        height: frame.height as f32 / frame.ppp,
        events: input.events,
    };
    if inputs.try_send(batch).is_err() {
        return reply(stream, 429, &[], &[]);
    }
    shared
        .lock()
        .map_err(|_| io::Error::other("display state unavailable"))?
        .last_input = Instant::now();
    reply(stream, 200, &[], b"{}")
}

fn reply(stream: &mut TcpStream, code: u16, headers: &[u8], body: &[u8]) -> io::Result<()> {
    let header = format!(
        "HTTP/1.1 {code} Result\r\nConnection: close\r\nCache-Control: no-store\r\nContent-Length: {}\r\n",
        body.len()
    );
    let deadline = Instant::now() + Duration::from_secs(1);
    for mut bytes in [header.as_bytes(), headers, b"\r\n", body] {
        while !bytes.is_empty() {
            check_deadline(deadline)?;
            let written = stream.write(bytes)?;
            if written == 0 {
                return Err(io::ErrorKind::WriteZero.into());
            }
            bytes = &bytes[written..];
        }
    }
    Ok(())
}

fn check_deadline(deadline: Instant) -> io::Result<()> {
    if Instant::now() >= deadline {
        return Err(io::ErrorKind::TimedOut.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continuous_native_frames_do_not_starve_input() {
        let directory = tempfile::tempdir().unwrap();
        let config = directory.path().join("connection.json");
        let shared = Arc::new(Mutex::new(Shared::default()));
        let (send_input, receive_input) = mpsc::sync_channel(64);
        let (send_image, receive_image) = mpsc::sync_channel(1);
        let server = start(
            &config,
            "latency",
            shared.clone(),
            send_input,
            receive_image,
        )
        .unwrap();
        shared.lock().unwrap().generation = 1;
        let image = Arc::new(egui::ColorImage::filled([1920, 1080], egui::Color32::BLACK));
        let stop = Arc::new(AtomicBool::new(false));
        let producer_stop = stop.clone();
        let producer = std::thread::spawn(move || {
            let mut sequence = 1;
            while !producer_stop.load(Ordering::Relaxed) {
                let capture = Capture {
                    image: image.clone(),
                    ppp: 1.0,
                    sequence,
                    generation: 1,
                    targets: vec![],
                };
                if send_image.try_send(capture).is_ok() {
                    sequence += 1;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        });
        let deadline = Instant::now() + Duration::from_secs(3);
        while shared.lock().unwrap().frame.is_none() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        let config: serde_json::Value =
            serde_json::from_slice(&std::fs::read(config).unwrap()).unwrap();
        let address = config["url"]
            .as_str()
            .unwrap()
            .trim_start_matches("http://");
        let token = config["token"].as_str().unwrap();
        let mut longest = Duration::ZERO;
        for _ in 0..12 {
            let frame = shared.lock().unwrap().frame.as_ref().unwrap().sequence;
            let body = serde_json::json!({"session": &token[..16], "frame": frame, "generation": 1, "events": [{"kind": "keep_alive"}]}).to_string();
            let started = Instant::now();
            let mut stream = TcpStream::connect(address).unwrap();
            write!(stream, "POST /input HTTP/1.1\r\nAuthorization: Bearer {token}\r\nContent-Length: {}\r\n\r\n{body}", body.len()).unwrap();
            let mut response = String::new();
            stream.read_to_string(&mut response).unwrap();
            assert!(response.starts_with("HTTP/1.1 200"), "{response}");
            let batch = receive_input
                .recv_timeout(Duration::from_millis(300))
                .unwrap();
            assert!(batch.accepted.elapsed() < Duration::from_millis(300));
            longest = longest.max(started.elapsed());
        }
        stop.store(true, Ordering::Relaxed);
        producer.join().unwrap();
        drop(server);
        println!(
            "PASSED native input under continuous 1920x1080 captures: longest round trip {longest:?}"
        );
        assert!(
            longest < Duration::from_millis(300),
            "native encoding must not starve ordinary input: {longest:?}"
        );
    }

    #[test]
    fn trickling_requests_cannot_extend_context_shutdown() {
        let directory = tempfile::tempdir().unwrap();
        let config = directory.path().join("connection.json");
        let (inputs, _) = mpsc::sync_channel(1);
        let (_, images) = mpsc::sync_channel(1);
        let server = start(
            &config,
            "shutdown",
            Arc::new(Mutex::new(Shared::default())),
            inputs,
            images,
        )
        .unwrap();
        let connection: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&config).unwrap()).unwrap();
        let address = connection["url"]
            .as_str()
            .unwrap()
            .trim_start_matches("http://");
        let mut stream = TcpStream::connect(address).unwrap();
        stream.write_all(b"G").unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let writer_stop = stop.clone();
        let writer = std::thread::spawn(move || {
            while !writer_stop.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(20));
                if stream.write_all(b"x").is_err() {
                    break;
                }
            }
        });
        std::thread::sleep(Duration::from_millis(50));
        let started = Instant::now();
        drop(server);
        let elapsed = started.elapsed();
        stop.store(true, Ordering::Relaxed);
        writer.join().unwrap();
        assert!(
            elapsed < Duration::from_millis(1500),
            "trickling client prolonged shutdown: {elapsed:?}"
        );
        assert!(!config.exists());
    }

    #[test]
    fn transport_authenticates_and_rejects_stale_or_invalid_input() {
        let directory = tempfile::tempdir().unwrap();
        let config = directory.path().join("connection.json");
        let shared = Arc::new(Mutex::new(Shared::default()));
        let (send_input, receive_input) = mpsc::sync_channel(64);
        let (send_image, receive_image) = mpsc::sync_channel(1);
        let server = start(&config, "test", shared.clone(), send_input, receive_image).unwrap();
        shared.lock().unwrap().generation = 1;
        send_image
            .send(Capture {
                image: Arc::new(egui::ColorImage::filled([16, 12], egui::Color32::BLACK)),
                ppp: 2.0,
                sequence: 1,
                generation: 1,
                targets: vec![crate::nodes::Target {
                    id: "native-id".into(),
                    path: "demo/apply".into(),
                    label: "Apply".into(),
                    source: Some("demo.rs:1".into()),
                    role: "Button".into(),
                    z_order: 0,
                    enabled: true,
                    rect: [0.1, 0.2, 0.3, 0.4],
                }],
            })
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while shared.lock().unwrap().frame.is_none() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(shared.lock().unwrap().frame.is_some());
        let connection: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&config).unwrap()).unwrap();
        let address = connection["url"]
            .as_str()
            .unwrap()
            .trim_start_matches("http://");
        let token = connection["token"].as_str().unwrap();
        let session = &token[..16];
        let request = |auth: &str, method: &str, path: &str, body: &[u8]| {
            let mut stream = TcpStream::connect(address).unwrap();
            write!(stream, "{method} {path} HTTP/1.1\r\nAuthorization: Bearer {auth}\r\nContent-Length: {}\r\n\r\n", body.len()).unwrap();
            stream.write_all(body).unwrap();
            let mut reply = Vec::new();
            stream.read_to_end(&mut reply).unwrap();
            String::from_utf8_lossy(&reply).into_owned()
        };
        assert!(request("wrong", "GET", "/frame", b"").starts_with("HTTP/1.1 403"));
        let origin = format!("{token}\r\nOrigin: https://example.com");
        assert!(request(&origin, "GET", "/frame", b"").starts_with("HTTP/1.1 403"));
        let image = request(token, "GET", "/frame", b"");
        assert!(image.starts_with("HTTP/1.1 200"));
        assert!(image.contains("X-Width: 16\r\n"));
        let mut stream = TcpStream::connect(address).unwrap();
        write!(
            stream,
            "GET /snapshot HTTP/1.1\r\nAuthorization: Bearer {token}\r\n\r\n"
        )
        .unwrap();
        let mut snapshot = Vec::new();
        stream.read_to_end(&mut snapshot).unwrap();
        let body = snapshot
            .windows(4)
            .position(|part| part == b"\r\n\r\n")
            .unwrap()
            + 4;
        let length = u32::from_le_bytes(snapshot[body..body + 4].try_into().unwrap()) as usize;
        let metadata: serde_json::Value =
            serde_json::from_slice(&snapshot[body + 4..body + 4 + length]).unwrap();
        assert_eq!(metadata["frame"], 1);
        assert_eq!(metadata["targets"][0]["path"], "demo/apply");
        assert_eq!(
            metadata["targets"][0]["rect"][2].as_f64().unwrap() as f32,
            0.3
        );
        assert_eq!(
            &snapshot[body + 4 + length..body + 12 + length],
            b"\x89PNG\r\n\x1a\n"
        );
        let body = |generation, x| {
            serde_json::to_vec(&serde_json::json!({"session":session,"generation":generation,"frame":1,"events":[{"kind":"move","x":x,"y":0.5}]})).unwrap()
        };
        assert!(request(token, "POST", "/input", &body(2, 0.5)).starts_with("HTTP/1.1 409"));
        assert!(request(token, "POST", "/input", &body(1, -1.0)).starts_with("HTTP/1.1 400"));
        assert!(receive_input.try_recv().is_err());
        assert!(request(token, "POST", "/input", &body(1, 0.5)).starts_with("HTTP/1.1 200"));
        let batch = receive_input.try_recv().unwrap();
        assert_eq!((batch.width, batch.height), (8.0, 6.0));
        shared.lock().unwrap().last_input = Instant::now() - Duration::from_secs(3);
        assert!(request(token, "GET", "/frame", b"").starts_with("HTTP/1.1 200"));
        assert!(
            shared.lock().unwrap().last_input.elapsed() >= Duration::from_secs(3),
            "read-only views must not keep input held"
        );
        shared.lock().unwrap().generation = 2;
        assert!(request(token, "POST", "/input", &body(1, 0.5)).starts_with("HTTP/1.1 409"));
        let (send, _) = mpsc::sync_channel(1);
        let (_, images) = mpsc::sync_channel(1);
        assert!(start(&config, "duplicate", shared.clone(), send, images).is_err());
        drop(server);
        assert!(!config.exists());
    }
}
