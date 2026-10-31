// Pixelate (#204): blocks of `pixel` render pixels, each the colour at its centre (nearest – textureLoad), then
// optionally quantised to `levels` per channel or snapped to the nearest colour of the scene's `palette`.
@fragment
fn fs(v: VsOut) -> @location(0) vec4<f32> {
    let dim = vec2<f32>(textureDimensions(src));
    let b = max(post.pixel.x, 1.0);
    let centre = (floor(v.uv * dim / b) + 0.5) * b;
    let at = clamp(vec2<i32>(centre), vec2<i32>(0), vec2<i32>(dim) - vec2<i32>(1));
    var c = textureLoad(src, at, 0).rgb;
    if post.pixel.y >= 2.0 {
        let n = floor(post.pixel.y) - 1.0;
        c = floor(c * n + 0.5) / n;
    }
    let count = i32(post.pixel.z);
    if count > 0 {
        var best = c;
        var dist = 1e9;
        for (var i = 0; i < 16; i++) {
            if i >= count {
                break;
            }
            let p = post.palette[i].rgb;
            // weighted like the eye: green counts most
            let d = (c - p) * vec3<f32>(0.3, 0.59, 0.11);
            let e = dot(d, d);
            if e < dist {
                dist = e;
                best = p;
            }
        }
        c = best;
    }
    return vec4<f32>(c, 1.0);
}
