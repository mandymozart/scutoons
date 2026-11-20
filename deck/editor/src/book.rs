//! The toon's address book on the editor (#477): `GET /api/addresses/<toon>` with the editor's token (sign_in.rs) – for a
//! show of several toons their books in one request, `GET /api/addresses?toons=a,b` (#708) –, kept in the deck's cache as
//! the document itself (`<cache>/addresses/<toon>.json`, `<a>+<b>.json`, its `version` inside), asked again with
//! `If-None-Match` (304: the kept one stays), and read from the kept file when the API does not answer. The toons are the
//! ones of what is open (toon.rs); main.rs makes a new loader when they or the sign-in change. Glue only, on a thread so
//! the window never waits: what the book holds and what a query finds is deck-space `rig/book.rs`. When there is no fresh
//! book, `Why` says why – inside the address typeahead.
use deck_space::rig::book::Book;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// why the typeahead has no fresh book
#[derive(Debug, Clone, PartialEq)]
pub enum Why {
    /// no token: the editor waits for its approval on the account page
    SignedOut,
    /// nothing open names a toon
    NoToon,
    /// the API did not answer; when the kept copy was written (None: there is none)
    Offline { kept: Option<SystemTime> },
    /// asked, no answer yet
    Loading,
    /// the API refused, a file did not read
    Failed(String),
}

impl Why {
    /// in words, inside the typeahead: `code` = the sign-in code that waits, `now` = the clock the kept copy's age is read on
    pub fn text(&self, code: Option<&str>, now: SystemTime) -> String {
        match self {
            Why::SignedOut => match code {
                Some(c) => format!("signed out – approve code {} on the account page (Decks)", deck_link::format_code(c)),
                None => "signed out – waiting for a sign-in code".into(),
            },
            Why::NoToon => "no toon – pick the show's toon (or open a character of a toon)".into(),
            Why::Offline { kept: Some(at) } => format!("offline – the kept copy from {} ago", age(now.duration_since(*at).unwrap_or_default())),
            Why::Offline { kept: None } => "offline – no kept copy of this toon's book yet".into(),
            Why::Loading => "the address book is on its way".into(),
            Why::Failed(e) => e.clone(),
        }
    }
}

fn age(d: Duration) -> String {
    match d.as_secs() {
        s if s < 120 => format!("{s} s"),
        s if s < 2 * 3600 => format!("{} min", s / 60),
        s if s < 2 * 86400 => format!("{} h", s / 3600),
        s => format!("{} days", s / 86400),
    }
}

/// what a load gave: the book (fetched or kept), and why the API did not give a fresh one
pub struct Loaded {
    pub book: Option<Arc<Book>>,
    pub why: Option<Why>,
}

/// The editor's book: none asked for, on its way, or there.
pub struct Loader {
    rx: Option<Receiver<Loaded>>,
    pub book: Option<Arc<Book>>,
    pub why: Option<Why>,
}

impl Loader {
    /// `file`: a book from a file (a check run's, no API); else the book of `toons` from `api` with `token`, on a thread;
    /// signed out, only the kept copy
    pub fn new(api: &str, toons: &[String], file: Option<&Path>, token: Option<String>, cache: &Path) -> Loader {
        let mut l = Loader { rx: None, book: None, why: None };
        if let Some(file) = file {
            let Loaded { book, why } = read(file);
            (l.book, l.why) = (book, why);
        } else if !toons.is_empty() {
            let (url, kept) = (url(api, toons), kept(cache, toons));
            if token.is_none() {
                (l.book, l.why) = (read(&kept).book, Some(Why::SignedOut));
                return l;
            }
            let (tx, rx) = channel();
            std::thread::spawn(move || {
                let _ = tx.send(load(&url, token.as_deref(), &kept));
            });
            (l.rx, l.why) = (Some(rx), Some(Why::Loading));
        } else {
            l.why = Some(Why::NoToon);
        }
        l
    }

    /// the thread's answer, once it is there
    pub fn poll(&mut self) {
        if let Some(Ok(Loaded { book, why })) = self.rx.as_ref().map(Receiver::try_recv) {
            if let Some(b) = &book {
                tracing::info!("address book {}: {} entries, version {}{}", b.toon(), b.entries().len(), b.version(), if why.is_some() { " (kept)" } else { "" });
            }
            (self.book, self.why, self.rx) = (book, why, None);
        }
    }
}

/// where the book of `toons` is asked: one toon's route, several toons' in one request
pub fn url(api: &str, toons: &[String]) -> String {
    match toons {
        [one] => format!("{api}/addresses/{one}"),
        more => format!("{api}/addresses?toons={}", more.join(",")),
    }
}

/// where the book of `toons` is kept
pub fn kept(cache: &Path, toons: &[String]) -> PathBuf {
    cache.join("addresses").join(format!("{}.json", toons.join("+")))
}

/// a book from a file
pub fn read(file: &Path) -> Loaded {
    match std::fs::read_to_string(file).map_err(|e| e.to_string()).and_then(|t| Book::parse(&t)) {
        Ok(b) => Loaded { book: Some(Arc::new(b)), why: None },
        Err(e) => Loaded { book: None, why: Some(Why::Failed(format!("{}: {e}", file.display()))) },
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
        Ok(r) => r.into_string().map_err(|e| Why::Failed(format!("{url}: {e}"))).and_then(|text| Book::parse(&text).map(|b| (b, text)).map_err(|e| Why::Failed(format!("{url}: {e}")))),
        Err(ureq::Error::Status(401, _)) => Err(Why::SignedOut),
        Err(ureq::Error::Status(code, _)) => Err(Why::Failed(format!("{url}: HTTP {code}"))),
        Err(ureq::Error::Transport(t)) => {
            tracing::info!("address book: {url} offline ({})", t.kind());
            Err(Why::Offline { kept: held.as_ref().and_then(|_| std::fs::metadata(kept).and_then(|m| m.modified()).ok()) })
        }
    };
    match fresh {
        Ok((b, text)) => {
            if let Err(e) = kept.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(kept, text)) {
                tracing::warn!("address book not kept ({}): {e}", kept.display());
            }
            Loaded { book: Some(Arc::new(b)), why: None }
        }
        Err(why) => Loaded { why: Some(why), book: held },
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

    /// no API (nothing listens on port 9): the kept book, and how old it is
    #[test]
    fn offline_it_works_from_the_kept_file() {
        let cache = dir("kept");
        let file = kept(&cache, &["catpope".into()]);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, BOOK).unwrap();
        let l = load("http://127.0.0.1:9/api/addresses/catpope", Some("sce_t"), &file);
        assert_eq!(l.book.as_ref().map(|b| b.version()), Some("v7"));
        assert!(matches!(l.why, Some(Why::Offline { kept: Some(_) })), "{:?}", l.why);
        let none = load("http://127.0.0.1:9/api/addresses/catpope", Some("sce_t"), &kept(&cache, &["nobody".into()]));
        assert!(none.book.is_none() && none.why == Some(Why::Offline { kept: None }));
        let _ = std::fs::remove_dir_all(&cache);
    }

    /// the three reasons the typeahead gives inside itself: signed out (with the code), no toon, offline (the kept copy's age)
    #[test]
    fn the_typeahead_says_why_it_has_no_fresh_book() {
        let d = dir("why");
        let signed_out = Loader::new("http://unused", &["catpope".into()], None, None, &d);
        assert_eq!((signed_out.why.as_ref(), signed_out.rx.is_none()), (Some(&Why::SignedOut), true), "signed out: the API is not asked");
        let now = SystemTime::now();
        assert_eq!(Why::SignedOut.text(Some("K7Q42M"), now), "signed out – approve code K7Q-42M on the account page (Decks)");
        assert_eq!(Why::SignedOut.text(None, now), "signed out – waiting for a sign-in code");
        assert_eq!(Loader::new("http://unused", &[], None, Some("sce_t".into()), &d).why, Some(Why::NoToon));
        assert!(Why::NoToon.text(None, now).starts_with("no toon – "));
        let kept_at = now - Duration::from_secs(3 * 3600 + 5);
        assert_eq!(Why::Offline { kept: Some(kept_at) }.text(None, now), "offline – the kept copy from 3 h ago");
        assert_eq!(Why::Offline { kept: Some(now - Duration::from_secs(600)) }.text(None, now), "offline – the kept copy from 10 min ago");
        assert_eq!(Why::Offline { kept: None }.text(None, now), "offline – no kept copy of this toon's book yet");
    }

    /// #708: a show of several toons asks their books in one request and keeps them under all their names
    #[test]
    fn several_toons_one_request_one_kept_file() {
        let two = ["catpope".to_string(), "yellowmouse".to_string()];
        assert_eq!(url("https://api.test/api", &two), "https://api.test/api/addresses?toons=catpope,yellowmouse");
        assert_eq!(url("https://api.test/api", &two[..1]), "https://api.test/api/addresses/catpope", "one toon: its own route");
        assert_eq!(kept(Path::new("c"), &two), Path::new("c").join("addresses").join("catpope+yellowmouse.json"));
    }

    #[test]
    fn a_book_from_a_file_or_why_not() {
        let d = dir("file");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("good.json"), BOOK).unwrap();
        std::fs::write(d.join("bad.json"), "{}").unwrap();
        let l = Loader::new("http://unused", &[], Some(&d.join("good.json")), None, &d);
        assert_eq!(l.book.map(|b| b.entries().len()), Some(1));
        let bad = Loader::new("http://unused", &[], Some(&d.join("bad.json")), None, &d).why.unwrap();
        assert!(bad.text(None, SystemTime::now()).contains("not an address book"), "{bad:?}");
        let _ = std::fs::remove_dir_all(&d);
    }
}
