// The input inverted, mixed by a (0 = the input, 1 = inverted).
fn effect(uv: vec2<f32>) -> vec4<f32> {
    let c = textureSample(src, samp, uv);
    return vec4<f32>(mix(c.rgb, 1.0 - c.rgb, clamp(u.p0.x, 0.0, 1.0)), c.a);
}
