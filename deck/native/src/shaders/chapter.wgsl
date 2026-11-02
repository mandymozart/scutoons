// The chapter node (render.rs): the current slice (src2) fitted to the height, the previous one (src3) fading out,
// the sides from `in` (src) darkened – or, without an input, a dark, stretched copy of the slice. p0 = zoom, pulse,
// the cross-fade 0–1, has an input; p1 = the current / previous slice's aspect (w / h), the slice's age in s, the hold.
fn slice_uv(uv: vec2<f32>, aspect: f32, s: f32) -> vec2<f32> {
    let screen = u.res.x / u.res.y;
    return vec2<f32>((uv.x - 0.5) * screen / max(aspect, 0.01) / s + 0.5, (uv.y - 0.5) / s + 0.5);
}

fn inside(p: vec2<f32>) -> f32 {
    let a = step(vec2<f32>(0.0), p) * step(p, vec2<f32>(1.0));
    return a.x * a.y;
}

fn layer(t: texture_2d<f32>, uv: vec2<f32>, aspect: f32, s: f32) -> vec4<f32> {
    let p = slice_uv(uv, aspect, s);
    let c = textureSampleLevel(t, samp, clamp(p, vec2<f32>(0.0), vec2<f32>(1.0)), 0.0);
    return vec4<f32>(c.rgb, c.a * inside(p));
}

fn sides(uv: vec2<f32>, aspect: f32) -> vec3<f32> {
    if (u.p0.w > 0.5) {
        return textureSampleLevel(src, samp, uv, 0.0).rgb * 0.55;
    }
    // the slice stretched to cover, a few taps for a cheap blur, dark
    let screen = u.res.x / u.res.y;
    let cover = max(screen / max(aspect, 0.01), 1.0);
    let p = vec2<f32>((uv.x - 0.5) / cover * screen / max(aspect, 0.01) * 0.5 + 0.5, (uv.y - 0.5) * 0.5 + 0.5);
    var c = vec3<f32>(0.0);
    for (var i = 0; i < 4; i++) {
        let o = vec2<f32>(f32(i % 2) - 0.5, f32(i / 2) - 0.5) * 0.03;
        c += textureSampleLevel(src2, samp, clamp(p + o, vec2<f32>(0.0), vec2<f32>(1.0)), 0.0).rgb;
    }
    return c * 0.25 * 0.3;
}

fn effect(uv: vec2<f32>) -> vec4<f32> {
    let age = u.p1.z;
    let hold = max(u.p1.w, 0.5);
    // a slow push-in over the hold, a kick on the beat
    let s = max(u.p0.x, 0.1) * (1.0 + 0.05 * clamp(age / hold, 0.0, 1.5)) * (1.0 + 0.06 * u.p0.y);
    let cur = layer(src2, uv, u.p1.x, s);
    let prev = layer(src3, uv, u.p1.y, s * 1.04);
    let t = smoothstep(0.0, 1.0, u.p0.z);
    var col = sides(uv, u.p1.x);
    col = mix(col, prev.rgb, prev.a * (1.0 - t));
    col = mix(col, cur.rgb, cur.a * t);
    // a flash at the cut
    col += vec3<f32>(0.25 * (1.0 - t) * step(0.001, 1.0 - t));
    return vec4<f32>(col, 1.0);
}
