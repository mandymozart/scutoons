// Curves (#204): lift / gamma / gain – lift raises the blacks (white stays), gain scales toward white, gamma bends the
// middle (> 1 brighter). One draw, three knobs; a mood's preset later.
@fragment
fn fs(v: VsOut) -> @location(0) vec4<f32> {
    let c = tap(v.uv);
    let lift = post.curves.x;
    let gamma = max(post.curves.y, 0.01);
    let gain = post.curves.z;
    let g = clamp(c * gain + lift * (1.0 - c), vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(pow(g, vec3<f32>(1.0 / gamma)), 1.0);
}
