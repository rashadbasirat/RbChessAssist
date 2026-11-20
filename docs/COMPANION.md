# Chess.com bookmark connection

RbChessAssist uses your existing signed-in Chrome tab. It draws arrows inside Chess.com without an extension, another browser profile, or board calibration.

## One-time setup

1. Extract the full Windows ZIP. Keep the `stockfish` folder beside **RbChessAssist.exe**.
2. Open the app and click **Open bookmark setup**. This starts the local service and opens instructions in an ordinary Chrome tab.
3. Show Chrome's bookmarks bar with **Ctrl + Shift + B**.
4. Drag **Connect RbChessAssist** from the setup page onto the bookmarks bar.
5. Return to your signed-in Chess.com game and click that bookmark.
6. If Chrome asks to connect to your computer or local network, allow the app's loopback connection yourself. No security settings need to be disabled.

## Each session

Open the app, click **Start helping**, then click **Connect RbChessAssist** on your Chess.com game. After an engine search the arrow appears on the board. The app minimizes after its first suggestion and continues running in the background. Restore it from the taskbar to use the local library or review tools.

On Chess.com, **Details** sits immediately below **Start/Stop helping**. It expands move instructions, tactical observations, material balance, evaluation, expected reply, continuation, alternatives and attributed book passages. **Analyze deeper** searches for up to 15 seconds. Ordinary searches use up to three seconds, eight threads and 512 MB of hash, capped by your saved settings.

You play the moves yourself. The coach updates when the position changes. Suggestions describe the side to move; positive evaluations favor White and negative favor Black. The expected reply is an engine prediction.

**Stop helping** pauses analysis and hides arrows. **Start helping** resumes without installing the bookmark again. Closing the app ends the connection. After refreshing Chess.com, click the bookmark again. Connecting a second game tab replaces the previous connection.

## If it does not connect

- Keep RbChessAssist running and click Start helping before the bookmark.
- Click the bookmark on Chess.com, not on the setup page or a search page.
- Respond to Chrome's local connection prompt. The app cannot grant that permission for you.
- Read the error in the app or the website panel.
- Check **Arrow on Chess.com** if the panel works but the arrow is hidden.
- If Chrome blocks the bookmark or connection through an organization policy, that browser policy must be resolved by its administrator; the app does not bypass it.
- Keep the `stockfish` folder beside the executable, or select its path in Settings.
- If the page's full FEN is unavailable, start a fresh game or load the exact current FEN in the app. The fallback tracks up to two legal moves between observations.

## Privacy and scope

The app binds only to `127.0.0.1:38271`. Connections require a 256-bit random local pairing token and a Chess.com Origin header. Messages are size-limited and only carry board snapshots or coaching controls. No passwords, cookies, browser profiles, arbitrary code, or files are requested. Keep your personal bookmark private. The token is stored beside the app's settings; it is not included in the repository or distributed package.

Windows 10/11 x64, desktop Chrome and an OpenGL-capable driver are required. The executable is unsigned. Site redesigns and browser policies may affect the connection. The offline studio and PGN review work independently.
