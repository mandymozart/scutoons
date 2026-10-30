// The outlet node (render.rs, live/, #186): the cell's plane – its flat layers, transparent where the backdrop shows –
// over `in` (src: the space node, where a cell has a 3D set). src2 = the current plane, src3 = the previous one; the
// new plane fades in by p0.y, the old one out by p0.z; each shows the backdrop only if its cell had a 3D set (p0.w the
// current, p1.x the previous). p0.x = opacity, p1.y = whether `in` is linked.
fn plane_over(plane: vec4<f32>, bg: vec3<f32>, show_bg: f32) -> vec3<f32> {
    return mix(bg * show_bg, plane.rgb, plane.a);
}

fn effect(uv: vec2<f32>) -> vec4<f32> {
    let bg = textureSampleLevel(src, samp, uv, 0.0).rgb * u.p1.y;
    let cur = plane_over(textureSampleLevel(src2, samp, uv, 0.0), bg, u.p0.w);
    let prev = plane_over(textureSampleLevel(src3, samp, uv, 0.0), bg, u.p1.x);
    let a = clamp(u.p0.y, 0.0, 1.0);
    let gone = clamp(u.p0.z, 0.0, 1.0);
    let col = cur * a + prev * (1.0 - gone) * (1.0 - a);
    return vec4<f32>(col * clamp(u.p0.x, 0.0, 1.0), 1.0);
}
