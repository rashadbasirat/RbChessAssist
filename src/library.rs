use serde::Serialize;

pub struct Book {
    pub title: &'static str,
    pub author: &'static str,
    pub url: &'static str,
    pub text: &'static str,
}
pub static BOOKS: [Book; 6] = [
    Book {
        title: "Chess Fundamentals",
        author: "José Raúl Capablanca",
        url: "https://www.gutenberg.org/ebooks/33870",
        text: include_str!("../assets/books/33870.txt"),
    },
    Book {
        title: "Chess Strategy",
        author: "Edward Lasker",
        url: "https://www.gutenberg.org/ebooks/5614",
        text: include_str!("../assets/books/5614.txt"),
    },
    Book {
        title: "The Blue Book of Chess",
        author: "Howard Staunton and others",
        url: "https://www.gutenberg.org/ebooks/16377",
        text: include_str!("../assets/books/16377.txt"),
    },
    Book {
        title: "Chess and Checkers: The Way to Mastership",
        author: "Edward Lasker",
        url: "https://www.gutenberg.org/ebooks/4913",
        text: include_str!("../assets/books/4913.txt"),
    },
    Book {
        title: "Chess Generalship, Vol. I",
        author: "Franklin K. Young",
        url: "https://www.gutenberg.org/ebooks/55278",
        text: include_str!("../assets/books/55278.txt"),
    },
    Book {
        title: "Chess History and Reminiscences",
        author: "H. E. Bird",
        url: "https://www.gutenberg.org/ebooks/4902",
        text: include_str!("../assets/books/4902.txt"),
    },
];

#[derive(Clone, Serialize)]
pub struct Passage {
    pub title: &'static str,
    pub author: &'static str,
    pub url: &'static str,
    pub passage: String,
}

/// Search the original prose, not an invented quotation or an engine evaluation.
pub fn search(query: &str, limit: usize) -> Vec<Passage> {
    let words: Vec<_> = query
        .split_whitespace()
        .filter(|s| s.len() > 2)
        .map(str::to_lowercase)
        .collect();
    if words.is_empty() {
        return Vec::new();
    }
    let mut found = Vec::new();
    for book in &BOOKS {
        let normalized = book.text.replace('\r', "");
        let mut best = None;
        for paragraph in normalized.split("\n\n") {
            let text = paragraph.split_whitespace().collect::<Vec<_>>().join(" ");
            if text.len() < 160 || text.len() > 1800 {
                continue;
            }
            let lower = text.to_lowercase();
            if ["gutenberg", "copyright", "license", "donation", "trademark"]
                .iter()
                .any(|s| lower.contains(s))
            {
                continue;
            }
            let score: usize = words
                .iter()
                .map(|word| lower.matches(word).count().min(3))
                .sum();
            if score > 0 && best.as_ref().is_none_or(|(old, _)| score > *old) {
                best = Some((score, text));
            }
        }
        if let Some((score, passage)) = best {
            found.push((
                score,
                Passage {
                    title: book.title,
                    author: book.author,
                    url: book.url,
                    passage,
                },
            ));
        }
    }
    found.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
    found.into_iter().take(limit).map(|(_, p)| p).collect()
}
