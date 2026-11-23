//! cloud-puff – the cloud's default puff model, a low-poly bubble (deck-space `space::bubble`), as a .glb:
//!
//!   cargo run -p deck-space --bin cloud-puff -- cloud-puff.glb
//!
//! Its place in the library is `scutoons/3d/cloud-puff.glb` (the path module:cloud's `puff` slot names in the cloud-storm
//! rig); a model put there instead – the library's Replace – is what a deck draws the puffs with from its next load.
use anyhow::{bail, Context, Result};
use deck_space::space::bubble;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [out] = args.as_slice() else { bail!("usage: cloud-puff OUT.glb") };
    let bytes = bubble::glb();
    std::fs::write(out, &bytes).with_context(|| format!("writing {out}"))?;
    let (corners, faces) = bubble::mesh();
    println!("{out}: {} bytes · {} corners · {} triangles", bytes.len(), corners.len(), faces.len());
    Ok(())
}
