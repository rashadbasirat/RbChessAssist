# Test report

## Verified locally

The Windows x64 Rust release build was compiled with Rust 1.100.0 and the GNU Windows toolchain. Stockfish 19 is the engine used for integration tests.

`cargo test --release -- --test-threads=2` passes **35 integration tests**, with zero failures.

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

`node tests/browser.cjs` passes nine test groups covering DOM board decoding, duplicate-piece animation rejection, arrow coordinates, rotation, Start/Stop, stale-position clearing, heartbeat timeout, cleanup and unrelated-site rejection.

## Remaining verification and limits

- The authenticated localhost connection supports the automatic Chrome extension and optional bookmark. Both connected in the existing signed-in browser. Automated tests cover real Stockfish replies, pause/resume, early streaming results, extension scope, origin/token rules and authentic book passages. The integration server uses a separate test port so the installed extension cannot interfere with tests.
- Screenshot tests cover the bundled themes/piece set and synthetic screenshots exported by the app. Broad recognition of arbitrary third-party chess themes is unsupported.
- Syzygy path configuration is implemented, but no external Syzygy dataset was available for probing tests.
- Automated strength tests verify legal play and configuration. They do not calibrate the approximate Elo labels.
- Engine failure recovery was tested using an unavailable engine path. Exhaustive mid-search process-kill and operating-system shutdown scenarios are not covered.
- The tests do not establish that heuristic brilliant/great labels match any commercial service.
- Manual GUI coverage is limited to launch/render and engine analysis. The engine/rules tests cover underlying chess operations, but a complete manual interaction pass on every dialog remains outstanding.

GitHub Actions is configured to run the Rust and browser tests and build/package the MSVC Windows executable. Its result is independent of the local GNU build and should be checked on the Actions tab.

## Automatic connection verification, November 19, 2026

Installed the scoped RbChessAssist extension in the existing signed-in Chrome profile. Reloaded Chess.com, started the native app, and verified automatic connection without a bookmark click. The engine displayed Rfe1+ on the current bot position. Details displayed numbered turquoise, red and blue continuation arrows with matching explanations. Website Stop removed all arrows, and Start resumed the suggestion. A visual check found long arrows ended too far short of their destinations; endpoint geometry was corrected, the extension reloaded, and a second live check verified the turquoise a3-d6, red c8-b7 and blue f3-f4 arrows reached their target squares.

## November 20 stability fixes

Local training, imports and manual analysis now detach the website coach and discard queued website events, preventing the website from replacing a local game. Normal coaching publishes only completed search results. Automatic minimizing is disabled. A second board receives HTTP 409 without disposing the first connection. Accepted sockets explicitly use blocking handshakes, then bounded read timeouts. All 35 Rust tests passed, including the two-client regression and settled-result assertion. Manual local play with Chrome open retained 1. e4 e5 2. Nf3 Nc6 and its move history.

## November 20 review and strength fixes

All 38 Rust integration tests and nine browser test groups pass. Added regression coverage for full-FEN move-history retention through castling, incompatible-state rejection, PGN round trips, finished-game review completion, full-strength preset configuration and optional page PGN extraction. Removed the three-second website search cap. Review reuses adjacent position evaluations instead of analyzing each twice. Restored games and all Review entry points now use the captured history; a missing history opens PGN import. The maximum-strength preset is a resource configuration, not a measured Elo guarantee.

## Fast suggestions and scroll alignment

The live connection test verifies one legal preliminary hint followed by a final result, without intermediate arrow changes. Browser tests cover next-frame scroll/resize alignment, coalesced event handling, and listener cleanup. Full search time is preserved; the preliminary hint is labeled as refining. Ten browser groups and 38 Rust tests pass.
