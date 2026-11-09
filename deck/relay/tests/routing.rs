//! Two roles, two groups, one relay: a message addressed to `{ role: output, group: a }` reaches only that client; a
//! message without `to` reaches everyone else. Runs the real WebSocket server on a free port.

use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use protocol::{Cue, Envelope, Hello, Message, Panic, Ping, Resume, Role, To, VoteCast, VoteHow, VoteOpen, VoteRule};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message as Ws;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

async fn start() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(relay::serve(listener, Arc::new(relay::Relay::new())));
    port
}

/// The next message that is not `presence` (the relay tells the conductor about every join / leave / ready).
async fn recv(ws: &mut Socket) -> Envelope {
    loop {
        let frame = tokio::time::timeout(Duration::from_secs(3), ws.next()).await.expect("a message").unwrap().unwrap();
        if matches!(frame, Ws::Ping(_) | Ws::Pong(_)) {
            continue; // the relay's keep-alive
        }
        let Ws::Text(text) = frame else { panic!("text expected, got {frame:?}") };
        let e: Envelope = serde_json::from_str(&text).unwrap();
        if !matches!(e.message, Message::Presence(_)) {
            return e;
        }
    }
}

/// No message other than `presence` within 300 ms.
async fn nothing(ws: &mut Socket) {
    let deadline = tokio::time::Instant::now() + Duration::from_millis(300);
    while let Ok(frame) = tokio::time::timeout_at(deadline, ws.next()).await {
        if matches!(frame, Some(Ok(Ws::Ping(_) | Ws::Pong(_)))) {
            continue;
        }
        let Some(Ok(Ws::Text(text))) = frame else { return };
        let e: Envelope = serde_json::from_str(&text).unwrap();
        assert!(matches!(e.message, Message::Presence(_)), "no message expected, got {e:?}");
    }
}

async fn send(ws: &mut Socket, message: Message, to: Option<To>) {
    let mut e = Envelope::new("", message);
    e.to = to;
    ws.send(Ws::Text(serde_json::to_string(&e).unwrap().into())).await.unwrap();
}

/// Connect, say hello, return the socket + the client id from `welcome`.
async fn join(port: u16, code: &str, role: Role, group: Option<&str>, token: Option<&str>) -> (Socket, String) {
    join_resume(port, code, role, group, token, None).await
}

async fn join_resume(
    port: u16,
    code: &str,
    role: Role,
    group: Option<&str>,
    token: Option<&str>,
    resume: Option<Resume>,
) -> (Socket, String) {
    let url = format!("ws://127.0.0.1:{port}/live/{code}?role={}", role.as_str());
    let (mut ws, _) = connect_async(url).await.unwrap();
    let hello = Hello {
        code: code.into(),
        role,
        group: group.map(str::to_owned),
        nick: None,
        token: token.map(str::to_owned),
        resume: resume.clone(),
    };
    send(&mut ws, Message::Hello(hello), None).await;
    let welcome = recv(&mut ws).await;
    let Message::Welcome(w) = welcome.message else { panic!("welcome expected: {welcome:?}") };
    if resume.is_none() {
        assert_eq!(w.group.as_deref(), group);
    }
    assert_eq!(welcome.from, "relay");
    (ws, w.client)
}

#[tokio::test]
async fn routes_by_role_and_group() {
    let port = start().await;
    let (mut cond, cond_id) = join(port, "T1", Role::Conductor, None, Some("dev-token")).await;
    let (mut out_a, _) = join(port, "T1", Role::Output, Some("a"), None).await;
    let (mut out_b, _) = join(port, "T1", Role::Output, Some("b"), None).await;
    let (mut other, _) = join(port, "OTHER", Role::Output, Some("a"), None).await;

    // a cue for the left screens only
    let cue = Cue { id: "cue-1".into(), actions: vec![] };
    send(&mut cond, Message::Cue(cue), Some(To::role_group(Role::Output, "a"))).await;
    let got = recv(&mut out_a).await;
    assert_eq!(got.from, cond_id);
    assert!(got.seq > 0, "the relay stamps seq");
    assert!(matches!(got.message, Message::Cue(ref c) if c.id == "cue-1"));
    nothing(&mut out_b).await;
    nothing(&mut other).await; // another session
    nothing(&mut cond).await; // not back to the sender

    // panic without `to`: everyone else in the session
    send(&mut cond, Message::Panic(Panic { black: Some(true), stop: None }), None).await;
    let a = recv(&mut out_a).await;
    let b = recv(&mut out_b).await;
    assert!(matches!(a.message, Message::Panic(_)));
    assert_eq!(a.seq, b.seq, "one broadcast, one seq");
    nothing(&mut other).await;
}

/// A frame far larger than the relay's 4 KiB read buffer (a big `deck.graph` or `scene.sync`) arrives whole (#437).
#[tokio::test]
async fn a_frame_larger_than_the_read_buffer_arrives_whole() {
    let port = start().await;
    let (mut cond, _) = join(port, "BIG", Role::Conductor, None, Some("dev-token")).await;
    let (mut out, _) = join(port, "BIG", Role::Output, None, None).await;
    let id = "x".repeat(64 * relay::READ_BUFFER);
    send(&mut cond, Message::Cue(Cue { id: id.clone(), actions: vec![] }), None).await;
    let got = recv(&mut out).await;
    assert!(matches!(got.message, Message::Cue(ref c) if c.id == id));
}

#[tokio::test]
async fn ping_pong_and_refusals() {
    let port = start().await;
    let (mut out, _) = join(port, "T2", Role::Output, None, None).await;
    send(&mut out, Message::Ping(Ping { t0: 123 }), None).await;
    let pong = recv(&mut out).await;
    let Message::Pong(p) = pong.message else { panic!("pong expected") };
    assert_eq!(p.t0, 123);
    assert!(p.t1 > 0);

    // a conductor without a token is refused
    let (mut ws, _) = connect_async(format!("ws://127.0.0.1:{port}/live/T2")).await.unwrap();
    let hello = Hello { code: "T2".into(), role: Role::Conductor, group: None, nick: None, token: None, resume: None };
    send(&mut ws, Message::Hello(hello), None).await;
    let e = recv(&mut ws).await;
    assert!(matches!(e.message, Message::Error(ref err) if err.code == "token"), "{e:?}");

    // the first message must be hello
    let (mut ws, _) = connect_async(format!("ws://127.0.0.1:{port}/live/T2")).await.unwrap();
    send(&mut ws, Message::Ping(Ping { t0: 1 }), None).await;
    let e = recv(&mut ws).await;
    assert!(matches!(e.message, Message::Error(ref err) if err.code == "hello"), "{e:?}");
}

#[tokio::test]
async fn status_is_plain_http() {
    let port = start().await;
    let (_cond, _) = join(port, "T3", Role::Conductor, None, Some("t")).await;
    let (_out, _) = join(port, "T4", Role::Output, None, None).await;
    let mut tcp = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    tcp.write_all(b"GET /status HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n").await.unwrap();
    let mut buf = String::new();
    tcp.read_to_string(&mut buf).await.unwrap();
    assert!(buf.starts_with("HTTP/1.1 200"), "{buf}");
    assert!(buf.ends_with(r#"{"sessions":2,"live":1}"#), "{buf}");
}

#[tokio::test]
async fn a_vote_closes_by_itself() {
    let port = start().await;
    let (mut cond, _) = join(port, "V1", Role::Conductor, None, Some("t")).await;
    let (mut a1, _) = join(port, "V1", Role::Audience, Some("left"), None).await;
    let (mut a2, _) = join(port, "V1", Role::Audience, Some("right"), None).await;
    let open = VoteOpen {
        vote: "v1".into(),
        options: vec!["stay".into(), "go".into()],
        rule: VoteRule::Majority,
        n: None,
        seconds: 1,
        fallback: None,
        groups: None,
        show: None,
        question: None,
        labels: None,
    };
    send(&mut cond, Message::VoteOpen(open), None).await;
    assert!(matches!(recv(&mut a1).await.message, Message::VoteOpen(_)));
    assert!(matches!(recv(&mut a2).await.message, Message::VoteOpen(_)));
    send(&mut a1, Message::VoteCast(VoteCast { vote: "v1".into(), option: 1 }), None).await;
    // the conductor sees the vote open too, then gets tallies every 250 ms, then the result after ~1 s
    assert!(matches!(recv(&mut cond).await.message, Message::VoteOpen(_)));
    let first = recv(&mut cond).await;
    let Message::VoteTally(t) = first.message else { panic!("tally expected: {first:?}") };
    assert!(t.left <= 1000);
    let result = loop {
        let e = recv(&mut cond).await;
        if let Message::VoteResult(r) = e.message {
            break r;
        }
    };
    assert_eq!((result.option, result.how, result.counts), (1, VoteHow::Rule, vec![0, 1]));
    assert!(matches!(recv(&mut a2).await.message, Message::VoteResult(_)), "everyone gets the result");
}

#[tokio::test]
async fn a_phone_resumes_without_losing_messages() {
    let port = start().await;
    let (mut cond, _) = join(port, "R1", Role::Conductor, None, Some("t")).await;
    let (mut phone, phone_id) = join(port, "R1", Role::Audience, Some("left"), None).await;
    let say = |t: &str| Message::Say(serde_json::from_str(&format!(r#"{{"outlet":"left","text":"{t}"}}"#)).unwrap());
    send(&mut cond, say("one"), None).await;
    let seen = recv(&mut phone).await.seq;
    drop(phone); // the phone sleeps
    tokio::time::sleep(Duration::from_millis(100)).await;
    send(&mut cond, say("two"), None).await;
    send(&mut cond, say("three"), None).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    let resume = Resume { client: phone_id.clone(), seq: seen as i64 };
    let (mut phone, again) = join_resume(port, "R1", Role::Audience, None, None, Some(resume)).await;
    assert_eq!(again, phone_id, "the same client id (its vote stays counted)");
    // (join_resume skipped the group check: a resumed phone keeps its QR's group "left" without naming it)
    for want in ["two", "three"] {
        let e = recv(&mut phone).await;
        let Message::Say(p) = e.message else { panic!("say expected: {e:?}") };
        assert_eq!(p["text"], want);
    }
    nothing(&mut phone).await;
}

/// A values message of ~60 KB from deck `d1`: 4000 keys plus `n` = [i] – a few fill any socket buffer.
fn big_values(i: usize) -> Message {
    let mut values: std::collections::HashMap<String, Vec<f64>> = (0..4000).map(|k| (format!("k{k:04}"), vec![0.5])).collect();
    values.insert("n".into(), vec![i as f64]);
    Message::DeckValues(protocol::DeckValues { values, mood: None })
}

fn graph() -> Message {
    Message::DeckGraph(serde_json::from_str(r#"{"nodes":[{"id":"lfo","kind":"lfo"}],"links":[]}"#).unwrap())
}

/// #223: a client that doesn't read (its socket full) gets the newest values when it drains – not a backlog – and every
/// cue, in order
#[tokio::test]
async fn a_stalled_client_gets_the_newest_values_and_every_cue() {
    let port = start().await;
    let (mut cond, _) = join(port, "S1", Role::Conductor, None, Some("t")).await;
    let (mut d1, _) = join(port, "S1", Role::Musician, Some("decks"), None).await;
    let (mut d2, d2_id) = join(port, "S1", Role::Musician, Some("decks"), None).await;
    send(&mut d1, graph(), None).await;
    // 15 values (the relay's burst) to d2, which doesn't read: a few fill its socket, the rest wait in its outbox
    for i in 0..15 {
        send(&mut d1, big_values(i), Some(To::client(d2_id.clone()))).await;
    }
    tokio::time::sleep(Duration::from_millis(300)).await;
    for i in 0..20 {
        send(&mut cond, Message::Cue(Cue { id: format!("c{i}"), actions: vec![] }), Some(To::client(d2_id.clone()))).await;
    }
    let (mut values, mut newest, mut cues) = (0, -1.0, vec![]);
    while cues.len() < 20 {
        let e = recv(&mut d2).await;
        match e.message {
            Message::DeckValues(v) => {
                values += 1;
                newest = v.values["n"][0];
            }
            Message::Cue(c) => cues.push(c.id),
            _ => {}
        }
    }
    assert_eq!(cues, (0..20).map(|i| format!("c{i}")).collect::<Vec<_>>(), "every cue, in order");
    assert_eq!(newest, 14.0, "the newest values arrived (before the cues)");
    assert!(values < 15, "the stale ones were replaced: {values} of 15 arrived");
    nothing(&mut d2).await;
}

/// #223: a client whose ordered backlog grows past the bound is dropped (it reconnects and resyncs from its snapshot);
/// the others go on
#[tokio::test]
async fn a_client_too_far_behind_is_dropped() {
    let port = start().await;
    let (mut cond, _) = join(port, "S2", Role::Conductor, None, Some("t")).await;
    let (mut stalled, _) = join(port, "S2", Role::Output, Some("a"), None).await;
    let (mut fine, _) = join(port, "S2", Role::Output, Some("b"), None).await;
    // more cues than the outbox keeps for the stalled output, ~4 KB each (12 MB): its socket fills, the outbox overflows
    let action: protocol::Loose = serde_json::from_str(&format!(r#"{{"say":{{"outlet":"a","text":"{}"}}}}"#, "x".repeat(4000))).unwrap();
    let n = relay::outbox::MAX_QUEUED + 1000;
    for i in 0..n {
        send(&mut cond, Message::Cue(Cue { id: format!("c{i}"), actions: vec![action.clone()] }), Some(To::group("a"))).await;
    }
    send(&mut cond, Message::Panic(Panic { black: Some(true), stop: None }), Some(To::group("b"))).await;
    assert!(matches!(recv(&mut fine).await.message, Message::Panic(_)), "the others go on");
    // the stalled one reads what reached its socket, then the connection ends – short of the last cue
    let mut last = None;
    loop {
        match tokio::time::timeout(Duration::from_secs(5), stalled.next()).await.expect("the relay closes it") {
            Some(Ok(Ws::Text(text))) => {
                if let Ok(Envelope { message: Message::Cue(c), .. }) = serde_json::from_str::<Envelope>(&text) {
                    last = Some(c.id);
                }
            }
            Some(Ok(Ws::Close(_))) | Some(Err(_)) | None => break,
            Some(Ok(_)) => {}
        }
    }
    assert_ne!(last, Some(format!("c{}", n - 1)), "dropped before the end");
}

/// #223: presence shows the conductor each link (not the audience's): its rate, and – once a ping came back – its round
/// trip
#[tokio::test]
async fn the_conductor_sees_each_link() {
    let port = start().await;
    let (mut cond, _) = join(port, "L1", Role::Conductor, None, Some("t")).await;
    let (mut out, out_id) = join(port, "L1", Role::Output, Some("a"), None).await;
    let (_phone, phone_id) = join(port, "L1", Role::Audience, Some("a"), None).await;
    // the relay pings every PING_EVERY; tungstenite answers when the socket is read – keep the output's read side busy
    let reader = tokio::spawn(async move { while let Some(Ok(_)) = out.next().await {} });
    tokio::time::sleep(relay::PING_EVERY + Duration::from_millis(500)).await;
    let (_late, _) = join(port, "L1", Role::Output, Some("b"), None).await;
    let presence = loop {
        let frame = tokio::time::timeout(Duration::from_secs(3), cond.next()).await.expect("presence").unwrap().unwrap();
        let Ws::Text(text) = frame else { continue };
        let e: Envelope = serde_json::from_str(&text).unwrap();
        if let Message::Presence(p) = e.message {
            if p.links.as_ref().is_some_and(|l| l.len() == 3) {
                break p;
            }
        }
    };
    let links = presence.links.unwrap();
    assert_eq!(links[&out_id].hz, 30, "a good link: the full rate");
    assert!(links[&out_id].rtt.is_some(), "a pong came back: {links:?}");
    assert!(!links.contains_key(&phone_id), "no audience phones");
    reader.abort();
}

/// #438: a client whose new link comes before the relay saw the old one go (QUIC to WebSocket and back) takes its own id
/// over; the old link ends, and its ending does not take the new one along.
#[tokio::test]
async fn a_resume_takes_over_a_client_still_attached() {
    let port = start().await;
    let (mut cond, _) = join(port, "R9", Role::Conductor, None, Some("dev-token")).await;
    let (mut old, id) = join(port, "R9", Role::Output, Some("a"), None).await;
    let resume = Resume { client: id.clone(), seq: 0 };
    let (mut new, again) = join_resume(port, "R9", Role::Output, Some("a"), None, Some(resume)).await;
    assert_eq!(again, id, "the same client, not a new one beside a ghost");
    tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(Ok(frame)) = old.next().await {
            if frame.is_close() {
                break;
            }
        }
    })
    .await
    .expect("the old link is ended");
    tokio::time::sleep(Duration::from_millis(100)).await;
    send(&mut cond, Message::Cue(Cue { id: "after".into(), actions: vec![] }), Some(To::role_group(Role::Output, "a"))).await;
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if matches!(recv(&mut new).await.message, Message::Cue(c) if c.id == "after") {
                break;
            }
        }
    })
    .await
    .expect("the new link is still the client");
}
