// Prepended to every pass (render.rs): the uniforms, the input texture, a fullscreen triangle. A pass body defines
// `fn effect(uv: vec2<f32>) -> vec4<f32>`; uv is 0–1 from the top left, u.p0 / u.p1 are the node's eight params.
struct U {
    time: f32,
    dt: f32,
    res: vec2<f32>,
    p0: vec4<f32>,
    p1: vec4<f32>,
};
@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var src: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) i: u32) -> VsOut {
    let x = f32((i << 1u) & 2u);
    let y = f32(i & 2u);
    var o: VsOut;
    o.pos = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    o.uv = vec2<f32>(x, y);
    return o;
}

fn hsv(h: f32, s: f32, v: f32) -> vec3<f32> {
    let k = vec3<f32>(1.0, 2.0 / 3.0, 1.0 / 3.0);
    let p = abs(fract(vec3<f32>(h) + k) * 6.0 - 3.0);
    return v * mix(vec3<f32>(1.0), clamp(p - 1.0, vec3<f32>(0.0), vec3<f32>(1.0)), s);
}

@fragment
fn fs(v: VsOut) -> @location(0) vec4<f32> {
    return effect(v.uv);
}
