# Test report

## Verified locally

The Windows x64 Rust release build was compiled with Rust 1.100.0 and the GNU Windows toolchain. Stockfish 19 is the engine used for integration tests.

`cargo test --release -- --test-threads=2` passes **33 integration tests**, with zero failures.

| Area | Checks performed | Result |
| --- | --- | --- |
| Legal chess | Starting move count, legal and illegal moves, both castling sides, en passant, all four promotions | Passed |
| Position/game imports | Valid and invalid FEN, corrupt PGN, comments/variations, PGN round trip | Passed |
| Endings | Stalemate, insufficient material, threefold repetition, terminal engine search | Passed |
| Stockfish | Legal output at every strength preset, known mate in one, known mate in three, missing-engine failure followed by recovery, cancellation | Passed |
| Analysis | White-relative score conversion, mate ordering, classification thresholds, accuracy calculation | Passed |
| Settings/book | JSON settings round trip, Polyglot position key and legal book move | Passed |
| Vision | Six native theme/orientation combinations, board location in a larger image, invalid crop | Passed |
| Companion tracking | Two legal moves inferred from placement, castling/turn/EP preservation, invalid board rejection, chess-window identification | Passed |
| Screen templates | Learned pieces follow moves across three palettes and both orientations; serialized cache round trip; obscured board rejected | Passed |
| Coach guidance | Castling and promotion destinations/instructions; overlay square centers in both orientations | Passed |
| Native UI | Release executable launched, home screen rendered, studio rendered after real Stockfish analysis | Passed |

`docs/home.png` and `docs/studio.png` were captured from the running native app. The studio image shows an actual engine evaluation and continuation. The portable release includes the engine executable, upstream engine source, and licensing information.

`node tests/browser.cjs` passes seven test groups covering DOM board decoding, duplicate-piece animation rejection, arrow coordinates, rotation, Start/Stop, stale-position clearing, heartbeat timeout, cleanup and unrelated-site rejection.

## Remaining verification and limits

- The default connection is now an authenticated localhost WebSocket and user-installed bookmark in existing Chrome. Automated integration tests verify a real connection, legal Stockfish reply, pause, origin/token rejection rules, and authentic book passage retrieval. Live installation and permission handling in the signed-in browser still require verification. The earlier separate-profile prototype displayed live updated suggestions, but that does not establish bookmark-mode compatibility.
- Screenshot tests cover the bundled themes/piece set and synthetic screenshots exported by the app. Broad recognition of arbitrary third-party chess themes is unsupported.
- Syzygy path configuration is implemented, but no external Syzygy dataset was available for probing tests.
- Automated strength tests verify legal play and configuration. They do not calibrate the approximate Elo labels.
- Engine failure recovery was tested using an unavailable engine path. Exhaustive mid-search process-kill and operating-system shutdown scenarios are not covered.
- The tests do not establish that heuristic brilliant/great labels match any commercial service.
- Manual GUI coverage is limited to launch/render and engine analysis. The engine/rules tests cover underlying chess operations, but a complete manual interaction pass on every dialog remains outstanding.

GitHub Actions is configured to run the Rust and browser tests and build/package the MSVC Windows executable. Its result is independent of the local GNU build and should be checked on the Actions tab.
