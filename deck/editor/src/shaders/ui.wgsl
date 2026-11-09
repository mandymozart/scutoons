// The proof's one 2D pass (#444): instanced quads in pixels, each with a uv rectangle into the bound texture (the glyph
// atlas, or the stage's texture) and a colour the sample is multiplied with. Premultiplied alpha; the screen is in
// pixels, y down.
struct Screen {
    size: vec2<f32>,
    _pad: vec2<f32>,
};
@group(0) @binding(0) var<uniform> screen: Screen;
@group(0) @binding(1) var tex: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

struct Quad {
    @location(0) rect: vec4<f32>,   // x, y, w, h in pixels
    @location(1) uv: vec4<f32>,     // u0, v0, u1, v1
    @location(2) color: vec4<f32>,
    @location(3) shape: vec4<f32>,  // 0 = the texture; 1 = a fill; 1 + t = an outline t wide; then half w, half h, corner
};

struct Out {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) shape: vec4<f32>,
};

@vertex
fn vs(@builtin(vertex_index) v: u32, q: Quad) -> Out {
    // two triangles over the quad: 0 1 2, 2 1 3 – the corner from the vertex index
    let corner = vec2<f32>(f32((v == 1u) || (v == 3u) || (v == 4u)), f32(v >= 2u && v != 3u));
    let px = q.rect.xy + corner * q.rect.zw;
    let ndc = vec2<f32>(px.x / screen.size.x * 2.0 - 1.0, 1.0 - px.y / screen.size.y * 2.0);
    var o: Out;
    o.pos = vec4<f32>(ndc, 0.0, 1.0);
    o.uv = mix(q.uv.xy, q.uv.zw, corner);
    o.color = q.color;
    o.shape = q.shape;
    return o;
}

@fragment
fn fs(i: Out) -> @location(0) vec4<f32> {
    let s = textureSample(tex, samp, i.uv);
    // a shape: its distance per pixel, exact at any size (uv = pixels from its middle)
    let q = abs(i.uv) - i.shape.yz + vec2<f32>(i.shape.w);
    let d = length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - i.shape.w;
    let aa = max(fwidth(d), 0.001);
    let fill = clamp(0.5 - d / aa, 0.0, 1.0);
    let hole = clamp(0.5 - (d + i.shape.x - 1.0) / aa, 0.0, 1.0);
    let shaped = select(fill, fill - hole, i.shape.x > 1.5);
    let is_shape = i.shape.x > 0.5;
    let alpha = select(s.a, shaped, is_shape);
    let rgb = select(s.rgb, vec3<f32>(1.0), is_shape);
    let a = alpha * i.color.a;
    return vec4<f32>(rgb * i.color.rgb * a, a);
}
