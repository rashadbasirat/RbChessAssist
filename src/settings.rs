use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub level: usize,
    pub seconds: f32,
    pub depth: u32,
    pub threads: usize,
    pub hash: usize,
    pub multipv: usize,
    pub flipped: bool,
    pub coordinates: bool,
    pub arrows: bool,
    pub evaluation: bool,
    pub animation: f32,
    pub sound: bool,
    pub theme: usize,
    pub engine_path: String,
    pub tablebase_path: String,
    pub book_path: String,
    pub companion_auto_start: bool,
    pub coach_compact: bool,
    pub coach_overlay: bool,
    pub capture_diagnostics: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            level: 4,
            seconds: 5.0,
            depth: 40,
            threads: std::thread::available_parallelism().map_or(2, |n| n.get().min(4)),
            hash: 256,
            multipv: 3,
            flipped: false,
            coordinates: true,
            arrows: true,
            evaluation: true,
            animation: 0.18,
            sound: false,
            theme: 0,
            engine_path: String::new(),
            tablebase_path: String::new(),
            book_path: String::new(),
            companion_auto_start: false,
            coach_compact: true,
            coach_overlay: true,
            capture_diagnostics: false,
        }
    }
}
impl Settings {
    pub fn maximum_strength(&mut self) {
        self.level = 4;
        self.seconds = 10.0;
        self.depth = 60;
        self.threads = std::thread::available_parallelism()
            .map_or(2, |n| n.get().saturating_sub(2).clamp(1, 8));
        self.hash = 512;
        self.multipv = 1;
        self.book_path.clear();
    }
    pub fn path() -> PathBuf {
        directories::ProjectDirs::from("org", "RashadBasirat", "ChessAssistantPro")
            .map(|d| d.config_dir().join("settings.json"))
            .unwrap_or_else(|| PathBuf::from("settings.json"))
    }
    pub fn load() -> Self {
        let mut s: Self = std::fs::read(Self::path())
            .ok()
            .and_then(|v| serde_json::from_slice(&v).ok())
            .unwrap_or_default();
        s.level = s.level.min(4);
        s.threads = s.threads.clamp(1, 32);
        s.hash = s.hash.clamp(16, 4096);
        s.multipv = s.multipv.clamp(1, 3);
        s.seconds = s.seconds.clamp(0.2, 30.0);
        s.depth = s.depth.clamp(1, 60);
        s.animation = s.animation.clamp(0.0, 0.8);
        s.theme = s.theme.min(2);
        s
    }
    pub fn save(&self) -> Result<(), String> {
        let path = Self::path();
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        let temporary = path.with_extension("tmp");
        std::fs::write(
            &temporary,
            serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        std::fs::rename(temporary, path).map_err(|e| e.to_string())
    }
    pub fn engine(&self) -> PathBuf {
        if !self.engine_path.is_empty() {
            return PathBuf::from(&self.engine_path);
        }
        let base = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_default();
        let bundled = base.join("stockfish/stockfish-windows-x86-64-universal.exe");
        if bundled.exists() {
            return bundled;
        }
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("vendor/stockfish/stockfish-windows-x86-64-universal.exe")
    }
}
