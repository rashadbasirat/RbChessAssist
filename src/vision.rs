use image::{Rgb, RgbImage, RgbaImage, imageops::FilterType};
use shakmaty::{Chess, Position, Square};

pub const SYMBOLS: [char; 12] = ['P', 'N', 'B', 'R', 'Q', 'K', 'p', 'n', 'b', 'r', 'q', 'k'];
pub const PIECES: [&[u8]; 12] = [
    include_bytes!("../assets/pieces/wp.png"),
    include_bytes!("../assets/pieces/wn.png"),
    include_bytes!("../assets/pieces/wb.png"),
    include_bytes!("../assets/pieces/wr.png"),
    include_bytes!("../assets/pieces/wq.png"),
    include_bytes!("../assets/pieces/wk.png"),
    include_bytes!("../assets/pieces/bp.png"),
    include_bytes!("../assets/pieces/bn.png"),
    include_bytes!("../assets/pieces/bb.png"),
    include_bytes!("../assets/pieces/br.png"),
    include_bytes!("../assets/pieces/bq.png"),
    include_bytes!("../assets/pieces/bk.png"),
];
pub const THEMES: [([u8; 3], [u8; 3]); 3] = [
    ([209, 218, 207], [100, 132, 124]),
    ([227, 209, 179], [151, 116, 91]),
    ([187, 202, 218], [82, 107, 138]),
];
pub fn piece_index(c: char) -> usize {
    SYMBOLS.iter().position(|v| *v == c).unwrap_or(0)
}
pub fn templates() -> Vec<RgbaImage> {
    PIECES
        .iter()
        .map(|p| image::load_from_memory(p).unwrap().to_rgba8())
        .collect()
}
pub fn draw_piece(tile: &mut RgbImage, piece: &RgbaImage) {
    let size = tile.width();
    let piece_size = (size as f32 * 0.86).round() as u32;
    let resized;
    let pixels = if piece.width() == piece_size && piece.height() == piece_size {
        piece
    } else {
        resized = image::imageops::resize(piece, piece_size, piece_size, FilterType::Lanczos3);
        &resized
    };
    let offset = (size - piece_size) / 2;
    for (x, y, p) in pixels.enumerate_pixels() {
        let dest = tile.get_pixel_mut(x + offset, y + offset);
        let a = p[3] as f32 / 255.;
        for c in 0..3 {
            dest[c] = (p[c] as f32 * a + dest[c] as f32 * (1. - a)).round() as u8;
        }
    }
}
pub fn board_image(pos: &Chess, flipped: bool, theme: usize, size: u32) -> RgbImage {
    let pieces = templates();
    let size = size / 8 * 8;
    let step = size / 8;
    let mut result = RgbImage::new(size, size);
    let colors = THEMES[theme.min(2)];
    for row in 0..8 {
        for col in 0..8 {
            let index = if flipped {
                row * 8 + 7 - col
            } else {
                (7 - row) * 8 + col
            };
            let sq = Square::new(index);
            let bg = if (row + col) % 2 == 0 {
                colors.0
            } else {
                colors.1
            };
            let mut tile = RgbImage::from_pixel(step, step, Rgb(bg));
            if let Some(p) = pos.board().piece_at(sq) {
                draw_piece(&mut tile, &pieces[piece_index(p.char())]);
            }
            image::imageops::replace(&mut result, &tile, (col * step) as i64, (row * step) as i64);
        }
    }
    result
}
fn distance(a: &[u8], b: &[u8]) -> u32 {
    (0..3)
        .map(|i| (a[i] as i32 - b[i] as i32).unsigned_abs())
        .sum()
}
pub fn detect_bounds(img: &RgbImage) -> Option<[u32; 4]> {
    detect_with_palettes(img, &THEMES)
}
pub fn detect_screen_bounds(img: &RgbImage) -> Option<[u32; 4]> {
    detect_with_palettes(
        img,
        &[
            ([238, 238, 210], [118, 150, 86]),
            ([235, 236, 208], [119, 149, 86]),
            THEMES[0],
            THEMES[1],
            THEMES[2],
        ],
    )
    .or_else(|| detect_any_palette(img))
}
fn checker_score(img: &RgbImage, bounds: [u32; 4], a: [u8; 3], b: [u8; 3]) -> f32 {
    let [x, y, w, h] = bounds;
    let mut correct = 0.0_f32;
    let mut total = 0.0_f32;
    for row in 0..8 {
        for col in 0..8 {
            for (ox, oy) in [(0.08, 0.08), (0.92, 0.08), (0.08, 0.92), (0.92, 0.92)] {
                let xx = (x as f32 + (col as f32 + ox) * w as f32 / 8.) as u32;
                let yy = (y as f32 + (row as f32 + oy) * h as f32 / 8.) as u32;
                if xx >= img.width() || yy >= img.height() {
                    return 0.;
                }
                let p = img.get_pixel(xx, yy).0;
                let da = distance(&p, &a);
                let db = distance(&p, &b);
                if da.min(db) < 85 && (da < db) == ((row + col) % 2 == 0) {
                    correct += 1.;
                }
                total += 1.;
            }
        }
    }
    (correct / total).max(1. - correct / total)
}
fn detect_any_palette(img: &RgbImage) -> Option<[u32; 4]> {
    let ratio = (img.width().max(img.height()) as f32 / 480.).max(1.);
    let small = image::imageops::resize(
        img,
        (img.width() as f32 / ratio) as u32,
        (img.height() as f32 / ratio) as u32,
        FilterType::Nearest,
    );
    let mut hist = vec![(0u32, [0u64; 3]); 4096];
    for p in small.pixels() {
        let k = ((p[0] as usize >> 4) << 8) | ((p[1] as usize >> 4) << 4) | (p[2] as usize >> 4);
        hist[k].0 += 1;
        for c in 0..3 {
            hist[k].1[c] += p[c] as u64;
        }
    }
    hist.sort_by_key(|v| std::cmp::Reverse(v.0));
    let colors: Vec<[u8; 3]> = hist
        .iter()
        .take(14)
        .filter(|v| v.0 > 80)
        .map(|(n, s)| std::array::from_fn(|c| (s[c] / *n as u64) as u8))
        .collect();
    let mut best = None;
    let mut best_area = 0;
    for i in 0..colors.len() {
        for j in i + 1..colors.len() {
            if distance(&colors[i], &colors[j]) < 70 {
                continue;
            }
            let Some(bounds) = detect_palette_rows(&small, &[(colors[i], colors[j])], 34, 36)
            else {
                continue;
            };
            if checker_score(&small, bounds, colors[i], colors[j]) < 0.80 {
                continue;
            }
            let area = bounds[2] * bounds[3];
            if area > best_area {
                best_area = area;
                let [x, y, w, h] = bounds;
                let original = [
                    (x as f32 * ratio).round() as u32,
                    (y as f32 * ratio).round() as u32,
                    (w as f32 * ratio).round() as u32,
                    (h as f32 * ratio).round() as u32,
                ];
                if original[0] + original[2] <= img.width()
                    && original[1] + original[3] <= img.height()
                {
                    best = Some(original);
                }
            }
        }
    }
    best
}
fn detect_with_palettes(img: &RgbImage, palettes: &[([u8; 3], [u8; 3])]) -> Option<[u32; 4]> {
    detect_palette_rows(img, palettes, 12, 128)
}
fn detect_palette_rows(
    img: &RgbImage,
    palettes: &[([u8; 3], [u8; 3])],
    tolerance: u32,
    minimum: u32,
) -> Option<[u32; 4]> {
    // Native themes have distinctive square colors. Largest contiguous rows reject toolbar pixels.
    let mut best = None;
    let mut area = 0;
    for colors in palettes {
        let mut rows = Vec::new();
        for y in 0..img.height() {
            let mut first = img.width();
            let mut last = 0;
            let mut hits = 0;
            for x in 0..img.width() {
                let p = img.get_pixel(x, y);
                if distance(&p.0, &colors.0) < tolerance || distance(&p.0, &colors.1) < tolerance {
                    first = first.min(x);
                    last = x;
                    hits += 1;
                }
            }
            if hits > minimum && last > first && hits * 100 / (last - first + 1) > 40 {
                rows.push((y, first, last));
            }
        }
        if let (Some(top), Some(bottom)) = (rows.first(), rows.last()) {
            let width = top.2 - top.1 + 1;
            let height = bottom.0 - top.0 + 1;
            if width.abs_diff(height) < 8 && width * height > area {
                area = width * height;
                best = Some([top.1, top.0, width.min(height), width.min(height)]);
            }
        }
    }
    best
}
pub struct Recognition {
    pub cells: [Option<char>; 64],
    pub uncertain: Vec<usize>,
    pub flipped: bool,
}
pub fn recognize(img: &RgbImage, bounds: [u32; 4]) -> Result<Recognition, String> {
    let [x, y, w, h] = bounds;
    if w < 80 || h < 80 || x + w > img.width() || y + h > img.height() {
        return Err("Select a square board area inside the image.".into());
    }
    let crop = image::imageops::crop_imm(img, x, y, w, h).to_image();
    let normalized = image::imageops::resize(&crop, 512, 512, FilterType::Lanczos3);
    let pieces: Vec<_> = templates()
        .iter()
        .map(|piece| image::imageops::resize(piece, 55, 55, FilterType::Lanczos3))
        .collect();
    let mut cells = [None; 64];
    let mut uncertain = Vec::new();
    for row in 0..8 {
        for col in 0..8 {
            let cell =
                image::imageops::crop_imm(&normalized, col * 64, row * 64, 64, 64).to_image();
            let bg = *cell.get_pixel(3, 3);
            let mut scores = Vec::new();
            for p in std::iter::once(None).chain(pieces.iter().map(Some)) {
                let mut reference = RgbImage::from_pixel(64, 64, bg);
                if let Some(p) = p {
                    draw_piece(&mut reference, p);
                }
                let mut error = 0.0;
                for yy in 7..57 {
                    for xx in 7..57 {
                        let a = cell.get_pixel(xx, yy);
                        let b = reference.get_pixel(xx, yy);
                        error += (0..3)
                            .map(|c| ((a[c] as f32 - b[c] as f32) / 255.0).powi(2))
                            .sum::<f32>();
                    }
                }
                scores.push(error / 7500.0);
            }
            let mut ranking: Vec<_> = scores.iter().enumerate().collect();
            ranking.sort_by(|a, b| a.1.total_cmp(b.1));
            let (chosen, error) = ranking[0];
            let idx = ((7 - row) * 8 + col) as usize;
            if *error > 0.025 || (*ranking[1].1 - *error < 0.002 && chosen > 0) {
                uncertain.push(idx);
            }
            if chosen > 0 {
                cells[idx] = Some(SYMBOLS[chosen - 1]);
            }
        }
    }
    let white_rank: i32 = cells
        .iter()
        .enumerate()
        .filter(|(_, p)| p.is_some_and(|p| p.is_uppercase()))
        .map(|(i, _)| (i / 8) as i32)
        .sum();
    let black_rank: i32 = cells
        .iter()
        .enumerate()
        .filter(|(_, p)| p.is_some_and(|p| p.is_lowercase()))
        .map(|(i, _)| (i / 8) as i32)
        .sum();
    let flipped = white_rank > black_rank;
    if flipped {
        cells.reverse();
        uncertain = uncertain.iter().map(|i| 63 - i).collect();
    }
    Ok(Recognition {
        cells,
        uncertain,
        flipped,
    })
}
pub fn cells_fen(cells: &[Option<char>; 64], black: bool, castling: &str, ep: &str) -> String {
    let mut rows = Vec::new();
    for rank in (0..8).rev() {
        let mut row = String::new();
        let mut empty = 0;
        for file in 0..8 {
            match cells[rank * 8 + file] {
                Some(c) => {
                    if empty > 0 {
                        row += &empty.to_string();
                        empty = 0;
                    }
                    row.push(c)
                }
                None => empty += 1,
            }
        }
        if empty > 0 {
            row += &empty.to_string();
        }
        rows.push(row);
    }
    format!(
        "{} {} {} {} 0 1",
        rows.join("/"),
        if black { "b" } else { "w" },
        if castling.is_empty() { "-" } else { castling },
        if ep.is_empty() { "-" } else { ep }
    )
}
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ScreenTemplates {
    features: Vec<Vec<f32>>,
    flipped: bool,
    #[serde(default)]
    backgrounds: Option<Vec<Vec<u8>>>,
}
fn tile_features(cell: &RgbImage) -> Vec<f32> {
    let sample = image::imageops::resize(cell, 64, 64, FilterType::Lanczos3);
    let corners = [
        sample.get_pixel(4, 4),
        sample.get_pixel(59, 4),
        sample.get_pixel(4, 59),
        sample.get_pixel(59, 59),
    ];
    let mut bg = [0u8; 3];
    for c in 0..3 {
        let mut values: Vec<u8> = corners.iter().map(|p| p[c]).collect();
        values.sort();
        bg[c] = ((values[1] as u16 + values[2] as u16) / 2) as u8;
    }
    let mut result = Vec::with_capacity(2048);
    let hi = (0..3).max_by_key(|&c| bg[c]).unwrap();
    let lo = (0..3).min_by_key(|&c| bg[c]).unwrap();
    let chroma = (bg[hi] as f32 - bg[lo] as f32).max(1.0);
    let background_luma = bg.iter().map(|&c| c as f32).sum::<f32>() / 765.0;
    for y in 0..32 {
        for x in 0..32 {
            let p = sample.get_pixel(7 + x * 50 / 32, 7 + y * 50 / 32);
            // Neutral pieces over colored squares: channel differences isolate
            // the background contribution, including antialiased edge pixels.
            let background = ((p[hi] as f32 - p[lo] as f32) / chroma).clamp(0.0, 1.0);
            let alpha = 1.0 - background;
            let luma = (p[0] as f32 + p[1] as f32 + p[2] as f32) / 765.0;
            result.push(alpha);
            result.push((luma - background * background_luma).clamp(0.0, alpha));
        }
    }
    result
}
fn feature_distance(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| (x - y).powi(2)).sum::<f32>() / a.len() as f32
}
fn occupancy(a: &[f32]) -> f32 {
    a.iter().step_by(2).sum::<f32>() / 1024.0
}
fn cell_features(board: &RgbImage) -> Vec<Vec<f32>> {
    let img = image::imageops::resize(board, 512, 512, FilterType::Lanczos3);
    (0..64)
        .map(|i| {
            tile_features(
                &image::imageops::crop_imm(&img, (i % 8) * 64, (7 - i / 8) * 64, 64, 64).to_image(),
            )
        })
        .collect()
}

fn background_features(img: &RgbImage, backgrounds: &[Vec<u8>]) -> Vec<Vec<f32>> {
    (0..64)
        .map(|i| {
            let bg = &backgrounds[(i % 8 + i / 8) % 2];
            let mut f = Vec::with_capacity(2048);
            for y in 0..32 {
                for x in 0..32 {
                    let xx = 7 + x * 50 / 32;
                    let yy = 7 + y * 50 / 32;
                    let p = img.get_pixel((i % 8 * 64) as u32 + xx, (7 - i / 8) as u32 * 64 + yy);
                    let n = ((yy * 64 + xx) * 3) as usize;
                    let delta = distance(&p.0, &bg[n..n + 3]) as f32;
                    let alpha = ((delta - 18.) / 55.).clamp(0., 1.);
                    f.push(alpha);
                    f.push(alpha * (p[0] as f32 + p[1] as f32 + p[2] as f32) / 765.);
                }
            }
            f
        })
        .collect()
}

impl ScreenTemplates {
    pub fn is_flipped(&self) -> bool {
        self.flipped
    }
    pub fn learn_start(board: &RgbImage) -> Result<Self, String> {
        Self::learn_position(board, &Chess::default())
    }
    pub fn learn_position(board: &RgbImage, position: &Chess) -> Result<Self, String> {
        let img = image::imageops::resize(board, 512, 512, FilterType::Lanczos3);
        let expected: Vec<Option<char>> = (0..64)
            .map(|i| position.board().piece_at(Square::new(i)).map(|p| p.char()))
            .collect();
        let mut last_error = "The board does not match the expected position.".to_string();
        for flipped in [false, true] {
            let mut physical = expected.clone();
            if flipped {
                physical.reverse();
            }
            let mut backgrounds = Vec::new();
            for parity in 0..2 {
                let empty: Vec<usize> = (0..64)
                    .filter(|&i| physical[i].is_none() && (i % 8 + i / 8) % 2 == parity)
                    .collect();
                if empty.len() < 3 {
                    return Err("At least three empty squares of each color are needed to learn this theme.".into());
                }
                let mut bg = vec![0u8; 64 * 64 * 3];
                for y in 0..64 {
                    for x in 0..64 {
                        for c in 0..3 {
                            let mut values: Vec<u8> = empty
                                .iter()
                                .map(|&i| {
                                    img.get_pixel(
                                        (i % 8) as u32 * 64 + x,
                                        (7 - i / 8) as u32 * 64 + y,
                                    )[c]
                                })
                                .collect();
                            values.sort_unstable();
                            bg[((y * 64 + x) * 3) as usize + c] = values[values.len() / 2];
                        }
                    }
                }
                backgrounds.push(bg);
            }
            let mut cells = background_features(&img, &backgrounds);
            if flipped {
                cells.reverse();
            }
            if cells
                .iter()
                .zip(&expected)
                .any(|(f, p)| (occupancy(f) > 0.035) != p.is_some())
            {
                last_error="The visible pieces do not match the known position. Start at the initial board, or load its current FEN first.".into();
                continue;
            }
            let mut features = vec![Vec::new(); 12];
            let mut color_brightness = [(0.0_f32, 0.0_f32); 2];
            let mut consistent = true;
            for (i, p) in expected.iter().enumerate() {
                let Some(symbol) = p else {
                    continue;
                };
                let idx = piece_index(*symbol);
                if features[idx].is_empty() {
                    features[idx] = cells[i].clone();
                } else if feature_distance(&features[idx], &cells[i]) > 0.08 {
                    consistent = false;
                }
                let side = if symbol.is_uppercase() { 0 } else { 1 };
                for pair in cells[i].chunks_exact(2) {
                    color_brightness[side].0 += pair[1];
                    color_brightness[side].1 += pair[0];
                }
            }
            if !consistent {
                last_error =
                    "Repeated pieces do not match. Keep the board still and unobstructed.".into();
                continue;
            }
            let white = color_brightness[0].0 / color_brightness[0].1.max(1.0_f32);
            let black = color_brightness[1].0 / color_brightness[1].1.max(1.0_f32);
            if white < black + 0.04 {
                last_error = "Piece colors or board orientation are ambiguous.".into();
                continue;
            }
            let model = Self {
                features,
                flipped,
                backgrounds: Some(backgrounds),
            };
            match model.recognize(board) {
                Ok(detected)
                    if detected
                        == crate::game::fen(position)
                            .split_whitespace()
                            .next()
                            .unwrap() =>
                {
                    return Ok(model);
                }
                Err(e) => last_error = e,
                _ => last_error = "Theme calibration did not reproduce the known position.".into(),
            }
        }
        Err(last_error)
    }
    pub fn recognize(&self, board: &RgbImage) -> Result<String, String> {
        if self.features.len() != 12
            || self
                .features
                .iter()
                .any(|f| !f.is_empty() && f.len() != 2048)
        {
            return Err(
                "Cached screen templates are invalid. Start a fresh game to relearn.".into(),
            );
        }
        let mut features = if let Some(backgrounds) = &self.backgrounds {
            if backgrounds.len() != 2 || backgrounds.iter().any(|b| b.len() != 64 * 64 * 3) {
                return Err("Invalid theme cache; relearn the theme.".into());
            }
            background_features(
                &image::imageops::resize(board, 512, 512, FilterType::Lanczos3),
                backgrounds,
            )
        } else {
            cell_features(board)
        };
        if self.flipped {
            features.reverse();
        }
        let mut cells = [None; 64];
        for (i, f) in features.iter().enumerate() {
            if occupancy(f) < 0.025 {
                continue;
            }
            let mut scores: Vec<_> = self
                .features
                .iter()
                .enumerate()
                .filter(|(_, t)| !t.is_empty())
                .map(|(j, t)| (j, feature_distance(f, t)))
                .collect();
            scores.sort_by(|a, b| a.1.total_cmp(&b.1));
            if scores.len() < 2 || scores[0].1 > 0.08 || scores[1].1 - scores[0].1 < 0.002 {
                return Err(format!(
                    "Uncertain piece on {}. Keep the board unobstructed and wait for animations to finish.",
                    Square::new(i as u32)
                ));
            }
            cells[i] = Some(SYMBOLS[scores[0].0]);
        }
        if cells.iter().filter(|p| **p == Some('K')).count() != 1
            || cells.iter().filter(|p| **p == Some('k')).count() != 1
        {
            return Err("Both kings must be visible before analyzing.".into());
        }
        Ok(cells_fen(&cells, false, "", "")
            .split_whitespace()
            .next()
            .unwrap()
            .to_string())
    }
}
