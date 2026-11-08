//! The routes, the queue and the checks in front of a compile.
//!
//! `POST /compile` (header `X-Build-Token`) `{ "files": { "lib.rs": "…" } }`
//!   → `200 { hash, descriptor, wasm_b64, toolchain, warnings }`
//!   · `422 { errors: [{ file, line, column, level, message }] }` – the files break a rule, or rustc's errors
//!   · `503 { error: "live_show" }` while the relay reports a live session · `503 { error: "busy" }` when the queue is full
//!     or the wait too long · `401` without the token · `400` / `413` for a body that is not one
//! `POST /validate` (header `X-Build-Token`, #313) `{ kind: instruments | presets | rigs, id, document, instruments?, presets?,
//!   modules? }` (`modules`: a code rig's policy descriptor, #408)
//!   → `200 { problems: [..], warnings?: [..] }` (none = valid) – deck-space's own checks (rig/validate.rs), the words the creator shows;
//!   the API asks before every catalog write of the three kinds. No queue and no live-show refusal: a check is
//!   microseconds of the core, never rustc. `400` for a body that is not such a request, `401` without the token.
//! `GET /health` → `{ ok, waiting }` (no token: a probe).
//!
//! **One compile at a time** (the relay beside us must stay smooth): a request that finds the slot taken waits, up to
//! `max_queue` of them for up to `queue_wait`; the next is turned away at once.
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use deck_space::rig::validate;
use serde_json::json;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;

use crate::compile::{Built, CompileError, Compiler};
use crate::config::Config;
use crate::diag::Diag;
use crate::files::{self, Files};
use crate::http::{read_request, write_response, ReadError, Request, Response};
use crate::runner::BoxFuture;

/// What the service does to a request's files: the real thing is `Compiler`; the tests put a fake in its place
pub trait Build: Send + Sync {
    fn live_show(&self) -> BoxFuture<'_, bool>;
    fn compile<'a>(&'a self, files: &'a Files) -> BoxFuture<'a, Result<Built, CompileError>>;
    fn toolchain(&self) -> Option<String>;
}

impl Build for Compiler {
    fn live_show(&self) -> BoxFuture<'_, bool> {
        Box::pin(Compiler::live_show(self))
    }

    fn compile<'a>(&'a self, files: &'a Files) -> BoxFuture<'a, Result<Built, CompileError>> {
        Box::pin(Compiler::compile(self, files))
    }

    fn toolchain(&self) -> Option<String> {
        Compiler::toolchain(self).map(str::to_string)
    }
}

pub struct Service {
    config: Arc<Config>,
    build: Arc<dyn Build>,
    slot: Semaphore,
    waiting: AtomicUsize,
}

/// compares without stopping at the first difference
fn same_secret(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut diff = a.len() ^ b.len();
    for i in 0..a.len().max(b.len()) {
        diff |= usize::from(a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0));
    }
    diff == 0
}

impl Service {
    pub fn new(config: Arc<Config>, build: Arc<dyn Build>) -> Service {
        Service { config, build, slot: Semaphore::new(1), waiting: AtomicUsize::new(0) }
    }

    pub async fn handle(&self, req: &Request) -> Response {
        match (req.method.as_str(), req.path.as_str()) {
            ("GET", "/health") => Response::json(200, &json!({ "ok": true, "waiting": self.waiting.load(Ordering::Relaxed), "busy": self.slot.available_permits() == 0 })),
            ("POST", "/compile") => self.compile(req).await,
            ("POST", "/validate") => self.validate(req),
            (_, "/health" | "/compile" | "/validate") => Response::error(405, "method_not_allowed"),
            _ => Response::error(404, "not_found"),
        }
    }

    fn authorized(&self, req: &Request) -> bool {
        req.header("x-build-token").is_some_and(|t| same_secret(t, &self.config.token))
    }

    fn validate(&self, req: &Request) -> Response {
        if !self.authorized(req) {
            return Response::error(401, "unauthorized");
        }
        let Ok(request) = serde_json::from_slice::<validate::Request>(&req.body) else {
            return Response::error(400, "bad_request");
        };
        match request.answer() {
            Ok(answer) => Response::json(200, &json!(answer)),
            Err(_) => Response::error(400, "unknown_kind"),
        }
    }

    async fn compile(&self, req: &Request) -> Response {
        if !self.authorized(req) {
            return Response::error(401, "unauthorized");
        }
        if serde_json::from_slice::<serde_json::Value>(&req.body).is_err() {
            return Response::error(400, "bad_request");
        }
        let files = match files::from_json(&req.body) {
            Ok(f) => f,
            Err(errors) => return errors_response(&errors),
        };
        // fast refusals before anyone waits
        if self.build.live_show().await {
            return Response::error(503, "live_show");
        }
        let _slot = match self.slot.try_acquire() {
            Ok(permit) => permit,
            Err(_) => {
                if self.waiting.fetch_add(1, Ordering::SeqCst) >= self.config.max_queue {
                    self.waiting.fetch_sub(1, Ordering::SeqCst);
                    return Response::error(503, "busy");
                }
                let got = tokio::time::timeout(self.config.queue_wait, self.slot.acquire()).await;
                self.waiting.fetch_sub(1, Ordering::SeqCst);
                match got {
                    Ok(Ok(permit)) => permit,
                    _ => return Response::error(503, "busy"),
                }
            }
        };
        // a show may have started while we waited
        if self.build.live_show().await {
            return Response::error(503, "live_show");
        }
        match self.build.compile(&files).await {
            Ok(built) => Response::json(
                200,
                &json!({
                    "hash": built.hash,
                    "descriptor": built.descriptor,
                    "wasm_b64": built.wasm_b64(),
                    "toolchain": self.build.toolchain(),
                    "warnings": built.warnings,
                }),
            ),
            Err(CompileError::Rejected(errors) | CompileError::Failed(errors)) => errors_response(&errors),
            Err(CompileError::LiveShow) => Response::error(503, "live_show"),
            Err(CompileError::Internal(message)) => {
                tracing::error!(%message, "compile failed inside the service");
                Response::error(500, "internal")
            }
        }
    }
}

fn errors_response(errors: &[Diag]) -> Response {
    Response::json(422, &json!({ "errors": errors }))
}

/// Accepts connections until the future is dropped; one task per connection
pub async fn serve(listener: TcpListener, service: Arc<Service>) -> std::io::Result<()> {
    loop {
        let (mut stream, peer) = listener.accept().await?;
        let service = service.clone();
        tokio::spawn(async move {
            // the body limit: all files plus the JSON around them
            let response = match read_request(&mut stream, files::MAX_TOTAL_BYTES * 2 + 64 * 1024).await {
                Ok(req) => service.handle(&req).await,
                Err(ReadError::TooLarge) => Response::error(413, "too_large"),
                Err(ReadError::Bad(why)) => {
                    tracing::debug!(%peer, why, "bad request");
                    Response::error(400, "bad_request")
                }
                Err(ReadError::Gone) => return,
            };
            let _ = write_response(&mut stream, &response).await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::sync::Notify;

    /// A build that waits for the test to let it finish
    struct Fake {
        live: std::sync::atomic::AtomicBool,
        gate: Notify,
        started: AtomicUsize,
        result: std::sync::Mutex<Option<Result<(), Vec<Diag>>>>,
    }

    impl Fake {
        fn new() -> Arc<Fake> {
            Arc::new(Fake { live: false.into(), gate: Notify::new(), started: 0.into(), result: std::sync::Mutex::new(None) })
        }
    }

    impl Build for Fake {
        fn live_show(&self) -> BoxFuture<'_, bool> {
            Box::pin(async { self.live.load(Ordering::SeqCst) })
        }

        fn compile<'a>(&'a self, _files: &'a Files) -> BoxFuture<'a, Result<Built, CompileError>> {
            Box::pin(async {
                self.started.fetch_add(1, Ordering::SeqCst);
                self.gate.notified().await;
                match self.result.lock().unwrap().clone() {
                    Some(Err(errors)) => Err(CompileError::Failed(errors)),
                    _ => Ok(Built { hash: "ab".repeat(32), descriptor: json!({ "abi": 2, "name": "fake" }), wasm: b"\0asm\x01\0\0\0".to_vec(), warnings: vec![] }),
                }
            })
        }

        fn toolchain(&self) -> Option<String> {
            Some("rustc 1.99.1".into())
        }
    }

    fn config(queue: usize, wait_ms: u64) -> Arc<Config> {
        let env = |k: &str| match k {
            "MODULE_BUILD_TOKEN" => Some("sesame".to_string()),
            "BUILD_QUEUE" => Some(queue.to_string()),
            _ => None,
        };
        let mut c = Config::from_env(env).unwrap();
        c.queue_wait = Duration::from_millis(wait_ms);
        Arc::new(c)
    }

    fn post(token: Option<&str>, body: &str) -> Request {
        Request {
            method: "POST".into(),
            path: "/compile".into(),
            headers: token.map(|t| vec![("x-build-token".to_string(), t.to_string())]).unwrap_or_default(),
            body: body.as_bytes().to_vec(),
        }
    }

    const GOOD: &str = r#"{"files":{"lib.rs":"// m"}}"#;

    fn status(r: &Response) -> (u16, String) {
        let v: serde_json::Value = serde_json::from_str(&r.body).unwrap();
        (r.status, v.get("error").and_then(|e| e.as_str()).unwrap_or("").to_string())
    }

    #[test]
    fn secrets_are_compared_whole() {
        assert!(same_secret("sesame", "sesame"));
        assert!(!same_secret("sesame", "sesamf"));
        assert!(!same_secret("sesame", "sesam"));
        assert!(!same_secret("", "sesame"));
        assert!(same_secret("", ""));
    }

    #[tokio::test]
    async fn the_token_is_checked_before_anything_else() {
        let fake = Fake::new();
        let service = Service::new(config(2, 100), fake.clone());
        for token in [None, Some(""), Some("sesam"), Some("Sesame")] {
            assert_eq!(status(&service.handle(&post(token, GOOD)).await), (401, "unauthorized".into()), "{token:?}");
        }
        assert_eq!(status(&service.handle(&post(Some("x"), "not json")).await), (401, "unauthorized".into()), "a stranger learns nothing about the body");
        assert_eq!(fake.started.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn routes_and_bodies() {
        let service = Service::new(config(2, 100), Fake::new());
        let get = |path: &str| Request { method: "GET".into(), path: path.into(), headers: vec![], body: vec![] };
        assert_eq!(service.handle(&get("/health")).await.status, 200);
        assert_eq!(status(&service.handle(&get("/compile")).await), (405, "method_not_allowed".into()));
        assert_eq!(status(&service.handle(&get("/other")).await), (404, "not_found".into()));
        assert_eq!(status(&service.handle(&post(Some("sesame"), "nope")).await), (400, "bad_request".into()));
        let r = service.handle(&post(Some("sesame"), r#"{"files":{"osc.rs":"//"}}"#)).await;
        assert_eq!(r.status, 422);
        assert!(r.body.contains("needs a lib.rs"), "{}", r.body);
    }

    fn validate(token: Option<&str>, body: &str) -> Request {
        Request { path: "/validate".into(), ..post(token, body) }
    }

    /// #313: the fixture set (server/tests/fixtures/rigs/validate.json) through the route – the requests as the API builds
    /// them, the answers in the retired PHP validator's words; no compile runs, a live show does not stop a check
    #[tokio::test]
    async fn validate_answers_the_cores_problems() {
        let fake = Fake::new();
        fake.live.store(true, Ordering::SeqCst);
        let service = Service::new(config(0, 100), fake.clone());
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../server/tests/fixtures/rigs/validate.json");
        let set: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let mut slowest = Duration::ZERO;
        for case in set["cases"].as_array().unwrap() {
            let mut request = case.clone();
            let o = request.as_object_mut().unwrap();
            let name = o.remove("name").unwrap();
            let expected = o.remove("problems").unwrap();
            let t = std::time::Instant::now();
            let r = service.handle(&validate(Some("sesame"), &request.to_string())).await;
            slowest = slowest.max(t.elapsed());
            assert_eq!(r.status, 200, "{name}: {}", r.body);
            let v: serde_json::Value = serde_json::from_str(&r.body).unwrap();
            assert_eq!(v, json!({ "problems": expected }), "{name}");
        }
        // the latency the API adds on a publish, without the network: the slowest case (a rig played in every mood)
        eprintln!("validate: slowest fixture case {slowest:?}");
        assert!(slowest < Duration::from_millis(500), "{slowest:?}");
        assert_eq!(fake.started.load(Ordering::SeqCst), 0, "no compile");
    }

    /// #414: a code rig as the API sends it – the rig, what it names and the descriptor of its policy module from the
    /// catalog's `module:<id>` document (server/tests/fixtures/rigs/validate-code-rig.json, the request ValidateByCoreTest
    /// builds from the catalog): the real core accepts it with `modules`, refuses it without, and says what it warns about
    #[tokio::test]
    async fn validate_accepts_a_code_rig_with_its_policy_module() {
        let service = Service::new(config(0, 100), Fake::new());
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../server/tests/fixtures/rigs/validate-code-rig.json");
        let set: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        for case in set["cases"].as_array().unwrap() {
            let mut request = case.clone();
            let o = request.as_object_mut().unwrap();
            let name = o.remove("name").unwrap();
            let problems = o.remove("problems").unwrap();
            let warnings = o.remove("warnings").unwrap();
            assert_eq!(request["modules"], json!([set["module"]["descriptor"]]), "{name}: the module document's descriptor, by value");
            let r = service.handle(&validate(Some("sesame"), &request.to_string())).await;
            assert_eq!(r.status, 200, "{name}: {}", r.body);
            let v: serde_json::Value = serde_json::from_str(&r.body).unwrap();
            assert_eq!(v["problems"], problems, "{name}");
            assert_eq!(v["warnings"], warnings, "{name}");
            // the API never leaves the descriptor out of a code rig's request: if it did, the core would refuse it
            request.as_object_mut().unwrap().remove("modules");
            let r = service.handle(&validate(Some("sesame"), &request.to_string())).await;
            let v: serde_json::Value = serde_json::from_str(&r.body).unwrap();
            assert_eq!(v["problems"], json!(["rig character.catpope: no module 'catpope_rig' for its policy – the policy's module:catpope_rig document comes with the rig"]),"{name}: without `modules`");
        }
    }

    #[tokio::test]
    async fn validate_wants_the_token_and_a_request() {
        let service = Service::new(config(2, 100), Fake::new());
        let good = r#"{"kind":"instruments","id":"x","document":{}}"#;
        for token in [None, Some("sesam")] {
            assert_eq!(status(&service.handle(&validate(token, good)).await), (401, "unauthorized".into()), "{token:?}");
        }
        assert_eq!(status(&service.handle(&validate(Some("sesame"), "nope")).await), (400, "bad_request".into()));
        assert_eq!(status(&service.handle(&validate(Some("sesame"), r#"{"kind":"rigs"}"#)).await), (400, "bad_request".into()));
        assert_eq!(status(&service.handle(&validate(Some("sesame"), r#"{"kind":"items","id":"x","document":{}}"#)).await), (400, "unknown_kind".into()));
        let get = Request { method: "GET".into(), ..validate(Some("sesame"), "") };
        assert_eq!(status(&service.handle(&get).await), (405, "method_not_allowed".into()));
        // a broken document is an answer, not an error: its problem
        let r = service.handle(&validate(Some("sesame"), good)).await;
        assert_eq!(r.status, 200);
        let v: serde_json::Value = serde_json::from_str(&r.body).unwrap();
        assert!(v["problems"][0].as_str().unwrap().starts_with("instrument x: missing field"), "{}", r.body);
    }

    #[tokio::test]
    async fn a_compile_answers_with_the_module() {
        let fake = Fake::new();
        let service = Arc::new(Service::new(config(2, 100), fake.clone()));
        let s = service.clone();
        let task = tokio::spawn(async move { s.handle(&post(Some("sesame"), GOOD)).await });
        tokio::time::sleep(Duration::from_millis(50)).await;
        fake.gate.notify_one();
        let r = task.await.unwrap();
        assert_eq!(r.status, 200, "{}", r.body);
        let v: serde_json::Value = serde_json::from_str(&r.body).unwrap();
        assert_eq!((v["hash"].as_str().unwrap().len(), v["toolchain"].as_str(), v["wasm_b64"].as_str()), (64, Some("rustc 1.99.1"), Some("AGFzbQEAAAA=")));
        assert_eq!(v["descriptor"]["name"], "fake");
    }

    #[tokio::test]
    async fn rustcs_errors_are_a_422_with_places() {
        let fake = Fake::new();
        *fake.result.lock().unwrap() = Some(Err(vec![Diag::error("lib.rs", 3, 9, "cannot find value `x`")]));
        let service = Arc::new(Service::new(config(2, 100), fake.clone()));
        let s = service.clone();
        let task = tokio::spawn(async move { s.handle(&post(Some("sesame"), GOOD)).await });
        tokio::time::sleep(Duration::from_millis(50)).await;
        fake.gate.notify_one();
        let r = task.await.unwrap();
        let v: serde_json::Value = serde_json::from_str(&r.body).unwrap();
        assert_eq!(r.status, 422);
        assert_eq!(v["errors"][0], json!({ "file": "lib.rs", "line": 3, "column": 9, "level": "error", "message": "cannot find value `x`" }));
    }

    #[tokio::test]
    async fn no_compile_while_a_show_is_live() {
        let fake = Fake::new();
        fake.live.store(true, Ordering::SeqCst);
        let service = Service::new(config(2, 100), fake.clone());
        assert_eq!(status(&service.handle(&post(Some("sesame"), GOOD)).await), (503, "live_show".into()));
        assert_eq!(fake.started.load(Ordering::SeqCst), 0, "rustc never started");
    }

    #[tokio::test]
    async fn a_show_that_starts_while_waiting_stops_the_wait() {
        let fake = Fake::new();
        let service = Arc::new(Service::new(config(2, 2000), fake.clone()));
        let (a, b) = (service.clone(), service.clone());
        let first = tokio::spawn(async move { a.handle(&post(Some("sesame"), GOOD)).await });
        tokio::time::sleep(Duration::from_millis(50)).await;
        let second = tokio::spawn(async move { b.handle(&post(Some("sesame"), GOOD)).await });
        tokio::time::sleep(Duration::from_millis(50)).await;
        fake.live.store(true, Ordering::SeqCst);
        fake.gate.notify_one();
        assert_eq!(first.await.unwrap().status, 200);
        assert_eq!(status(&second.await.unwrap()), (503, "live_show".into()));
        assert_eq!(fake.started.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn one_compile_at_a_time_a_short_queue_then_busy() {
        let fake = Fake::new();
        let service = Arc::new(Service::new(config(1, 200), fake.clone()));
        let spawn = |s: Arc<Service>| tokio::spawn(async move { s.handle(&post(Some("sesame"), GOOD)).await });
        let running = spawn(service.clone());
        tokio::time::sleep(Duration::from_millis(50)).await;
        let waiting = spawn(service.clone());
        tokio::time::sleep(Duration::from_millis(50)).await;
        // the queue (1) is full: the third is turned away at once
        let t = std::time::Instant::now();
        assert_eq!(status(&service.handle(&post(Some("sesame"), GOOD)).await), (503, "busy".into()));
        assert!(t.elapsed() < Duration::from_millis(100));
        // the waiting one gives up after the wait while the first still runs
        assert_eq!(status(&waiting.await.unwrap()), (503, "busy".into()));
        assert_eq!(fake.started.load(Ordering::SeqCst), 1, "only one compile ever ran at once");
        fake.gate.notify_one();
        assert_eq!(running.await.unwrap().status, 200);
        // the slot is free again
        let again = spawn(service.clone());
        tokio::time::sleep(Duration::from_millis(50)).await;
        fake.gate.notify_one();
        assert_eq!(again.await.unwrap().status, 200);
    }

    #[tokio::test]
    async fn a_waiting_request_gets_the_slot_when_it_frees() {
        let fake = Fake::new();
        let service = Arc::new(Service::new(config(2, 5000), fake.clone()));
        let spawn = |s: Arc<Service>| tokio::spawn(async move { s.handle(&post(Some("sesame"), GOOD)).await });
        let first = spawn(service.clone());
        tokio::time::sleep(Duration::from_millis(50)).await;
        let second = spawn(service.clone());
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(fake.started.load(Ordering::SeqCst), 1);
        fake.gate.notify_one();
        assert_eq!(first.await.unwrap().status, 200);
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(fake.started.load(Ordering::SeqCst), 2);
        fake.gate.notify_one();
        assert_eq!(second.await.unwrap().status, 200);
    }
}
