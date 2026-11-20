#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod ui;
fn main() -> eframe::Result<()> {
    chess_assistant_pro::platform::prepare_dpi();
    let Some(_instance) = chess_assistant_pro::platform::AppInstance::acquire() else {
        return Ok(());
    };
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_transparent(true)
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1020.0, 700.0])
            .with_title("RbChessAssist"),
        ..Default::default()
    };
    eframe::run_native(
        "RbChessAssist",
        options,
        Box::new(|cc| Ok(Box::new(ui::Studio::new(cc)))),
    )
}
