// The input as it is (the screen's blit, the HUD overlay, the fallback for a shader that does not compile).
fn effect(uv: vec2<f32>) -> vec4<f32> {
    return textureSample(src, samp, uv);
}
