// A video frame on the GPU (video/gpu.rs): its planes into RGB – AV1's YUV 4:2:0, HAP Q's scaled YCoCg in DXT5 blocks –
// and AMD's FidelityFX Super Resolution 1 (MIT): EASU upscales, RCAS sharpens. Each pass draws one triangle over its
// target. The values stay as the video has them (gamma-encoded, what FSR expects) up to the last pass, which decodes
// sRGB when its target encodes it again (`u.out.x`).

struct U {
    // YUV: (a, rv, gu, gv) / 256, (bu / 256, the black level, –, –), 0..255 units
    m0: vec4<f32>,
    m1: vec4<f32>,
    // x: 1 = the target is sRGB (write linear); y: RCAS's sharpness (2^-stops); zw: the target's size
    out: vec4<f32>,
}

@group(0) @binding(0) var t0: texture_2d<f32>;
@group(0) @binding(1) var t1: texture_2d<f32>;
@group(0) @binding(2) var t2: texture_2d<f32>;
@group(0) @binding(3) var samp: sampler;
@group(0) @binding(4) var<uniform> u: U;

struct V {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs(@builtin(vertex_index) i: u32) -> V {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var o: V;
    o.pos = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    o.uv = uv;
    return o;
}

fn written(c: vec3<f32>) -> vec4<f32> {
    let g = clamp(c, vec3<f32>(0.0), vec3<f32>(1.0));
    if u.out.x > 0.5 {
        return vec4<f32>(select(pow((g + 0.055) / 1.055, vec3<f32>(2.4)), g / 12.92, g <= vec3<f32>(0.04045)), 1.0);
    }
    return vec4<f32>(g, 1.0);
}

// 4:2:0: luma at its own size, the chroma planes sampled between their texels (centred)
@fragment
fn fs_yuv(v: V) -> @location(0) vec4<f32> {
    let y = textureSampleLevel(t0, samp, v.uv, 0.0).r * 255.0 - u.m1.y;
    let cb = textureSampleLevel(t1, samp, v.uv, 0.0).r * 255.0 - 128.0;
    let cr = textureSampleLevel(t2, samp, v.uv, 0.0).r * 255.0 - 128.0;
    let l = u.m0.x * y;
    let rgb = vec3<f32>(l + u.m0.y * cr, l - u.m0.z * cb - u.m0.w * cr, l + u.m1.x * cb);
    return written(rgb / 255.0);
}

// HAP Q: Co, Cg in r, g scaled by the block's b (scale = b·255 / 8 + 1), luma in a
@fragment
fn fs_ycocg(v: V) -> @location(0) vec4<f32> {
    let c = textureSampleLevel(t0, samp, v.uv, 0.0) * 255.0;
    let s = 8.0 / (c.b + 8.0);
    let co = (c.r - 128.0) * s;
    let cg = (c.g - 128.0) * s;
    return written(vec3<f32>(c.a + co - cg, c.a + cg, c.a - co - cg) / 255.0);
}

fn tap(p: vec2<i32>) -> vec3<f32> {
    let last = vec2<i32>(textureDimensions(t0)) - vec2<i32>(1);
    return textureLoad(t0, clamp(p, vec2<i32>(0), last), 0).rgb;
}

fn luma(c: vec3<f32>) -> f32 {
    return c.b * 0.5 + (c.r * 0.5 + c.g);
}

// EASU's direction and length from one of the four centre texels' '+' (a above, b c d across, e below), weighted `w`
fn easu_set(dir: ptr<function, vec2<f32>>, len: ptr<function, f32>, w: f32, a: f32, b: f32, c: f32, d: f32, e: f32) {
    let dc = d - c;
    let cb = c - b;
    let dir_x = d - b;
    var len_x = abs(dir_x) / max(max(abs(dc), abs(cb)), 1e-8);
    len_x = clamp(len_x, 0.0, 1.0);
    let ec = e - c;
    let ca = c - a;
    let dir_y = e - a;
    var len_y = abs(dir_y) / max(max(abs(ec), abs(ca)), 1e-8);
    len_y = clamp(len_y, 0.0, 1.0);
    *dir += vec2<f32>(dir_x, dir_y) * w;
    *len += (len_x * len_x + len_y * len_y) * w;
}

// one tap of EASU's lobe: approximated Lanczos 2, stretched along the edge, windowed
fn easu_tap(ac: ptr<function, vec3<f32>>, aw: ptr<function, f32>, off: vec2<f32>, dir: vec2<f32>, len: vec2<f32>, lob: f32, clp: f32, c: vec3<f32>) {
    var v = vec2<f32>(off.x * dir.x + off.y * dir.y, off.x * -dir.y + off.y * dir.x) * len;
    let d2 = min(dot(v, v), clp);
    var wb = 2.0 / 5.0 * d2 - 1.0;
    var wa = lob * d2 - 1.0;
    wb = wb * wb;
    wa = wa * wa;
    wb = 25.0 / 16.0 * wb - (25.0 / 16.0 - 1.0);
    let w = wb * wa;
    *ac += c * w;
    *aw += w;
}

// EASU: the input (t0) to the target's size; 12 taps around the output pixel's place in the input
//     b c
//   e f g h
//   i j k l
//     n o
@fragment
fn fs_easu(v: V) -> @location(0) vec4<f32> {
    let scale = vec2<f32>(textureDimensions(t0)) / u.out.zw;
    var pp = floor(v.pos.xy) * scale + (0.5 * scale - 0.5);
    let fp = floor(pp);
    pp -= fp;
    let o = vec2<i32>(fp);
    let b = tap(o + vec2<i32>(0, -1));
    let c = tap(o + vec2<i32>(1, -1));
    let e = tap(o + vec2<i32>(-1, 0));
    let f = tap(o);
    let g = tap(o + vec2<i32>(1, 0));
    let h = tap(o + vec2<i32>(2, 0));
    let i = tap(o + vec2<i32>(-1, 1));
    let j = tap(o + vec2<i32>(0, 1));
    let k = tap(o + vec2<i32>(1, 1));
    let l = tap(o + vec2<i32>(2, 1));
    let n = tap(o + vec2<i32>(0, 2));
    let oo = tap(o + vec2<i32>(1, 2));
    let bl = luma(b); let cl = luma(c); let el = luma(e); let fl = luma(f); let gl = luma(g); let hl = luma(h);
    let il = luma(i); let jl = luma(j); let kl = luma(k); let ll = luma(l); let nl = luma(n); let ol = luma(oo);
    var dir = vec2<f32>(0.0);
    var len = 0.0;
    easu_set(&dir, &len, (1.0 - pp.x) * (1.0 - pp.y), bl, el, fl, gl, jl);
    easu_set(&dir, &len, pp.x * (1.0 - pp.y), cl, fl, gl, hl, kl);
    easu_set(&dir, &len, (1.0 - pp.x) * pp.y, fl, il, jl, kl, nl);
    easu_set(&dir, &len, pp.x * pp.y, gl, jl, kl, ll, ol);
    let dir2 = dir * dir;
    var dir_r = dir2.x + dir2.y;
    let zero = dir_r < 1.0 / 32768.0;
    dir_r = select(inverseSqrt(dir_r), 1.0, zero);
    dir.x = select(dir.x, 1.0, zero);
    dir *= dir_r;
    len = len * 0.5;
    len *= len;
    let stretch = dot(dir, dir) / max(abs(dir.x), abs(dir.y));
    let len2 = vec2<f32>(1.0 + (stretch - 1.0) * len, 1.0 - 0.5 * len);
    let lob = 0.5 + ((1.0 / 4.0 - 0.04) - 0.5) * len;
    let clp = 1.0 / lob;
    var ac = vec3<f32>(0.0);
    var aw = 0.0;
    easu_tap(&ac, &aw, vec2<f32>(0.0, -1.0) - pp, dir, len2, lob, clp, b);
    easu_tap(&ac, &aw, vec2<f32>(1.0, -1.0) - pp, dir, len2, lob, clp, c);
    easu_tap(&ac, &aw, vec2<f32>(-1.0, 1.0) - pp, dir, len2, lob, clp, i);
    easu_tap(&ac, &aw, vec2<f32>(0.0, 1.0) - pp, dir, len2, lob, clp, j);
    easu_tap(&ac, &aw, vec2<f32>(0.0, 0.0) - pp, dir, len2, lob, clp, f);
    easu_tap(&ac, &aw, vec2<f32>(-1.0, 0.0) - pp, dir, len2, lob, clp, e);
    easu_tap(&ac, &aw, vec2<f32>(1.0, 1.0) - pp, dir, len2, lob, clp, k);
    easu_tap(&ac, &aw, vec2<f32>(2.0, 1.0) - pp, dir, len2, lob, clp, l);
    easu_tap(&ac, &aw, vec2<f32>(2.0, 0.0) - pp, dir, len2, lob, clp, h);
    easu_tap(&ac, &aw, vec2<f32>(1.0, 0.0) - pp, dir, len2, lob, clp, g);
    easu_tap(&ac, &aw, vec2<f32>(1.0, 2.0) - pp, dir, len2, lob, clp, oo);
    easu_tap(&ac, &aw, vec2<f32>(0.0, 2.0) - pp, dir, len2, lob, clp, n);
    // no ringing: within the four centre texels' range
    let lo = min(min(f, g), min(j, k));
    let hi = max(max(f, g), max(j, k));
    return written(min(hi, max(lo, ac / aw)));
}

// RCAS: the most sharpening that does not clip, from the cross around each pixel of t0 (the same size as the target)
@fragment
fn fs_rcas(v: V) -> @location(0) vec4<f32> {
    let p = vec2<i32>(floor(v.pos.xy));
    let b = tap(p + vec2<i32>(0, -1));
    let d = tap(p + vec2<i32>(-1, 0));
    let e = tap(p);
    let f = tap(p + vec2<i32>(1, 0));
    let h = tap(p + vec2<i32>(0, 1));
    let mn = min(min(b, d), min(f, h));
    let mx = max(max(b, d), max(f, h));
    let hit_min = min(mn, e) / (4.0 * mx + 1e-5);
    let hit_max = (vec3<f32>(1.0) - max(mx, e)) / (4.0 * mn - 4.0 - 1e-5);
    let lobe3 = max(-hit_min, hit_max);
    let limit = 0.25 - 1.0 / 16.0;
    let lobe = max(-limit, min(max(lobe3.r, max(lobe3.g, lobe3.b)), 0.0)) * u.out.y;
    return written((lobe * (b + d + f + h) + e) / (4.0 * lobe + 1.0));
}
