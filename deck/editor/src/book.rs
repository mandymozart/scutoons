//! The toon's address book on the editor (#477): `GET /api/addresses/<toon>` with the editor's token (the shows' one,
//! show/api.rs), kept in the deck's cache as the document itself (`<cache>/addresses/<toon>.json`, its `version` inside),
//! asked again with `If-None-Match` (304: the kept one stays), and read from the kept file when the API does not answer.
//! Glue only, on a thread so the window never waits: what the book holds and what a query finds is deck-space `rig/book.rs`.
use deck_space::rig::book::Book;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::Duration;

/// what a load gave: the book (fetched or kept), and why the API did not give a fresh one
pub struct Loaded {
    pub book: Option<Arc<Book>>,
    pub why: Option<String>,
}

/// The editor's book: none asked for, on its way, or there.
pub struct Loader {
    rx: Option<Receiver<Loaded>>,
    pub book: Option<Arc<Book>>,
    pub why: Option<String>,
}

impl Loader {
    /// `file`: a book from a file (a check run's, no API); else `toon`'s from `api`, on a thread
    pub fn new(api: &str, toon: Option<&str>, file: Option<&Path>, token: Option<String>, cache: &Path) -> Loader {
        let mut l = Loader { rx: None, book: None, why: None };
        if let Some(file) = file {
            let Loaded { book, why } = read(file);
            (l.book, l.why) = (book, why);
        } else if let Some(toon) = toon {
            let (tx, rx) = channel();
            let (url, kept) = (format!("{api}/addresses/{toon}"), kept(cache, toon));
            std::thread::spawn(move || {
                let _ = tx.send(load(&url, token.as_deref(), &kept));
            });
            l.rx = Some(rx);
        } else {
            l.why = Some("no address book – the editor reads one with --toon <toon> (or --book FILE)".into());
        }
        l
    }

    /// the thread's answer, once it is there
    pub fn poll(&mut self) {
        if let Some(Ok(Loaded { book, why })) = self.rx.as_ref().map(Receiver::try_recv) {
            if let Some(b) = &book {
                tracing::info!("address book {}: {} entries, version {}{}", b.toon(), b.entries().len(), b.version(), why.as_deref().map(|w| format!(" (kept: {w})")).unwrap_or_default());
            }
            (self.book, self.why, self.rx) = (book, why, None);
        }
    }
}

/// where a toon's book is kept
pub fn kept(cache: &Path, toon: &str) -> PathBuf {
    cache.join("addresses").join(format!("{toon}.json"))
}

/// a book from a file
pub fn read(file: &Path) -> Loaded {
    match std::fs::read_to_string(file).map_err(|e| e.to_string()).and_then(|t| Book::parse(&t)) {
        Ok(b) => Loaded { book: Some(Arc::new(b)), why: None },
        Err(e) => Loaded { book: None, why: Some(format!("{}: {e}", file.display())) },
    }
}

/// The API's book (kept for the next time), the kept one when it is current (304) or the API does not answer.
pub fn load(url: &str, token: Option<&str>, kept: &Path) -> Loaded {
    let held = read(kept).book;
    let mut req = ureq::AgentBuilder::new().timeout(Duration::from_secs(4)).build().get(url);
    if let Some(t) = token {
        req = req.set("Authorization", &format!("Bearer {t}"));
    }
    if let Some(b) = &held {
        req = req.set("If-None-Match", &format!("\"{}\"", b.version()));
    }
    let fresh = match req.call() {
        Ok(r) if r.status() == 304 => return Loaded { book: held, why: None },
        Ok(r) => r.into_string().map_err(|e| e.to_string()).and_then(|text| Book::parse(&text).map(|b| (b, text))),
        Err(ureq::Error::Status(code, _)) => Err(format!("HTTP {code}")),
        Err(ureq::Error::Transport(t)) => Err(format!("offline ({})", t.kind())),
    };
    match fresh {
        Ok((b, text)) => {
            if let Err(e) = kept.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(kept, text)) {
                tracing::warn!("address book not kept ({}): {e}", kept.display());
            }
            Loaded { book: Some(Arc::new(b)), why: None }
        }
        Err(why) => Loaded { why: Some(format!("{url}: {why}{}", if held.is_some() { " – the kept book" } else { "" })), book: held },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOOK: &str = r#"{"address_book":2,"toon":"catpope","version":"v7","source":"en","entries":[{"ref":"item:incense","address":"item:incense","kind":"item","name":"incense","labels":{"en":"Incense"},"origin":{"key":"item:incense","state":"published"}}]}"#;

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("deck-editor-book-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    /// no API (nothing listens on port 9): the kept book, and why it is not fresh
    #[test]
    fn offline_it_works_from_the_kept_file() {
        let cache = dir("kept");
        let file = kept(&cache, "catpope");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, BOOK).unwrap();
        let l = load("http://127.0.0.1:9/api/addresses/catpope", None, &file);
        assert_eq!(l.book.as_ref().map(|b| b.version()), Some("v7"));
        assert!(l.why.as_deref().is_some_and(|w| w.contains("the kept book")), "{:?}", l.why);
        let none = load("http://127.0.0.1:9/api/addresses/catpope", None, &kept(&cache, "nobody"));
        assert!(none.book.is_none() && none.why.is_some());
        let _ = std::fs::remove_dir_all(&cache);
    }

    #[test]
    fn a_book_from_a_file_or_why_not() {
        let d = dir("file");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("good.json"), BOOK).unwrap();
        std::fs::write(d.join("bad.json"), "{}").unwrap();
        let l = Loader::new("http://unused", None, Some(&d.join("good.json")), None, &d);
        assert_eq!(l.book.map(|b| b.entries().len()), Some(1));
        assert!(Loader::new("http://unused", None, Some(&d.join("bad.json")), None, &d).why.unwrap().contains("not an address book"));
        assert!(Loader::new("http://unused", None, None, None, &d).why.unwrap().contains("--toon"));
        let _ = std::fs::remove_dir_all(&d);
    }
}
