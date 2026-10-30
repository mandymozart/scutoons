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
        let Some(Ok(Ws::Text(text))) = frame else { return };
        let e: Envelope = serde_json::from_str(&text).unwrap();
        assert!(matches!(e.message, Message::Presence(_)), "no message expected, got {e:?}");
    }
}

async fn send(ws: &mut Socket, message: Message, to: Option<To>) {
    let mut e = Envelope::new("", message);
    e.to = to;
    ws.send(Ws::Text(serde_json::to_string(&e).unwrap())).await.unwrap();
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
    };
    send(&mut cond, Message::VoteOpen(open), None).await;
    assert!(matches!(recv(&mut a1).await.message, Message::VoteOpen(_)));
    assert!(matches!(recv(&mut a2).await.message, Message::VoteOpen(_)));
    send(&mut a1, Message::VoteCast(VoteCast { vote: "v1".into(), option: 1 }), None).await;
    // the conductor gets tallies every 250 ms, then the result after ~1 s
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
