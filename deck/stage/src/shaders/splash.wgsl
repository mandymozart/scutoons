// The splash (render.rs, splash.rs): black with the white logo centred at half the width, over everything; the
// loading line at the top (2 px at 1080p, sweeping – the reader's boot screen line). p0 = alpha, the logo's aspect,
// whether the line runs.
fn effect(uv: vec2<f32>) -> vec4<f32> {
    let screen = u.res.x / u.res.y;
    let w = 0.5;
    let h = w * screen / max(u.p0.y, 0.01);
    let p = (uv - vec2<f32>(0.5)) / vec2<f32>(w, h) + 0.5;
    let inside = step(vec2<f32>(0.0), p) * step(p, vec2<f32>(1.0));
    let logo = textureSampleLevel(src, samp, clamp(p, vec2<f32>(0.0), vec2<f32>(1.0)), 0.0).a * inside.x * inside.y;
    var col = vec3<f32>(logo);
    if (u.p0.z > 0.5 && uv.y * u.res.y < max(u.res.y / 540.0, 1.0)) {
        let x = fract(u.time * 0.6);
        let d = abs(uv.x - x);
        col = max(col, vec3<f32>(step(d, 0.15)));
    }
    return vec4<f32>(col, u.p0.x);
}
