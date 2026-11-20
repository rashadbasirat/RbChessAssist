use chess_assistant_pro::{
    browser::{BrowserCoach, BrowserEvent},
    companion::BoardLocation,
    engine::{Analysis, Event, ReviewRow, Worker},
    game::{self, Game},
    settings::Settings,
    vision,
};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2,
};
use shakmaty::{CastlingMode, Chess, Color, Move, Position, Role, Square};
use std::time::{Duration, Instant};

const BG: Color32 = Color32::from_rgb(13, 19, 27);
const PANEL: Color32 = Color32::from_rgb(21, 30, 41);
const GOLD: Color32 = Color32::from_rgb(224, 186, 108);
const CYAN: Color32 = Color32::from_rgb(112, 207, 218);
const GREEN: Color32 = Color32::from_rgb(134, 201, 168);
const MUTED: Color32 = Color32::from_rgb(140, 157, 177);
const TEXT: Color32 = Color32::from_rgb(229, 235, 242);
const LEVELS: [&str; 5] = [
    "1500 · Intermediate",
    "2000 · Advanced",
    "2500 · Expert",
    "3000 · Elite",
    "3500 target · Maximum",
];

pub struct Studio {
    game: Game,
    settings: Settings,
    worker: Worker,
    job: u64,
    busy: bool,
    analysis: Analysis,
    selected: Option<Square>,
    promotion: Vec<Move>,
    history: Vec<ReviewRow>,
    home: bool,
    settings_open: bool,
    import_open: bool,
    import_pgn: bool,
    review_after_import: bool,
    import_text: String,
    status: String,
    playing: bool,
    human: Color,
    color_choice: usize,
    review_open: bool,
    textures: Vec<egui::TextureHandle>,
    variation: Vec<Chess>,
    variation_step: usize,
    animation: Option<(Instant, Square, Square, char)>,
    eval_display: f32,
    shot: Option<Screenshot>,
    capture: Option<(String, Instant, bool)>,
    screen_reader: Option<BrowserCoach>,
    browser_paused: bool,
    minimize_after_connect: bool,
    details_open: bool,
    library_open: bool,
    library_book: usize,
    library_query: String,
    library_results: Vec<chess_assistant_pro::library::Passage>,
    coach_open: bool,
    compact: bool,
    last_board: Option<Instant>,
    board_location: Option<BoardLocation>,
    coach_demo: bool,
}
struct Screenshot {
    image: image::RgbImage,
    texture: egui::TextureHandle,
    bounds: [u32; 4],
    cells: [Option<char>; 64],
    uncertain: Vec<usize>,
    brush: Option<char>,
    black: bool,
    castling: String,
    ep: String,
    message: String,
    detected: bool,
    flipped: bool,
}
impl Studio {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "Inter".into(),
            egui::FontData::from_static(include_bytes!("../assets/fonts/Inter.ttf")).into(),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "Inter".into());
        cc.egui_ctx.set_fonts(fonts);
        let mut style = (*cc.egui_ctx.style()).clone();
        style.visuals = egui::Visuals::dark();
        style.visuals.panel_fill = BG;
        style.visuals.window_fill = PANEL;
        style.visuals.extreme_bg_color = BG;
        style.visuals.override_text_color = Some(TEXT);
        style.visuals.selection.bg_fill = Color32::from_rgb(46, 88, 89);
        style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(31, 44, 58);
        style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(47, 65, 81);
        style.visuals.widgets.active.bg_fill = Color32::from_rgb(57, 83, 91);
        style.visuals.widgets.inactive.corner_radius = CornerRadius::same(7);
        style.spacing.item_spacing = Vec2::new(10., 10.);
        style.spacing.button_padding = Vec2::new(14., 9.);
        style
            .text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(15.));
        style
            .text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(14.));
        cc.egui_ctx.set_style(style);
        let textures = vision::PIECES
            .iter()
            .enumerate()
            .map(|(i, bytes)| {
                let img = image::load_from_memory(bytes).unwrap().into_rgba8();
                cc.egui_ctx.load_texture(
                    format!("piece{i}"),
                    egui::ColorImage::from_rgba_unmultiplied([128, 128], img.as_raw()),
                    egui::TextureOptions::LINEAR,
                )
            })
            .collect();
        let mut app = Self {
            game: Game::default(),
            settings: Settings::load(),
            worker: Worker::default(),
            job: 0,
            busy: false,
            analysis: Analysis::default(),
            selected: None,
            promotion: Vec::new(),
            history: Vec::new(),
            home: true,
            settings_open: false,
            import_open: false,
            import_pgn: false,
            review_after_import: false,
            import_text: String::new(),
            status: "Ready when you are.".into(),
            playing: false,
            human: Color::White,
            color_choice: 0,
            review_open: false,
            textures,
            variation: Vec::new(),
            variation_step: 0,
            animation: None,
            eval_display: 0.,
            shot: None,
            screen_reader: None,
            browser_paused: false,
            minimize_after_connect: false,
            details_open: false,
            library_open: false,
            library_book: 0,
            library_query: String::new(),
            library_results: Vec::new(),
            coach_open: false,
            compact: false,
            last_board: None,
            board_location: None,
            coach_demo: false,
            capture: std::env::var("CHESS_CAPTURE_PATH")
                .ok()
                .map(|p| (p, Instant::now(), false)),
        };
        if std::env::var_os("CHESS_CAPTURE_STUDIO").is_some() {
            let p = game::parse_fen(
                "r1bq1rk1/ppp2ppp/2np1n2/2b1p3/2B1P3/2NP1N2/PPP2PPP/R1BQ1RK1 w - - 4 6",
            )
            .unwrap();
            app.load(Game::new(p));
            app.search(false, false);
        }
        if app.capture.is_none() {
            app.compact = app.settings.coach_compact;
            app.resize_coach(&cc.egui_ctx);
            if app.settings.companion_auto_start {
                let path = Settings::path().with_file_name("companion-game.pgn");
                if let Ok(text) = std::fs::read_to_string(path) {
                    if let Ok(mut game) = game::parse_pgn(&text) {
                        game.cursor = game.moves.len();
                        app.game = game;
                    }
                }
                app.enable_screen_reader();
            } else {
                app.status = "Click Start helping to connect to your Chess.com board.".into();
            }
        }
        if let Ok(mode) = std::env::var("CHESS_CAPTURE_COACH") {
            app.compact = true;
            app.resize_coach(&cc.egui_ctx);
            if mode == "demo" {
                app.coach_sample();
            }
        }
        app
    }
    fn enable_screen_reader(&mut self) {
        self.reset_analysis();
        self.coach_demo = false;
        self.last_board = None;
        self.board_location = None;
        self.playing = false;
        self.home = false;
        self.settings.arrows = true;
        self.browser_paused = false;
        self.minimize_after_connect = false;
        if self
            .screen_reader
            .as_ref()
            .is_some_and(|coach| coach.finished())
        {
            self.screen_reader = None;
        }
        if let Some(coach) = &self.screen_reader {
            coach.enable(true);
        } else {
            self.screen_reader = Some(BrowserCoach::start(
                self.game.clone(),
                self.settings.clone(),
            ));
        }
        self.status = "Waiting for your Chess.com tab. The extension connects automatically; the optional bookmark also works.".into();
    }
    fn stop_helping(&mut self) {
        self.minimize_after_connect = false;
        if let Some(coach) = &self.screen_reader {
            coach.enable(false);
        }
        self.browser_paused = true;
        self.last_board = None;
        self.board_location = None;
        self.coach_demo = false;
        self.reset_analysis();
        self.status = "Helping is stopped. Click Start helping to reconnect.".into();
    }
    fn coach_sample(&mut self) {
        self.stop_helping();
        self.load(Game::new(
            game::parse_fen(
                "r1bq1rk1/ppp2ppp/2np1n2/2b1p3/2B1P3/2NP1N2/PPP2PPP/R1BQ1RK1 w - - 4 6",
            )
            .unwrap(),
        ));
        self.coach_demo = true;
        self.settings.flipped = false;
        self.settings.arrows = true;
        self.search(false, false);
    }
    fn resize_coach(&self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            egui::WindowLevel::Normal,
        ));
        ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(if self.compact {
            Vec2::new(380., 480.)
        } else {
            Vec2::new(1020., 700.)
        }));
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(if self.compact {
            Vec2::new(480., 760.)
        } else {
            Vec2::new(1440., 900.)
        }));
    }
    fn reset_analysis(&mut self) {
        self.job = self.worker.cancel();
        self.busy = false;
        self.analysis = Analysis::default();
        self.variation.clear();
        self.variation_step = 0;
        self.selected = None;
    }
    fn disconnect_browser_for_local_game(&mut self) {
        // Local play/import/analysis owns this board until Start helping is used again.
        // Dropping the receiver also discards already queued website positions.
        self.screen_reader = None;
        self.browser_paused = true;
        self.last_board = None;
        self.minimize_after_connect = false;
    }
    fn load(&mut self, g: Game) {
        self.disconnect_browser_for_local_game();
        self.reset_analysis();
        self.game = g;
        self.history.clear();
        self.playing = false;
        self.home = false;
        self.status = "Position loaded. Select Analyze to search.".into();
    }
    fn new_game(&mut self, play: bool) {
        self.load(Game::default());
        self.playing = play;
        self.human = match self.color_choice {
            0 => Color::White,
            1 => Color::Black,
            _ => {
                if std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .subsec_nanos()
                    % 2
                    == 0
                {
                    Color::White
                } else {
                    Color::Black
                }
            }
        };
        self.settings.flipped = self.human == Color::Black;
        if play && self.human == Color::Black {
            self.search(true, false);
        }
    }
    fn search(&mut self, play: bool, review: bool) {
        self.disconnect_browser_for_local_game();
        self.home = false;
        self.variation.clear();
        self.variation_step = 0;
        if review && self.game.moves.is_empty() {
            self.status = "Import or play a game before starting review.".into();
            return;
        }
        if !review && self.game.terminal().is_some() {
            self.status = self.game.terminal().unwrap();
            return;
        }
        if review {
            self.history.clear();
            self.review_open = true;
            self.playing = false;
        }
        self.job = self
            .worker
            .start(self.game.clone(), self.settings.clone(), play, review);
        self.busy = true;
        self.status = if review {
            "Reviewing every move with Stockfish…"
        } else if play {
            "Stockfish is thinking…"
        } else {
            "Searching the position…"
        }
        .into();
    }
    fn push(&mut self, m: Move) {
        self.disconnect_browser_for_local_game();
        let from = m.from();
        let symbol = from
            .and_then(|sq| self.game.pos().board().piece_at(sq))
            .map(|p| p.char());
        let u = m.to_uci(CastlingMode::Standard).to_string();
        let to = u.get(2..4).and_then(|s| s.parse::<Square>().ok());
        self.reset_analysis();
        match self.game.push(m) {
            Ok(()) => {
                self.history.clear();
                if let (Some(from), Some(to), Some(c)) = (from, to, symbol) {
                    self.animation = Some((Instant::now(), from, to, c));
                }
                if self.settings.sound {
                    #[cfg(windows)]
                    {
                        unsafe extern "system" {
                            fn MessageBeep(kind: u32) -> i32;
                        }
                        unsafe {
                            MessageBeep(0xFFFFFFFF);
                        }
                    }
                }
                self.status = self.game.terminal().unwrap_or_else(|| {
                    format!(
                        "{} to move{}",
                        if self.game.pos().turn() == Color::White {
                            "White"
                        } else {
                            "Black"
                        },
                        if self.game.pos().is_check() {
                            " · Check"
                        } else {
                            ""
                        }
                    )
                });
                if self.playing
                    && self.game.pos().turn() != self.human
                    && self.game.terminal().is_none()
                {
                    self.search(true, false);
                }
            }
            Err(e) => self.status = e,
        }
    }
    fn poll(&mut self, ctx: &egui::Context) {
        let updates = self
            .screen_reader
            .as_ref()
            .map(|r| r.receiver.try_iter().collect::<Vec<_>>())
            .unwrap_or_default();
        for event in updates {
            match event {
                BrowserEvent::Board { game, flipped } => {
                    self.browser_paused = false;
                    self.last_board = Some(Instant::now());
                    self.settings.flipped = flipped;
                    if game::fen(game.pos()) != game::fen(self.game.pos()) {
                        self.reset_analysis();
                        self.game = game;
                    }
                }
                BrowserEvent::Thinking => {
                    self.reset_analysis();
                    self.busy = true;
                    self.status = "Connected to Chess.com · finding your next move…".into();
                }
                BrowserEvent::Analysis(analysis) => {
                    self.busy = false;
                    self.analysis = analysis;
                    self.status = "Suggestion displayed directly on Chess.com.".into();
                    if self.minimize_after_connect {
                        self.minimize_after_connect = false;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                    }
                }
                BrowserEvent::Paused => {
                    self.browser_paused = true;
                    self.last_board = None;
                    self.reset_analysis();
                    self.status = "Stopped on Chess.com. Click Start helping to resume.".into();
                }
                BrowserEvent::Status(message) => {
                    self.last_board = None;
                    self.reset_analysis();
                    self.status = message;
                }
            }
        }
        while let Ok(event) = self.worker.receiver.try_recv() {
            match event {
                Event::Analysis(id, a, play) if id == self.job => {
                    self.busy = false;
                    if play {
                        match game::uci_move(self.game.pos(), &a.best) {
                            Ok(m) => self.push(m),
                            Err(e) => self.status = e,
                        }
                    } else {
                        self.status = format!(
                            "Analysis ready · {:.1}s · strongest available search",
                            a.elapsed
                        );
                        self.analysis = a;
                    }
                }
                Event::Review(id, row) if id == self.job => {
                    self.status = format!(
                        "Reviewing move {} of {}",
                        row.ply + 1,
                        self.game.moves.len()
                    );
                    self.history.push(row)
                }
                Event::Done(id) if id == self.job => {
                    self.busy = false;
                    self.status = format!(
                        "Review complete · {} half-moves analyzed",
                        self.history.len()
                    );
                }
                Event::Error(id, e) if id == self.job => {
                    self.busy = false;
                    self.status = e;
                }
                _ => {}
            }
        }
    }
    fn open_import(&mut self, pgn: bool) {
        self.import_pgn = pgn;
        self.review_after_import = false;
        self.import_text = if pgn {
            String::new()
        } else {
            game::fen(self.game.pos())
        };
        self.import_open = true;
    }
    fn open_pgn(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Chess game", &["pgn"])
            .pick_file()
        {
            match std::fs::read_to_string(path)
                .map_err(|e| e.to_string())
                .and_then(|s| game::parse_pgn(&s))
            {
                Ok(g) => {
                    self.load(g);
                    self.status = "PGN loaded. Browse the moves or select Review game.".into();
                }
                Err(e) => self.status = e,
            }
        }
    }
    fn screenshot(&mut self, ctx: &egui::Context, clipboard: bool) {
        let result: Result<image::RgbImage, String> = if clipboard {
            arboard::Clipboard::new()
                .map_err(|e| e.to_string())
                .and_then(|mut c| c.get_image().map_err(|e| e.to_string()))
                .and_then(|d| {
                    image::RgbaImage::from_raw(d.width as u32, d.height as u32, d.bytes.to_vec())
                        .ok_or("Invalid clipboard image".into())
                })
                .map(|i| image::DynamicImage::ImageRgba8(i).to_rgb8())
        } else {
            match rfd::FileDialog::new()
                .add_filter("Image", &["png", "jpg", "jpeg"])
                .pick_file()
            {
                Some(p) => image::open(p)
                    .map(|i| i.to_rgb8())
                    .map_err(|e| e.to_string()),
                None => return,
            }
        };
        match result {
            Ok(img) => {
                if img.width() > 8000 || img.height() > 8000 {
                    self.status =
                        "Please use a screenshot smaller than 8000 pixels per side.".into();
                    return;
                }
                let bounds = vision::detect_bounds(&img).unwrap_or([
                    0,
                    0,
                    img.width().min(img.height()),
                    img.width().min(img.height()),
                ]);
                let texture = ctx.load_texture(
                    "imported-board",
                    egui::ColorImage::from_rgb(
                        [img.width() as usize, img.height() as usize],
                        img.as_raw(),
                    ),
                    Default::default(),
                );
                self.shot=Some(Screenshot{image:img,texture,bounds,cells:[None;64],uncertain:Vec::new(),brush:Some('P'),black:false,castling:String::new(),ep:String::new(),message:"Adjust the crop to the board edges, then detect. Supported pieces are the app's native set.".into(),detected:false,flipped:false});
            }
            Err(e) => self.status = e,
        }
    }
    fn header(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("header")
            .frame(egui::Frame::new().fill(PANEL).inner_margin(18.))
            .show(ctx, |ui| {
                if self.compact {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("RbChessAssist").color(GOLD).strong());
                        if ui.button("Studio").clicked() {
                            self.compact = false;
                            self.home = false;
                            self.resize_coach(ctx);
                        }
                        if ui.button("Connection").clicked() {
                            self.coach_open = true;
                        }
                        if ui.button("Settings").clicked() {
                            self.settings_open = true;
                        }
                    });
                    return;
                }
                ui.horizontal(|ui| {
                    ui.label(RichText::new("♜").size(28.).color(GOLD));
                    ui.label(RichText::new("RbChessAssist").size(18.).strong());
                    ui.label(RichText::new("PRO").size(12.).color(GOLD));
                    ui.add_space(24.);
                    if ui.selectable_label(self.home, "Home").clicked() {
                        self.home = true;
                    }
                    if ui.selectable_label(!self.home, "Studio").clicked() {
                        self.home = false;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Screen coach").clicked() {
                            self.compact = true;
                            self.resize_coach(ctx);
                        }
                        if ui.button("Settings").clicked() {
                            self.settings_open = true;
                        }
                        ui.label(
                            RichText::new("STOCKFISH 19  /  OFFLINE")
                                .size(11.)
                                .color(MUTED),
                        );
                    });
                });
            });
        egui::TopBottomPanel::bottom("status")
            .frame(egui::Frame::new().fill(PANEL).inner_margin(10.))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if self.busy {
                        ui.spinner();
                        if ui.small_button("Cancel").clicked() {
                            self.job = self.worker.cancel();
                            self.busy = false;
                            self.status = "Search cancelled.".into();
                        }
                    } else {
                        ui.label(RichText::new("●").color(GREEN));
                    }
                    ui.label(RichText::new(&self.status).size(12.));
                });
            });
    }
    fn home_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.add_space(32.);
        ui.horizontal(|ui|{
            ui.vertical(|ui|{ui.set_width(ui.available_width()*0.48);ui.add_space(28.);ui.label(RichText::new("YOUR NEXT MOVE, UNDERSTOOD.").size(12.).color(GOLD));ui.add_space(18.);
                ui.label(RichText::new("See deeper.\nPlay stronger.").size(54.).strong());ui.add_space(18.);
                ui.label(RichText::new("A focused chess studio for analysis,\ntraining, and the games worth revisiting.").size(19.).color(MUTED));ui.add_space(26.);
                if primary(ui,"Open analysis studio  →").clicked(){self.home=false;}
                ui.add_space(14.);ui.label(RichText::new("Private by design. Analysis runs on your computer.").size(12.).color(MUTED));
            });
            ui.vertical(|ui|{ui.set_width(ui.available_width());let p=game::parse_fen("r1bq1rk1/ppp2ppp/2np1n2/2b1p3/2B1P3/2NP1N2/PPP2PPP/R1BQ1RK1 w - - 4 6").unwrap();self.paint_board(ui,&p,ui.available_width().min(430.),false);});
        });
        ui.add_space(28.);
        ui.columns(3, |cols| {
            card(&mut cols[0], |ui| {
                ui.label(RichText::new("ANALYZE").color(CYAN));
                ui.heading("Find the strongest line");
                ui.label("Compare three candidates and follow the engine's continuation.");
                if ui.button("Load FEN").clicked() {
                    self.open_import(false);
                }
                if ui.button("Open PGN").clicked() {
                    self.open_pgn();
                }
            });
            card(&mut cols[1], |ui| {
                ui.label(RichText::new("TRAIN").color(GREEN));
                ui.heading("Meet your next opponent");
                ui.label("Five strength targets. Legal moves, hints, and a fresh challenge.");
                if ui.button("Play Stockfish").clicked() {
                    self.new_game(true);
                }
                if ui.button("Review completed game").clicked() {
                    self.open_import(true);
                }
            });
            card(&mut cols[2], |ui| {
                ui.label(RichText::new("EXPLORE").color(GOLD));
                ui.heading("Bring a position to life");
                ui.label("Import a board image, correct the pieces, and confirm the position.");
                if ui.button("Import screenshot").clicked() {
                    self.screenshot(ctx, false);
                }
                if ui.button("Paste clipboard image").clicked() {
                    self.screenshot(ctx, true);
                }
            });
        });
    }
    fn display_pos(&self) -> Chess {
        if self.variation.is_empty() {
            self.game.pos().clone()
        } else {
            self.variation[self.variation_step].clone()
        }
    }
    fn studio_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(if self.playing {
                    "Training room"
                } else {
                    "Analysis studio"
                })
                .size(27.)
                .strong(),
            );
            ui.add_space(20.);
            if ui.button("FEN").clicked() {
                self.open_import(false);
            }
            if ui.button("PGN").clicked() {
                self.open_pgn();
            }
            if ui.button("Screenshot").clicked() {
                self.screenshot(ctx, false);
            }
            if ui.button("Paste image").clicked() {
                self.screenshot(ctx, true);
            }
            if ui.button("New board").clicked() {
                self.new_game(false);
            }
        });
        ui.add_space(12.);
        let total = ui.available_width();
        let board_width = (total * 0.49).min(ui.available_height() - 112.).max(340.);
        let middle = ((total - board_width) * 0.54).max(240.);
        let right = (total - board_width - middle - 30.).max(190.);
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(board_width);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(if self.settings.flipped {
                            "WHITE"
                        } else {
                            "BLACK"
                        })
                        .color(MUTED)
                        .size(12.),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Flip board").clicked() {
                            self.settings.flipped = !self.settings.flipped;
                        }
                    });
                });
                let pos = self.display_pos();
                let evalspace = if self.settings.evaluation { 24. } else { 0. };
                ui.horizontal(|ui| {
                    if self.settings.evaluation {
                        self.eval_bar(ui, board_width - evalspace);
                    }
                    self.paint_board(ui, &pos, board_width - evalspace, true);
                });
                ui.label(
                    RichText::new(if self.settings.flipped {
                        "BLACK"
                    } else {
                        "WHITE"
                    })
                    .color(MUTED)
                    .size(12.),
                );
                ui.horizontal(|ui| {
                    if ui.button("|◀").clicked() {
                        self.reset_analysis();
                        self.game.cursor = 0;
                        self.playing = false;
                    }
                    if ui.button("◀").clicked() {
                        self.reset_analysis();
                        self.game.cursor = self.game.cursor.saturating_sub(1);
                        self.playing = false;
                    }
                    ui.label(format!("{} / {}", self.game.cursor, self.game.moves.len()));
                    if ui.button("▶").clicked() {
                        self.reset_analysis();
                        self.game.cursor = (self.game.cursor + 1).min(self.game.moves.len());
                        self.playing = false;
                    }
                    if ui.button("▶|").clicked() {
                        self.reset_analysis();
                        self.game.cursor = self.game.moves.len();
                        self.playing = false;
                    }
                });
                let mut counts = String::new();
                for (color, label) in [(Color::White, "White"), (Color::Black, "Black")] {
                    let mut missing = Vec::new();
                    for (role, start) in [
                        (Role::Queen, 1),
                        (Role::Rook, 2),
                        (Role::Bishop, 2),
                        (Role::Knight, 2),
                        (Role::Pawn, 8),
                    ] {
                        let n = pos
                            .board()
                            .iter()
                            .filter(|(_, p)| p.color == color && p.role == role)
                            .count();
                        if n < start {
                            missing.push(format!("{} {}", start - n, game::role_name(role)));
                        }
                    }
                    if !missing.is_empty() {
                        counts += &format!("{label} missing: {}  ", missing.join(", "));
                    }
                }
                ui.label(RichText::new(counts).size(11.).color(MUTED));
                if let Some(result) = self.game.terminal() {
                    ui.label(RichText::new(result).color(GOLD).strong());
                }
            });
            ui.vertical(|ui| {
                ui.set_width(middle);
                self.analysis_ui(ui);
            });
            ui.vertical(|ui| {
                ui.set_width(right);
                self.controls_ui(ui);
            });
        });
    }
    fn analysis_ui(&mut self, ui: &mut egui::Ui) {
        card(ui, |ui| {
            ui.label(RichText::new("ENGINE INSIGHT").color(CYAN).size(12.));
            if primary(
                ui,
                if self.playing {
                    "Get a hint"
                } else {
                    "Analyze position"
                },
            )
            .clicked()
            {
                self.search(false, false);
            }
            ui.add_space(8.);
            if self.analysis.lines.is_empty() {
                ui.label(RichText::new("Every position\nhas a story.").size(25.));
                ui.label(
                    RichText::new(
                        "Run an analysis to find the best move, evaluation, and continuation.",
                    )
                    .color(MUTED),
                );
            }
            let lines = self.analysis.lines.clone();
            for (i, line) in lines.iter().enumerate() {
                ui.separator();
                let m = line
                    .pv
                    .first()
                    .and_then(|u| game::uci_move(self.game.pos(), u).ok());
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(if i == 0 { "BEST MOVE" } else { "ALTERNATIVE" })
                            .color(if i == 0 { GOLD } else { MUTED })
                            .size(11.),
                    );
                    ui.label(RichText::new(line.label()).strong().color(CYAN));
                });
                if let Some(m) = m {
                    ui.label(
                        RichText::new(game::san(self.game.pos(), m))
                            .size(if i == 0 { 40. } else { 25. })
                            .strong(),
                    );
                    if let Some(mate) = line.mate {
                        ui.label(
                            RichText::new(format!(
                                "{} has mate in {}",
                                if mate > 0 { "White" } else { "Black" },
                                mate.abs()
                            ))
                            .color(GOLD),
                        );
                    }
                    ui.label(
                        RichText::new(game::continuation(
                            self.game.pos(),
                            &line.pv.iter().take(12).cloned().collect::<Vec<_>>(),
                        ))
                        .size(14.)
                        .color(MUTED),
                    );
                    if i == 0 {
                        ui.add_space(4.);
                        ui.label(game::explanation(self.game.pos(), m));
                    }
                    ui.horizontal(|ui| {
                        if ui.small_button("Explore line").clicked() {
                            let mut p = self.game.pos().clone();
                            self.variation = vec![p.clone()];
                            for u in &line.pv {
                                if let Ok(m) = game::uci_move(&p, u) {
                                    p.play_unchecked(m);
                                    self.variation.push(p.clone());
                                }
                            }
                            self.variation_step = 0;
                        }
                        ui.label(
                            RichText::new(format!(
                                "Depth {} · {}k nodes",
                                line.depth,
                                line.nodes / 1000
                            ))
                            .size(10.)
                            .color(MUTED),
                        );
                    });
                }
            }
            if !self.variation.is_empty() {
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("◀ line").clicked() {
                        self.variation_step = self.variation_step.saturating_sub(1);
                    }
                    if ui.button("line ▶").clicked() {
                        self.variation_step =
                            (self.variation_step + 1).min(self.variation.len() - 1);
                    }
                    if ui.button("Exit").clicked() {
                        self.variation.clear();
                    }
                });
                ui.label(format!("Continuation step {}", self.variation_step));
            }
        });
    }
    fn controls_ui(&mut self, ui: &mut egui::Ui) {
        card(ui, |ui| {
            ui.label(RichText::new("TRAINING CONTROLS").color(MUTED).size(12.));
            egui::ComboBox::from_id_salt("level")
                .selected_text(LEVELS[self.settings.level])
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for (i, l) in LEVELS.iter().enumerate() {
                        ui.selectable_value(&mut self.settings.level, i, *l);
                    }
                });
            ui.label(
                RichText::new("Approximate strength targets")
                    .size(10.)
                    .color(MUTED),
            );
            egui::ComboBox::from_id_salt("color")
                .selected_text(
                    ["Play as White", "Play as Black", "Random color"][self.color_choice],
                )
                .show_ui(ui, |ui| {
                    for (i, s) in ["Play as White", "Play as Black", "Random color"]
                        .iter()
                        .enumerate()
                    {
                        ui.selectable_value(&mut self.color_choice, i, *s);
                    }
                });
            if ui.button("New training game").clicked() {
                self.new_game(true);
            }
            ui.horizontal(|ui| {
                if ui.button("Undo").clicked() {
                    self.reset_analysis();
                    let n = if self.playing && self.game.pos().turn() == self.human {
                        2
                    } else {
                        1
                    };
                    self.game.cursor = self.game.cursor.saturating_sub(n);
                    self.game.moves.truncate(self.game.cursor);
                    self.game.positions.truncate(self.game.cursor + 1);
                    self.game.result = None;
                    self.history.clear();
                }
                if self.playing && ui.button("Resign").clicked() {
                    self.reset_analysis();
                    self.game.result = Some(format!(
                        "{} wins by resignation",
                        if self.human == Color::White {
                            "Black"
                        } else {
                            "White"
                        }
                    ));
                    self.playing = false;
                }
            });
            if self.game.can_claim_draw() && ui.button("Claim draw").clicked() {
                self.reset_analysis();
                self.game.result = Some("Draw claimed by repetition / fifty-move rule".into());
                self.playing = false;
            }
            ui.separator();
            ui.label(RichText::new("GAME NOTEBOOK").color(MUTED).size(12.));
            if ui.button("Review game").clicked() {
                self.search(false, true);
            }
            if !self.history.is_empty() && ui.button("Show review report").clicked() {
                self.review_open = true;
            }
            egui::ScrollArea::vertical()
                .id_salt("history")
                .max_height(250.)
                .show(ui, |ui| {
                    let moves = self.game.moves.clone();
                    for (i, m) in moves.iter().enumerate() {
                        let p = &self.game.positions[i];
                        let label = format!(
                            "{}{} {}",
                            p.fullmoves(),
                            if p.turn() == Color::White { "." } else { "..." },
                            game::san(p, *m)
                        );
                        let color = self
                            .history
                            .iter()
                            .find(|r| r.ply == i)
                            .map(|r| class_color(&r.class))
                            .unwrap_or(TEXT);
                        if ui
                            .selectable_label(
                                self.game.cursor == i + 1,
                                RichText::new(label).color(color),
                            )
                            .clicked()
                        {
                            self.reset_analysis();
                            self.game.cursor = i + 1;
                            self.playing = false;
                        }
                    }
                });
            ui.separator();
            if ui.button("Copy FEN").clicked() {
                ui.ctx().copy_text(game::fen(self.game.pos()));
                self.status = "FEN copied.".into();
            }
            if ui.button("Export PGN").clicked() {
                if let Some(p) = rfd::FileDialog::new()
                    .set_file_name("my-game.pgn")
                    .save_file()
                {
                    self.status = match std::fs::write(p, self.game.pgn()) {
                        Ok(()) => "PGN saved.".into(),
                        Err(e) => e.to_string(),
                    };
                }
            }
            if ui.button("Export board image").clicked() {
                if let Some(p) = rfd::FileDialog::new()
                    .set_file_name("chess-position.png")
                    .save_file()
                {
                    self.status = match vision::board_image(
                        self.game.pos(),
                        self.settings.flipped,
                        self.settings.theme,
                        800,
                    )
                    .save(p)
                    {
                        Ok(()) => "Board image saved.".into(),
                        Err(e) => e.to_string(),
                    };
                }
            }
        });
    }
    fn paint_board(&mut self, ui: &mut egui::Ui, pos: &Chess, size: f32, interactive: bool) {
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
        let cell = size / 8.;
        let painter = ui.painter();
        let colors = vision::THEMES[self.settings.theme];
        let square_rect = |sq: Square| {
            let i = sq as usize;
            let file = i % 8;
            let rank = i / 8;
            let (col, row) = if self.settings.flipped {
                (7 - file, rank)
            } else {
                (file, 7 - rank)
            };
            Rect::from_min_size(
                rect.min + Vec2::new(col as f32 * cell, row as f32 * cell),
                Vec2::splat(cell),
            )
        };
        let anim = self.animation.filter(|(start, _, _, _)| {
            start.elapsed().as_secs_f32() < self.settings.animation
                && interactive
                && self.variation.is_empty()
        });
        for i in 0..64 {
            let sq = Square::new(i);
            let r = square_rect(sq);
            let bg = if (i / 8 + i % 8) % 2 == 1 {
                colors.0
            } else {
                colors.1
            };
            painter.rect_filled(r, 0., Color32::from_rgb(bg[0], bg[1], bg[2]));
            if interactive && self.selected == Some(sq) {
                painter.rect_filled(r, 0., Color32::from_rgba_unmultiplied(224, 186, 108, 130));
            }
            if interactive && self.variation.is_empty() && self.game.cursor > 0 {
                let m = self.game.moves[self.game.cursor - 1];
                if m.from() == Some(sq) || m.to() == sq {
                    painter.rect_filled(r, 0., Color32::from_rgba_unmultiplied(220, 198, 99, 60));
                }
            }
            if pos.is_check() && pos.board().king_of(pos.turn()) == Some(sq) {
                painter.rect_filled(r, 0., Color32::from_rgba_unmultiplied(222, 80, 80, 130));
            }
            if let Some(piece) = pos.board().piece_at(sq) {
                if anim.is_none_or(|(_, _, to, _)| to != sq) {
                    let texture = &self.textures[vision::piece_index(piece.char())];
                    painter.image(
                        texture.id(),
                        r.shrink(cell * 0.07),
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)),
                        Color32::WHITE,
                    );
                }
            }
            if interactive
                && self.selected.is_some_and(|s| {
                    pos.legal_moves()
                        .iter()
                        .any(|m| m.from() == Some(s) && m.to() == sq)
                })
            {
                painter.circle_filled(
                    r.center(),
                    cell * 0.09,
                    Color32::from_rgba_unmultiplied(23, 46, 43, 120),
                );
            }
        }
        if let Some((start, from, to, c)) = anim {
            let t = (start.elapsed().as_secs_f32() / self.settings.animation).min(1.);
            let t = t * t * (3. - 2. * t);
            let a = square_rect(from).min;
            let b = square_rect(to).min;
            let r = Rect::from_min_size(a + (b - a) * t, Vec2::splat(cell)).shrink(cell * 0.07);
            painter.image(
                self.textures[vision::piece_index(c)].id(),
                r,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)),
                Color32::WHITE,
            );
            ui.ctx().request_repaint();
        }
        if (interactive || (self.compact && (self.last_board.is_some() || self.coach_demo)))
            && self.settings.arrows
            && self.variation.is_empty()
        {
            if let Some(line) = self.analysis.lines.first() {
                if let Some(u) = line.pv.first() {
                    if u.len() >= 4 {
                        if let (Ok(a), Ok(b)) =
                            (u[..2].parse::<Square>(), u[2..4].parse::<Square>())
                        {
                            let from = square_rect(a).center();
                            let to = square_rect(b).center();
                            painter.arrow(
                                from,
                                (to - from) * 0.90,
                                Stroke::new(
                                    cell * 0.10,
                                    Color32::from_rgba_unmultiplied(72, 196, 213, 180),
                                ),
                            );
                        }
                    }
                }
            }
        }
        if self.settings.coordinates {
            for col in 0..8 {
                let label = (b'a' + if self.settings.flipped { 7 - col } else { col }) as char;
                painter.text(
                    rect.min + Vec2::new(col as f32 * cell + cell - 4., size - 3.),
                    Align2::RIGHT_BOTTOM,
                    label,
                    FontId::proportional(11.),
                    Color32::from_rgb(40, 59, 58),
                );
            }
            for row in 0..8 {
                let label = if self.settings.flipped {
                    row + 1
                } else {
                    8 - row
                };
                painter.text(
                    rect.min + Vec2::new(4., row as f32 * cell + 3.),
                    Align2::LEFT_TOP,
                    label,
                    FontId::proportional(11.),
                    Color32::from_rgb(40, 59, 58),
                );
            }
        }
        if interactive
            && response.clicked()
            && self.variation.is_empty()
            && self.game.terminal().is_none()
        {
            if self.playing && pos.turn() != self.human {
                return;
            }
            if let Some(mouse) = response.interact_pointer_pos() {
                let col = ((mouse.x - rect.min.x) / cell).floor() as u32;
                let row = ((mouse.y - rect.min.y) / cell).floor() as u32;
                if col < 8 && row < 8 {
                    let sq = Square::new(if self.settings.flipped {
                        row * 8 + 7 - col
                    } else {
                        (7 - row) * 8 + col
                    });
                    if let Some(from) = self.selected {
                        let choices: Vec<_> = pos
                            .legal_moves()
                            .into_iter()
                            .filter(|m| {
                                let u = m.to_uci(CastlingMode::Standard).to_string();
                                m.from() == Some(from)
                                    && u.get(2..4) == Some(sq.to_string().as_str())
                            })
                            .collect();
                        if choices.len() > 1 {
                            self.promotion = choices;
                            return;
                        }
                        if let Some(m) = choices.first() {
                            self.push(*m);
                            return;
                        }
                    }
                    self.selected = if pos
                        .board()
                        .piece_at(sq)
                        .is_some_and(|p| p.color == pos.turn())
                    {
                        Some(sq)
                    } else {
                        None
                    };
                }
            }
        }
    }
    fn eval_bar(&mut self, ui: &mut egui::Ui, height: f32) {
        let target = self
            .analysis
            .lines
            .first()
            .map_or(0., |l| l.numeric() as f32 / 100.);
        self.eval_display += (target.clamp(-10., 10.) - self.eval_display) * 0.12;
        let fraction = 1. / (1. + (-self.eval_display * 0.55).exp());
        let (r, _) = ui.allocate_exact_size(Vec2::new(14., height), Sense::hover());
        ui.painter()
            .rect_filled(r, 4., Color32::from_rgb(36, 45, 58));
        let white = Rect::from_min_max(Pos2::new(r.min.x, r.max.y - height * fraction), r.max);
        ui.painter()
            .rect_filled(white, 3., Color32::from_rgb(225, 230, 224));
    }
    fn dialogs(&mut self, ctx: &egui::Context) {
        if self.import_open {
            let mut open = true;
            egui::Window::new(if self.import_pgn {
                "Import game notation"
            } else {
                "Load a FEN position"
            })
            .open(&mut open)
            .default_width(650.)
            .show(ctx, |ui| {
                ui.label(if self.import_pgn {
                    "Paste the first game's PGN. Mainline moves will be imported."
                } else {
                    "Paste a complete FEN, including side to move, castling, and en passant."
                });
                ui.add(
                    egui::TextEdit::multiline(&mut self.import_text)
                        .desired_rows(if self.import_pgn { 12 } else { 3 })
                        .desired_width(f32::INFINITY),
                );
                if self.import_pgn && ui.button("Choose PGN file").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Chess game", &["pgn"])
                        .pick_file()
                    {
                        match std::fs::read_to_string(path) {
                            Ok(text) => self.import_text = text,
                            Err(e) => self.status = e.to_string(),
                        }
                    }
                }
                if primary(
                    ui,
                    if self.review_after_import {
                        "Analyze and review game"
                    } else {
                        "Load into studio"
                    },
                )
                .clicked()
                {
                    let result = if self.import_pgn {
                        game::parse_pgn(&self.import_text)
                    } else {
                        game::parse_fen(&self.import_text).map(Game::new)
                    };
                    match result {
                        Ok(g) => {
                            self.load(g);
                            self.import_open = false;
                            if self.review_after_import {
                                self.review_after_import = false;
                                self.search(false, true);
                            }
                        }
                        Err(e) => self.status = e,
                    }
                }
            });
            self.import_open &= open;
        }
        if !self.promotion.is_empty() {
            egui::Window::new("Choose your promotion")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        for m in self.promotion.clone() {
                            if let Some(role) = m.promotion() {
                                if ui.button(game::role_name(role)).clicked() {
                                    self.promotion.clear();
                                    self.push(m);
                                }
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.promotion.clear();
                        }
                    });
                });
        }
        if self.coach_open {
            self.coach_ui(ctx);
        }
        if self.library_open {
            self.library_ui(ctx);
        }
        if self.settings_open {
            self.settings_ui(ctx);
        }
        if self.review_open {
            self.review_ui(ctx);
        }
        if self.shot.is_some() {
            self.screenshot_ui(ctx);
        }
    }
    fn coach_controls(&mut self, ui: &mut egui::Ui) {
        ui.heading("Your next move");
        ui.label(
            RichText::new("Chess.com in your existing Chrome window")
                .color(MUTED)
                .size(12.),
        );
        let running = self.screen_reader.is_some() && !self.browser_paused;
        if ui
            .add_sized(
                [ui.available_width(), 48.],
                egui::Button::new(
                    RichText::new(if running {
                        "Stop helping"
                    } else {
                        "Start helping"
                    })
                    .size(18.)
                    .strong()
                    .color(BG),
                )
                .fill(if running { CYAN } else { GOLD }),
            )
            .clicked()
        {
            if running {
                self.stop_helping();
            } else {
                self.enable_screen_reader();
            }
        }
        ui.horizontal(|ui| {
            if ui
                .button(if self.details_open {
                    "Hide details"
                } else {
                    "Details"
                })
                .clicked()
            {
                self.details_open = !self.details_open;
            }
            if ui.button("Chess library · 6 books").clicked() {
                self.library_open = true;
                self.compact = false;
                self.resize_coach(ui.ctx());
            }
        });
        if ui.button("Review game · import PGN").clicked() {
            self.stop_helping();
            self.compact = false;
            self.resize_coach(ui.ctx());
            self.open_import(true);
            self.review_after_import = true;
        }
        ui.horizontal(|ui| {
            if ui
                .checkbox(&mut self.settings.coach_overlay, "Arrow on Chess.com")
                .changed()
            {
                if let Some(coach) = &self.screen_reader {
                    coach.arrows(self.settings.coach_overlay);
                }
            }
            if ui.small_button("Connection").clicked() {
                self.coach_open = true;
            }
        });
        let state = if self.coach_demo {
            "SAMPLE POSITION · not connected"
        } else if !running {
            "Ready to connect"
        } else if self.last_board.is_none() {
            "Connecting to the Chess.com page"
        } else if self.busy {
            "Board connected · analyzing"
        } else {
            "Board connected · helping"
        };
        ui.label(
            RichText::new(state)
                .color(if self.last_board.is_some() {
                    GREEN
                } else {
                    GOLD
                })
                .size(12.),
        );
        ui.separator();
    }
    fn coach_body(&mut self, ui: &mut egui::Ui) {
        if self.last_board.is_none() && !self.coach_demo {
            card(ui, |ui| {
                ui.label(
                    RichText::new(if self.screen_reader.is_some() {
                        &self.status
                    } else {
                        "Click Start helping. The Chrome extension connects your board automatically."
                    })
                    .strong(),
                );
                ui.label("Keep your Chess.com tab open. With the RbChessAssist extension installed, Start and Stop work directly from this app. Open Connection for the one-time setup or the optional bookmark method.");
                if ui.button("Open bookmark setup").clicked() {
                    if self.screen_reader.is_none() {
                        self.enable_screen_reader();
                    }
                    if let Err(e) = chess_assistant_pro::browser::open_setup() {
                        self.status = e;
                    }
                }
                ui.label("The arrow and a RbChessAssist control panel appear directly on Chess.com. Board colors, piece themes, zoom, and orientation are read from the page.");
                if ui.button("Try a sample position").clicked() {
                    self.coach_sample();
                }
                if ui.button("Load exact current FEN").clicked() {
                    self.open_import(false);
                }
            });
            return;
        }
        if let Some(result) = self.game.terminal() {
            ui.label(RichText::new(result).color(GOLD).strong());
        }
        let Some(line) = self.analysis.lines.first().cloned() else {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(if self.busy {
                    "Stockfish is finding a move…"
                } else {
                    &self.status
                });
            });
            return;
        };
        let Some(m) = line
            .pv
            .first()
            .and_then(|u| game::uci_move(self.game.pos(), u).ok())
        else {
            return;
        };
        let pos = self.game.pos().clone();
        let side = if pos.turn() == Color::White {
            "White"
        } else {
            "Black"
        };
        card(ui, |ui| {
            ui.label(
                RichText::new(format!("SUGGESTED MOVE · {side} to move"))
                    .color(CYAN)
                    .size(11.),
            );
            ui.horizontal(|ui| {
                ui.label(RichText::new(game::san(&pos, m)).size(40.).strong());
                ui.label(RichText::new(line.label()).size(25.).color(GOLD));
            });
            ui.label(RichText::new(game::move_instruction(&pos, m)).strong());
            ui.label(game::explanation(&pos, m));
            ui.label(
                RichText::new(
                    "Evaluation is from White's perspective; + favors White, − favors Black.",
                )
                .size(11.)
                .color(MUTED),
            );
        });
        if !self.details_open {
            return;
        }
        if let Some(text) =
            chess_assistant_pro::browser::detailed_explanation(&self.game, &self.analysis)
        {
            card(ui, |ui| {
                ui.label(text);
            });
        }
        ui.add_space(8.);
        let board_size = ui.available_width().min(285.);
        ui.horizontal(|ui| {
            ui.add_space((ui.available_width() - board_size).max(0.) / 2.);
            self.paint_board(ui, &pos, board_size, false);
        });
        ui.label(
            RichText::new(if self.coach_demo {
                "Sample board · real Stockfish analysis"
            } else {
                "Connected position · arrow is also shown on Chess.com"
            })
            .size(11.)
            .color(MUTED),
        );
        card(ui, |ui| {
            ui.label(RichText::new("EXPECTED REPLY").color(CYAN).size(11.));
            let mut after = pos.clone();
            after.play_unchecked(m);
            if let Some(reply) = line.pv.get(1).and_then(|u| game::uci_move(&after, u).ok()) {
                ui.label(RichText::new(game::san(&after, reply)).size(22.).strong());
                ui.label(game::move_instruction(&after, reply));
                ui.label(game::explanation(&after, reply));
            } else {
                ui.label("No reply in this engine line.");
            }
            ui.label(RichText::new("The continuation is an engine forecast, not a guarantee of your opponent's reply.").size(11.).color(MUTED));
            ui.separator();
            ui.label(RichText::new("CONTINUATION").color(CYAN).size(11.));
            ui.label(game::continuation(
                &pos,
                &line.pv.iter().take(10).cloned().collect::<Vec<_>>(),
            ));
            ui.label(
                RichText::new(format!(
                    "Depth {} · {}k nodes · {:.1}s",
                    line.depth,
                    line.nodes / 1000,
                    self.analysis.elapsed
                ))
                .size(11.)
                .color(MUTED),
            );
        });
        let alternatives = self
            .analysis
            .lines
            .iter()
            .skip(1)
            .cloned()
            .collect::<Vec<_>>();
        if !alternatives.is_empty() {
            ui.collapsing("Other candidate moves", |ui| {
                for candidate in alternatives {
                    if let Some(m) = candidate
                        .pv
                        .first()
                        .and_then(|u| game::uci_move(&pos, u).ok())
                    {
                        ui.label(format!("{}  {}", game::san(&pos, m), candidate.label()));
                        ui.label(game::explanation(&pos, m));
                        ui.separator();
                    }
                }
            });
        }
    }
    fn coach_ui(&mut self, ctx: &egui::Context) {
        let mut open = true;
        egui::Window::new("Chess.com connection").open(&mut open).default_width(440.).show(ctx, |ui| {
            ui.label("Use your existing signed-in Chrome tab. The optional Chrome extension connects automatically when you click Start helping. The bookmark is also available.");
            if ui.button("Prepare automatic Chrome connection").clicked() {
                match chess_assistant_pro::browser::prepare_extension() {
                    Ok(folder) => {ctx.copy_text(folder.display().to_string());self.status=format!("Extension folder copied: {}. In chrome://extensions enable Developer mode, click Load unpacked and choose this folder. Then reload Chess.com once.",folder.display());},
                    Err(error) => self.status=error,
                }
            }
            ui.label("One-time extension setup: chrome://extensions → Developer mode → Load unpacked. The button above prepares your private connection files and copies their folder path. Chrome may ask to allow the local app connection.");
            if ui.button("Open bookmark setup").clicked() {
                if self.screen_reader.is_none() {self.enable_screen_reader();}
                if let Err(e)=chess_assistant_pro::browser::open_setup(){self.status=e;}
            }
            if ui.button("Copy bookmark code").clicked() {
                match chess_assistant_pro::browser::bookmarklet(){Ok(code)=>ctx.copy_text(code),Err(e)=>self.status=e}
            }
            if self.screen_reader.is_none() || self.browser_paused {
                if primary(ui,"Start helping").clicked() { self.enable_screen_reader(); }
            } else if ui.button("Stop helping").clicked() { self.stop_helping(); }
            ui.label(&self.status);
            ui.checkbox(&mut self.settings.companion_auto_start,"Start helping automatically when the app opens");
            if ui.checkbox(&mut self.settings.coach_overlay,"Show move arrow on Chess.com").changed() {
                if let Some(coach)=&self.screen_reader {coach.arrows(self.settings.coach_overlay);}
            }
            if ui.checkbox(&mut self.compact,"Compact app window").changed() { self.resize_coach(ctx); }
            if ui.button("Load current FEN (fallback)").clicked() { self.open_import(false); }
            ui.label(RichText::new("A settled move appears after the short engine search and stays until the position changes. Details adds numbered turquoise, red and blue arrows. Stop removes them immediately. The app stays visible while helping. You can minimize it yourself.").size(12.).color(MUTED));
        });
        self.coach_open = open;
    }
    fn library_ui(&mut self, ctx: &egui::Context) {
        use chess_assistant_pro::library::{BOOKS, search};
        let mut open = true;
        egui::Window::new("Chess library · six complete classics").open(&mut open).default_size([760.,600.]).show(ctx,|ui|{
            ui.label("411,000+ words stored locally. Historical texts use older notation; engine analysis uses modern rules.");
            ui.horizontal(|ui| {ui.label("Search all books");if ui.text_edit_singleline(&mut self.library_query).changed(){self.library_results=search(&self.library_query,6);}});
            if !self.library_query.trim().is_empty(){
                egui::ScrollArea::vertical().max_height(400.).show(ui,|ui|{
                    if self.library_results.is_empty(){ui.label("No matching passages. Try development, centre, pawn, rook or ending.");}
                    for passage in &self.library_results{ui.heading(passage.title);ui.label(passage.author);ui.label(&passage.passage);ui.hyperlink_to("Source edition",passage.url);ui.separator();}
                });
            }else{
                egui::ComboBox::from_id_salt("library_book").selected_text(BOOKS[self.library_book].title).show_ui(ui,|ui|{for(index,book)in BOOKS.iter().enumerate(){ui.selectable_value(&mut self.library_book,index,book.title);}});
                let book=&BOOKS[self.library_book];ui.label(book.author);ui.hyperlink_to("Source edition and rights",book.url);
                let lines=book.text.lines().collect::<Vec<_>>();
                egui::ScrollArea::both().id_salt(("book_text",self.library_book)).max_height(480.).show_rows(ui,18.,lines.len(),|ui,range|{for index in range{ui.add_sized([ui.available_width().max(800.),18.],egui::Label::new(RichText::new(lines[index]).monospace().size(13.)).halign(egui::Align::Min).extend());}});
            }
        });
        self.library_open = open;
    }
    fn settings_ui(&mut self, ctx: &egui::Context) {
        let mut open = true;
        egui::Window::new("Studio settings").open(&mut open).default_width(530.).show(ctx,|ui|{
        egui::ScrollArea::vertical().max_height(650.).show(ui,|ui|{
            ui.heading("Engine resources");ui.add(egui::Slider::new(&mut self.settings.seconds,0.2..=30.).text("Seconds per position"));ui.add(egui::Slider::new(&mut self.settings.depth,1..=60).text("Depth ceiling"));ui.add(egui::Slider::new(&mut self.settings.threads,1..=32).text("CPU threads"));ui.add(egui::Slider::new(&mut self.settings.hash,16..=4096).text("Hash memory (MB)"));ui.add(egui::Slider::new(&mut self.settings.multipv,1..=3).text("Candidate moves"));
            ui.label("Search stops at the time limit or depth ceiling. Analysis always uses full strength.");
            if ui.button("Choose Stockfish executable").clicked(){if let Some(p)=rfd::FileDialog::new().add_filter("Engine",&["exe"]).pick_file(){self.settings.engine_path=p.display().to_string();}}
            ui.label(RichText::new(self.settings.engine().display().to_string()).size(10.).color(MUTED));
            if ui.button("Choose Syzygy tablebase folder").clicked(){if let Some(p)=rfd::FileDialog::new().pick_folder(){self.settings.tablebase_path=p.display().to_string();}}ui.label(RichText::new(&self.settings.tablebase_path).size(10.));
            if ui.button("Choose Polyglot opening book").clicked(){if let Some(p)=rfd::FileDialog::new().add_filter("Polyglot", &["bin"]).pick_file(){self.settings.book_path=p.display().to_string();}}
            ui.label(RichText::new(&self.settings.book_path).size(10.));if ui.button("Disable book and tablebases").clicked(){self.settings.book_path.clear();self.settings.tablebase_path.clear();}
            ui.separator();ui.heading("Board & appearance");egui::ComboBox::from_id_salt("theme").selected_text(["Sage","Walnut","Ocean"][self.settings.theme]).show_ui(ui,|ui|{for(i,t)in["Sage","Walnut","Ocean"].iter().enumerate(){ui.selectable_value(&mut self.settings.theme,i,*t);}});
            ui.checkbox(&mut self.settings.flipped,"Black at the bottom");ui.checkbox(&mut self.settings.coordinates,"Coordinates");ui.checkbox(&mut self.settings.arrows,"Recommended move arrow");ui.checkbox(&mut self.settings.evaluation,"Evaluation bar");ui.checkbox(&mut self.settings.sound,"Move sound");ui.add(egui::Slider::new(&mut self.settings.animation,0.0..=0.8).text("Move animation seconds"));
            if primary(ui,"Save settings").clicked(){self.status=match self.settings.save(){Ok(())=>"Settings saved.".into(),Err(e)=>format!("Could not save settings: {e}")};self.settings_open=false;}
        });});
        self.settings_open &= open;
    }
    fn review_ui(&mut self, ctx: &egui::Context) {
        let mut open = true;
        egui::Window::new("Game review")
            .open(&mut open)
            .default_width(850.)
            .vscroll(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for color in [Color::White, Color::Black] {
                        let rows: Vec<_> = self
                            .history
                            .iter()
                            .filter(|r| self.game.positions[r.ply].turn() == color)
                            .collect();
                        let mean = if rows.is_empty() {
                            0.
                        } else {
                            rows.iter().map(|r| r.accuracy).sum::<f32>() / rows.len() as f32
                        };
                        card(ui, |ui| {
                            ui.label(
                                RichText::new(format!(
                                    "{} ACCURACY",
                                    if color == Color::White {
                                        "WHITE"
                                    } else {
                                        "BLACK"
                                    }
                                ))
                                .color(MUTED)
                                .size(11.),
                            );
                            ui.label(RichText::new(format!("{mean:.1}%")).size(32.).color(GOLD));
                        });
                    }
                });
                ui.label(
                    RichText::new(
                        "Estimated from centipawn loss. Results depend on search time and depth.",
                    )
                    .size(12.)
                    .color(MUTED),
                );
                ui.label(format!("Reviewed {} of {} moves",self.history.len(),self.game.moves.len()));
                egui::Grid::new("review_counts").num_columns(3).striped(true).show(ui,|ui|{
                    ui.strong("Move quality");ui.strong("White");ui.strong("Black");ui.end_row();
                    for class in ["Brilliant","Great","Best","Excellent","Good","Inaccuracy","Mistake","Blunder"] {
                        ui.colored_label(class_color(class),class);
                        for color in [Color::White,Color::Black] {ui.label(self.history.iter().filter(|r|r.class==class&&self.game.positions[r.ply].turn()==color).count().to_string());}
                        ui.end_row();
                    }
                });
                ui.label(RichText::new("RbChessAssist's own engine-based criteria; labels and accuracy may differ from Chess.com.").size(11.).color(MUTED));
                let (r, _) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), 100.), Sense::hover());
                ui.painter().rect_filled(r, 8., BG);
                ui.painter()
                    .hline(r.x_range(), r.center().y, Stroke::new(1.0_f32, MUTED));
                let n = self.history.len().max(2);
                for pair in self.history.windows(2) {
                    let point = |row: &ReviewRow| {
                        Pos2::new(
                            r.min.x + row.ply as f32 / (n - 1) as f32 * r.width(),
                            r.center().y - row.white_eval.clamp(-8., 8.) / 8. * r.height() * 0.45,
                        )
                    };
                    ui.painter().line_segment(
                        [point(&pair[0]), point(&pair[1])],
                        Stroke::new(2.0_f32, CYAN),
                    );
                }
                let rows = self.history.clone();
                egui::ScrollArea::vertical()
                    .max_height(420.)
                    .show(ui, |ui| {
                        for row in rows {
                            ui.horizontal(|ui| {
                                let p=&self.game.positions[row.ply];
                                if ui.button(format!("{}{} {}", p.fullmoves(), if p.turn()==Color::White {"."} else {"..."}, row.san)).clicked() {
                                    self.game.cursor = row.ply + 1;
                                    self.playing = false;
                                    self.home = false;
                                }
                                ui.label(
                                    RichText::new(&row.class)
                                        .color(class_color(&row.class))
                                        .strong(),
                                );
                                ui.label(format!("{} cp loss", row.loss));
                            });
                            ui.label(RichText::new(&row.note).size(12.).color(MUTED));
                            ui.label(format!("Engine preferred line: {}",row.best_line));
                            ui.separator();
                        }
                    });
                if self.busy {
                    ui.label("Review in progress…");
                }
            });
        self.review_open = open;
    }
    fn screenshot_ui(&mut self, ctx: &egui::Context) {
        let mut open = true;
        let mut accepted = None;
        let textures = &self.textures;
        let shot = self.shot.as_mut().unwrap();
        egui::Window::new("Import & verify a chessboard").open(&mut open).default_width(980.).show(ctx,|ui|{
            ui.label(&shot.message);ui.horizontal_top(|ui|{
                ui.vertical(|ui|{ui.set_width(440.);let size=Vec2::new(440.,330.);let response=ui.add(egui::Image::new(&shot.texture).max_size(size));let r=response.rect;let scale=r.width()/shot.image.width() as f32;let [x,y,w,h]=shot.bounds;let bounds=Rect::from_min_size(r.min+Vec2::new(x as f32,y as f32)*scale,Vec2::new(w as f32,h as f32)*scale);ui.painter().rect_stroke(bounds,0.,Stroke::new(2.0_f32,CYAN),egui::StrokeKind::Inside);
                    ui.horizontal(|ui|{ui.label("X");ui.add(egui::DragValue::new(&mut shot.bounds[0]).range(0..=shot.image.width()));ui.label("Y");ui.add(egui::DragValue::new(&mut shot.bounds[1]).range(0..=shot.image.height()));ui.label("Size");ui.add(egui::DragValue::new(&mut shot.bounds[2]).range(80..=shot.image.width().min(shot.image.height())));shot.bounds[3]=shot.bounds[2];});
                    if ui.button("Detect pieces").clicked(){match vision::recognize(&shot.image,shot.bounds){Ok(r)=>{shot.cells=r.cells;shot.uncertain=r.uncertain;shot.flipped=r.flipped;shot.detected=true;shot.message=format!("{} uncertain squares. Correct pieces and confirm the game state below.",shot.uncertain.len());},Err(e)=>shot.message=e}}
                    if ui.button("Rotate recognized position").clicked(){shot.cells.reverse();shot.uncertain=shot.uncertain.iter().map(|i|63-i).collect();shot.flipped=!shot.flipped;}
                    ui.label("Screenshot metadata must be confirmed manually.");ui.checkbox(&mut shot.black,"Black to move");ui.horizontal(|ui|{ui.label("Castling rights");ui.text_edit_singleline(&mut shot.castling);});ui.horizontal(|ui|{ui.label("En passant square");ui.text_edit_singleline(&mut shot.ep);});
                    ui.label(RichText::new("Use KQkq or leave blank for no castling. En passant may be blank. Orientation is inferred and can be rotated.").size(12.).color(MUTED));
                });
                ui.vertical(|ui|{ui.set_width(420.);let (r,response)=ui.allocate_exact_size(Vec2::splat(360.),Sense::click());let cell=45.;for i in 0..64{let rank=i/8;let file=i%8;let tile=Rect::from_min_size(r.min+Vec2::new(file as f32*cell,(7-rank) as f32*cell),Vec2::splat(cell));let c=if (rank+file)%2==1{vision::THEMES[0].0}else{vision::THEMES[0].1};ui.painter().rect_filled(tile,0.,Color32::from_rgb(c[0],c[1],c[2]));if let Some(c)=shot.cells[i]{ui.painter().image(textures[vision::piece_index(c)].id(),tile.shrink(3.),Rect::from_min_max(Pos2::ZERO,Pos2::new(1.,1.)),Color32::WHITE);}if shot.uncertain.contains(&i){ui.painter().rect_stroke(tile,0.,Stroke::new(2.0_f32,GOLD),egui::StrokeKind::Inside);}}
                    if response.clicked(){if let Some(p)=response.interact_pointer_pos(){let file=((p.x-r.min.x)/cell) as usize;let row=((p.y-r.min.y)/cell) as usize;if file<8&&row<8{let index=(7-row)*8+file;shot.cells[index]=shot.brush;shot.uncertain.retain(|i|*i!=index);shot.detected=true;}}}
                    ui.horizontal_wrapped(|ui|{for c in vision::SYMBOLS{if ui.selectable_label(shot.brush==Some(c),c.to_string()).clicked(){shot.brush=Some(c);}}if ui.selectable_label(shot.brush.is_none(),"Erase").clicked(){shot.brush=None;}});
                    ui.label(RichText::new("Uppercase = White · lowercase = Black").size(12.).color(MUTED));
                    if shot.detected&&primary(ui,"Confirm & load position").clicked(){let f=vision::cells_fen(&shot.cells,shot.black,&shot.castling,&shot.ep);match game::parse_fen(&f){Ok(p)=>accepted=Some(p),Err(e)=>shot.message=format!("Position needs correction: {e}")}}
                });
            });
        });
        if let Some(pos) = accepted {
            self.load(Game::new(pos));
            self.shot = None;
        } else if !open {
            self.shot = None;
        }
    }
}
impl eframe::App for Studio {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some((path, started, requested)) = &mut self.capture {
            if started.elapsed() > Duration::from_secs(3) && !self.busy && !*requested {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
                *requested = true;
            }
            for event in ctx.input(|i| i.events.clone()) {
                if let egui::Event::Screenshot { image, .. } = event {
                    let bytes: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                    if let Some(img) = image::RgbaImage::from_raw(
                        image.size[0] as u32,
                        image.size[1] as u32,
                        bytes,
                    ) {
                        let _ = img.save(&*path);
                    }
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
        self.poll(ctx);
        self.header(ctx);
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(24.))
            .show(ctx, |ui| {
                if self.compact {
                    self.coach_controls(ui);
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        self.coach_body(ui);
                    });
                } else {
                    egui::ScrollArea::both().show(ui, |ui| {
                        if self.home {
                            self.home_ui(ui, ctx);
                        } else {
                            self.studio_ui(ui, ctx);
                        }
                    });
                }
            });

        self.dialogs(ctx);
        ctx.request_repaint_after(Duration::from_millis(if self.busy { 30 } else { 100 }));
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.worker.cancel();
        if self.capture.is_none() {
            self.settings.coach_compact = self.compact;
            let _ = self.settings.save();
        }
    }
}
fn primary(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).color(BG).strong())
            .fill(GOLD)
            .min_size(Vec2::new(120., 38.)),
    )
}
fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::new()
        .fill(PANEL)
        .corner_radius(12.)
        .inner_margin(18.)
        .show(ui, add)
        .inner
}
fn class_color(class: &str) -> Color32 {
    match class {
        "Brilliant" | "Great" => CYAN,
        "Best" | "Excellent" | "Good" => GREEN,
        "Inaccuracy" => GOLD,
        "Mistake" => Color32::from_rgb(233, 152, 91),
        "Blunder" => Color32::from_rgb(240, 110, 117),
        _ => MUTED,
    }
}
