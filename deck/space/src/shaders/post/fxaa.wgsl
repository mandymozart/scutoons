// FXAA (#204): the console variant of Timothy Lottes' FXAA – the luma of the four diagonal neighbours gives the edge's
// direction, two and four taps along it blend the step; the four-tap blend unless it leaves the local luma range.
// One draw, nine taps, no parameters.
const REDUCE_MIN: f32 = 1.0 / 128.0;
const REDUCE_MUL: f32 = 1.0 / 8.0;
const SPAN_MAX: f32 = 8.0;

@fragment
fn fs(v: VsOut) -> @location(0) vec4<f32> {
    let t = texel();
    let m = tap(v.uv);
    let nw = luma(tap(v.uv + vec2<f32>(-t.x, -t.y)));
    let ne = luma(tap(v.uv + vec2<f32>(t.x, -t.y)));
    let sw = luma(tap(v.uv + vec2<f32>(-t.x, t.y)));
    let se = luma(tap(v.uv + vec2<f32>(t.x, t.y)));
    let lm = luma(m);
    let lo = min(lm, min(min(nw, ne), min(sw, se)));
    let hi = max(lm, max(max(nw, ne), max(sw, se)));
    if hi - lo < max(0.0312, hi * 0.125) {
        // no edge here: as it is
        return vec4<f32>(m, 1.0);
    }
    var dir = vec2<f32>(-((nw + ne) - (sw + se)), (nw + sw) - (ne + se));
    let reduce = max((nw + ne + sw + se) * 0.25 * REDUCE_MUL, REDUCE_MIN);
    let rcp = 1.0 / (min(abs(dir.x), abs(dir.y)) + reduce);
    dir = clamp(dir * rcp, vec2<f32>(-SPAN_MAX), vec2<f32>(SPAN_MAX)) * t;
    let a = 0.5 * (tap(v.uv + dir * (1.0 / 3.0 - 0.5)) + tap(v.uv + dir * (2.0 / 3.0 - 0.5)));
    let b = a * 0.5 + 0.25 * (tap(v.uv - dir * 0.5) + tap(v.uv + dir * 0.5));
    let lb = luma(b);
    if lb < lo || lb > hi {
        return vec4<f32>(a, 1.0);
    }
    return vec4<f32>(b, 1.0);
}
