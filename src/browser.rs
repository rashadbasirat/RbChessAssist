use crate::{
    companion,
    engine::{Analysis, Event, Worker},
    game::{self, Game},
    settings::Settings,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
use tungstenite::{
    Message, WebSocket,
    handshake::server::{Request, Response},
    protocol::WebSocketConfig,
};
pub const PORT: u16 = 38271;
pub enum BrowserEvent {
    Board { game: Game, flipped: bool },
    Thinking,
    Analysis(Analysis),
    Paused,
    Status(String),
}
enum Control {
    Enabled(bool),
    Arrows(bool),
}
pub struct BrowserCoach {
    pub receiver: mpsc::Receiver<BrowserEvent>,
    commands: mpsc::Sender<Control>,
    stop: Arc<AtomicBool>,
}
impl BrowserCoach {
    pub fn start(seed: Game, settings: Settings) -> Self {
        Self::start_on_port(seed, settings, PORT)
    }
    #[doc(hidden)]
    pub fn start_on_port(seed: Game, mut settings: Settings, port: u16) -> Self {
        let (events, receiver) = mpsc::channel();
        let (commands, controls) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        settings.seconds = settings.seconds.clamp(0.2, 30.0);
        settings.threads = settings.threads.min(8);
        settings.hash = settings.hash.min(512);
        std::thread::spawn(move || {
            if let Err(e) = run(seed, settings, &stopping, &controls, &events, port) {
                let _ = events.send(BrowserEvent::Status(e));
            }
            stopping.store(true, Ordering::Relaxed);
        });
        Self {
            receiver,
            commands,
            stop,
        }
    }
    pub fn enable(&self, value: bool) {
        let _ = self.commands.send(Control::Enabled(value));
    }
    pub fn arrows(&self, value: bool) {
        let _ = self.commands.send(Control::Arrows(value));
    }
    pub fn finished(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }
}
impl Drop for BrowserCoach {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}
fn send(s: &mut WebSocket<TcpStream>, v: Value) -> Result<(), String> {
    s.send(Message::Text(v.to_string().into()))
        .map_err(|e| e.to_string())
}
fn run(
    mut current: Game,
    settings: Settings,
    stop: &AtomicBool,
    controls: &mpsc::Receiver<Control>,
    events: &mpsc::Sender<BrowserEvent>,
    port: u16,
) -> Result<(), String> {
    let token = connection_token()?;
    let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|e| {
        format!("Local connection failed: {e}. Close any other RbChessAssist copy and try again.")
    })?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let mut socket: Option<WebSocket<TcpStream>> = None;
    let mut worker = Worker::default();
    let mut job = 0;
    let mut previous = String::new();
    let mut searched = String::new();
    let mut enabled = true;
    let mut arrows = settings.coach_overlay;
    let mut paused = false;
    let mut deep = 0;
    let mut heartbeat = Instant::now();
    let mut last_board = Instant::now();
    let mut last_status = String::new();
    let status = |text: String, last: &mut String| {
        if *last != text {
            let _ = events.send(BrowserEvent::Status(text.clone()));
            *last = text;
        }
    };
    status("Ready. Click your Connect RbChessAssist bookmark on Chess.com. First time? Open bookmark setup below.".into(),&mut last_status);
    while !stop.load(Ordering::Relaxed) {
        if let Ok((stream, _)) = listener.accept() {
            if let Ok(Some(mut new)) = accept_connection(stream, &token, socket.is_some()) {
                send(
                    &mut new,
                    json!({"type":"connected","enabled":enabled,"arrows":arrows}),
                )?;
                socket = Some(new);
                worker.cancel();
                job = 0;
                previous.clear();
                searched.clear();
                last_board = Instant::now();
                deep = 0;
            }
        }
        for command in controls.try_iter() {
            let payload = match command {
                Control::Enabled(value) => {
                    enabled = value;
                    paused = !value;
                    worker.cancel();
                    job = 0;
                    previous.clear();
                    searched.clear();
                    json!({"type":"enabled","value":value})
                }
                Control::Arrows(value) => {
                    arrows = value;
                    json!({"type":"arrows","value":value})
                }
            };
            if let Some(s) = socket.as_mut() {
                let _ = send(s, payload);
            }
        }
        let mut disconnect = false;
        if let Some(s) = socket.as_mut() {
            if heartbeat.elapsed() > Duration::from_millis(500) {
                if send(s, json!({"type":"heartbeat"})).is_err() {
                    disconnect = true;
                }
                heartbeat = Instant::now();
            }
            match s.read() {
                Ok(Message::Text(text)) => {
                    if let Ok(snapshot) = serde_json::from_str::<Snapshot>(&text) {
                        last_board = Instant::now();
                        if snapshot.paused {
                            if !paused {
                                worker.cancel();
                                job = 0;
                                searched.clear();
                                previous.clear();
                                let _ = events.send(BrowserEvent::Paused);
                                paused = true;
                                enabled = false;
                            }
                        } else if let Some(error) = snapshot.error {
                            worker.cancel();
                            job = 0;
                            searched.clear();
                            previous.clear();
                            let _ = send(s, json!({"type":"status","message":error}));
                            status(error, &mut last_status);
                        } else {
                            paused = false;
                            enabled = true;
                            let placement = snapshot.placement.unwrap_or_default();
                            if placement != previous {
                                previous = placement;
                                worker.cancel();
                                job = 0;
                                searched.clear();
                                let _ = send(s, json!({"type":"hint","value":null}));
                                let _ = events.send(BrowserEvent::Thinking);
                            } else {
                                match position_from_snapshot(
                                    &current,
                                    &placement,
                                    snapshot.fen.as_deref(),
                                ) {
                                    Err(error) => {
                                        let _ = send(s, json!({"type":"status","message":error}));
                                        status(error, &mut last_status);
                                    }
                                    Ok(game) => {
                                        current = game;
                                        if let Some(pgn) = snapshot.pgn.as_deref() {
                                            if let Ok(mut full) = game::parse_pgn(pgn) {
                                                full.cursor = full.moves.len();
                                                if game::fen(full.pos()) == game::fen(current.pos())
                                                    && full.moves.len() > current.moves.len()
                                                {
                                                    current = full;
                                                }
                                            }
                                        }
                                        last_status.clear();
                                        let _ = events.send(BrowserEvent::Board {
                                            game: current.clone(),
                                            flipped: snapshot.flipped,
                                        });
                                        let fen = game::fen(current.pos());
                                        if searched != fen || snapshot.deep != deep {
                                            let is_deep = snapshot.deep != deep;
                                            deep = snapshot.deep;
                                            searched = fen;
                                            let mut options = settings.clone();
                                            if is_deep {
                                                options.seconds = 15.0;
                                                options.depth = 40;
                                            }
                                            job = worker.start(
                                                current.clone(),
                                                options,
                                                false,
                                                false,
                                            );
                                            let _ = send(
                                                s,
                                                json!({"type":"status","message":if is_deep{"Analyzing deeper (up to 15 seconds)..."}else{"Finding a move..."}}),
                                            );
                                            let _ = events.send(BrowserEvent::Thinking);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(Message::Close(_)) => disconnect = true,
                Err(tungstenite::Error::Io(e))
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) => {}
                Err(_) => disconnect = true,
                _ => {}
            }
            for result in worker.receiver.try_iter() {
                match result {
                    Event::Progress(id, analysis) if id == job && enabled && !paused => {
                        if let Some(mut hint) = Hint::new(&current, &analysis) {
                            hint.searching = true;
                            hint.details = format!(
                                "LIVE ANALYSIS\nThis move is being refined as Stockfish searches.\n\n{}",
                                hint.details
                            );
                            let _ = send(s, json!({"type":"hint","value":hint}));
                            let _ = events.send(BrowserEvent::Analysis(analysis));
                        }
                    }
                    Event::Analysis(id, analysis, _) if id == job => {
                        if let Some(hint) = Hint::new(&current, &analysis) {
                            let _ = send(s, json!({"type":"hint","value":hint}));
                        } else {
                            let _ = send(
                                s,
                                json!({"type":"status","message":current.terminal().unwrap_or_else(||"No legal suggestion is available.".into())}),
                            );
                        }
                        let _ = events.send(BrowserEvent::Analysis(analysis));
                    }
                    Event::Error(id, error) if id == job => {
                        let _ = send(s, json!({"type":"status","message":error}));
                        status(error, &mut last_status);
                    }
                    _ => {}
                }
            }
            if last_board.elapsed() > Duration::from_secs(10) {
                disconnect = true;
            }
        } else {
            std::thread::sleep(Duration::from_millis(80));
        }
        if disconnect {
            socket = None;
            worker.cancel();
            job = 0;
            previous.clear();
            searched.clear();
            status("Connection closed. Click Connect RbChessAssist on your Chess.com tab to reconnect.".into(),&mut last_status);
        }
    }
    worker.cancel();
    if let Some(mut s) = socket {
        let _ = send(
            &mut s,
            json!({"type":"status","message":"App closed. Reopen RbChessAssist and click Start helping; this tab will reconnect."}),
        );
        let _ = s.close(None);
    }
    Ok(())
}
pub fn authorized_request(origin: &str, path: &str, host: &str, token: &str) -> bool {
    matches!(origin, "https://www.chess.com" | "https://chess.com")
        && path == format!("/coach/{token}")
        && host == format!("127.0.0.1:{PORT}")
}
fn accept_connection(
    mut stream: TcpStream,
    token: &str,
    busy: bool,
) -> Result<Option<WebSocket<TcpStream>>, String> {
    stream.set_nonblocking(false).map_err(|e| e.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| e.to_string())?;
    let mut bytes = [0u8; 8192];
    let count = stream.peek(&mut bytes).map_err(|e| e.to_string())?;
    let head = String::from_utf8_lossy(&bytes[..count]);
    if head.lines().next() == Some(&format!("GET /setup/{token} HTTP/1.1"))
        && head
            .lines()
            .any(|l| l.eq_ignore_ascii_case(&format!("Host: 127.0.0.1:{PORT}")))
    {
        let mut discard = [0; 8192];
        let _ = stream.read(&mut discard);
        let html = setup_html()?;
        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'none'; style-src 'unsafe-inline'; frame-ancestors 'none'\r\nConnection: close\r\n\r\n{}",html.len(),html).map_err(|e|e.to_string())?;
        return Ok(None);
    }
    let callback = |request: &Request, response: Response| {
        let origin = request
            .headers()
            .get("origin")
            .and_then(|s| s.to_str().ok())
            .unwrap_or("");
        let host = request
            .headers()
            .get("host")
            .and_then(|s| s.to_str().ok())
            .unwrap_or("");
        if authorized_request(origin, request.uri().path(), host, token) {
            if busy {
                return Err(tungstenite::http::Response::builder()
                    .status(409)
                    .body(Some("Another board is already connected".into()))
                    .unwrap());
            }
            Ok(response)
        } else {
            Err(tungstenite::http::Response::builder()
                .status(403)
                .body(Some("Connection not authorized".into()))
                .unwrap())
        }
    };
    let config = WebSocketConfig::default()
        .max_message_size(Some(16_384))
        .max_frame_size(Some(16_384));
    let socket = tungstenite::accept_hdr_with_config(stream, callback, Some(config))
        .map_err(|e| e.to_string())?;
    socket
        .get_ref()
        .set_read_timeout(Some(Duration::from_millis(250)))
        .map_err(|e| e.to_string())?;
    Ok(Some(socket))
}
pub fn connection_token() -> Result<String, String> {
    static TOKEN_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = TOKEN_LOCK.lock().map_err(|e| e.to_string())?;
    let path = Settings::path().with_file_name("bookmark-token.txt");
    if let Ok(token) = std::fs::read_to_string(&path) {
        let token = token.trim();
        if token.len() == 64 && token.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Ok(token.into());
        }
    }
    let mut random = [0u8; 32];
    getrandom::fill(&mut random).map_err(|e| e.to_string())?;
    let token = random
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(path, &token).map_err(|e| e.to_string())?;
    Ok(token)
}
pub fn bookmarklet() -> Result<String, String> {
    let script = format!(
        "(()=>{{if(location.hostname!=='www.chess.com'&&location.hostname!=='chess.com'){{alert('Open your Chess.com game, then click this bookmark.');return;}}const RB_ENDPOINT='ws://127.0.0.1:{PORT}/coach/{}';{}\n{}\n{}\n}})();void(0)",
        connection_token()?,
        include_str!("../assets/browser/decoder.js"),
        include_str!("../assets/browser/coach.js"),
        include_str!("../assets/browser/bridge.js")
    );
    let encoded = script
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect::<String>();
    Ok(format!("javascript:{encoded}"))
}
pub fn prepare_extension() -> Result<std::path::PathBuf, String> {
    let folder = Settings::path().with_file_name("RbChessAssistChrome");
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let manifest = json!({
        "manifest_version":3, "name":"RbChessAssist", "version":"1.0.0",
        "description":"Connect your Chess.com board to the local RbChessAssist app.",
        "content_scripts":[{"matches":["https://www.chess.com/*","https://chess.com/*"],
            "js":["coach.js"],"run_at":"document_idle","world":"MAIN"}]
    });
    let script = format!(
        "(()=>{{const RB_AUTOMATIC=true;const RB_ENDPOINT='ws://127.0.0.1:{PORT}/coach/{}';{}\n{}\n{}\n}})();",
        connection_token()?,
        include_str!("../assets/browser/decoder.js"),
        include_str!("../assets/browser/coach.js"),
        include_str!("../assets/browser/bridge.js")
    );
    std::fs::write(
        folder.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    std::fs::write(folder.join("coach.js"), script).map_err(|e| e.to_string())?;
    Ok(folder)
}
pub fn setup_html() -> Result<String, String> {
    Ok(include_str!("../assets/browser/setup.html").replace(
        "BOOKMARK_URL",
        &bookmarklet()?.replace('&', "&amp;").replace('"', "&quot;"),
    ))
}
pub fn open_setup() -> Result<(), String> {
    let url = format!("http://127.0.0.1:{PORT}/setup/{}", connection_token()?);
    for base in [
        std::env::var_os("ProgramFiles"),
        std::env::var_os("ProgramFiles(x86)"),
        std::env::var_os("LOCALAPPDATA"),
    ]
    .into_iter()
    .flatten()
    {
        let path = std::path::PathBuf::from(base).join("Google/Chrome/Application/chrome.exe");
        if path.is_file() {
            let mut command = std::process::Command::new(path);
            command.arg(&url);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                command.creation_flags(0x08000000);
            }
            command.spawn().map_err(|e| e.to_string())?;
            return Ok(());
        }
    }
    Err("Google Chrome was not found.".into())
}
include!("browser_model.rs");
