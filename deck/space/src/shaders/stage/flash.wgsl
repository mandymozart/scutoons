// The strobe on the beat (#321, the show clock's test): white while a (the tempo's beat phase, 0–1) is below b (the
// flash's share of a beat, 0.08 when unset), a red frame on the bar's first beat (c = the bar's phase), black otherwise.
fn effect(uv: vec2<f32>) -> vec4<f32> {
    let width = select(0.08, u.p0.y, u.p0.y > 0.0);
    let on = step(u.p0.x, width - 0.0001);
    let bar = step(u.p0.z, width * 0.25 - 0.0001);
    let edge = step(0.45, max(abs(uv.x - 0.5), abs(uv.y - 0.5)));
    return vec4<f32>(vec3<f32>(on) * mix(vec3<f32>(1.0), vec3<f32>(1.0, 0.1, 0.1), bar * edge), 1.0);
}