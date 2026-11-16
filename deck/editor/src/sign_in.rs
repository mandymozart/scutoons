//! The editor's sign-in (research/NATIVE-EDITOR-GUI.md §5 "Sign-in"): the decks' handshake as the kind `editor` – a key
//! pair in the deck cache, a short code the member approves on the account page beside their decks, then a short token
//! renewed by itself (deck-link's `Link`, on a thread). Its token and state line are kept in `<cache>/editor-link`
//! (deck-link pairing's files), so a restart within the hour has its token at once. `SCUTOONS_TOKEN` overrides it all.
//! The token only reads and saves drafts; publishing stays in the browser (decisions/deck.md 2026-11-09).
use deck_link::link::{DeckInfo, Link, State};
use deck_link::pairing::{self, Shown};
use deck_link::{HttpTransport, Identity};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};

/// while a code waits it is asked again after this; trouble waits longer; a held token is looked at now and then
const POLL: u64 = 5;
const RETRY: u64 = 15;
const HELD: u64 = 30;

#[derive(Default)]
struct Inner {
    fixed: Option<String>,
    token: Option<(String, Instant)>,
    shown: Option<Shown>,
    refused: bool,
}

/// the toolbar's account icon: animated, ink, red
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signed {
    Signing,
    In,
    Out,
}

/// The editor's sign-in, shared by everything that asks the API.
#[derive(Clone, Default)]
pub struct Auth(Arc<Mutex<Inner>>);

static CURRENT: OnceLock<Auth> = OnceLock::new();

/// the sign-in `start` began, else `SCUTOONS_TOKEN` alone (tests, a check run)
pub fn current() -> Auth {
    CURRENT.get().cloned().unwrap_or_else(Auth::from_env)
}

/// where the key, the token and the state line are kept
pub fn dir(cache: &Path) -> PathBuf {
    cache.join("editor-link")
}

impl Auth {
    /// a fixed token, or none: no handshake
    pub fn fixed(token: Option<String>) -> Auth {
        let auth = Auth::default();
        auth.0.lock().unwrap().fixed = token;
        auth
    }

    fn from_env() -> Auth {
        Auth::fixed(std::env::var("SCUTOONS_TOKEN").ok().filter(|t| !t.trim().is_empty()).map(|t| t.trim().to_string()))
    }

    /// `SCUTOONS_TOKEN` when set; else the kept token while it is fresh, and – `handshake` – the handshake against `api`
    /// on a thread (a check run only uses what is kept)
    pub fn start(api: &str, cache: &Path, handshake: bool) -> Auth {
        let env = Auth::from_env();
        if env.0.lock().unwrap().fixed.is_some() {
            return CURRENT.get_or_init(|| env).clone();
        }
        let auth = Auth::default();
        let dir = dir(cache);
        if let Some(token) = pairing::read_token(&dir, SystemTime::now()) {
            let age = std::fs::metadata(dir.join("token")).and_then(|m| m.modified()).ok().and_then(|m| m.elapsed().ok()).unwrap_or_default();
            let mut i = auth.0.lock().unwrap();
            i.token = Some((token, Instant::now().checked_sub(age).unwrap_or_else(Instant::now)));
            i.shown = pairing::read_state(&dir);
        }
        if handshake {
            let (thread, api) = (auth.clone(), api.to_string());
            std::thread::spawn(move || thread.run(&api, &dir));
        }
        CURRENT.get_or_init(|| auth).clone()
    }

    /// the token for `Authorization: Bearer` now
    pub fn token(&self) -> Option<String> {
        let i = self.0.lock().unwrap();
        i.fixed.clone().or_else(|| i.token.as_ref().filter(|(_, at)| at.elapsed() < pairing::TOKEN_FRESH).map(|(t, _)| t.clone()))
    }

    /// the code to approve on the account page while one waits
    pub fn code(&self) -> Option<String> {
        if self.token().is_some() {
            return None;
        }
        match &self.0.lock().unwrap().shown {
            Some(Shown::Pending { code }) => Some(code.clone()),
            _ => None,
        }
    }

    /// The API refused the token (401): it goes at once and the thread asks again
    pub fn refused(&self) {
        let mut i = self.0.lock().unwrap();
        if i.fixed.is_none() {
            (i.token, i.refused) = (None, true);
        }
    }

    /// where the sign-in stands: a code waiting for approval counts as signed out, the member has to act
    pub fn state(&self) -> Signed {
        if self.token().is_some() {
            return Signed::In;
        }
        match &self.0.lock().unwrap().shown {
            None => Signed::Signing,
            Some(_) => Signed::Out,
        }
    }

    /// the name the account page gave this editor, once signed in by the handshake
    pub fn name(&self) -> Option<String> {
        self.token()?;
        match &self.0.lock().unwrap().shown {
            Some(Shown::Ready { name }) => Some(name.clone()),
            _ => None,
        }
    }

    /// what the top bar says while the editor is not signed in ("" when it is)
    pub fn line(&self, app_url: &str) -> String {
        if self.token().is_some() {
            return String::new();
        }
        match &self.0.lock().unwrap().shown {
            Some(Shown::Pending { code }) => format!("sign in: approve code {} at {}", pairing::format_code(code), pairing_page(app_url)),
            Some(Shown::Offline { reason }) => format!("signed out – {reason}"),
            _ => "signing in …".into(),
        }
    }

    fn set(&self, shown: Shown, token: Option<String>) {
        let mut i = self.0.lock().unwrap();
        match (&shown, token) {
            (Shown::Ready { .. }, Some(t)) => {
                if i.token.as_ref().map(|(held, _)| held != &t).unwrap_or(true) {
                    i.token = Some((t, Instant::now()));
                }
            }
            (Shown::Pending { .. }, _) => i.token = None,
            _ => {}
        }
        if i.shown.as_ref() != Some(&shown) {
            match &shown {
                Shown::Pending { code } => tracing::info!("sign in: approve the editor's code {} at {}", pairing::format_code(code), pairing_page(&pairing::app_url())),
                Shown::Ready { name } => tracing::info!("signed in as the editor {name:?}"),
                Shown::Offline { reason } => tracing::warn!("sign-in: {reason}"),
            }
        }
        i.shown = Some(shown);
    }

    fn run(&self, api: &str, dir: &Path) {
        let identity = match Identity::load_or_create(&dir.join("identity")) {
            Ok(i) => i,
            Err(e) => return self.set(Shown::Offline { reason: format!("no editor key: {e:#}") }, None),
        };
        let info = DeckInfo { kind: "editor".into(), build: deck_native::BUILD.chars().take(64).collect(), hostname: hostname(), class: String::new() };
        let mut link = Link::new(HttpTransport::new(api), identity, info);
        loop {
            if std::mem::take(&mut self.0.lock().unwrap().refused) {
                link.forget_token();
            }
            let (shown, token, wait) = match link.step(Instant::now()) {
                Ok(State::Ready { token, name }) => (Shown::Ready { name }, Some(token), HELD),
                Ok(State::Pending { code }) => (Shown::Pending { code }, None, POLL),
                Err(e) => (Shown::Offline { reason: format!("{e:#}") }, None, RETRY),
            };
            if let Err(e) = pairing::write(dir, &shown, token.as_deref()) {
                tracing::warn!("the editor's token not kept ({}): {e}", dir.display());
            }
            self.set(shown, token);
            for _ in 0..wait {
                std::thread::sleep(Duration::from_secs(1));
                if self.0.lock().unwrap().refused {
                    break;
                }
            }
        }
    }
}

/// the account page where the member approves the editor
pub fn pairing_page(app_url: &str) -> String {
    format!("{}/account/decks", app_url.trim_end_matches('/'))
}

fn hostname() -> String {
    let name = std::env::var("COMPUTERNAME").or_else(|_| std::env::var("HOSTNAME")).unwrap_or_default();
    name.trim().chars().take(64).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fixed_token_never_goes_and_a_refused_one_does() {
        let fixed = Auth::fixed(Some("sct_x".into()));
        fixed.refused();
        assert_eq!(fixed.token().as_deref(), Some("sct_x"));
        assert_eq!(fixed.line("https://scutoons.com"), "");

        let auth = Auth::default();
        assert_eq!(auth.line("https://scutoons.com"), "signing in …");
        auth.set(Shown::Pending { code: "K7Q42M".into() }, None);
        assert_eq!((auth.token(), auth.code().as_deref()), (None, Some("K7Q42M")));
        assert_eq!(auth.line("https://scutoons.com/"), "sign in: approve code K7Q-42M at https://scutoons.com/account/decks");
        auth.set(Shown::Ready { name: "studio".into() }, Some("sce_a".into()));
        assert_eq!((auth.token().as_deref(), auth.code(), auth.line("x").as_str()), (Some("sce_a"), None, ""));
        auth.set(Shown::Offline { reason: "timed out".into() }, None);
        assert_eq!(auth.token().as_deref(), Some("sce_a"), "offline: the fresh token stays");
        auth.refused();
        assert_eq!((auth.token(), auth.line("x").as_str()), (None, "signed out – timed out"));
    }

    #[test]
    fn the_account_icon_signs_in_goes_red_and_is_ink_once_in() {
        let auth = Auth::default();
        assert_eq!(auth.state(), Signed::Signing, "no answer yet: animated");
        auth.set(Shown::Pending { code: "K7Q42M".into() }, None);
        assert_eq!(auth.state(), Signed::Out, "a code to approve: red");
        auth.set(Shown::Ready { name: "studio".into() }, Some("sce_a".into()));
        assert_eq!((auth.state(), auth.name().as_deref()), (Signed::In, Some("studio")));
        auth.refused();
        assert_eq!((auth.state(), auth.name()), (Signed::Out, None), "refused: red");
        assert_eq!(Auth::fixed(Some("sct_x".into())).state(), Signed::In);
    }
}
