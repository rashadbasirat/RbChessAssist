//! Interactive smoke test: render a controlled board, capture its real Windows
//! window, recognize two positions, then ask Stockfish for a legal continuation.
//! Run with `cargo run --release --example screen_smoke -- --verify` on a Windows desktop.
use chess_assistant_pro::{
    companion, engine,
    game::{self, Game},
    overlay,
    settings::Settings,
    vision,
};
use eframe::egui;
use std::{
    sync::atomic::AtomicU64,
    time::{Duration, Instant},
};

const TITLE: &str = "Chess computer capture test fixture";
struct Fixture {
    texture: egui::TextureHandle,
    game: Game,
    model: Option<vision::ScreenTemplates>,
    ready: Instant,
    capture_job: Option<std::thread::JoinHandle<Result<image::RgbImage, String>>>,
    deadline: Instant,
    hold: bool,
}
impl Fixture {
    fn texture(ctx: &egui::Context, game: &Game) -> egui::TextureHandle {
        let img = vision::board_image(game.pos(), false, 0, 640);
        ctx.load_texture(
            "fixture",
            egui::ColorImage::from_rgb([640, 640], img.as_raw()),
            Default::default(),
        )
    }
    fn capture() -> Result<image::RgbImage, String> {
        // XCap intentionally excludes its own process; capture from a child.
        let mut command =
            std::process::Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
        command.arg("--capture");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let output = command.output().map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).to_string());
        }
        Ok(image::load_from_memory(&output.stdout)
            .map_err(|e| e.to_string())?
            .to_rgb8())
    }
    fn verify(&mut self, ctx: &egui::Context, pixels: image::RgbImage) -> Result<(), String> {
        let [x, y, w, h] =
            vision::detect_screen_bounds(&pixels).ok_or("Board not located in captured window")?;
        let board = image::imageops::crop_imm(&pixels, x, y, w, h).to_image();
        if let Some(model) = &self.model {
            let placement = model.recognize(&board)?;
            let tracked = companion::synchronize(&Game::default(), &placement)?;
            if game::fen(tracked.pos()) != game::fen(self.game.pos()) {
                return Err("Position mismatch".into());
            }
            let settings = Settings {
                seconds: 0.2,
                hash: 32,
                threads: 1,
                ..Default::default()
            };
            let analysis = engine::search(tracked.pos(), &settings, false, &AtomicU64::new(1), 1)?;
            game::uci_move(tracked.pos(), &analysis.best)?;
            println!(
                "PASS: native capture -> recognition -> legal tracking -> Stockfish {}",
                analysis.best
            );
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else {
            self.model = Some(vision::ScreenTemplates::learn_start(&board)?);
            println!("PASS: captured and learned the starting board");
            for uci in ["e2e4", "c7c5"] {
                self.game.push(game::uci_move(self.game.pos(), uci)?)?;
            }
            self.texture = Self::texture(ctx, &self.game);
            self.ready = Instant::now() + Duration::from_secs(2);
        }
        Ok(())
    }
}
impl eframe::App for Fixture {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.; 4]
    }
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        let mut board_rect = egui::Rect::NOTHING;
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Automated native-capture test fixture — closes after verification");
            board_rect = ui
                .add(egui::Image::new((
                    self.texture.id(),
                    egui::Vec2::splat(640.0),
                )))
                .rect;
        });
        if self.hold {
            if let Some(inner) = ctx.input(|i| i.viewport().inner_rect) {
                let scale = ctx.pixels_per_point();
                let origin = inner.min + board_rect.min.to_vec2();
                overlay::show(
                    ctx,
                    companion::BoardLocation {
                        x: origin.x * scale,
                        y: origin.y * scale,
                        size: board_rect.width() * scale,
                        scale,
                        flipped: false,
                        focused: true,
                    },
                    game::uci_move(self.game.pos(), "e2e4").unwrap(),
                );
            }
        }
        if Instant::now() > self.deadline {
            if self.hold {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                eprintln!("FAIL: capture smoke test timed out");
                std::process::exit(1);
            }
        }
        if !self.hold {
            if self.capture_job.as_ref().is_some_and(|j| j.is_finished()) {
                let result = self
                    .capture_job
                    .take()
                    .unwrap()
                    .join()
                    .unwrap()
                    .and_then(|pixels| self.verify(ctx, pixels));
                if let Err(error) = result {
                    eprintln!("FAIL: {error}");
                    std::process::exit(1);
                }
            } else if self.capture_job.is_none() && Instant::now() > self.ready {
                self.capture_job = Some(std::thread::spawn(Self::capture));
            }
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}
fn main() -> eframe::Result<()> {
    chess_assistant_pro::platform::prepare_dpi();
    if std::env::args().any(|a| a == "--capture") {
        let window = xcap::Window::all()
            .unwrap()
            .into_iter()
            .find(|w| w.title().is_ok_and(|title| title == TITLE))
            .expect("Fixture window missing");
        let image = image::DynamicImage::ImageRgba8(window.capture_image().unwrap());
        let mut bytes = std::io::Cursor::new(Vec::new());
        image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        use std::io::Write;
        std::io::stdout().write_all(bytes.get_ref()).unwrap();
        return Ok(());
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_transparent(true)
            .with_inner_size([680.0, 700.0])
            .with_title(TITLE),
        ..Default::default()
    };
    eframe::run_native(
        TITLE,
        options,
        Box::new(|cc| {
            let game = Game::default();
            Ok(Box::new(Fixture {
                texture: Fixture::texture(&cc.egui_ctx, &game),
                game,
                model: None,
                ready: Instant::now() + Duration::from_secs(2),
                capture_job: None,
                deadline: Instant::now()
                    + Duration::from_secs(if !std::env::args().any(|a| a == "--verify") {
                        240
                    } else {
                        20
                    }),
                hold: !std::env::args().any(|a| a == "--verify"),
            }))
        }),
    )
}
