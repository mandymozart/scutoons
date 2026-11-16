//! A catalog module's published build, as `GET /api/modules/<id>` answers it: the file a `module:<id>` node plays and
//! why there is none. The platform moves the bytes (deck-native `Library::published`); what it asks and what it says
//! are decided here.
use super::Bundle;
use serde_json::Value;

/// The build a module document names: the SHA-256 of its wasm and, in the API's answer, where to fetch it
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Build {
    pub hash: String,
    pub url: Option<String>,
}

/// the API's path for module `id`
pub fn path(id: &str) -> String {
    format!("modules/{id}")
}

/// the file of a build in the module cache folder (content-addressed: the same name is the same bytes)
pub fn file_name(hash: &str) -> String {
    format!("{hash}.wasm")
}

fn is_hash(h: &str) -> bool {
    h.len() == 64 && h.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// `{ module: { id, wasm: { hash, url? } } }` (the API's answer) or the module document itself → its build, else why
/// module `id` plays no sound
pub fn of(id: &str, answer: &Value) -> Result<Build, String> {
    let doc = answer.get("module").unwrap_or(answer);
    let wasm = doc.get("wasm").filter(|w| w.is_object());
    match wasm.and_then(|w| w.get("hash")).and_then(Value::as_str) {
        Some(h) if is_hash(h) => Ok(Build { hash: h.to_string(), url: wasm.and_then(|w| w.get("url")).and_then(Value::as_str).filter(|u| !u.is_empty()).map(str::to_string) }),
        Some(_) => Err(format!("module:{id}: its build is not a SHA-256 – build it again")),
        None => Err(unbuilt(id)),
    }
}

/// why a module without a published build is silent
pub fn unbuilt(id: &str) -> String {
    format!("module:{id} has no published build – build it in the Module workspace or the creator, then publish it")
}

/// the modules `bundle` plays (the descriptors it carries), each once
pub fn needed(bundle: &Bundle) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for d in &bundle.modules {
        let name = d.name.to_string();
        if !out.contains(&name) {
            out.push(name);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const H: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn the_answer_names_the_build_and_where_it_is() {
        let b = of("walk", &json!({ "status": "success", "module": { "id": "walk", "wasm": { "hash": H, "bytes": 9, "url": "https://cdn.scutoons.com/modules/x.wasm" } } })).unwrap();
        assert_eq!(b, Build { hash: H.into(), url: Some("https://cdn.scutoons.com/modules/x.wasm".into()) });
        assert_eq!(of("walk", &json!({ "id": "walk", "wasm": { "hash": H, "bytes": 9 } })).unwrap().url, None, "a catalog document has no url");
        assert_eq!(file_name(H), format!("{H}.wasm"));
        assert_eq!(path("walk"), "modules/walk");
    }

    #[test]
    fn a_module_without_a_build_says_why() {
        for doc in [json!({ "module": { "id": "drone", "files": {} } }), json!({ "module": { "id": "drone", "wasm": null } }), json!({ "module": { "id": "drone", "wasm": {} } })] {
            assert_eq!(of("drone", &doc), Err(unbuilt("drone")));
        }
        assert!(unbuilt("drone").starts_with("module:drone has no published build"));
        assert!(of("drone", &json!({ "module": { "wasm": { "hash": "../../etc" } } })).unwrap_err().contains("not a SHA-256"));
        assert!(of("drone", &json!({ "module": { "wasm": { "hash": H.to_uppercase() } } })).is_err(), "the server's form: lowercase hex");
    }

    #[test]
    fn a_bundle_needs_each_module_it_carries_once() {
        let doc = json!({ "id": "walk", "descriptor": { "name": "walk", "ports": [{ "name": "out", "dir": "out", "type": "audio" }] } });
        let (inst, d) = super::super::assign::module_track(&doc).unwrap();
        let (one, _) = Bundle::empty("character.boy").module_track_added(&inst, &d).unwrap();
        let (two, _) = one.track_added(&inst).unwrap();
        assert_eq!(needed(&Bundle::empty("character.boy")), Vec::<String>::new());
        assert_eq!(needed(&two), ["walk"]);
    }
}
