use super::family::{self, families, version_name, Family, Origin};
use super::shelf::{self, at_home, epoch_ms, home_of, natural, select, Cast, Context, Fit, GroupKind, Grouping, Query, Scope, Shelf, Sort, Special, Use};
use super::{Kind, Preview};
use serde_json::{json, Value};
use std::cmp::Ordering;

fn file(path: &str, extra: Value) -> Value {
    let mut f = json!({ "path": path, "size": 100, "origin": "imported" });
    f.as_object_mut().unwrap().extend(extra.as_object().cloned().unwrap_or_default());
    f
}

fn asset(slug: &str, kind: &str, files: Vec<Value>, extra: Value) -> Value {
    let mut a = json!({ "id": format!("a-{slug}"), "slug": slug, "kind": kind, "title": slug, "tags": [], "version": 1, "usages": 1, "updated_at": null, "renditions": files });
    a.as_object_mut().unwrap().extend(extra.as_object().cloned().unwrap_or_default());
    a
}

fn fam() -> Vec<Family> {
    families(&[
        asset("catpope/s1/c1/s1c1-slice-10", "image", vec![file("catpope/s1/c1/s1c1-slice-10.webp", json!({ "width": 390, "height": 624 }))], json!({})),
        asset(
            "catpope/s1/c1/s1c1-slice-2",
            "image",
            vec![file("catpope/s1/c1/s1c1-slice-2.webp", json!({})), file("catpope/s1/c1/s1c1-slice-2.2x.webp", json!({})), file("catpope/s1/c1/s1c1-slice-2.3x.webp", json!({}))],
            json!({ "usages": 4, "updated_at": "2026-11-02 10:00:00" }),
        ),
        asset("catpope/s1/c1/s1c1-bells", "audio", vec![file("catpope/s1/c1/s1c1-bells.128.mp3", json!({ "origin": "uploaded" }))], json!({ "usages": 0 })),
        asset("catpope/s1/c2/s1c2-slice-1", "image", vec![file("catpope/s1/c2/s1c2-slice-1.webp", json!({}))], json!({})),
        asset("characters/catpope/catpope_avatar-angry", "image", vec![file("characters/catpope/catpope_avatar-angry.webp", json!({ "width": 512, "height": 512 }))], json!({ "usages": 0 })),
        asset("characters/catpope-human/catpope-human_avatar-neutral", "image", vec![file("characters/catpope-human/catpope-human_avatar-neutral.webp", json!({ "width": 500, "height": 520 }))], json!({})),
        asset("catpope/s1/trailer", "video", vec![file("catpope/s1/trailer.720p.mp4", json!({ "width": 1280, "height": 720 }))], json!({})),
        asset("catpope/s1/trailer.en", "other", vec![file("catpope/s1/trailer.en.vtt", json!({}))], json!({})),
        asset("catpope/cover", "image", vec![file("catpope/cover.webp", json!({}))], json!({})),
        asset("scutoons/items/camera", "image", vec![file("scutoons/items/camera.webp", json!({ "origin": "generated" }))], json!({ "updated_at": "2026-11-18T09:00:00Z", "credits": "" })),
    ])
}

fn by_name<'a>(list: &'a [Family], name: &str) -> &'a Family {
    list.iter().find(|f| f.name == name).unwrap_or_else(|| panic!("{name}"))
}

const NOW: &str = "2026-11-20 12:00:00";

fn chapter_one() -> Shelf {
    let mut s = Shelf::new(fam());
    let used = |path: &str, item: &str, part: &str| Use { path: path.into(), item: item.into(), part: part.into(), at: part.into() };
    s.set_context(Context {
        open: Some("chapter:catpope:1".into()),
        toon: Some("catpope".into()),
        by_home: false,
        uses: vec![
            used("catpope/s1/c1/s1c1-slice-2.2x.webp", "chapter:catpope:1", "slice 2"),
            used("characters/catpope/catpope_avatar-angry.webp", "chapter:catpope:1", "slice 2 · speaker"),
            used("catpope/s1/c1/s1c1-slice-10.webp", "chapter:catpope:1", "slice 10"),
        ],
        links: Vec::new(),
        casts: Vec::new(),
    });
    s
}

fn q(scope: Scope) -> Query {
    Query { scope, ..Query::default() }
}

#[test]
fn a_version_name_reads_the_markers_in_use() {
    let n = version_name("catpope/s1/c1/s1c1-key-visual.jpg");
    assert_eq!((n.base.as_str(), n.label.as_str(), n.ext.as_str(), n.kind), ("catpope/s1/c1/s1c1-key-visual", "source", "jpg", Kind::Image));
    assert_eq!(version_name("catpope/s1/c1/k.2x.webp").label, "2x");
    assert_eq!(version_name("catpope/s1/c1/k.480p.mp4").label, "480p");
    assert_eq!(version_name("catpope/s1/c1/k.900w.mp4").label, "900w");
    assert_eq!(version_name("catpope/s1/loop.poster.webp").label, "poster");
    let a = version_name("catpope/s1/c1/bells.128.mp3");
    assert_eq!((a.base.as_str(), a.label.as_str(), a.kind), ("catpope/s1/c1/bells", "128k", Kind::Audio));
    assert_eq!(version_name("catpope/items/v.128.webp").label, "source", "a number is a bitrate only on audio");
    assert_eq!(version_name("catpope/v1.2/x.webp").base, "catpope/v1.2/x", "a dot in a folder is no marker");
    assert_eq!(version_name("characters/catpope/catpope-regular.glb").kind, Kind::Model);
}

#[test]
fn a_video_its_stills_and_its_poster_are_one_family() {
    let list = families(&[
        asset(
            "catpope/s1/c1/kv",
            "video",
            vec![
                file("catpope/s1/c1/kv.480p.mp4", json!({ "width": 854, "height": 480 })),
                file("catpope/s1/c1/kv.1080p.mp4", json!({ "width": 1920, "height": 1080, "duration": 12.0 })),
                file("catpope/s1/c1/kv.720p.mp4", json!({ "width": 1280, "height": 720, "duration": 12.0 })),
            ],
            json!({}),
        ),
        asset("catpope/s1/c1/kv-still", "image", vec![file("catpope/s1/c1/kv.jpg", json!({})), file("catpope/s1/c1/kv.webp", json!({})), file("catpope/s1/c1/kv.2x.webp", json!({}))], json!({ "usages": 2, "version": 3 })),
    ]);
    assert_eq!(list.len(), 1);
    let f = &list[0];
    assert_eq!((f.kind, f.name.as_str(), f.folder.as_str()), (Kind::Video, "kv", "catpope/s1/c1"));
    assert_eq!(f.versions.iter().map(|v| v.label.as_str()).collect::<Vec<_>>(), ["source", "1080p", "720p", "480p", "1x", "2x"]);
    assert_eq!((f.assets.len(), f.usages, f.duration, f.version), (2, 3, Some(12.0), 3));
    assert_eq!(f.main, "catpope/s1/c1/kv.720p.mp4");
    assert_eq!(f.preview, Preview::Picture { url: "catpope/s1/c1/kv.webp".into() }, "the 1x before the JPEG master");
    assert_eq!(f.missing, ["poster"]);
}

#[test]
fn the_expected_versions_follow_the_kind_and_place() {
    let one = |a: Value| families(&[a]).remove(0);
    assert_eq!(one(asset("t/v", "video", vec![file("t/v.mp4", json!({ "width": 1280, "height": 720 }))], json!({}))).missing, ["480p", "720p", "poster"]);
    assert_eq!(one(asset("t/s1/c1/a", "audio", vec![file("t/s1/c1/a.128.mp3", json!({})), file("t/s1/c1/a.320.mp3", json!({}))], json!({}))).missing, ["96k"]);
    assert_eq!(one(asset("t/s1/c2/sl", "image", vec![file("t/s1/c2/sl.webp", json!({}))], json!({}))).missing, ["2x", "3x"]);
    assert!(one(asset("t/vectors/dot", "image", vec![file("t/vectors/dot.svg", json!({}))], json!({}))).missing.is_empty());
}

#[test]
fn previews_and_who_a_family_is_made_for() {
    let list = families(&[asset("characters/catpope/catpope-regular", "model", vec![file("characters/catpope/catpope-regular.glb", json!({}))], json!({}))]);
    assert_eq!(list[0].preview, Preview::Still { key: "asset:characters/catpope/catpope-regular.glb".into() });
    assert_eq!((list[0].made_for.as_slice(), list[0].kind), (["catpope".to_string()].as_slice(), Kind::Model));
    let f = fam();
    assert_eq!(by_name(&f, "s1c1-slice-10").preview, Preview::Picture { url: "catpope/s1/c1/s1c1-slice-10.webp".into() });
    assert_eq!(by_name(&f, "s1c1-bells").preview, Preview::Kind);
}

#[test]
fn credits_are_known_only_when_the_answer_carries_them() {
    let f = fam();
    assert_eq!(by_name(&f, "camera").credits, Some(false));
    assert_eq!(by_name(&f, "cover").credits, None);
}

#[test]
fn subtitles_stand_in_their_videos_frame_and_links_make_frames() {
    let mut s = Shelf::new(fam());
    assert_eq!(s.frames().len(), 1);
    assert_eq!((s.frames()[0].main.as_str(), s.frames()[0].kind, s.frames()[0].parts.clone()), ("catpope/s1/trailer", Kind::Video, vec!["catpope/s1/trailer.en".to_string()]));
    s.set_context(Context { links: vec![("catpope/cover.webp".into(), "catpope/s1/c2/s1c2-slice-1.webp".into())], ..Context::default() });
    assert_eq!(s.frames().len(), 2);
    let v = s.view(&Query { scope: Scope::All, grouping: Grouping::None, ..q(Scope::All) });
    let cells = &v.groups[0].cells;
    let framed: Vec<_> = cells.iter().filter(|c| c.frame.is_some()).map(|c| c.keys.clone()).collect();
    assert!(framed.contains(&vec!["catpope/s1/trailer".to_string(), "catpope/s1/trailer.en".to_string()]), "{framed:?}");
    assert!(!cells.iter().any(|c| c.frame.is_none() && c.keys == ["catpope/s1/trailer.en"]), "a part stands only in its frame");
    let single = s.view(&Query { single: true, grouping: Grouping::None, ..q(Scope::All) });
    assert!(single.groups[0].cells.iter().all(|c| c.frame.is_none() && c.keys.len() == 1), "single levels: no frames");
}

#[test]
fn single_levels_show_every_file_on_its_own() {
    let s = Shelf::new(fam());
    let stacked = s.view(&q(Scope::All)).total;
    let single = s.view(&Query { single: true, ..q(Scope::All) });
    assert_eq!(stacked, 10);
    assert_eq!(single.total, 12, "slice 2's 2x and 3x stand alone");
    assert!(single.order.contains(&"catpope/s1/c1/s1c1-slice-2.3x.webp".to_string()));
}

#[test]
fn the_context_is_what_the_open_item_uses_wherever_it_lies() {
    let s = chapter_one();
    let v = s.view(&q(Scope::Context));
    let mut shown = v.order.clone();
    shown.sort();
    assert_eq!(shown, ["catpope/s1/c1/s1c1-slice-10", "catpope/s1/c1/s1c1-slice-2", "characters/catpope/catpope_avatar-angry"], "the portrait it uses is in, the unused bells in its folder are not");
    assert_eq!((v.scopes.context, v.scopes.toon, v.scopes.all, v.scopes.unused), (3, 8, 10, 1), "the toon holds the portrait its chapter uses");
    assert_eq!(v.groups.len(), 1);
    assert!(v.groups[0].pinned && v.groups[0].what == GroupKind::Open && !v.groups[0].folded);
    assert_eq!(s.uses("catpope/s1/c1/s1c1-slice-2").iter().map(|u| u.part.as_str()).collect::<Vec<_>>(), ["slice 2"]);
}

#[test]
fn without_an_index_the_context_is_the_items_folder() {
    let mut s = Shelf::new(fam());
    s.set_context(Context { open: Some("character:catpope".into()), by_home: true, ..Context::default() });
    let mut shown = s.view(&q(Scope::Context)).order;
    shown.sort();
    assert_eq!(shown, ["characters/catpope-human/catpope-human_avatar-neutral", "characters/catpope/catpope_avatar-angry"]);
    assert!(at_home("season:catpope:1", "catpope/s1/c2/s1c2-slice-1.webp"));
    assert!(at_home("storyboard:catpope:1", "catpope/s1/c1/x.webp"));
    assert!(at_home("toon:catpope", "catpope/cover.webp"));
    assert!(!at_home("chapter:catpope:1", "catpope/s1/c2/s1c2-slice-1.webp"));
}

#[test]
fn a_files_home_is_its_folders_item() {
    assert_eq!(home_of("catpope/s1/c3/x").as_deref(), Some("chapter:catpope:3"));
    assert_eq!(home_of("catpope/s2/trailer").as_deref(), Some("season:catpope:2"));
    assert_eq!(home_of("characters/catpope-biker/x").as_deref(), Some("character:catpope"));
    assert_eq!(home_of("scutoons/items/camera").as_deref(), Some("catalog"));
    assert_eq!(home_of("catpope/map/tile").as_deref(), Some("map"));
    assert_eq!(home_of("catpope/cover").as_deref(), Some("toon:catpope"));
    assert_eq!(home_of("shared/x"), None);
    assert_eq!(home_of("loose"), None);
}

#[test]
fn all_pins_the_open_item_then_the_unused_then_one_group_per_item_then_the_toon() {
    let s = chapter_one();
    let v = s.view(&q(Scope::All));
    let groups: Vec<(&str, bool, bool, usize)> = v.groups.iter().map(|g| (g.key.as_str(), g.pinned, g.folded, g.count)).collect();
    assert_eq!(
        groups,
        [
            ("open", true, false, 3),
            ("unused", false, true, 1),
            ("item:catalog", false, true, 1),
            ("item:chapter:catpope:2", false, true, 1),
            ("item:character:catpope", false, true, 1),
            ("item:season:catpope:1", false, true, 2),
            ("toon", false, true, 1),
        ]
    );
    let searched = s.view(&Query { query: "slice".into(), ..q(Scope::All) });
    assert!(searched.groups.iter().all(|g| !g.folded), "a search unfolds them all");
    let mut home = Shelf::new(fam());
    home.set_context(Context::default());
    let v = home.view(&q(Scope::All));
    assert!(!v.groups[0].folded && v.groups[1..].iter().all(|g| g.folded), "nothing pinned: the first group starts open");
}

#[test]
fn grouping_by_kind_or_none_and_the_unused_scope() {
    let s = chapter_one();
    let by_kind = s.view(&Query { grouping: Grouping::Kind, ..q(Scope::All) });
    assert_eq!(by_kind.groups.iter().map(|g| (g.key.as_str(), g.count)).collect::<Vec<_>>(), [("kind:image", 7), ("kind:audio", 1), ("kind:video", 1), ("kind:file", 1)]);
    let none = s.view(&Query { grouping: Grouping::None, ..q(Scope::All) });
    assert_eq!((none.groups.len(), none.groups[0].count, none.groups[0].folded), (1, 10, false));
    let unused = s.view(&q(Scope::Unused));
    assert_eq!((unused.groups.len(), unused.groups[0].what.clone()), (1, GroupKind::Unused));
    assert_eq!(unused.order, ["catpope/s1/c1/s1c1-bells"], "the portrait the chapter uses is not unused");
}

#[test]
fn the_sorts_natural_newest_biggest_most_used() {
    let s = chapter_one();
    let order = |sort: Sort| s.view(&Query { sort, ..q(Scope::Context) }).order;
    assert_eq!(order(Sort::Name)[..2], ["catpope/s1/c1/s1c1-slice-2".to_string(), "catpope/s1/c1/s1c1-slice-10".to_string()]);
    assert_eq!(order(Sort::Recent)[0], "catpope/s1/c1/s1c1-slice-2");
    assert_eq!(order(Sort::Size)[0], "catpope/s1/c1/s1c1-slice-2");
    assert_eq!(order(Sort::Uses)[0], "catpope/s1/c1/s1c1-slice-2");
    assert_eq!(natural("slice 2", "slice 10"), Ordering::Less);
    assert_eq!(natural("Slice-02", "slice-2x"), Ordering::Less);
}

#[test]
fn the_places_credits_and_items() {
    let s = chapter_one();
    let v = s.view(&q(Scope::All));
    assert_eq!(v.specials.credits, Some(1));
    assert_eq!(v.specials.missing, 4, "slice 1 of chapter 2, slice 10, the bells, the trailer");
    assert_eq!(s.view(&Query { special: Some(Special::Missing), ..q(Scope::All) }).total, 4);
    assert!(v.specials.items.contains(&("chapter:catpope:1".to_string(), 3)), "{:?}", v.specials.items);
    let item = s.view(&Query { special: Some(Special::Item("character:catpope".into())), ..q(Scope::All) });
    assert_eq!(item.total, 2);
    assert_eq!(s.view(&Query { origin: Some(Origin::Generated), ..q(Scope::All) }).order, ["scutoons/items/camera"]);
    assert_eq!(epoch_ms("1970-01-02"), Some(86_400_000.0));
    assert_eq!(epoch_ms("2026-11-20T12:00:00Z"), epoch_ms(NOW));
}

#[test]
fn kinds_and_search() {
    let s = chapter_one();
    let v = s.view(&Query { query: "slice 2".into(), ..q(Scope::All) });
    assert_eq!(v.total, 2);
    assert_eq!(s.view(&Query { query: "3x".into(), ..q(Scope::All) }).order, ["catpope/s1/c1/s1c1-slice-2"]);
    let kinds = s.view(&q(Scope::All)).kinds;
    assert_eq!(kinds, [(Kind::Image, 7), (Kind::Audio, 1), (Kind::Video, 1), (Kind::File, 1)]);
    assert_eq!(s.view(&Query { kind: Some(Kind::Audio), ..q(Scope::All) }).total, 1);
}

#[test]
fn a_picker_shows_only_what_fits_and_says_why_not() {
    let s = chapter_one();
    let portrait = Fit { kinds: vec![Kind::Image], square: true };
    let v = s.view(&Query { fit: Some(portrait.clone()), ..q(Scope::All) });
    assert!(v.order.contains(&"characters/catpope/catpope_avatar-angry".to_string()));
    assert!(!v.order.iter().any(|k| k.contains("neutral") || k.contains("bells") || k.contains("slice-10")), "{:?}", v.order);
    let f = fam();
    assert!(portrait.takes(by_name(&f, "s1c2-slice-1")).is_ok(), "unknown sizes fit");
    let not_square = portrait.takes(by_name(&f, "catpope-human_avatar-neutral")).unwrap_err();
    assert!(not_square.square && not_square.got == Kind::Image);
    let wrong = portrait.takes(by_name(&f, "s1c1-bells")).unwrap_err();
    assert!(!wrong.square && wrong.got == Kind::Audio && wrong.wants == [Kind::Image]);
    assert_eq!(portrait.start_kind(), Some(Kind::Image));
    assert_eq!(Fit { kinds: vec![Kind::Image, Kind::Video], square: false }.start_kind(), None);
}

#[test]
fn selection_click_ctrl_and_shift_in_the_order_shown() {
    let order: Vec<String> = ["a", "b", "c", "d"].map(String::from).to_vec();
    let one = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert_eq!(select(&[], None, "b", &order, false, false), (one(&["b"]), Some("b".into())));
    assert_eq!(select(&one(&["b"]), Some("b"), "d", &order, false, true), (one(&["b", "d"]), Some("d".into())));
    assert_eq!(select(&one(&["b", "d"]), Some("d"), "b", &order, false, true), (one(&["d"]), Some("b".into())));
    assert_eq!(select(&one(&["b"]), Some("b"), "d", &order, true, false), (one(&["b", "c", "d"]), Some("b".into())));
    assert_eq!(select(&one(&["c"]), Some("c"), "a", &order, true, false).0, one(&["c", "a", "b"]));
}

#[test]
fn an_upload_lands_in_the_slots_path_or_the_open_items_folder() {
    let mut s = chapter_one();
    assert_eq!(s.upload_path("My Cat (1).PNG", None).as_deref(), Some("catpope/s1/c1/my-cat--1-.png"), "a chapter's folder is where its files are");
    assert_eq!(s.upload_path("x.webp", Some("characters/catpope/catpope_avatar-happy.png")).as_deref(), Some("characters/catpope/catpope_avatar-happy.webp"));
    s.set_context(Context { open: Some("character:yellow_mouse".into()), ..Context::default() });
    assert_eq!(s.upload_folder().as_deref(), Some("characters/yellow_mouse/"));
    s.set_context(Context { open: Some("chapter:catpope:9".into()), toon: Some("catpope".into()), ..Context::default() });
    assert_eq!(s.upload_folder().as_deref(), Some("catpope/"), "a chapter without files: the toon");
    s.set_context(Context::default());
    assert_eq!(s.upload_path("x.webp", None), None);
    assert_eq!(s.upload_path_json(r#"{"name":"a.mp3","slot":"t/s1/c1/a.128.mp3"}"#), "\"t/s1/c1/a.128.mp3\"");
}

#[test]
fn the_wire_takes_and_gives_json() {
    let answer = json!({ "items": [asset("catpope/s1/c1/x", "image", vec![file("catpope/s1/c1/x.webp", json!({ "width": 10, "height": 10 }))], json!({ "usages": 0 }))] }).to_string();
    let mut s = Shelf::from_json(&answer).unwrap();
    let fams: Value = serde_json::from_str(&s.families_json(false)).unwrap();
    assert_eq!(fams[0]["kind"], "image");
    assert_eq!(fams[0]["origin"], "imported");
    assert_eq!(fams[0]["preview"], json!({ "picture": { "url": "catpope/s1/c1/x.webp" } }));
    s.context_json(&json!({ "open": "chapter:catpope:1", "toon": "catpope", "uses": [{ "path": "catpope/s1/c1/x.webp", "item": "chapter:catpope:1", "part": "slice 1", "at": "/slices/0" }] }).to_string()).unwrap();
    let v: Value = serde_json::from_str(&s.view_json(&json!({ "scope": "context", "now": 0 }).to_string()).unwrap()).unwrap();
    assert_eq!(v["groups"][0]["what"], "open");
    assert_eq!(v["groups"][0]["cells"][0]["keys"][0], "catpope/s1/c1/x");
    assert_eq!(v["kinds"], json!([["image", 1]]));
    assert_eq!(v["scopes"]["unused"], 0, "the index's use counts");
    let uses: Value = serde_json::from_str(&s.uses_json("catpope/s1/c1/x")).unwrap();
    assert_eq!(uses[0]["at"], "/slices/0");
    assert_eq!(serde_json::from_str::<Value>(&s.takes_json("catpope/s1/c1/x.webp", r#"{"kinds":["audio"]}"#)).unwrap(), json!({ "ok": false, "wants": ["audio"], "got": "image", "square": false }));
    assert_eq!(serde_json::from_str::<Value>(&s.takes_json("catpope/s1/c1/x.webp", r#"{"kinds":["image"],"square":true}"#)).unwrap(), json!({ "ok": true }));
    assert_eq!(shelf::start_kind_json(r#"{"kinds":["audio"]}"#), "\"audio\"");
    assert_eq!(serde_json::from_str::<Value>(&shelf::select_json(r#"{"chosen":["a"],"anchor":"a","id":"c","order":["a","b","c"],"shift":true}"#).unwrap()).unwrap(), json!({ "chosen": ["a", "b", "c"], "anchor": "a" }));
    assert!(Shelf::from_json("{}").is_err());
    assert_eq!(family::kind_of_asset("archive"), Kind::File);
}

/// the shelf's kind of a registry asset is the server's (server/schema/api/library.vectors.json, the file cases every
/// reader holds to)
#[test]
fn the_registry_kind_is_the_servers_file_kind() {
    let file: Value = serde_json::from_str(include_str!("../../../../server/schema/api/library.vectors.json")).unwrap();
    for case in file["cases"].as_array().unwrap() {
        let kind = family::kind_of_asset(case["asset"]["kind"].as_str().unwrap());
        assert_eq!(kind.id(), case["expect"]["kind"].as_str().unwrap(), "{}", case["what"]);
    }
}

#[test]
fn a_family_is_found_by_any_version() {
    let shelf = Shelf::new(fam());
    let f = shelf.family_of("catpope/s1/c1/s1c1-slice-2.3x.webp").expect("by a version's path");
    assert_eq!((f.name.as_str(), f.versions.len()), ("s1c1-slice-2", 3));
    assert_eq!(shelf.family_of("catpope/s1/c1/s1c1-slice-2").map(|f| f.versions.len()), Some(3), "by its key");
    assert!(Kind::Image.is_file() && !Kind::Module.is_file());
}

#[test]
fn a_misfit_says_why_beside_the_purposes_sentence() {
    use super::Purpose;
    let f = fam();
    let avatar = by_name(&f, "catpope_avatar-angry");
    let bells = by_name(&f, "s1c1-bells");
    assert_eq!(Purpose::Scene.takes_family(avatar), Ok(()));
    assert_eq!(Purpose::Scene.takes_family(bells), Err("audio cannot be dropped on a scene: its slots take characters and items, its layers pictures, the Modules of its camera and members modules".into()), "the purpose's sentence first");
    let wide = Fit { kinds: vec![Kind::Image], square: true };
    let slice = by_name(&f, "s1c1-slice-10");
    assert_eq!(wide.takes(slice).unwrap_err().sentence(), "a square picture fits here: this one is not square");
    assert_eq!(Fit { kinds: vec![Kind::Video, Kind::Image], square: false }.takes(bells).unwrap_err().sentence(), "video or images fit here, not audio");
    assert!(Purpose::Build.takes_family(avatar).is_err() && Purpose::Build.fit().is_none(), "a rig's tracks take no file");
}

#[test]
fn a_workspace_counts_only_the_kinds_it_lists() {
    let s = chapter_one();
    let every = s.view(&q(Scope::All));
    let pictures = s.view(&Query { within: vec![Kind::Image], ..q(Scope::All) });
    assert!(every.order.iter().any(|k| k.ends_with("s1c1-bells")));
    assert!(!pictures.order.iter().any(|k| k.ends_with("s1c1-bells")), "no audio in a pictures' workspace");
    assert_eq!(pictures.scopes.all, s.families(false).iter().filter(|f| f.kind == Kind::Image).count(), "the rest is not counted either");
    assert_eq!(pictures.scopes.context, every.scopes.context);
    assert!(pictures.groups[0].pinned && pictures.groups.iter().skip(1).all(|g| g.folded), "All: the open item pinned, the rest folded");
    assert_eq!(shelf::query_of(&json!({ "within": ["image"] })).within, [Kind::Image]);
}

#[test]
fn a_frames_parts_come_with_their_main_into_a_workspace_that_lists_its_kind() {
    let s = Shelf::new(fam());
    let videos = s.view(&Query { within: vec![Kind::Video], grouping: Grouping::None, ..q(Scope::All) });
    assert_eq!(videos.groups[0].cells.len(), 1);
    assert_eq!(videos.groups[0].cells[0].keys, ["catpope/s1/trailer", "catpope/s1/trailer.en"], "the subtitles stand in the video's frame");
    let pictures = s.view(&Query { within: vec![Kind::Image], ..q(Scope::All) });
    assert!(!pictures.order.iter().any(|k| k.starts_with("catpope/s1/trailer")), "no video listed: nor its parts");
    let single = s.view(&Query { within: vec![Kind::Video], single: true, ..q(Scope::All) });
    assert_eq!(single.order, ["catpope/s1/trailer.720p.mp4"], "single levels have no frames");
}

#[test]
fn a_file_is_found_in_its_family_or_on_its_own_as_the_levels_show() {
    let s = Shelf::new(fam());
    let path = "catpope/s1/c1/s1c1-slice-2.2x.webp";
    assert_eq!(s.family_in(path, false).map(|f| f.versions.len()), Some(3), "stacked: the family of three");
    let one = s.family_in(path, true).expect("single: the file on its own");
    assert_eq!((one.key.as_str(), one.main.as_str(), one.versions.len()), (path, path, 1));
    assert!(s.family_in("catpope/s1/c1/s1c1-slice-2", true).is_none(), "a family's key is no file");
}

#[test]
fn a_cast_is_a_facet_of_its_characters_files() {
    let mut s = Shelf::new(fam());
    let cast = |address: &str, characters: &[&str]| Cast { address: address.into(), characters: characters.iter().map(|c| c.to_string()).collect() };
    s.set_context(Context { casts: vec![cast("season:catpope:1", &["catpope"]), cast("season:catpope:2", &["yellow_mouse"])], ..Context::default() });
    let v = s.view(&q(Scope::All));
    assert_eq!(v.specials.casts, [("season:catpope:1".to_string(), 2)], "a cast without files is not offered");
    let season = s.view(&Query { special: Some(Special::Cast("season:catpope:1".into())), ..q(Scope::All) });
    assert_eq!(season.total, 2, "the character's and its persona's files: {:?}", season.order);
    assert_eq!(s.view(&Query { special: Some(Special::Cast("season:other:1".into())), ..q(Scope::All) }).total, 0);
    let parsed = shelf::query_of(&json!({ "special": "cast:season:catpope:1" }));
    assert_eq!(parsed.special, Some(Special::Cast("season:catpope:1".into())));
    let context = shelf::context_of(&json!({ "casts": [{ "address": "season:catpope:1", "characters": ["catpope", "boy"] }] }));
    assert_eq!(context.casts, [cast("season:catpope:1", &["catpope", "boy"])]);
}
