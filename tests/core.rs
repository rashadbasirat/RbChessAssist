use chess_assistant_pro::{
    engine::{self, Line},
    game::{self, Game},
    settings::Settings,
    vision,
};
use shakmaty::{Chess, Color, Position, Role, Square};
use std::sync::atomic::AtomicU64;

fn settings() -> Settings {
    Settings {
        seconds: 0.15,
        threads: 1,
        hash: 32,
        depth: 22,
        multipv: 3,
        ..Default::default()
    }
}
#[test]
fn legal_and_illegal_moves() {
    let mut g = Game::default();
    assert_eq!(g.pos().legal_moves().len(), 20);
    assert!(game::uci_move(g.pos(), "e2e5").is_err());
    g.push(game::uci_move(g.pos(), "e2e4").unwrap()).unwrap();
    assert_eq!(g.pos().turn(), Color::Black);
}
#[test]
fn castling_both_sides() {
    for u in ["e1g1", "e1c1"] {
        let p = game::parse_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
        let m = game::uci_move(&p, u).unwrap();
        assert!(m.is_castle());
    }
}
#[test]
fn en_passant() {
    let mut g = Game::default();
    for u in ["e2e4", "a7a6", "e4e5", "d7d5", "e5d6"] {
        g.push(game::uci_move(g.pos(), u).unwrap()).unwrap();
    }
    assert!(g.pos().board().piece_at(Square::D5).is_none());
    assert!(g.pos().board().piece_at(Square::D6).is_some());
}
#[test]
fn all_promotions() {
    for (s, role) in [
        ('q', Role::Queen),
        ('r', Role::Rook),
        ('b', Role::Bishop),
        ('n', Role::Knight),
    ] {
        let p = game::parse_fen("7k/P7/8/8/8/8/8/7K w - - 0 1").unwrap();
        assert_eq!(
            game::uci_move(&p, &format!("a7a8{s}")).unwrap().promotion(),
            Some(role)
        );
    }
}
#[test]
fn invalid_fen() {
    for s in [
        "garbage",
        "8/8/8/8/8/8/8/8 w - - 0 1",
        "8/8/8/8/8/8/4k3/4K3 w - - 0 1",
    ] {
        assert!(game::parse_fen(s).is_err());
    }
}
#[test]
fn pgn_roundtrip() {
    let g =
        game::parse_pgn("[Event \"Test\"]\n\n1. e4 {comment} e5 2.Nf3 (2.Bc4 Nc6) Nc6 3.Bb5 a6 *")
            .unwrap();
    assert_eq!(g.moves.len(), 6);
    let other = game::parse_pgn(&g.pgn()).unwrap();
    assert_eq!(
        game::fen(g.positions.last().unwrap()),
        game::fen(other.positions.last().unwrap())
    );
}
#[test]
fn corrupt_pgn() {
    assert!(game::parse_pgn("1. e4 e5 2. Ke5").is_err());
    assert!(game::parse_pgn("1. e4 {bad").is_err());
}
#[test]
fn draw_rules() {
    let g = Game::new(game::parse_fen("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1").unwrap());
    assert!(g.terminal().unwrap().contains("stalemate"));
    let g = Game::new(game::parse_fen("7k/8/6K1/8/8/8/8/8 w - - 0 1").unwrap());
    assert!(g.terminal().unwrap().contains("insufficient"));
}
#[test]
fn repetition() {
    let mut g = Game::default();
    for _ in 0..2 {
        for u in ["g1f3", "g8f6", "f3g1", "f6g8"] {
            g.push(game::uci_move(g.pos(), u).unwrap()).unwrap();
        }
    }
    assert_eq!(g.repetitions(), 3);
    assert!(g.can_claim_draw());
}
#[test]
fn classification_boundaries() {
    for (loss, label) in [
        (0, "Best"),
        (20, "Excellent"),
        (50, "Good"),
        (100, "Inaccuracy"),
        (200, "Mistake"),
        (201, "Blunder"),
    ] {
        assert_eq!(engine::classify(loss, loss == 0, false, false), label);
    }
    assert_eq!(engine::classify(0, true, true, false), "Great");
    assert_eq!(engine::classify(0, true, true, true), "Brilliant");
    assert_eq!(engine::classify(250, true, true, true), "Blunder");
}
#[test]
fn accuracy_formula() {
    assert_eq!(engine::accuracy(0), 100.);
    assert!((engine::accuracy(150) - 36.7879).abs() < 0.001);
    assert!(engine::accuracy(200) < engine::accuracy(100));
}
#[test]
fn score_white_perspective() {
    let (_, l) = engine::parse_info(
        "info depth 18 multipv 1 score cp 120 nodes 1200 pv e7e5 g1f3",
        Color::Black,
    )
    .unwrap();
    assert_eq!(l.cp, -120);
    let (_, l) = engine::parse_info("info depth 20 score mate 3 pv d3h7", Color::White).unwrap();
    assert_eq!(l.mate, Some(3));
}
#[test]
fn settings_roundtrip() {
    let s = Settings {
        seconds: 3.5,
        flipped: true,
        ..Default::default()
    };
    let bytes = serde_json::to_vec(&s).unwrap();
    let t: Settings = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(t.seconds, 3.5);
    assert!(t.flipped);
}
#[test]
fn recognition_native_themes() {
    for theme in 0..3 {
        for flipped in [false, true] {
            let p = Chess::default();
            let img = vision::board_image(&p, flipped, theme, 640);
            let r = vision::recognize(&img, [0, 0, 640, 640]).unwrap();
            let actual = game::parse_fen(&vision::cells_fen(&r.cells, false, "KQkq", "-")).unwrap();
            assert_eq!(
                actual.board(),
                p.board(),
                "theme {theme}, flipped {flipped}"
            );
            assert!(r.uncertain.is_empty());
        }
    }
}
#[test]
fn board_boundaries() {
    let img = vision::board_image(&Chess::default(), false, 0, 400);
    let mut screenshot = image::RgbImage::from_pixel(700, 600, image::Rgb([13, 19, 27]));
    image::imageops::replace(&mut screenshot, &img, 80, 90);
    assert_eq!(vision::detect_bounds(&screenshot), Some([80, 90, 400, 400]));
}
#[test]
fn bad_crop() {
    assert!(vision::recognize(&image::RgbImage::new(50, 50), [0, 0, 500, 500]).is_err());
}
#[test]
fn mate_one() {
    let p = game::parse_fen("7k/5Q2/6K1/8/8/8/8/8 w - - 0 1").unwrap();
    let a = engine::search(&p, &settings(), false, &AtomicU64::new(0), 0).unwrap();
    assert_eq!(a.lines[0].mate, Some(1));
    let mut next = p.clone();
    next.play_unchecked(game::uci_move(&p, &a.best).unwrap());
    assert!(next.is_checkmate());
}
#[test]
fn mate_three() {
    let p = game::parse_fen("8/1k6/8/2K5/8/3Q4/8/8 w - - 0 1").unwrap();
    let mut s = settings();
    s.seconds = 1.;
    s.multipv = 1;
    let a = engine::search(&p, &s, false, &AtomicU64::new(0), 0).unwrap();
    assert_eq!(a.lines[0].mate, Some(3));
}
#[test]
fn strength_presets_legal() {
    for level in 0..5 {
        let mut s = settings();
        s.level = level;
        let p = Chess::default();
        let a = engine::search(&p, &s, true, &AtomicU64::new(0), 0).unwrap();
        assert!(game::uci_move(&p, &a.best).is_ok());
    }
}
#[test]
fn failure_then_recovery() {
    let mut s = settings();
    s.engine_path = "missing-stockfish.exe".into();
    assert!(engine::search(&Chess::default(), &s, false, &AtomicU64::new(0), 0).is_err());
    s.engine_path.clear();
    assert!(engine::search(&Chess::default(), &s, false, &AtomicU64::new(0), 0).is_ok());
}
#[test]
fn cancelled_search() {
    assert!(
        engine::search(&Chess::default(), &settings(), false, &AtomicU64::new(2), 1)
            .unwrap_err()
            .contains("Cancelled")
    );
}
#[test]
fn terminal_search() {
    let p = game::parse_fen("7k/6Q1/6K1/8/8/8/8/8 b - - 0 1").unwrap();
    assert!(
        engine::search(&p, &settings(), false, &AtomicU64::new(0), 0)
            .unwrap()
            .best
            .is_empty()
    );
}
#[test]
fn mate_score_order() {
    let a = Line {
        mate: Some(1),
        ..Default::default()
    };
    let b = Line {
        mate: Some(3),
        ..Default::default()
    };
    assert!(a.numeric() > b.numeric());
}

#[test]
fn polyglot_book_move() {
    use shakmaty::{EnPassantMode, zobrist::Zobrist64};
    let pos = Chess::default();
    let key = pos.zobrist_hash::<Zobrist64>(EnPassantMode::PseudoLegal).0;
    let raw: u16 = (12 << 6) | 28; // e2-e4, Polyglot square encoding.
    let mut entry = key.to_be_bytes().to_vec();
    entry.extend(raw.to_be_bytes());
    entry.extend(10_u16.to_be_bytes());
    entry.extend(0_u32.to_be_bytes());
    let path = std::env::temp_dir().join(format!("chess-book-test-{}.bin", std::process::id()));
    std::fs::write(&path, entry).unwrap();
    assert_eq!(
        engine::book_move(&pos, path.to_str().unwrap()).unwrap(),
        Some("e2e4".into())
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn companion_tracks_legal_moves_and_rights() {
    use chess_assistant_pro::companion;
    let start = Game::default();
    let mut moved = start.clone();
    for u in ["e2e4", "c7c5"] {
        moved.push(game::uci_move(moved.pos(), u).unwrap()).unwrap();
    }
    let f = game::fen(moved.pos());
    let placement = f.split_whitespace().next().unwrap();
    let synced = companion::synchronize(&start, placement).unwrap();
    assert_eq!(game::fen(synced.pos()), game::fen(moved.pos()));
    assert_eq!(synced.moves.len(), 2);
    assert!(companion::synchronize(&start, "8/8/8/8/8/8/8/8").is_err());
    assert!(companion::is_chess_window(
        "Play Chess Online Against Computer - Chess.com - Google Chrome"
    ));
    assert!(companion::is_chess_window(
        "Play Chess Online - Chess.com - Google Chrome"
    ));
    assert!(!companion::is_chess_window("Chess Assistant Pro"));
}
#[test]
fn native_screen_templates_follow_moves() {
    for (theme, flipped) in (0..3).flat_map(|t| [(t, false), (t, true)]) {
        let start = Chess::default();
        let image = vision::board_image(&start, flipped, theme, 640);
        let model = vision::ScreenTemplates::learn_start(&image).unwrap();
        let mut next = start.clone();
        for u in ["e2e4", "e7e5", "g1f3"] {
            let m = game::uci_move(&next, u).unwrap();
            next.play_unchecked(m);
        }
        let changed = vision::board_image(&next, flipped, theme, 640);
        assert_eq!(
            model.recognize(&changed).unwrap(),
            game::fen(&next).split_whitespace().next().unwrap()
        );
        assert!(vision::ScreenTemplates::learn_start(&changed).is_err());
        let cached = serde_json::to_vec(&model).unwrap();
        let restored: vision::ScreenTemplates = serde_json::from_slice(&cached).unwrap();
        assert_eq!(
            restored.recognize(&changed).unwrap(),
            model.recognize(&changed).unwrap()
        );
    }
}
#[test]
fn native_screen_templates_reject_obscured_board() {
    let img = vision::board_image(&Chess::default(), false, 0, 640);
    let model = vision::ScreenTemplates::learn_start(&img).unwrap();
    let mut obscured = img.clone();
    for y in 0..200 {
        for x in 0..400 {
            obscured.put_pixel(x, y, image::Rgb([25, 30, 40]));
        }
    }
    assert!(model.recognize(&obscured).is_err());
    assert!(vision::ScreenTemplates::learn_start(&obscured).is_err());
}

#[test]
fn coach_arrow_and_instructions_handle_special_moves() {
    for (fen, uci, from, to, instruction) in [
        (
            "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1",
            "e1g1",
            Square::E1,
            Square::G1,
            "Castle kingside",
        ),
        (
            "r3k2r/8/8/8/8/8/8/R3K2R b KQkq - 0 1",
            "e8c8",
            Square::E8,
            Square::C8,
            "Castle queenside",
        ),
        (
            "7k/P7/8/8/8/8/8/7K w - - 0 1",
            "a7a8n",
            Square::A7,
            Square::A8,
            "knight for promotion",
        ),
    ] {
        let pos = game::parse_fen(fen).unwrap();
        let m = game::uci_move(&pos, uci).unwrap();
        assert_eq!(game::move_squares(m), Some((from, to)));
        assert!(game::move_instruction(&pos, m).contains(instruction));
    }
}

#[test]
fn overlay_coordinates_match_both_orientations() {
    use chess_assistant_pro::overlay::square_center;
    use eframe::egui::{Pos2, Rect, Vec2};
    let rect = Rect::from_min_size(Pos2::new(100., 200.), Vec2::splat(800.));
    assert_eq!(
        square_center(rect, Square::A8, false),
        Pos2::new(150., 250.)
    );
    assert_eq!(
        square_center(rect, Square::H1, false),
        Pos2::new(850., 950.)
    );
    assert_eq!(square_center(rect, Square::A8, true), Pos2::new(850., 950.));
    assert_eq!(square_center(rect, Square::H1, true), Pos2::new(150., 250.));
}

#[test]
fn browser_position_preserves_exact_game_state() {
    use chess_assistant_pro::browser::position_from_snapshot;
    let previous = Game::default();
    let mut next = previous.clone();
    for u in ["e2e4", "c7c5"] {
        next.push(game::uci_move(next.pos(), u).unwrap()).unwrap();
    }
    let fen = game::fen(next.pos());
    let placement = fen.split_whitespace().next().unwrap();
    assert_eq!(
        game::fen(
            position_from_snapshot(&previous, placement, Some(&fen))
                .unwrap()
                .pos()
        ),
        fen
    );
    assert_eq!(
        game::fen(
            position_from_snapshot(&previous, placement, None)
                .unwrap()
                .pos()
        ),
        fen
    );
    assert!(
        position_from_snapshot(&previous, placement, Some(&game::fen(previous.pos()))).is_err()
    );
    assert!(
        position_from_snapshot(
            &previous,
            "8/8/8/8/8/8/8/8",
            Some("8/8/8/8/8/8/8/8 w - - 0 1")
        )
        .is_err()
    );
}
