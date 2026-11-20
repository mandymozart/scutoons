// The video node: its frame (src2) covering the picture, centred, never distorted – p1 = the part of the frame shown
// (deck-core fit::cover inside the asset's crop: u, v, width, height); p0.y = the asset's fade level once a frame is
// there, else 0 (black).
fn effect(uv: vec2<f32>) -> vec4<f32> {
    let c = textureSampleLevel(src2, samp, u.p1.xy + uv * u.p1.zw, 0.0);
    return vec4<f32>(c.rgb, c.a) * u.p0.y;
}
