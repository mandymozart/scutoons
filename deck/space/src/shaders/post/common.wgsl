// The post stack's prelude (#204, space/post_gpu.rs): prepended to every pass. One uniform for all passes (written
// once a frame), the input (`src`), a second input (bloom's blur for its composite; a 1 × 1 texture elsewhere), a
// linear clamping sampler, the fullscreen triangle. Fragment shaders only – the same passes run on WebGL2. Inputs are
// display values (the space's Rgba8 target), like every pass of the stage. Sample with `textureSampleLevel` /
// `textureLoad`: no derivatives, so a pass may branch freely.
struct Post {
    // bloom: amount, threshold, knee, 0
    bloom: vec4<f32>,
    // curves: lift, gamma, gain, 0
    curves: vec4<f32>,
    // pixelate: block size (px), levels per channel (< 2 = off), palette colours, 0
    pixel: vec4<f32>,
    palette: array<vec4<f32>, 16>,
};
@group(0) @binding(0) var<uniform> post: Post;
@group(0) @binding(1) var src: texture_2d<f32>;
@group(0) @binding(2) var smp: sampler;
@group(0) @binding(3) var src2: texture_2d<f32>;

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

// one texel of the input, in uv
fn texel() -> vec2<f32> {
    return 1.0 / vec2<f32>(textureDimensions(src));
}

fn tap(uv: vec2<f32>) -> vec3<f32> {
    return textureSampleLevel(src, smp, uv, 0.0).rgb;
}

fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.299, 0.587, 0.114));
}
