#[derive(Default, Deserialize)]
struct Snapshot {
    #[serde(default)]
    deep: u64,
    #[serde(default)]
    paused: bool,
    error: Option<String>,
    placement: Option<String>,
    fen: Option<String>,
    pgn: Option<String>,
    #[serde(default)]
    flipped: bool,
}
pub fn position_from_snapshot(
    previous: &Game,
    placement: &str,
    fen: Option<&str>,
) -> Result<Game, String> {
    if let Some(fen) = fen {
        if fen.split_whitespace().next() != Some(placement) {
            return Err("Waiting for the board and game state to agree.".into());
        }
        let position = game::parse_fen(fen)?;
        if game::fen(previous.pos()) == game::fen(&position) {
            return Ok(previous.clone());
        }
        // Exact FEN is authoritative, but don't discard legal move history.
        if let Ok(next) = companion::synchronize(previous, placement) {
            if game::fen(next.pos()) == game::fen(&position) {
                return Ok(next);
            }
        }
        return Ok(Game::new(position));
    }
    companion::synchronize(previous,placement).map_err(|_|"The page did not expose the full game state. Start a fresh game, or load the current FEN in the app and reconnect.".into())
}
pub fn detailed_explanation(game: &Game, analysis: &Analysis) -> Option<String> {
    Hint::new(game, analysis).map(|hint| hint.details)
}
#[derive(Serialize)]
struct BoardArrow {
    uci: String,
    label: String,
    color: String,
    description: String,
}
#[derive(Serialize)]
struct Hint {
    placement: String,
    uci: String,
    san: String,
    evaluation: String,
    details: String,
    arrows: Vec<BoardArrow>,
    searching: bool,
    references: Vec<crate::library::Passage>,
}
impl Hint {
    fn new(game: &Game, analysis: &Analysis) -> Option<Self> {
        let line = analysis.lines.first()?;
        let uci = line.pv.first()?;
        let m = game::uci_move(game.pos(), uci).ok()?;
        let mut arrow_position = game.pos().clone();
        let mut arrows = Vec::new();
        use shakmaty::Position;
        for (i, u) in line.pv.iter().take(3).enumerate() {
            let Ok(mv) = game::uci_move(&arrow_position, u) else { break };
            let (color, role) = [("#17e7cf", "Turquoise: suggested move"),
                ("#ff647c", "Red: expected reply"), ("#65a7ff", "Blue: following move")][i];
            arrows.push(BoardArrow {
                uci: u.clone(), label: (i + 1).to_string(), color: color.into(),
                description: format!("{}. {} — {}. {} {}", i + 1, role,
                    game::san(&arrow_position, mv), game::move_instruction(&arrow_position, mv),
                    game::explanation(&arrow_position, mv)),
            });
            arrow_position.play_unchecked(mv);
        }
        let mut details = format!(
            "YOUR MOVE\n{}\n\nWHAT IT DOES\n{}\n\nEVALUATION\n{} (White's perspective: + favors White, − favors Black).",
            game::move_instruction(game.pos(), m),
            game::explanation(game.pos(), m),
            line.label()
        );
        details = format!("BOARD ARROWS\n{}\nRead the numbered arrows in order. Red and blue show a possible future continuation, not moves to play together.\n\n{}",
            arrows.iter().map(|a| a.description.as_str()).collect::<Vec<_>>().join("\n\n"), details);
        let mut next = game.pos().clone();
        next.play_unchecked(m);
        if game.pos().is_check() {
            details += "\n\nYou are in check. This legal move resolves the check.";
        }
        let material = game::material(&next, game.pos().turn());
        details += &format!(
            "\n\nMATERIAL AFTER THE MOVE\n{:+.2} pawn units for the moving side. Material alone does not determine the evaluation.",
            material as f32 / 100.0
        );
        if let Some((_, to)) = game::move_squares(m) {
            let attackers = next
                .board()
                .attacks_to(to, next.turn(), next.board().occupied());
            if !attackers.is_empty() {
                details += &format!(
                    "\n\nTACTICAL CHECK\nThe destination {to} is attacked geometrically by {}. This can be intentional; check the engine continuation before assuming the piece is lost.",
                    attackers
                        .into_iter()
                        .map(|sq| sq.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
        }
        if let Some(reply) = line.pv.get(1).and_then(|u| game::uci_move(&next, u).ok()) {
            details += &format!(
                "\n\nExpected reply: {}\n{}",
                game::san(&next, reply),
                game::move_instruction(&next, reply)
            );
        }
        details += &format!(
            "\n\nEngine line: {}\nDepth {} · {:.1}s",
            game::continuation(
                game.pos(),
                &line.pv.iter().take(10).cloned().collect::<Vec<_>>()
            ),
            line.depth,
            analysis.elapsed
        );
        if analysis.lines.len() > 1 {
            details += "\n\nALTERNATIVES (same search)";
            for alternative in analysis.lines.iter().skip(1) {
                details += &format!(
                    "\n{} · {}",
                    game::continuation(
                        game.pos(),
                        &alternative.pv.iter().take(6).cloned().collect::<Vec<_>>()
                    ),
                    alternative.label()
                );
            }
        }
        details += "\n\nThese lines are engine predictions, not guaranteed replies. Use Analyze deeper for a longer search.";
        let topic = if m.is_castle() {
            "castling development"
        } else if next.board().occupied().count() < 14 {
            "pawn ending king"
        } else if m.capture().is_some() {
            "exchange attack combination"
        } else if next.is_check() {
            "check attack king"
        } else {
            "development centre position"
        };
        Some(Self {
            placement: game::fen(game.pos()).split_whitespace().next()?.into(),
            uci: uci.clone(),
            san: game::san(game.pos(), m),
            evaluation: line.label(),
            details,
            arrows,
            searching: false,
            references: crate::library::search(topic, 2),
        })
    }
}
