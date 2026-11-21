//! A model's bones through the host's raw exports (`hl_bones`): questions and a .glb in, each bone's world matrix out
//! as its bits – so one reference check runs against the native core and the wasm one (tests/wasm.rs).
use deck_space::space::glb;

/// The inbox' form: the questions' length (u32, little endian), the questions – a line each, `bone \t clip \t
/// seconds`, the clip `-` for the rest pose – then the .glb.
pub fn input(questions: &str, glb: &[u8]) -> Vec<u8> {
    let mut out = (questions.len() as u32).to_le_bytes().to_vec();
    out.extend(questions.as_bytes());
    out.extend(glb);
    out
}

/// A line per question: the matrix' 16 numbers column by column as hex bits, or `none` (no such bone or clip).
pub fn answer(input: &[u8]) -> Result<String, String> {
    let split = input.get(..4).map(|n| 4 + u32::from_le_bytes([n[0], n[1], n[2], n[3]]) as usize).filter(|&at| at <= input.len()).ok_or("a truncated question")?;
    let questions = std::str::from_utf8(&input[4..split]).map_err(|e| e.to_string())?;
    let anatomy = glb::anatomy(&input[split..]).map_err(|e| format!("{e:#}"))?;
    let mut lines = Vec::new();
    for q in questions.lines() {
        let mut fields = q.split('\t');
        let (Some(bone), Some(clip), Some(Ok(t))) = (fields.next(), fields.next(), fields.next().map(str::parse::<f32>)) else {
            return Err(format!("not a question: {q}"));
        };
        lines.push(match anatomy.bone_world(bone, Some(clip).filter(|c| *c != "-"), t) {
            Some(m) => m.to_cols_array().map(|x| format!("{:08x}", x.to_bits())).join(" "),
            None => "none".to_string(),
        });
    }
    Ok(lines.join("\n"))
}
