use crate::{
    game::{self, Game},
    settings::Settings,
    vision,
};
use image::RgbImage;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};

#[derive(Clone, Copy, Debug)]
pub struct BoardLocation {
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub scale: f32,
    pub flipped: bool,
    pub focused: bool,
}
pub enum ScreenEvent {
    Position {
        placement: String,
        location: BoardLocation,
    },
    Changing,
    Status(String),
}
pub fn is_chess_window(title: &str) -> bool {
    let title = title.to_ascii_lowercase();
    title.contains("chess")
        && !title.contains("chess assistant")
        && !title.contains("r.bchessassit")
}
pub fn synchronize(game: &Game, placement: &str) -> Result<Game, String> {
    let current = game::fen(game.pos());
    if current.split_whitespace().next() == Some(placement) {
        return Ok(game.clone());
    }
    if game::fen(&shakmaty::Chess::default())
        .split_whitespace()
        .next()
        == Some(placement)
    {
        return Ok(Game::default());
    }
    use shakmaty::Position;
    // Compare legal transitions, preserving turn, castling rights and en passant.
    for m in game.pos().legal_moves() {
        let mut one = game.clone();
        one.push(m)?;
        if game::fen(one.pos()).split_whitespace().next() == Some(placement) {
            return Ok(one);
        }
        for reply in one.pos().legal_moves() {
            let mut two = one.clone();
            two.push(reply)?;
            if game::fen(two.pos()).split_whitespace().next() == Some(placement) {
                return Ok(two);
            }
        }
    }
    Err("Board changed beyond two legal moves. Start a new game, or load its exact FEN into the studio and reconnect.".into())
}

pub struct ScreenReader {
    pub receiver: mpsc::Receiver<ScreenEvent>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl ScreenReader {
    pub fn start(seed: Game) -> Self {
        let (tx, rx) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let thread = std::thread::spawn(move || {
            crate::platform::prepare_dpi();
            let path = Settings::path().with_file_name("screen-templates.json");
            let diagnostics = Settings::load().capture_diagnostics;
            let diagnostic_status = |status: &str| {
                if diagnostics {
                    let _ = std::fs::create_dir_all(path.parent().unwrap());
                    let _ = std::fs::write(path.with_file_name("screen-status.txt"), status);
                }
            };
            let mut model = std::fs::read(&path)
                .ok()
                .and_then(|b| serde_json::from_slice::<vision::ScreenTemplates>(&b).ok());
            let mut previous = String::new();
            let mut last_status = String::new();
            let mut last_saved_model = Vec::new();
            while !stopping.load(Ordering::Relaxed) {
                let step = || -> Result<(RgbImage, BoardLocation), String> {
                    let windows = xcap::Window::all().map_err(|e| e.to_string())?;
                    let mut candidates: Vec<_> = windows
                        .into_iter()
                        .filter(|w| {
                            w.title().is_ok_and(|s| is_chess_window(&s))
                                && !w.is_minimized().unwrap_or(true)
                        })
                        .collect();
                    candidates.sort_by_key(|w| !w.is_focused().unwrap_or(false));
                    if candidates.is_empty() {
                        return Err(
                            "Open your Chess.com game in Chrome and keep the board visible.".into(),
                        );
                    }
                    let mut found = None;
                    let mut capture_errors = Vec::new();
                    for window in candidates.into_iter().take(6) {
                        let img = match window.capture_image() {
                            Ok(img) => img,
                            Err(e) => {
                                capture_errors.push(e.to_string());
                                continue;
                            }
                        };
                        let rgb = image::DynamicImage::ImageRgba8(img).to_rgb8();
                        if let Some(bounds) = vision::detect_screen_bounds(&rgb) {
                            found = Some((window, rgb, bounds));
                            break;
                        }
                    }
                    let (window, rgb, bounds) = found.ok_or_else(|| {
                        if !capture_errors.is_empty() { format!("Windows capture failed: {}. Keep the game window restored and visible.", capture_errors.join("; ")) }
                        else { "Chess.com is open, but no board was found. Select your game tab, keep the full square board visible, and close any dialogs over it.".into() }
                    })?;
                    let location = BoardLocation {
                        x: window.x().map_err(|e| e.to_string())? as f32 + bounds[0] as f32,
                        y: window.y().map_err(|e| e.to_string())? as f32 + bounds[1] as f32,
                        size: bounds[2] as f32,
                        scale: window
                            .current_monitor()
                            .and_then(|m| m.scale_factor())
                            .unwrap_or(1.0)
                            .max(0.5),
                        flipped: false,
                        focused: window.is_focused().unwrap_or(false),
                    };
                    Ok((
                        image::imageops::crop_imm(&rgb, bounds[0], bounds[1], bounds[2], bounds[3])
                            .to_image(),
                        location,
                    ))
                };
                match step() {
                    Ok((board, mut location)) => {
                        if diagnostics {
                            let _ = board.save(path.with_file_name("diagnostic-board.png"));
                        }
                        // Learn only a verified initial arrangement. Retain local per-piece features.
                        let learning = vision::ScreenTemplates::learn_start(&board).or_else(|_| {
                            vision::ScreenTemplates::learn_position(&board, seed.pos())
                        });
                        let learning_error = learning.as_ref().err().cloned();
                        if let Ok(learned) = learning {
                            model = Some(learned);
                            if let Some(m) = &model {
                                if let Ok(bytes) = serde_json::to_vec(m) {
                                    if bytes != last_saved_model {
                                        let _ = std::fs::create_dir_all(path.parent().unwrap());
                                        if std::fs::write(&path, &bytes).is_ok() {
                                            last_saved_model = bytes;
                                        }
                                    }
                                }
                            }
                        }
                        match model
                            .as_ref()
                            .ok_or_else(|| {
                                format!(
                                    "Could not learn the board: {}",
                                    learning_error
                                        .as_deref()
                                        .unwrap_or("Start a fresh game to learn the piece theme.")
                                )
                            })
                            .and_then(|m| m.recognize(&board))
                        {
                            Ok(placement) => {
                                location.flipped = model.as_ref().unwrap().is_flipped();
                                diagnostic_status(&format!(
                                    "Recognized board: {placement}\n{location:?}"
                                ));
                                if placement == previous {
                                    let _ = tx.send(ScreenEvent::Position {
                                        placement,
                                        location,
                                    });
                                    last_status.clear();
                                } else {
                                    previous = placement;
                                    let _ = tx.send(ScreenEvent::Changing);
                                }
                            }
                            Err(e) => {
                                diagnostic_status(&format!(
                                    "Recognition: {e}\nLearning: {learning_error:?}\n{location:?}"
                                ));
                                previous.clear();
                                if e != last_status {
                                    let _ = tx.send(ScreenEvent::Status(e.clone()));
                                    last_status = e;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        diagnostic_status(&e);
                        previous.clear();
                        if e != last_status {
                            let _ = tx.send(ScreenEvent::Status(e.clone()));
                            last_status = e;
                        }
                    }
                }
                for _ in 0..8 {
                    if stopping.load(Ordering::Relaxed) {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        });
        Self {
            receiver: rx,
            stop,
            thread: Some(thread),
        }
    }
}
impl Drop for ScreenReader {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.thread.take() {
            // A browser may stop responding inside the OS capture call. Never
            // block the UI's Stop button waiting for another application's thread.
            if h.is_finished() {
                let _ = h.join();
            }
        }
    }
}
