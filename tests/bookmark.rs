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
    let coach = BrowserCoach::start(Game::default(), settings);
    let token = browser::connection_token().unwrap();
    let address = format!("127.0.0.1:{}", browser::PORT);
    let deadline = Instant::now() + Duration::from_secs(8);
    let stream = loop {
        match TcpStream::connect(&address) {
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
                assert!(
                    game::uci_move(Game::default().pos(), hint["uci"].as_str().unwrap()).is_ok()
                );
                assert!(hint["details"].as_str().unwrap().contains("EVALUATION"));
                assert!(hint["references"].as_array().unwrap().len() > 0);
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
    drop(coach);
    let _ = ws.close(None);
}
