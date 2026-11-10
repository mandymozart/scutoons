// Fireflies on black (#252, the idle scene): a few dozen soft glowing points wandering slowly around their homes, each
// blinking on its own slow rhythm. Draws on its own (no input). a = brightness (1), b = drift speed (1),
// c = size (1), d = how many (0–1 → 8–48).
fn ff_hash(n: f32) -> f32 {
    return fract(sin(n * 127.1) * 43758.5453);
}

fn effect(uv: vec2<f32>) -> vec4<f32> {
    let aspect = u.res.x / max(u.res.y, 1.0);
    let p = vec2<f32>(uv.x * aspect, uv.y);
    let t = u.time * max(u.p0.y, 0.0);
    let count = i32(8.0 + clamp(u.p0.w, 0.0, 1.0) * 40.0);
    let size = 0.004 * max(u.p0.z, 0.1);
    var col = vec3<f32>(0.0);
    for (var i = 0; i < 48; i = i + 1) {
        if (i >= count) {
            break;
        }
        let f = f32(i);
        let h1 = ff_hash(f + 1.0);
        let h2 = ff_hash(f + 11.3);
        let h3 = ff_hash(f + 23.7);
        let h4 = ff_hash(f + 37.1);
        // a slow wander around a home point: two slow sines per axis, each fly its own
        let home = vec2<f32>((0.05 + 0.9 * h1) * aspect, 0.15 + 0.75 * h2);
        let wander = vec2<f32>(
            sin(t * (0.07 + 0.05 * h3) + h4 * 6.2832) * 0.12 + sin(t * (0.19 + 0.1 * h1) + h2 * 6.2832) * 0.04,
            sin(t * (0.05 + 0.04 * h4) + h1 * 6.2832) * 0.08 + sin(t * (0.23 + 0.1 * h2) + h3 * 6.2832) * 0.03
        );
        // the blink: its own period (3–8 s), lit about a third of it, soft in and out
        let period = 3.0 + 5.0 * h4;
        let ph = fract(u.time / period + h3);
        let blink = smoothstep(0.0, 0.12, ph) * (1.0 - smoothstep(0.25, 0.45, ph));
        let d2 = dot(p - home - wander, p - home - wander);
        // a bright core and a wide faint halo
        let core = size * size / (d2 + size * size * 0.5);
        let halo = 0.12 * (16.0 * size * size) / (d2 + 16.0 * size * size);
        col += hsv(0.15 + 0.06 * h2, 0.7, 1.0) * (core + halo) * blink;
    }
    // a little dither: the halos' dark tails band into rings at 8 bits otherwise
    let dither = (ff_hash(dot(uv * u.res, vec2<f32>(12.9898, 78.233)) + fract(u.time)) - 0.5) / 255.0;
    return vec4<f32>(clamp(col * max(u.p0.x, 0.0) + vec3<f32>(dither), vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
