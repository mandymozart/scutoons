//! codecs-ts – the browser's codecs for the core's byte layouts, written from the layout tables (#372, #311):
//!
//!   shared/live-core/src/wire.ts         the boundary's `send` records and the analysis frame (wire.rs)
//!   shared/live-core/src/signals.ts      the device-signal records (signals.rs)
//!   shared/live-core/src/clock-state.ts  the clock's state block, its flags and sources, the MIDI messages that reach the clock
//!                                        (clock.rs `Slot`, `FLAGS`, `READS`, `GROUPS`; deck-core's `Source::ALL`, `CLOCK_STATUS`)
//!
//! `cargo run -p live-core --features codegen --bin codecs-ts` writes them; `-- --check` only compares (exit 1 when a file is
//! stale – `npm run generate -- --check`, part of `npm run check`). The tables sit next to the code that writes the bytes,
//! so the TypeScript has nothing of its own to keep honest; the fixtures (deck/live-core/fixtures) stay as the cross-check
//! both sides run. The shapes of the codecs (a DataView walk, the OSC payload) are the templates beside this file.
use deck_core::analysis::SPECTRUM;
use deck_core::tempo::{Source, CLOCK_STATUS};
use live_core::clock::{self, Gate, How, Read, Slot};
use live_core::layout::{camel, upper, Field, Ty};
use live_core::signals::{self, ArgTy};
use live_core::wire::{self, Mode};
use std::fmt::Write as _;
use std::path::PathBuf;

const WIRE: &str = include_str!("wire.ts.tpl");
const SIGNALS: &str = include_str!("signals.ts.tpl");
const CLOCK_STATE: &str = include_str!("clock-state.ts.tpl");

fn main() {
    let check = std::env::args().any(|a| a == "--check");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let files = [
        ("shared/live-core/src/wire.ts", wire_ts()),
        ("shared/live-core/src/signals.ts", signals_ts()),
        ("shared/live-core/src/clock-state.ts", clock_state_ts()),
    ];
    let mut stale = Vec::new();
    for (path, text) in &files {
        let full = root.join(path);
        let on_disk = std::fs::read_to_string(&full).unwrap_or_default().replace('\r', "");
        if on_disk == *text {
            continue;
        }
        if check {
            stale.push(*path);
        } else {
            std::fs::write(&full, text).unwrap_or_else(|e| panic!("{}: {e}", full.display()));
            println!("wrote {path}");
        }
    }
    if !stale.is_empty() {
        eprintln!("stale: {} – run `npm run generate`", stale.join(", "));
        std::process::exit(1);
    }
}

/// `template` with every `@@KEY@@` replaced; a key the template does not have, or one left over, is a bug here.
fn fill(template: &str, parts: &[(&str, String)]) -> String {
    let mut out = template.to_owned();
    for (key, value) in parts {
        let at = format!("@@{key}@@");
        assert!(out.contains(&at), "the template has no {at}");
        out = out.replace(&at, value);
    }
    assert!(!out.contains("@@"), "a placeholder is left in the template");
    out
}

fn hex(b: u8) -> String {
    format!("{b:#04x}")
}

fn quoted(names: impl IntoIterator<Item = impl AsRef<str>>) -> String {
    names.into_iter().map(|n| format!("'{}'", n.as_ref())).collect::<Vec<_>>().join(", ")
}

/// a DataView read of `f` in a record that starts at the TypeScript expression `at`
fn get(f: &Field, at: &str) -> String {
    let o = if f.at == 0 { at.to_owned() } else { format!("{at} + {}", f.at) };
    match f.ty {
        Ty::U8 => format!("view.getUint8({o})"),
        Ty::U16 => format!("view.getUint16({o}, true)"),
        Ty::U32 => format!("view.getUint32({o}, true)"),
        Ty::F32 => format!("view.getFloat32({o}, true)"),
    }
}

/// the statements that write `value` into `bytes` as `f` (little-endian) in a record that starts at `at`
fn put(f: &Field, at: &str, value: &str) -> String {
    let o = |k: usize| if f.at + k == 0 { at.to_owned() } else { format!("{at} + {}", f.at + k) };
    match f.ty {
        Ty::U8 => format!("\tbytes[{}] = {value};\n", o(0)),
        Ty::U16 => format!("\tbytes[{}] = {value} & 0xff;\n\tbytes[{}] = ({value} >> 8) & 0xff;\n", o(0), o(1)),
        Ty::F32 => format!("\tputF32(bytes, {}, {value});\n", o(0)),
        Ty::U32 => panic!("{}: the page writes no u32 field", f.name),
    }
}

fn field<'a>(fields: &'a [Field], name: &str) -> &'a Field {
    fields.iter().find(|f| f.name == name).unwrap_or_else(|| panic!("no field {name}"))
}

fn wire_ts() -> String {
    let f = &wire::SEND_FIELDS;
    let consts = [
        ("HEADER", wire::HEADER, ""),
        ("OP_SEND", usize::from(wire::OP_SEND), ""),
        ("SPECTRUM", SPECTRUM, "deck-core's analysis::SPECTRUM – bin k = k · rate / 1024 Hz"),
        ("SPECTRUM_AT", wire::SPECTRUM_AT, "the frame's slot of the first spectrum bin (the bands come before it)"),
        ("REDUCTION", wire::REDUCTION, "the frame's slot of the limiter's gain reduction (dB)"),
        ("OUTPUT_COUNT", wire::OUTPUT_COUNT, "… of the number of device channels the engine renders (#282)"),
        ("PEAKS", wire::PEAKS, "… of the first channel's peak"),
        ("MAX_OUTPUTS", wire::MAX_OUTPUTS, "the most channels a frame carries peaks for (deck-engine's device channel limit)"),
        ("VALUE_COUNT", wire::VALUE_COUNT, "… of the number of values that follow"),
        ("VALUES_AT", wire::VALUES_AT, "… of the first value"),
        ("FRAME", wire::FRAME, "the frame's fixed part; the values follow it"),
        ("MAX_FRAME_VALUES", wire::MAX_FRAME_VALUES, "the most values a frame carries (a row past them reads `at` = -1 in the table)"),
        ("MAX_VALUES", wire::MAX_VALUES, "the most values one record carries (u16 n)"),
    ];
    let mut c = String::new();
    for (name, v, doc) in consts {
        if !doc.is_empty() {
            let _ = writeln!(c, "/** {doc} */");
        }
        let _ = writeln!(c, "export const {name} = {v};");
    }
    let modes = format!(
        "export type Mode = {};\nexport const MODES: readonly Mode[] = [{}];",
        Mode::ALL.map(|m| format!("'{}'", m.name())).join(" | "),
        quoted(Mode::ALL.map(Mode::name))
    );
    let mut write = String::new();
    for (name, value) in [("op", "OP_SEND".to_owned()), ("mode", "mode".to_owned()), ("n", "n".to_owned()), ("node", "r.node".to_owned()), ("port", "r.port".to_owned()), ("amount", "r.amount".to_owned())] {
        write.push_str(&put(field(f, name), "at", &value));
    }
    let bands = wire::BANDS;
    fill(
        WIRE,
        &[
            ("FIELDS", f.iter().map(|f| format!("{} @ {}", f.name, f.at)).collect::<Vec<_>>().join(", ")),
            ("BANDS", bands.join(", ")),
            ("CONSTS", c.trim_end().to_owned()),
            ("MODES", modes),
            ("WRITE", write.trim_end().to_owned()),
            ("GET_N", get(field(f, "n"), "at")),
            ("GET_OP", get(field(f, "op"), "at")),
            ("GET_MODE", get(field(f, "mode"), "at")),
            ("GET_NODE", get(field(f, "node"), "at")),
            ("GET_PORT", get(field(f, "port"), "at")),
            ("GET_AMOUNT", get(field(f, "amount"), "at")),
            ("BAND_FIELDS", bands.iter().map(|b| format!("\t{b}: number;")).collect::<Vec<_>>().join("\n")),
            ("BAND_VALUES", bands.iter().enumerate().map(|(i, b)| format!("\t{b}: frame[{i}],")).collect::<Vec<_>>().join("\n")),
        ],
    )
}

fn signals_ts() -> String {
    let f = &signals::FIELDS;
    let kind = |tag: u8| signals::KINDS.iter().find(|(k, _)| *k == tag).map(|(_, n)| *n).expect("a kind");
    let mut c = format!("export const SIGNAL_HEADER = {};\n", signals::HEADER);
    for (tag, name) in signals::KINDS {
        let _ = writeln!(c, "export const KIND_{} = {tag};", upper(name));
    }
    let _ = writeln!(c, "/** midi: the conductor (over the relay) instead of the deck's MIDI out port */\nexport const FLAG_CONDUCTOR = {};", signals::FLAG_CONDUCTOR);
    let _ = writeln!(c, "/** dmx: a flash, sent at once */\nexport const FLAG_FLASH = {};", signals::FLAG_FLASH);
    for (tag, name, _) in signals::OSC_ARGS {
        let _ = writeln!(c, "export const OSC_{} = {tag};", upper(name));
    }
    let ts_type = |t: ArgTy| match t {
        ArgTy::I32 | ArgTy::F32 => "number",
        ArgTy::Str => "string",
        ArgTy::Bool => "boolean",
    };
    let types = signals::OSC_ARGS.iter().map(|(_, name, t)| format!("\t| {{ kind: '{name}'; p: {} }}", ts_type(*t))).collect::<Vec<_>>().join("\n");
    let mut args = String::new();
    for (i, (_, name, t)) in signals::OSC_ARGS.iter().enumerate() {
        let push = format!("args.push({{ kind: '{name}', p: ");
        let lead = if i == 0 { "\t\t" } else { "\t\t} else " };
        let _ = writeln!(args, "{lead}if (tag === OSC_{}) {{", upper(name));
        let body: Vec<String> = match t {
            ArgTy::I32 | ArgTy::F32 => {
                let get = if *t == ArgTy::I32 { "getInt32" } else { "getFloat32" };
                vec!["need(4);".into(), format!("{push}view.{get}(p, true) }});"), "p += 4;".into()]
            }
            ArgTy::Str => vec![
                "need(2);".into(),
                "const l = view.getUint16(p, true);".into(),
                "p += 2;".into(),
                "need(l);".into(),
                format!("{push}utf8.decode(bytes.subarray(p, p + l)) }});"),
                "p += l;".into(),
            ],
            ArgTy::Bool => vec!["need(1);".into(), format!("{push}view.getUint8(p++) !== 0 }});")],
        };
        for l in body {
            let _ = writeln!(args, "\t\t\t{l}");
        }
    }
    fill(
        SIGNALS,
        &[
            ("CONSTS", c.trim_end().to_owned()),
            ("OSC_TYPES", types),
            ("KIND_MIDI", kind(signals::KIND_MIDI).to_owned()),
            ("KIND_DMX", kind(signals::KIND_DMX).to_owned()),
            ("KIND_OSC", kind(signals::KIND_OSC).to_owned()),
            ("GET_KIND", get(field(f, "kind"), "at")),
            ("GET_FLAGS", get(field(f, "flags"), "at")),
            ("GET_N", get(field(f, "n"), "at")),
            ("GET_CH", get(field(f, "ch"), "at")),
            ("GET_AT", get(field(f, "at"), "at")),
            ("OSC_ARGS", args.trim_end().to_owned()),
        ],
    )
}

fn slot_name(s: Slot) -> String {
    let d = format!("{s:?}");
    let mut c = d.chars();
    c.next().map(|f| f.to_lowercase().chain(c).collect()).unwrap_or_default()
}

fn clock_state_ts() -> String {
    let slots = Slot::ALL.iter().map(|&s| format!("\t{}: {}", slot_name(s), s as usize)).collect::<Vec<_>>().join(",\n");
    let flags = clock::FLAGS.iter().map(|(n, b)| format!("\t{}: {b}", camel(n))).collect::<Vec<_>>().join(",\n");
    let slot = |s: Slot| format!("s[SLOT.{}]", slot_name(s));
    let flag = |b: u32| {
        let (n, _) = clock::FLAGS.iter().find(|(_, x)| *x == b).expect("a flag");
        format!("FLAG.{}", camel(n))
    };
    let value = |r: &Read| -> String {
        let s = slot(r.slot);
        match r.how {
            How::Num => s,
            How::NonZero => format!("{s} !== 0"),
            How::Flag(b) => format!("({s} & {}) !== 0", flag(b)),
            How::Source => format!("SOURCES[{s}] ?? SOURCES[0]"),
        }
    };
    let ts_type = |r: &Read| match r.how {
        How::Num => "number",
        How::Source => "ClockSource",
        How::NonZero | How::Flag(_) => "boolean",
    };
    let doc = |indent: &str, text: &str| if text.is_empty() { String::new() } else { format!("{indent}/** {text} */\n") };

    let mut types = String::new();
    for g in &clock::GROUPS {
        let _ = writeln!(types, "/** {} */\nexport interface {} {{", g.ty_doc, g.ty);
        for r in g.reads {
            types.push_str(&doc("\t", r.doc));
            let _ = writeln!(types, "\t{}: {};", camel(r.name), ts_type(r));
        }
        types.push_str("}\n\n");
    }
    types.push_str("export interface ClockState {\n");
    for r in &clock::READS {
        types.push_str(&doc("\t", r.doc));
        let _ = writeln!(types, "\t{}: {};", camel(r.name), ts_type(r));
    }
    for g in &clock::GROUPS {
        types.push_str(&doc("\t", g.doc));
        let _ = writeln!(types, "\t{}: {} | null;", g.name, g.ty);
    }
    types.push_str("\t/** deck-core's line (`Controller::line`: the tempo line, the source's state – the native HUD's words) – the host fills it */\n\tline: string;\n}");

    let mut of = String::new();
    for r in &clock::READS {
        let _ = writeln!(of, "\t\t{}: {},", camel(r.name), value(r));
    }
    for g in &clock::GROUPS {
        let gate = match g.gate {
            Gate::Flag(b) => format!("{} & {}", slot(Slot::Flags), flag(b)),
            Gate::Positive(s) => format!("{} > 0", slot(s)),
        };
        let inner = g.reads.iter().map(|r| format!("{}: {}", camel(r.name), value(r))).collect::<Vec<_>>().join(", ");
        let _ = writeln!(of, "\t\t{}: {gate} ? {{ {inner} }} : null,", g.name);
    }
    fill(
        CLOCK_STATE,
        &[
            ("SOURCES", format!("[{}]", quoted(Source::ALL.map(Source::name)))),
            ("MESSAGES", format!("[{}]", CLOCK_STATUS.map(hex).join(", "))),
            ("STATE", clock::STATE.to_string()),
            ("SLOTS", slots),
            ("FLAGS", flags),
            ("TYPES", types.trim_end().to_owned()),
            ("CLOCK_OF", of.trim_end_matches('\n').to_owned()),
        ],
    )
}
