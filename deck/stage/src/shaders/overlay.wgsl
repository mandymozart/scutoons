// The overlay node (render.rs): `top` (src2) over `in` (src) by its alpha – fitted into the lower part of the frame,
// popping in when it changes, kicked by `pulse`, shaken by `shake`. p0 = opacity, pulse, shake, –; p1 = the top's
// aspect (w / h), its age in s, whether it has a top.
fn effect(uv: vec2<f32>) -> vec4<f32> {
    let base = textureSampleLevel(src, samp, uv, 0.0);
    if (u.p1.z < 0.5) {
        return base;
    }
    let screen = u.res.x / u.res.y;
    let aspect = max(u.p1.x, 0.01);
    let age = u.p1.y;
    // 80 % of the width, at most 40 % of the height
    var w = 0.8;
    var h = w * screen / aspect;
    if (h > 0.4) {
        h = 0.4;
        w = h * aspect / screen;
    }
    let pop = 1.0 + 0.35 * exp(-age * 9.0) + 0.08 * u.p0.y;
    let shake = u.p0.z * 0.012 * vec2<f32>(sin(u.time * 53.0), cos(u.time * 41.0));
    let centre = vec2<f32>(0.5, 0.72) + shake;
    let p = (uv - centre) / (vec2<f32>(w, h) * pop) + 0.5;
    let a = step(vec2<f32>(0.0), p) * step(p, vec2<f32>(1.0));
    let top = textureSampleLevel(src2, samp, clamp(p, vec2<f32>(0.0), vec2<f32>(1.0)), 0.0);
    let k = top.a * a.x * a.y * clamp(u.p0.x, 0.0, 1.0) * clamp(age * 8.0, 0.0, 1.0);
    return vec4<f32>(mix(base.rgb, top.rgb, k), 1.0);
}
