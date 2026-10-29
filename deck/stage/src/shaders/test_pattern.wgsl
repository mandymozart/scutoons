// The test pattern (S1): rings and spokes around the centre, a grid, colour bars at the bottom, a sweep line, a circle
// that breathes with `pulse`. p0 = speed, zoom, hue, pulse.
fn effect(uv: vec2<f32>) -> vec4<f32> {
    let speed = u.p0.x;
    let zoom = max(u.p0.y, 0.05);
    let hue = u.p0.z;
    let pulse = u.p0.w;
    let t = u.time * speed;
    let aspect = u.res.x / max(u.res.y, 1.0);
    let p = (uv - 0.5) * vec2<f32>(aspect, 1.0) * zoom * 2.0;
    let r = length(p);
    let a = atan2(p.y, p.x);
    let rings = 0.5 + 0.5 * sin(r * 12.0 - t * 4.0 + pulse * 6.0);
    let spokes = 0.5 + 0.5 * sin(a * 6.0 + t);
    let grid = step(0.97, fract(p.x * 4.0)) + step(0.97, fract(p.y * 4.0));
    var col = hsv(hue + r * 0.15 + t * 0.05 + spokes * 0.1, 0.8, 0.25 + 0.75 * rings * (0.6 + 0.4 * pulse));
    col = mix(col, vec3<f32>(1.0), clamp(grid, 0.0, 1.0) * 0.35);
    if (uv.y > 0.9) {
        col = hsv(floor(uv.x * 8.0) / 8.0 + hue, 0.9, 0.9);
    }
    col += vec3<f32>(0.6) * smoothstep(0.004, 0.0, abs(uv.x - fract(t * 0.1)));
    col = mix(col, vec3<f32>(1.0), smoothstep(0.012, 0.0, abs(r - (0.6 + 0.3 * pulse))));
    return vec4<f32>(col, 1.0);
}
