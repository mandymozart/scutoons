// RGB split + scanlines. a = shift amount (0–1 → up to 3 % of the width), b = scanline strength.
fn effect(uv: vec2<f32>) -> vec4<f32> {
    let s = vec2<f32>(clamp(u.p0.x, 0.0, 1.0) * 0.03, 0.0);
    let r = textureSample(src, samp, uv + s).r;
    let g = textureSample(src, samp, uv).g;
    let b = textureSample(src, samp, uv - s).b;
    let line = 1.0 - clamp(u.p0.y, 0.0, 1.0) * 0.5 * (0.5 + 0.5 * sin(uv.y * u.res.y * 3.14159));
    return vec4<f32>(vec3<f32>(r, g, b) * line, 1.0);
}
