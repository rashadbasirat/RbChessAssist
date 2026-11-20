use crate::{game, settings::Settings};
use shakmaty::{Chess, Color, Position};
use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Default)]
pub struct Line {
    pub cp: i32,
    pub mate: Option<i32>,
    pub depth: u32,
    pub pv: Vec<String>,
    pub nodes: u64,
}
impl Line {
    pub fn label(&self) -> String {
        self.mate.map_or_else(
            || format!("{:+.2}", self.cp as f32 / 100.0),
            |m| format!("{}M{}", if m < 0 { "−" } else { "+" }, m.abs()),
        )
    }
    pub fn numeric(&self) -> i32 {
        self.mate.map_or(self.cp, |m| {
            if m > 0 {
                100000 - m * 100
            } else {
                -100000 - m * 100
            }
        })
    }
}
#[derive(Clone, Debug, Default)]
pub struct Analysis {
    pub lines: Vec<Line>,
    pub best: String,
    pub elapsed: f32,
}
struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
pub fn search(
    pos: &Chess,
    settings: &Settings,
    play: bool,
    generation: &AtomicU64,
    id: u64,
) -> Result<Analysis, String> {
    search_with_progress(pos, settings, play, generation, id, None)
}
fn search_with_progress(
    pos: &Chess,
    settings: &Settings,
    play: bool,
    generation: &AtomicU64,
    id: u64,
    progress: Option<&dyn Fn(Analysis)>,
) -> Result<Analysis, String> {
    if pos.is_game_over() {
        return Ok(Analysis::default());
    }
    if generation.load(Ordering::Relaxed) != id {
        return Err("Cancelled".into());
    }
    if play && !settings.book_path.is_empty() {
        if let Some(m) = book_move(pos, &settings.book_path)? {
            return Ok(Analysis {
                best: m,
                ..Default::default()
            });
        }
    }
    let mut command = Command::new(settings.engine());
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut process = Process(command.spawn().map_err(|e| {
        format!("Stockfish could not start: {e}. Choose its executable in Settings.")
    })?);
    let mut input = process.0.stdin.take().ok_or("Engine input unavailable")?;
    let output = process.0.stdout.take().ok_or("Engine output unavailable")?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(output).lines() {
            match line {
                Ok(s) => {
                    if tx.send(s).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
    let send = |input: &mut std::process::ChildStdin, s: &str| -> Result<(), String> {
        writeln!(input, "{s}")
            .and_then(|_| input.flush())
            .map_err(|e| e.to_string())
    };
    send(&mut input, "uci")?;
    let started = Instant::now();
    let mut min_elo = 1320;
    let mut max_elo = 3190;
    loop {
        if generation.load(Ordering::Relaxed) != id {
            return Err("Cancelled".into());
        }
        if started.elapsed() > Duration::from_secs(10) {
            return Err("Stockfish did not complete UCI startup".into());
        }
        match rx.recv_timeout(Duration::from_millis(30)) {
            Ok(s) => {
                if s.starts_with("option name UCI_Elo ") {
                    let t: Vec<_> = s.split_whitespace().collect();
                    for (i, v) in t.iter().enumerate() {
                        if *v == "min" {
                            min_elo = t.get(i + 1).and_then(|v| v.parse().ok()).unwrap_or(min_elo);
                        }
                        if *v == "max" {
                            max_elo = t.get(i + 1).and_then(|v| v.parse().ok()).unwrap_or(max_elo);
                        }
                    }
                }
                if s == "uciok" {
                    break;
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err("Stockfish exited during startup".into());
            }
            Err(_) => {}
        }
    }
    send(
        &mut input,
        &format!("setoption name Threads value {}", settings.threads),
    )?;
    send(
        &mut input,
        &format!("setoption name Hash value {}", settings.hash),
    )?;
    send(
        &mut input,
        &format!(
            "setoption name MultiPV value {}",
            if play { 1 } else { settings.multipv }
        ),
    )?;
    if play && settings.level < 4 {
        send(&mut input, "setoption name UCI_LimitStrength value true")?;
        send(
            &mut input,
            &format!(
                "setoption name UCI_Elo value {}",
                [1500, 2000, 2500, 3000][settings.level].clamp(min_elo, max_elo)
            ),
        )?;
    } else {
        send(&mut input, "setoption name UCI_LimitStrength value false")?;
        send(&mut input, "setoption name Skill Level value 20")?;
    }
    if !settings.tablebase_path.is_empty() {
        send(
            &mut input,
            &format!(
                "setoption name SyzygyPath value {}",
                settings.tablebase_path.replace(['\r', '\n'], "")
            ),
        )?;
    }
    send(&mut input, "isready")?;
    let ready = Instant::now();
    loop {
        if generation.load(Ordering::Relaxed) != id {
            return Err("Cancelled".into());
        }
        if ready.elapsed() > Duration::from_secs(15) {
            return Err("Engine readiness timeout".into());
        }
        match rx.recv_timeout(Duration::from_millis(30)) {
            Ok(s) if s == "readyok" => break,
            Err(mpsc::RecvTimeoutError::Disconnected) => return Err("Engine exited".into()),
            _ => {}
        }
    }
    send(&mut input, &format!("position fen {}", game::fen(pos)))?;
    send(
        &mut input,
        &format!(
            "go movetime {} depth {}",
            (settings.seconds * 1000.0) as u64,
            settings.depth
        ),
    )?;
    let clock = Instant::now();
    let mut result = Analysis::default();
    let mut last_preview = Duration::ZERO;
    loop {
        if generation.load(Ordering::Relaxed) != id {
            let _ = send(&mut input, "stop");
            return Err("Cancelled".into());
        }
        if clock.elapsed() > Duration::from_secs_f32(settings.seconds + 15.0) {
            return Err("Engine search timed out".into());
        }
        match rx.recv_timeout(Duration::from_millis(25)) {
            Ok(s) if s.starts_with("bestmove ") => {
                result.best = s.split_whitespace().nth(1).unwrap_or("").into();
                break;
            }
            Ok(s) if s.starts_with("info ") => {
                if let Some((index, line)) = parse_info(&s, pos.turn()) {
                    if index <= 3 {
                        result
                            .lines
                            .resize_with(result.lines.len().max(index), Line::default);
                        result.lines[index - 1] = line;
                        let elapsed = clock.elapsed();
                        if index == 1
                            && result.lines[0].depth >= 6
                            && elapsed >= Duration::from_millis(180)
                            && (last_preview.is_zero()
                                || elapsed.saturating_sub(last_preview)
                                    >= Duration::from_millis(650))
                        {
                            if let Some(publish) = progress {
                                let mut preview = result.clone();
                                preview.best =
                                    preview.lines[0].pv.first().cloned().unwrap_or_default();
                                if game::uci_move(pos, &preview.best).is_ok() {
                                    preview.elapsed = elapsed.as_secs_f32();
                                    preview.lines.retain(|line| !line.pv.is_empty());
                                    publish(preview);
                                    last_preview = elapsed;
                                }
                            }
                        }
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err("Stockfish stopped unexpectedly. Try Analyze again.".into());
            }
            _ => {}
        }
    }
    result.lines.retain(|l| !l.pv.is_empty());
    result.elapsed = clock.elapsed().as_secs_f32();
    if !result.best.is_empty() {
        game::uci_move(pos, &result.best)?;
    }
    let _ = send(&mut input, "quit");
    Ok(result)
}
pub fn book_move(pos: &Chess, path: &str) -> Result<Option<String>, String> {
    use shakmaty::{EnPassantMode, Role, Square, uci::UciMove, zobrist::Zobrist64};
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path).map_err(|e| format!("Opening book: {e}"))?;
    let size = file.metadata().map_err(|e| e.to_string())?.len();
    if size % 16 != 0 {
        return Err("The Polyglot book has an incomplete record.".into());
    }
    let key = pos.zobrist_hash::<Zobrist64>(EnPassantMode::PseudoLegal).0;
    let (mut low, mut high) = (0, size / 16);
    let mut bytes = [0; 16];
    while low < high {
        let mid = (low + high) / 2;
        file.seek(SeekFrom::Start(mid * 16))
            .map_err(|e| e.to_string())?;
        file.read_exact(&mut bytes).map_err(|e| e.to_string())?;
        if u64::from_be_bytes(bytes[..8].try_into().unwrap()) < key {
            low = mid + 1
        } else {
            high = mid
        }
    }
    file.seek(SeekFrom::Start(low * 16))
        .map_err(|e| e.to_string())?;
    let mut best = None;
    let mut weight = 0;
    while file.read_exact(&mut bytes).is_ok() {
        if u64::from_be_bytes(bytes[..8].try_into().unwrap()) != key {
            break;
        }
        let raw = u16::from_be_bytes(bytes[8..10].try_into().unwrap());
        let w = u16::from_be_bytes(bytes[10..12].try_into().unwrap());
        let promotion = match (raw >> 12) & 7 {
            0 => None,
            1 => Some(Role::Knight),
            2 => Some(Role::Bishop),
            3 => Some(Role::Rook),
            4 => Some(Role::Queen),
            _ => continue,
        };
        let u = UciMove::Normal {
            from: Square::new(((raw >> 6) & 63) as u32),
            to: Square::new((raw & 63) as u32),
            promotion,
        };
        if let Ok(m) = u.to_move(pos) {
            if w > weight {
                weight = w;
                best = Some(m.to_uci(shakmaty::CastlingMode::Standard).to_string());
            }
        }
    }
    Ok(best)
}
pub fn parse_info(s: &str, turn: Color) -> Option<(usize, Line)> {
    let t: Vec<_> = s.split_whitespace().collect();
    let at = |key: &str| {
        t.iter()
            .position(|v| *v == key)
            .and_then(|i| t.get(i + 1))
            .copied()
    };
    let pv = t.iter().position(|v| *v == "pv")?;
    let mut line = Line {
        depth: at("depth")?.parse().ok()?,
        pv: t[pv + 1..].iter().map(|s| s.to_string()).collect(),
        nodes: at("nodes").and_then(|n| n.parse().ok()).unwrap_or(0),
        ..Default::default()
    };
    let sign = if turn == Color::White { 1 } else { -1 };
    if let Some(cp) = at("cp") {
        line.cp = cp.parse::<i32>().ok()? * sign;
    } else if let Some(m) = at("mate") {
        line.mate = Some(m.parse::<i32>().ok()? * sign);
    } else {
        return None;
    }
    Some((
        at("multipv").and_then(|i| i.parse().ok()).unwrap_or(1),
        line,
    ))
}

#[derive(Clone, Debug)]
pub struct ReviewRow {
    pub ply: usize,
    pub san: String,
    pub class: String,
    pub loss: i32,
    pub accuracy: f32,
    pub white_eval: f32,
    pub note: String,
    pub best_line: String,
}
pub fn classify(loss: i32, is_best: bool, unique: bool, sacrifice: bool) -> &'static str {
    if loss <= 20 && is_best && sacrifice {
        "Brilliant"
    } else if loss <= 20 && is_best && unique {
        "Great"
    } else if loss <= 10 && is_best {
        "Best"
    } else if loss <= 20 {
        "Excellent"
    } else if loss <= 50 {
        "Good"
    } else if loss <= 100 {
        "Inaccuracy"
    } else if loss <= 200 {
        "Mistake"
    } else {
        "Blunder"
    }
}
pub fn accuracy(loss: i32) -> f32 {
    100.0 * (-loss.max(0) as f32 / 150.0).exp()
}
pub fn review_row(game: &game::Game, ply: usize, before: &Analysis, after: &Analysis) -> ReviewRow {
    let pos = &game.positions[ply];
    let m = game.moves[ply];
    let sign = if pos.turn() == Color::White { 1 } else { -1 };
    let best = before.lines.first().cloned().unwrap_or_default();
    let next = &game.positions[ply + 1];
    let score = if next.is_checkmate() {
        100000 * sign
    } else if next.is_game_over() {
        0
    } else {
        after.lines.first().map_or(0, Line::numeric)
    };
    let loss = ((best.numeric() - score) * sign).max(0);
    let is_best = m.to_uci(shakmaty::CastlingMode::Standard).to_string() == before.best;
    let gap = before
        .lines
        .get(1)
        .map_or(0, |l| (best.numeric() - l.numeric()) * sign);
    // A conservative, engine-supported material sacrifice over the next six plies.
    let baseline = game::material(pos, pos.turn());
    let mut variation = pos.clone();
    let mut minimum = baseline;
    for u in best.pv.iter().take(6) {
        if let Ok(m) = game::uci_move(&variation, u) {
            variation.play_unchecked(m);
            minimum = minimum.min(game::material(&variation, pos.turn()));
        }
    }
    let sacrifice = best.depth >= 16
        && minimum <= baseline - 200
        && score * sign >= -50
        && loss <= 20
        && gap >= 30;
    let class = classify(loss, is_best, gap >= 150, sacrifice).to_string();
    let missed = best.mate.is_some_and(|m| m * sign > 0)
        && !after
            .lines
            .first()
            .is_some_and(|l| l.mate.is_some_and(|m| m * sign > 0))
        && !next.is_checkmate();
    ReviewRow {
        ply,
        san: game::san(pos, m),
        class: class.clone(),
        loss,
        accuracy: accuracy(loss),
        white_eval: (score as f32 / 100.0).clamp(-10.0, 10.0),
        note: format!(
            "{} {}{}",
            match class.as_str() {
                "Brilliant" =>
                    "The engine's best move includes a material sacrifice in its searched continuation while preserving the evaluation.",
                "Great" =>
                    "The best move is at least 1.50 pawns better than the next searched candidate.",
                "Best" => "Matches the engine's first choice in this search.",
                "Excellent" => "Within 0.20 pawns of the engine's preferred result.",
                "Good" => "Within 0.50 pawns of the engine's preferred result.",
                "Inaccuracy" => "Loses between 0.51 and 1.00 pawns of evaluation.",
                "Mistake" => "Loses between 1.01 and 2.00 pawns of evaluation.",
                _ =>
                    "Loses more than 2.00 pawns of evaluation or changes the engine's mating assessment.",
            },
            game::explanation(pos, m),
            if missed {
                " A previously found forced mate is no longer established."
            } else {
                ""
            }
        ),
        best_line: game::continuation(pos, &best.pv.iter().take(10).cloned().collect::<Vec<_>>()),
    }
}

pub enum Event {
    Progress(u64, Analysis),
    Analysis(u64, Analysis, bool),
    Review(u64, ReviewRow),
    Done(u64),
    Error(u64, String),
}
pub struct Worker {
    pub generation: Arc<AtomicU64>,
    pub receiver: mpsc::Receiver<Event>,
    sender: mpsc::Sender<Event>,
    handles: Vec<std::thread::JoinHandle<()>>,
}
impl Default for Worker {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            generation: Arc::new(AtomicU64::new(0)),
            receiver: rx,
            sender: tx,
            handles: Vec::new(),
        }
    }
}
impl Worker {
    pub fn cancel(&mut self) -> u64 {
        self.generation.fetch_add(1, Ordering::Relaxed) + 1
    }
    pub fn start(&mut self, game: game::Game, settings: Settings, play: bool, review: bool) -> u64 {
        self.start_task(game, settings, play, review, false)
    }
    pub fn start_live(&mut self, game: game::Game, settings: Settings) -> u64 {
        self.start_task(game, settings, false, false, true)
    }
    fn start_task(
        &mut self,
        game: game::Game,
        settings: Settings,
        play: bool,
        review: bool,
        live: bool,
    ) -> u64 {
        let id = self.cancel();
        let gen_id = self.generation.clone();
        let tx = self.sender.clone();
        self.handles.retain(|h| !h.is_finished());
        self.handles.push(std::thread::spawn(move || {
            let task = || -> Result<(), String> {
                if review {
                    let mut s = settings.clone();
                    s.multipv = 3;
                    for ply in 0..game.moves.len() {
                        let before = search(&game.positions[ply], &s, false, &gen_id, id)?;
                        let after = search(&game.positions[ply + 1], &s, false, &gen_id, id)?;
                        if gen_id.load(Ordering::Relaxed) != id {
                            return Ok(());
                        }
                        let _ = tx.send(Event::Review(id, review_row(&game, ply, &before, &after)));
                    }
                    let _ = tx.send(Event::Done(id));
                } else {
                    let publish = |a| {
                        let _ = tx.send(Event::Progress(id, a));
                    };
                    let a = search_with_progress(
                        game.pos(),
                        &settings,
                        play,
                        &gen_id,
                        id,
                        if live { Some(&publish) } else { None },
                    )?;
                    let _ = tx.send(Event::Analysis(id, a, play));
                }
                Ok(())
            };
            if let Err(e) = task() {
                if e != "Cancelled" {
                    let _ = tx.send(Event::Error(id, e));
                }
            }
        }));
        id
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.cancel();
        for h in self.handles.drain(..) {
            let _ = h.join();
        }
    }
}
