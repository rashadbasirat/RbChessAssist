use chess_assistant_pro::{
    browser::{self, BrowserCoach, BrowserEvent},
    game::{self, Game},
    library,
    settings::Settings,
};
use std::{
    net::TcpStream,
    thread,
    time::{Duration, Instant},
};
use tungstenite::{Message, client::IntoClientRequest};

#[test]
fn origin_and_pairing_are_required() {
    let token = "test-token";
    let path = "/coach/test-token";
    let host = format!("127.0.0.1:{}", browser::PORT);
    assert!(browser::authorized_request(
        "https://www.chess.com",
        path,
        &host,
        token
    ));
    for origin in [
        "https://evil.example",
        "null",
        "",
        "https://www.chess.com.evil.example",
    ] {
        assert!(!browser::authorized_request(origin, path, &host, token));
    }
    assert!(!browser::authorized_request(
        "https://www.chess.com",
        "/coach/wrong",
        &host,
        token
    ));
    assert!(!browser::authorized_request(
        "https://www.chess.com",
        path,
        "evil.example",
        token
    ));
}
#[test]
fn automatic_extension_is_scoped_to_chess_and_keeps_pairing_local() {
    let folder = browser::prepare_extension().unwrap();
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(folder.join("manifest.json")).unwrap())
            .unwrap();
    assert_eq!(
        manifest["content_scripts"][0]["matches"],
        serde_json::json!(["https://www.chess.com/*", "https://chess.com/*"])
    );
    assert!(manifest.get("permissions").is_none());
    let code = std::fs::read_to_string(folder.join("coach.js")).unwrap();
    assert!(code.contains("const RB_AUTOMATIC=true"));
    assert!(code.contains(&browser::connection_token().unwrap()));
}

#[test]
fn books_are_complete_and_passages_are_real() {
    assert_eq!(library::BOOKS.len(), 6);
    assert!(
        library::BOOKS
            .iter()
            .all(|book| book.text.len() > 100_000 && book.text.contains("Gutenberg"))
    );
    let results = library::search("development centre", 2);
    assert_eq!(results.len(), 2);
    for result in results {
        let book = library::BOOKS
            .iter()
            .find(|book| book.title == result.title)
            .unwrap();
        assert!(
            book.text
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .contains(&result.passage)
        );
    }
    assert!(library::search("", 6).is_empty());
}

#[test]
fn local_bookmark_connection_analyzes_and_pauses() {
    let mut settings = Settings::default();
    settings.seconds = 0.2;
    settings.depth = 10;
    settings.threads = 1;
    let coach = BrowserCoach::start_on_port(Game::default(), settings, 38272);
    let token = browser::connection_token().unwrap();
    let address = format!("127.0.0.1:{}", browser::PORT);
    let deadline = Instant::now() + Duration::from_secs(8);
    let stream = loop {
        match TcpStream::connect("127.0.0.1:38272") {
            Ok(s) => break s,
            Err(_) if Instant::now() < deadline => thread::sleep(Duration::from_millis(50)),
            Err(e) => panic!("{e}"),
        }
    };
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut request = format!("ws://{address}/coach/{token}")
        .into_client_request()
        .unwrap();
    request
        .headers_mut()
        .insert("Origin", "https://www.chess.com".parse().unwrap());
    let (mut ws, _) = tungstenite::client(request, stream).unwrap();
    assert!(matches!(ws.read().unwrap(), Message::Text(_)));
    // A second tab must not dispose the active coach.
    let second_stream = TcpStream::connect("127.0.0.1:38272").unwrap();
    second_stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut second_request = format!("ws://{address}/coach/{token}")
        .into_client_request()
        .unwrap();
    second_request
        .headers_mut()
        .insert("Origin", "https://www.chess.com".parse().unwrap());
    match tungstenite::client(second_request, second_stream) {
        Err(tungstenite::HandshakeError::Failure(tungstenite::Error::Http(response))) => {
            assert_eq!(response.status(), 409)
        }
        other => panic!("Second board should receive a busy response: {other:?}"),
    }
    let fen = game::fen(Game::default().pos());
    let payload = serde_json::json!({"placement":fen.split_whitespace().next().unwrap(),"fen":fen,"flipped":false,"deep":0});
    for _ in 0..3 {
        ws.send(Message::Text(payload.to_string().into())).unwrap();
        thread::sleep(Duration::from_millis(100));
    }
    let mut verified = false;
    while Instant::now() < deadline {
        if let Message::Text(text) = ws.read().unwrap() {
            let value: serde_json::Value = serde_json::from_str(&text).unwrap();
            if value["type"] == "hint" && !value["value"].is_null() {
                let hint = &value["value"];
                assert_eq!(
                    hint["searching"], false,
                    "Only settled suggestions should reach the board"
                );
                assert!(
                    game::uci_move(Game::default().pos(), hint["uci"].as_str().unwrap()).is_ok()
                );
                assert!(hint["details"].as_str().unwrap().contains("EVALUATION"));
                assert!(hint["references"].as_array().unwrap().len() > 0);
                assert_eq!(hint["arrows"].as_array().unwrap().len(), 3);
                assert_eq!(hint["arrows"][1]["color"], "#ff647c");
                verified = true;
                break;
            }
        }
    }
    assert!(verified, "No engine hint reached the bookmark connection");
    ws.send(Message::Text("{\"paused\":true}".into())).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut paused = false;
    while Instant::now() < deadline {
        if let Ok(BrowserEvent::Paused) = coach.receiver.recv_timeout(Duration::from_millis(200)) {
            paused = true;
            break;
        }
    }
    assert!(paused);
    for _ in 0..3 {
        ws.send(Message::Text(payload.to_string().into())).unwrap();
        thread::sleep(Duration::from_millis(100));
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut resumed = false;
    while Instant::now() < deadline {
        if let Message::Text(text) = ws.read().unwrap() {
            let value: serde_json::Value = serde_json::from_str(&text).unwrap();
            if value["type"] == "hint" && !value["value"].is_null() {
                resumed = true;
                break;
            }
        }
    }
    assert!(resumed, "Start helping did not resume after Stop helping");
    drop(coach);
    let _ = ws.close(None);
}
