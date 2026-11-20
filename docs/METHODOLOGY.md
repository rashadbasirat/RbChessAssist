# Analysis methodology

## Engine scores

UCI scores arrive relative to the side to move and are converted to White's perspective. A positive number favors White. M3 indicates a forced mate in three moves reported by the engine at its current search depth. Mate distance is measured in moves, not half-moves. A search result is only accepted for the generation of the position that requested it.

## Move classification

The reviewer performs a full-strength MultiPV search before each move and a full-strength search after it. Centipawn loss is the nonnegative difference between the best score and the resulting position, expressed from the mover's perspective. Checkmate and terminal draws receive explicit scores. Forced mate values are mapped outside the ordinary centipawn scale, so losing a forced mate receives a large penalty.

| Class | Rule, evaluated in this order |
| --- | --- |
| Brilliant | Engine's best move, loss at most 20 cp, depth at least 16, material balance falls by at least 200 cp within its first six plies, resulting evaluation at least −50 cp for the mover, candidate gap at least 30 cp |
| Great | Best move, loss at most 20 cp, second candidate at least 150 cp worse |
| Best | Best move, loss at most 10 cp |
| Excellent | Loss at most 20 cp |
| Good | Loss at most 50 cp |
| Inaccuracy | Loss at most 100 cp |
| Mistake | Loss at most 200 cp |
| Blunder | Loss above 200 cp |

Brilliant is a conservative material-sacrifice heuristic supported by the searched continuation, not a proof of a sacrifice's ultimate soundness. Search instability and deeper defenses can change labels. The classification function is deterministic for fixed input scores. Timed multithreaded engine searches may vary between runs.

Per-move estimated accuracy is `100 × exp(−centipawn_loss / 150)`. Player accuracy is the arithmetic mean of that player's reviewed moves. This formula is intentionally simple and transparent. It is not an Elo estimate or a commercial site's accuracy score.

## Screenshot recognition

Board localization looks for contiguous areas matching the three supported square palettes. Each board is normalized to 512 × 512 pixels. Each cell is compared with thirteen references (empty and twelve pieces) rendered against the observed square background. Mean squared RGB error over the central 50 × 50 pixels chooses the nearest reference. High error or ambiguous matches mark the square for correction.

The initial orientation estimate compares White and Black piece rank distributions. Endgames can defeat this heuristic, so the editor offers rotation. The user confirms turn, castling rights, and en passant. Shakmaty validates the final FEN before analysis.


## Website coach and book passages

The default coach is a user-installed bookmark communicating with the Rust app over an authenticated loopback WebSocket. It reads Chess.com's piece classes and available full FEN. Piece placement must agree with that FEN, and Shakmaty validates the position. Two successive placements must agree before a search starts. Fallback synchronization infers up to two legal moves from a known position. Changed or invalid positions clear old arrows. The SVG arrow follows the page board's bounding rectangle and orientation.

The site sends only board snapshots and limited coach controls. Host, Origin and pairing token are checked at the handshake; messages are size-limited. No commands, file access or arbitrary script execution are exposed by the local server. Chrome's local-connection permission remains under user control.

Details combine verifiable move properties with the engine's candidate lines. Geometric attack warnings explicitly do not establish a forced capture. The local library selects up to two original paragraphs by keyword overlap with general concepts such as development, exchange or endings. These are attributed historical passages, not position-specific analysis or a trained language model. Deeper analysis uses a separate 15-second search request.

## Legacy native capture utilities

The following implementation remains available for development fixtures and screenshot experiments; it is not the default website connection.

A background Rust worker enumerates non-minimized windows and selects a window whose title identifies chess, preferring a focused window with a detectable board. XCap captures that window. Pixel-palette localization finds a supported board, which is normalized into 64 cells. For a fresh starting position, the learner checks occupied ranks, repeated pawn shapes, mirrored back-rank pairs, and white/black contrast. For neutral black-and-white piece themes, channel differences isolate the colored square background, including antialiased edges. It stores foreground opacity and premultiplied brightness features for each piece. A complete re-recognition of the initial board must succeed before accepting a new model.

Subsequent frames are compared against these templates. Ambiguous matches clear the recommendation. Two successive identical recognized boards are required before updating the game. The chess rules engine matches up to two legal moves to preserve turn, castling rights, and en passant. The last tracked game and learned templates are saved locally. No screenshot or game data is uploaded.

The native overlay uses a separate transparent egui viewport with mouse passthrough. Its position follows the captured board bounds and monitor scale. Standard UCI destinations handle castling and promotion arrows. It is hidden on capture failure, position changes, stale observations, or loss of game focus. Explanations describe concrete move properties and the engine principal variation; they do not claim a guaranteed opponent response.
