// The space node (#188): the room unlit (its texture × the baked vertex light), the being's spheres (solid nuclei and
// buttons, translucent core / shells / bubbles) and its tubes (hoses, tendrils). Linear maths, sRGB out (the offscreen
// target is plain Rgba8: display values, like every other pass).

struct Frame {
    view_proj: mat4x4<f32>,
    // xyz = the eye
    eye: vec4<f32>,
    // time, the room's light, the being's glow, its hue
    look: vec4<f32>,
    // rgb = the fog's colour, a = its density per metre
    fog: vec4<f32>,
};

@group(0) @binding(0) var<uniform> frame: Frame;
@group(1) @binding(0) var room_tex: texture_2d<f32>;
@group(1) @binding(1) var room_smp: sampler;

fn to_srgb(c: vec3<f32>) -> vec3<f32> {
    return pow(max(c, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.2));
}

fn fogged(c: vec3<f32>, world: vec3<f32>) -> vec3<f32> {
    let d = distance(world, frame.eye.xyz);
    return mix(frame.fog.rgb, c, exp(-d * frame.fog.a));
}

fn hsv(h: f32, s: f32, v: f32) -> vec3<f32> {
    let k = (vec3<f32>(5.0, 3.0, 1.0) + fract(h) * 6.0) % 6.0;
    return v - v * s * clamp(min(k, 4.0 - k), vec3<f32>(0.0), vec3<f32>(1.0));
}

// ---- the room ----

struct RoomOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) world: vec3<f32>,
};

@vertex
fn vs_room(@location(0) pos: vec3<f32>, @location(1) color: vec4<f32>, @location(2) uv: vec2<f32>) -> RoomOut {
    var o: RoomOut;
    o.clip = frame.view_proj * vec4<f32>(pos, 1.0);
    o.color = color;
    o.uv = uv;
    o.world = pos;
    return o;
}

@fragment
fn fs_room(i: RoomOut) -> @location(0) vec4<f32> {
    let tex = textureSample(room_tex, room_smp, i.uv);
    let lit = tex.rgb * i.color.rgb * frame.look.y;
    return vec4<f32>(to_srgb(fogged(lit, i.world)), 1.0);
}

// ---- the spheres (one unit sphere, instanced) ----

struct SphereOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) world: vec3<f32>,
    @location(2) color: vec4<f32>,
    // kind, glow, hover, value
    @location(3) extra: vec4<f32>,
    @location(4) local: vec3<f32>,
};

@vertex
fn vs_sphere(
    @location(0) unit: vec3<f32>,
    @location(1) center: vec3<f32>,
    @location(2) radius: f32,
    @location(3) color: vec4<f32>,
    @location(4) extra: vec4<f32>,
) -> SphereOut {
    var o: SphereOut;
    let world = center + unit * radius;
    o.clip = frame.view_proj * vec4<f32>(world, 1.0);
    o.normal = unit;
    o.world = world;
    o.color = color;
    o.extra = extra;
    o.local = unit;
    return o;
}

fn fresnel(n: vec3<f32>, world: vec3<f32>) -> f32 {
    let v = normalize(frame.eye.xyz - world);
    return pow(1.0 - abs(dot(normalize(n), v)), 2.0);
}

// nuclei and buttons: solid, a soft key light from above, a rim, lit up by the glow
@fragment
fn fs_solid(i: SphereOut) -> @location(0) vec4<f32> {
    let n = normalize(i.normal);
    let key = 0.45 + 0.55 * max(dot(n, normalize(vec3<f32>(0.3, 1.0, 0.4))), 0.0);
    let rim = fresnel(n, i.world);
    var c = i.color.rgb * key;
    if i.extra.x > 0.5 {
        // a button: its value as a bright band rising from the bottom (a level you read at a glance), a linked one
        // (alpha 1) with a dark collar – it can't be turned
        let level = step(i.local.y * 0.5 + 0.5, i.extra.w);
        c = c * (0.35 + 0.9 * level);
        if i.color.a > 0.5 && abs(i.local.y) < 0.18 {
            c = c * 0.3;
        }
    }
    c = c + i.color.rgb * rim * (0.4 + 0.6 * i.extra.y);
    // hover: lit up with a white rim, dragging: brighter still
    c = mix(c, vec3<f32>(1.0), 0.25 * i.extra.z) + vec3<f32>(1.0) * rim * i.extra.z * 0.9;
    return vec4<f32>(to_srgb(fogged(c, i.world)), 1.0);
}

// core, shells, bubbles: additive ghosts – the rim glows, the middle stays clear
@fragment
fn fs_ghost(i: SphereOut) -> @location(0) vec4<f32> {
    let f = fresnel(i.normal, i.world);
    let glow = 0.35 + 0.65 * i.extra.y;
    var a = i.color.a * (0.12 + 0.88 * f) * glow;
    if i.extra.x > 3.5 {
        // the core: a slow inner shimmer
        a = a * (0.8 + 0.2 * sin(frame.look.x * 1.7 + i.local.y * 6.0));
    } else if i.extra.x > 2.5 {
        // a bubble: water – a bright rim, a highlight
        let spec = pow(max(dot(normalize(i.normal), normalize(vec3<f32>(0.4, 0.8, 0.45))), 0.0), 24.0);
        a = i.color.a * (0.25 + 0.75 * f) * glow + spec;
    }
    let c = to_srgb(i.color.rgb) * a;
    let fade = exp(-distance(i.world, frame.eye.xyz) * frame.fog.a);
    return vec4<f32>(c * fade, a);
}

// ---- the tubes ----

struct TubeOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) world: vec3<f32>,
    // t, flow, tint, kind
    @location(2) data: vec4<f32>,
};

@vertex
fn vs_tube(@location(0) pos: vec3<f32>, @location(1) normal: vec3<f32>, @location(2) data: vec4<f32>) -> TubeOut {
    var o: TubeOut;
    o.clip = frame.view_proj * vec4<f32>(pos, 1.0);
    o.normal = normal;
    o.world = pos;
    o.data = data;
    return o;
}

@fragment
fn fs_tube(i: TubeOut) -> @location(0) vec4<f32> {
    let f = fresnel(i.normal, i.world);
    let glow = 0.35 + 0.65 * frame.look.z;
    let tendril = i.data.w > 0.5;
    let base = hsv(frame.look.w + i.data.z, select(0.45, 0.25, tendril), 1.0);
    // the flow: bands running from the source (t 0) to the target (t 1) while something moves through
    let bands = 0.5 + 0.5 * sin((i.data.x * 14.0 - frame.look.x * 3.0) * 6.2831853);
    var a = (0.1 + 0.9 * f) * select(0.55, 0.3, tendril) * glow;
    a = a * (1.0 + i.data.y * (0.6 + 1.4 * bands));
    let fade = exp(-distance(i.world, frame.eye.xyz) * frame.fog.a);
    return vec4<f32>(to_srgb(base) * a * fade, a);
}
