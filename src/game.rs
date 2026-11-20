use shakmaty::{
    CastlingMode, Chess, Color, EnPassantMode, Move, Position, Role, fen::Fen, san::SanPlus,
    uci::UciMove,
};

pub fn fen(pos: &Chess) -> String {
    Fen::from_position(pos, EnPassantMode::Legal).to_string()
}
pub fn parse_fen(text: &str) -> Result<Chess, String> {
    text.trim()
        .parse::<Fen>()
        .map_err(|e| e.to_string())?
        .into_position(CastlingMode::Standard)
        .map_err(|e| e.to_string())
}
pub fn uci_move(pos: &Chess, text: &str) -> Result<Move, String> {
    text.parse::<UciMove>()
        .map_err(|e| e.to_string())?
        .to_move(pos)
        .map_err(|e| e.to_string())
}
pub fn san(pos: &Chess, m: Move) -> String {
    SanPlus::from_move(pos.clone(), m).to_string()
}
pub fn continuation(pos: &Chess, moves: &[String]) -> String {
    let mut p = pos.clone();
    let mut result = Vec::new();
    for u in moves {
        if let Ok(m) = uci_move(&p, u) {
            result.push(san(&p, m));
            p.play_unchecked(m);
        } else {
            break;
        }
    }
    result.join("  ")
}
pub fn value(role: Role) -> i32 {
    match role {
        Role::Pawn => 100,
        Role::Knight => 320,
        Role::Bishop => 330,
        Role::Rook => 500,
        Role::Queen => 900,
        Role::King => 0,
    }
}
pub fn material(pos: &Chess, color: Color) -> i32 {
    pos.board()
        .iter()
        .map(|(_, p)| {
            if p.color == color {
                value(p.role)
            } else {
                -value(p.role)
            }
        })
        .sum()
}
pub fn explanation(pos: &Chess, m: Move) -> String {
    let mut next = pos.clone();
    next.play_unchecked(m);
    let mut reasons = Vec::new();
    if next.is_checkmate() {
        return "Delivers checkmate. The king has no legal escape.".into();
    }
    if m.is_castle() {
        reasons.push("Castles, moving the king and rook together.".to_string());
    }
    if let Some(role) = m.capture() {
        reasons.push(format!("Captures a {}.", role_name(role)));
    }
    if let Some(role) = m.promotion() {
        reasons.push(format!("Promotes the pawn to a {}.", role_name(role)));
    }
    if next.is_check() {
        reasons.push("Gives check, forcing an immediate response.".into());
    }
    let sq = m.to();
    if ["d4", "e4", "d5", "e5"].contains(&sq.to_string().as_str()) {
        reasons.push("Occupies a central square.".into());
    }
    if reasons.is_empty() {
        reasons.push("Stockfish prefers this move at the displayed search depth. Follow the continuation to inspect the resulting position.".into());
    }
    reasons.join(" ")
}
/// Actual board destinations, including standard castling's king destination.
pub fn move_squares(m: Move) -> Option<(shakmaty::Square, shakmaty::Square)> {
    let uci = m.to_uci(CastlingMode::Standard).to_string();
    if uci.len() < 4 {
        return None;
    }
    Some((uci[..2].parse().ok()?, uci[2..4].parse().ok()?))
}
pub fn move_instruction(pos: &Chess, m: Move) -> String {
    let Some((from, to)) = move_squares(m) else {
        return san(pos, m);
    };
    let piece = pos
        .board()
        .piece_at(from)
        .map(|p| role_name(p.role))
        .unwrap_or("piece");
    let mut text = format!("Move your {piece} from {from} to {to}.");
    if m.is_castle() {
        text += if to.file() == shakmaty::File::G {
            " Castle kingside; the rook moves to the f-file."
        } else {
            " Castle queenside; the rook moves to the d-file."
        };
    }
    if m.is_en_passant() {
        text += " This is an en passant capture.";
    }
    if let Some(role) = m.promotion() {
        text += &format!(" Choose a {} for promotion.", role_name(role));
    }
    text
}
pub fn role_name(r: Role) -> &'static str {
    match r {
        Role::Pawn => "pawn",
        Role::Knight => "knight",
        Role::Bishop => "bishop",
        Role::Rook => "rook",
        Role::Queen => "queen",
        Role::King => "king",
    }
}

#[derive(Clone)]
pub struct Game {
    pub positions: Vec<Chess>,
    pub moves: Vec<Move>,
    pub cursor: usize,
    pub result: Option<String>,
}
impl Default for Game {
    fn default() -> Self {
        Self::new(Chess::default())
    }
}
impl Game {
    pub fn new(p: Chess) -> Self {
        Self {
            positions: vec![p],
            moves: Vec::new(),
            cursor: 0,
            result: None,
        }
    }
    pub fn pos(&self) -> &Chess {
        &self.positions[self.cursor]
    }
    pub fn push(&mut self, m: Move) -> Result<(), String> {
        if !self.pos().is_legal(m) {
            return Err("Illegal move".into());
        }
        self.positions.truncate(self.cursor + 1);
        self.moves.truncate(self.cursor);
        let mut p = self.pos().clone();
        p.play_unchecked(m);
        self.moves.push(m);
        self.positions.push(p);
        self.cursor += 1;
        self.result = None;
        Ok(())
    }
    pub fn terminal(&self) -> Option<String> {
        if let Some(r) = &self.result {
            return Some(r.clone());
        }
        let p = self.pos();
        if p.is_checkmate() {
            return Some(format!(
                "Checkmate · {} wins",
                if p.turn() == Color::White {
                    "Black"
                } else {
                    "White"
                }
            ));
        }
        if p.is_stalemate() {
            return Some("Draw · stalemate".into());
        }
        if p.is_insufficient_material() {
            return Some("Draw · insufficient material".into());
        }
        if p.halfmoves() >= 150 {
            return Some("Draw · 75-move rule".into());
        }
        if self.repetitions() >= 5 {
            return Some("Draw · fivefold repetition".into());
        }
        None
    }
    pub fn repetitions(&self) -> usize {
        let key = fen(self.pos())
            .split_whitespace()
            .take(4)
            .collect::<Vec<_>>()
            .join(" ");
        self.positions[..=self.cursor]
            .iter()
            .filter(|p| {
                fen(p)
                    .split_whitespace()
                    .take(4)
                    .collect::<Vec<_>>()
                    .join(" ")
                    == key
            })
            .count()
    }
    pub fn can_claim_draw(&self) -> bool {
        self.pos().halfmoves() >= 100 || self.repetitions() >= 3
    }
    pub fn pgn(&self) -> String {
        let mut text = format!(
            "[Event \"Chess Assistant Pro\"]\n[Result \"*\"]\n[SetUp \"1\"]\n[FEN \"{}\"]\n\n",
            fen(&self.positions[0])
        );
        for (i, m) in self.moves.iter().enumerate() {
            let p = &self.positions[i];
            if p.turn() == Color::White {
                text += &format!("{}. ", p.fullmoves());
            } else if i == 0 {
                text += &format!("{}... ", p.fullmoves());
            }
            text += &format!("{} ", san(p, *m));
        }
        text + "*"
    }
}
pub fn parse_pgn(input: &str) -> Result<Game, String> {
    let mut start = Chess::default();
    let mut body = String::new();
    for line in input.lines() {
        let l = line.trim();
        if l.starts_with("[FEN ") {
            let f = l.split('"').nth(1).ok_or("Invalid FEN header")?;
            start = parse_fen(f)?;
        } else if !l.starts_with('[') {
            body.push_str(l);
            body.push('\n');
        }
    }
    let mut clean = String::new();
    let mut comment = false;
    let mut variation = 0;
    let mut semicolon = false;
    for c in body.chars() {
        if c == '\n' {
            semicolon = false;
        }
        if semicolon {
            continue;
        }
        if c == ';' && !comment {
            semicolon = true;
            continue;
        }
        if c == '{' {
            comment = true;
            continue;
        }
        if c == '}' {
            comment = false;
            continue;
        }
        if comment {
            continue;
        }
        if c == '(' {
            variation += 1;
            continue;
        }
        if c == ')' {
            variation -= 1;
            if variation < 0 {
                return Err("Unbalanced PGN variation".into());
            }
            continue;
        }
        if variation == 0 {
            clean.push(c);
        }
    }
    if comment || variation != 0 {
        return Err("Unclosed PGN comment or variation".into());
    }
    let mut game = Game::new(start);
    for raw in clean.split_whitespace() {
        let token = raw
            .rsplit('.')
            .next()
            .unwrap_or("")
            .trim_end_matches(['!', '?']);
        if ["1-0", "0-1", "1/2-1/2", "*"].contains(&token) {
            break;
        }
        if token.is_empty() || token.starts_with('$') {
            continue;
        }
        let s = token
            .parse::<SanPlus>()
            .map_err(|_| format!("Invalid PGN move: {token}"))?;
        let m = s
            .san
            .to_move(game.pos())
            .map_err(|_| format!("Illegal PGN move: {token}"))?;
        game.push(m)?;
    }
    if game.moves.is_empty() {
        return Err("No moves found in the first PGN game".into());
    }
    game.cursor = 0;
    Ok(game)
}
