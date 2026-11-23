//! extract-animation (#817) – a shared animation for the asset library out of a character's model: its skeleton and
//! clips, without meshes, materials or images (deck-space `space::animation::extract`). It prints what the stage will
//! read: the skeleton's family, its joints, the joints the first clip turns and the clip's length.
//!
//!   cargo run -p deck-space --bin extract-animation -- catpope.glb catpope-idle.glb
//!
//! Upload the result into an `animations/` folder of the library (e.g. `characters/animations/catpope-idle.glb`): the
//! library lists it as an animation, the creator offers it as an idle to every character whose model fits.
use anyhow::{bail, Context, Result};
use deck_space::space::{animation, glb};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [model, out] = args.as_slice() else {
        bail!("usage: extract-animation MODEL.glb OUT.glb")
    };
    let bytes = std::fs::read(model).with_context(|| format!("reading {model}"))?;
    let anim = animation::extract(&bytes).with_context(|| format!("in {model}"))?;
    let a = glb::skeleton(&anim).context("the extracted file does not read back")?;
    let s = a.skeleton();
    std::fs::write(out, &anim).with_context(|| format!("writing {out}"))?;
    let clips: Vec<String> = a
        .clips
        .iter()
        .map(|c| format!("{:?} {:.2} s", c.name, c.duration))
        .collect();
    println!(
        "{out}: {} bytes · family {} · {} joints · {} turned · clips {}",
        anim.len(),
        s.family.as_deref().unwrap_or("none"),
        s.joints.len(),
        s.animated.len(),
        clips.join(", ")
    );
    Ok(())
}
