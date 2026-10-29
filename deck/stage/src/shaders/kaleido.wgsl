// Kaleidoscope over the input. a = segments (0–1 → 3–12), b = spin speed, c = zoom (0–1 → 0.5–2), d = mix with the input.
fn effect(uv: vec2<f32>) -> vec4<f32> {
    let aspect = u.res.x / max(u.res.y, 1.0);
    let p = (uv - 0.5) * vec2<f32>(aspect, 1.0);
    let n = floor(3.0 + clamp(u.p0.x, 0.0, 1.0) * 9.0);
    let seg = 6.28318530718 / n;
    var a = atan2(p.y, p.x) + u.time * u.p0.y;
    a = abs((a - seg * floor(a / seg)) - seg * 0.5);
    let r = length(p) * (0.5 + 1.5 * clamp(u.p0.z, 0.0, 1.0));
    let q = vec2<f32>(cos(a), sin(a)) * r / vec2<f32>(aspect, 1.0) + 0.5;
    let k = textureSample(src, samp, fract(q));
    let o = textureSample(src, samp, uv);
    return mix(k, o, clamp(u.p0.w, 0.0, 1.0));
}
