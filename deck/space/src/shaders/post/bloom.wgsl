// Bloom (#204): the bright part of the picture (a soft-knee threshold) at half size, blurred down to a quarter and an
// eighth and back up (the dual filter – Marius Bjørge's "dual Kawase": 5 taps down, 8 up, each added onto the level
// above), then laid over the picture (screen blend: it brightens, never clips). Five small draws + the composite.

fn bright(c: vec3<f32>) -> vec3<f32> {
    let t = post.bloom.y;
    let k = max(post.bloom.z, 1e-4);
    let l = max(c.r, max(c.g, c.b));
    var soft = clamp(l - t + k, 0.0, 2.0 * k);
    soft = soft * soft / (4.0 * k);
    return c * (max(soft, l - t) / max(l, 1e-4));
}

// full → half: a 4 × 4 box (four bilinear taps), thresholded
@fragment
fn fs_prefilter(v: VsOut) -> @location(0) vec4<f32> {
    let t = texel();
    let c = tap(v.uv + vec2<f32>(-t.x, -t.y)) + tap(v.uv + vec2<f32>(t.x, -t.y)) + tap(v.uv + vec2<f32>(-t.x, t.y)) + tap(v.uv + t);
    return vec4<f32>(bright(c * 0.25), 1.0);
}

// one level down: the centre and four diagonal taps
@fragment
fn fs_down(v: VsOut) -> @location(0) vec4<f32> {
    let h = texel();
    var c = tap(v.uv) * 4.0;
    c += tap(v.uv - h) + tap(v.uv + h);
    c += tap(v.uv + vec2<f32>(h.x, -h.y)) + tap(v.uv - vec2<f32>(h.x, -h.y));
    return vec4<f32>(c / 8.0, 1.0);
}

// one level up (drawn additively onto the level above): a tent of eight taps
@fragment
fn fs_up(v: VsOut) -> @location(0) vec4<f32> {
    let h = texel() * 0.5;
    var c = tap(v.uv + vec2<f32>(-h.x * 2.0, 0.0)) + tap(v.uv + vec2<f32>(h.x * 2.0, 0.0));
    c += tap(v.uv + vec2<f32>(0.0, -h.y * 2.0)) + tap(v.uv + vec2<f32>(0.0, h.y * 2.0));
    c += (tap(v.uv + vec2<f32>(-h.x, h.y)) + tap(v.uv + vec2<f32>(h.x, h.y)) + tap(v.uv + vec2<f32>(h.x, -h.y)) + tap(v.uv + vec2<f32>(-h.x, -h.y))) * 2.0;
    return vec4<f32>(c / 12.0, 1.0);
}

// the picture + its bloom (src2, half size, the three levels summed)
@fragment
fn fs_composite(v: VsOut) -> @location(0) vec4<f32> {
    let base = tap(v.uv);
    let b = clamp(textureSampleLevel(src2, smp, v.uv, 0.0).rgb * post.bloom.x * 0.5, vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(1.0 - (1.0 - base) * (1.0 - b), 1.0);
}
