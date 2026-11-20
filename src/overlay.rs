use crate::{companion::BoardLocation, game};
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use shakmaty::{Move, Square};

pub fn square_center(rect: Rect, square: Square, flipped: bool) -> Pos2 {
    let index = square as usize;
    let (file, rank) = (index % 8, index / 8);
    let (col, row) = if flipped {
        (7 - file, rank)
    } else {
        (file, 7 - rank)
    };
    rect.min
        + Vec2::new(
            (col as f32 + 0.5) * rect.width() / 8.0,
            (row as f32 + 0.5) * rect.height() / 8.0,
        )
}
pub fn draw(painter: &egui::Painter, rect: Rect, m: Move, flipped: bool) {
    let Some((a, b)) = game::move_squares(m) else {
        return;
    };
    let from = square_center(rect, a, flipped);
    let to = square_center(rect, b, flipped);
    let cell = rect.width() / 8.0;
    let color = Color32::from_rgba_unmultiplied(45, 226, 213, 225);
    painter.circle_stroke(from, cell * 0.22, Stroke::new(cell * 0.045, color));
    painter.arrow(
        from,
        (to - from) * 0.93,
        Stroke::new(cell * 0.10, Color32::from_black_alpha(160)),
    );
    painter.arrow(from, (to - from) * 0.93, Stroke::new(cell * 0.065, color));
    painter.circle_stroke(to, cell * 0.25, Stroke::new(cell * 0.035, color));
}
pub fn show(ctx: &egui::Context, location: BoardLocation, m: Move) {
    // The native capture excludes this separate window, so its arrow cannot
    // contaminate recognition. Mouse passthrough leaves the game interactive.
    let scale = location.scale;
    ctx.show_viewport_immediate(
        egui::ViewportId::from_hash_of("chess_move_overlay"),
        egui::ViewportBuilder::default()
            .with_title("RbChessAssist — Move arrow")
            .with_position([location.x / scale, location.y / scale])
            .with_inner_size([location.size / scale, location.size / scale])
            .with_decorations(false)
            .with_transparent(true)
            .with_resizable(false)
            .with_always_on_top()
            .with_mouse_passthrough(true)
            .with_active(false)
            .with_taskbar(false),
        |ctx, _| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ctx, |ui| {
                    draw(ui.painter(), ui.max_rect(), m, location.flipped);
                });
        },
    );
}
