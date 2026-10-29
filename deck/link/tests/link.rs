//! The handshake against a fake server that follows server/docs/API.md "Decks" (the PHP side has its own PHPUnit:
//! server/tests/Unit/Http/DecksApiTest.php): first boot → a code, approval → a token from a signed challenge, the
//! token reused until it is due, revoke / expiry → a new code, a forged signature refused, the identity kept on disk.

use deck_link::identity::{hex, signed_message};
use deck_link::{format_code, pairing_url, DeckInfo, Identity, Link, Reply, State, Transport};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

#[derive(Default)]
struct Server {
    /// public key → (status, code)
    decks: Vec<(String, String, String)>,
    challenge: Option<String>,
    issued: u32,
    calls: Vec<String>,
    next_code: u32,
}

impl Server {
    fn find(&mut self, key: &str) -> Option<&mut (String, String, String)> {
        self.decks.iter_mut().find(|d| d.0 == key)
    }
    fn set(&mut self, key: &str, status: &str) {
        self.find(key).expect("deck").1 = status.to_string();
    }
}

#[derive(Clone, Default)]
struct Fake(Rc<RefCell<Server>>);

fn reply(status: u16, body: Value) -> anyhow::Result<Reply> {
    Ok(Reply { status, body })
}

impl Transport for Fake {
    fn post(&self, path: &str, body: &Value) -> anyhow::Result<Reply> {
        let mut s = self.0.borrow_mut();
        s.calls.push(path.to_string());
        let key = body["public_key"].as_str().unwrap_or("").to_string();
        match path {
            "/decks/request" => {
                if let Some(d) = s.find(&key) {
                    if d.1 == "approved" {
                        return reply(200, json!({ "deck": { "status": "approved" } }));
                    }
                    if d.1 == "pending" {
                        let code = d.2.clone();
                        return reply(
                            201,
                            json!({ "deck": { "status": "pending", "code": code } }),
                        );
                    }
                }
                s.next_code += 1;
                let code = format!("K7Q42{}", ["M", "N", "P", "R"][s.next_code as usize % 4]);
                s.decks.retain(|d| d.0 != key);
                s.decks.push((key, "pending".into(), code.clone()));
                reply(
                    201,
                    json!({ "deck": { "status": "pending", "code": code } }),
                )
            }
            "/decks/challenge" => {
                let Some(d) = s.find(&key).cloned() else {
                    return reply(401, json!({ "error": "unknown_deck" }));
                };
                match d.1.as_str() {
                    "pending" => {
                        reply(202, json!({ "deck": { "status": "pending", "code": d.2 } }))
                    }
                    "approved" => {
                        let challenge = hex(&[7u8; 32]);
                        s.challenge = Some(challenge.clone());
                        reply(
                            200,
                            json!({ "deck": { "status": "approved", "challenge": challenge } }),
                        )
                    }
                    other => reply(401, json!({ "error": other })),
                }
            }
            "/decks/token" => {
                let open = s.challenge.take();
                let challenge = body["challenge"].as_str().unwrap_or("");
                if open.as_deref() != Some(challenge) {
                    return reply(401, json!({ "error": "bad_challenge" }));
                }
                let status = s.find(&key).map(|d| d.1.clone()).unwrap_or_default();
                if status != "approved" {
                    return reply(401, json!({ "error": status }));
                }
                let bytes = |h: &str| {
                    (0..h.len())
                        .step_by(2)
                        .map(|i| u8::from_str_radix(&h[i..i + 2], 16).unwrap())
                        .collect::<Vec<u8>>()
                };
                let public = VerifyingKey::from_bytes(&bytes(&key).try_into().unwrap()).unwrap();
                let signature = Signature::from_bytes(
                    &bytes(body["signature"].as_str().unwrap())
                        .try_into()
                        .unwrap(),
                );
                if public
                    .verify(signed_message(challenge, &key).as_bytes(), &signature)
                    .is_err()
                {
                    return reply(401, json!({ "error": "bad_signature" }));
                }
                s.issued += 1;
                reply(
                    200,
                    json!({ "token": format!("scd_{}", s.issued), "deck": { "name": "Stage left" } }),
                )
            }
            _ => reply(404, json!({ "error": "not_found" })),
        }
    }
}

fn info() -> DeckInfo {
    DeckInfo {
        build: "2026.10.29-a1".into(),
        class: "A".into(),
        hostname: "x1-stage".into(),
    }
}

#[test]
fn first_boot_shows_a_code_then_the_approval_opens_the_api() {
    let fake = Fake::default();
    let mut link = Link::new(fake.clone(), Identity::from_seed([1; 32]), info());
    let key = link.public_key_hex();
    let t0 = Instant::now();

    // no data before approval: a code, polled again without a new request
    assert_eq!(
        link.step(t0).unwrap(),
        State::Pending {
            code: "K7Q42N".into()
        }
    );
    assert_eq!(
        link.step(t0).unwrap(),
        State::Pending {
            code: "K7Q42N".into()
        }
    );
    assert_eq!(
        fake.0.borrow().calls,
        ["/decks/challenge", "/decks/request", "/decks/challenge"]
    );
    assert_eq!(link.token(t0), None);

    // a member approves → a token from the signed challenge, reused until it is due
    fake.0.borrow_mut().set(&key, "approved");
    assert_eq!(
        link.step(t0).unwrap(),
        State::Ready {
            token: "scd_1".into(),
            name: "Stage left".into()
        }
    );
    assert_eq!(
        link.step(t0 + Duration::from_secs(60)).unwrap(),
        State::Ready {
            token: "scd_1".into(),
            name: "Stage left".into()
        }
    );
    assert_eq!(link.token(t0 + Duration::from_secs(60)), Some("scd_1"));
    let due = t0 + deck_link::RENEW_AFTER + Duration::from_secs(1);
    assert_eq!(link.token(due), None);
    assert_eq!(
        link.step(due).unwrap(),
        State::Ready {
            token: "scd_2".into(),
            name: "Stage left".into()
        }
    );
}

#[test]
fn revoked_or_expired_the_deck_asks_again_with_a_new_code() {
    for ended in ["revoked", "expired"] {
        let fake = Fake::default();
        let mut link = Link::new(fake.clone(), Identity::from_seed([2; 32]), info())
            .with_renew_after(Duration::from_secs(10));
        let key = link.public_key_hex();
        let t0 = Instant::now();
        let State::Pending { code: first } = link.step(t0).unwrap() else {
            panic!("pending first")
        };
        fake.0.borrow_mut().set(&key, "approved");
        assert!(matches!(link.step(t0).unwrap(), State::Ready { .. }));

        fake.0.borrow_mut().set(&key, ended);
        let State::Pending { code } = link.step(t0 + Duration::from_secs(11)).unwrap() else {
            panic!("{ended}: pending again")
        };
        assert_ne!(code, first, "{ended}: a new code");
        assert_eq!(link.token(t0 + Duration::from_secs(11)), None);
    }
}

#[test]
fn the_api_refusing_the_token_makes_the_next_step_renew() {
    let fake = Fake::default();
    let mut link = Link::new(fake.clone(), Identity::from_seed([3; 32]), info());
    let key = link.public_key_hex();
    let t0 = Instant::now();
    link.step(t0).unwrap();
    fake.0.borrow_mut().set(&key, "approved");
    assert!(matches!(link.step(t0).unwrap(), State::Ready { .. }));
    link.forget_token();
    assert_eq!(
        link.step(t0).unwrap(),
        State::Ready {
            token: "scd_2".into(),
            name: "Stage left".into()
        }
    );
}

#[test]
fn a_forged_signature_is_refused() {
    let fake = Fake::default();
    let mut real = Link::new(fake.clone(), Identity::from_seed([4; 32]), info());
    let key = real.public_key_hex();
    real.step(Instant::now()).unwrap();
    fake.0.borrow_mut().set(&key, "approved");

    // another key signs for this deck's public key
    let challenge = hex(&[7u8; 32]);
    fake.0.borrow_mut().challenge = Some(challenge.clone());
    let forged = Identity::from_seed([5; 32]).sign_challenge(&challenge);
    let r = fake
        .post(
            "/decks/token",
            &json!({ "public_key": key, "challenge": challenge, "signature": forged }),
        )
        .unwrap();
    assert_eq!(
        (r.status, r.body["error"].as_str()),
        (401, Some("bad_signature"))
    );
    assert!(
        matches!(real.step(Instant::now()).unwrap(), State::Ready { .. }),
        "its own signature works"
    );
}

#[test]
fn the_identity_is_made_once_and_kept() {
    let dir = std::env::temp_dir().join(format!("deck-link-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let first = Identity::load_or_create(&dir).unwrap();
    let again = Identity::load_or_create(&dir).unwrap();
    assert_eq!(first.public_key_hex(), again.public_key_hex());
    assert_eq!(first.public_key_hex().len(), 64);
    assert_eq!(std::fs::read(dir.join("ed25519.key")).unwrap().len(), 32);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(dir.join("ed25519.key"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
    std::fs::write(dir.join("ed25519.key"), b"short").unwrap();
    assert!(
        Identity::load_or_create(&dir).is_err(),
        "a broken key is an error, never silently replaced"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_splash_shows_the_code_and_the_pairing_link() {
    assert_eq!(format_code("K7Q42M"), "K7Q-42M");
    assert_eq!(format_code("odd"), "odd");
    assert_eq!(
        pairing_url("https://scutoons.com/", "K7Q42M"),
        "https://scutoons.com/account/decks?deck=K7Q42M"
    );
}
