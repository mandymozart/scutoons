// The space node (#188): the room unlit (its texture × the baked vertex light), the being's spheres (solid nuclei and
// buttons, translucent core / shells / bubbles, the buttons' rings and gauges) and its tubes (hoses, tendrils, the
// buttons' tethers), the heads, the buttons' labels. Linear maths, sRGB out (the offscreen target is plain Rgba8:
// display values, like every other pass).

struct Frame {
    view_proj: mat4x4<f32>,
    // xyz = the eye
    eye: vec4<f32>,
    // time, the room's light, the being's glow, its hue
    look: vec4<f32>,
    // rgb = the fog's colour, a = its density per metre
    fog: vec4<f32>,
    // the target's size in pixels, z = how many lights
    screen: vec4<f32>,
    // the beings' cores as point lights (#204): per light (x, y, z, range), (r, g, b, 0) – colour × intensity
    lights: array<vec4<f32>, 16>,
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

// The beings' glow on the room (#204): each core a point light – inverse-square (+1, no singularity at the core),
// windowed to zero at its range, on the face's normal. The rooms carry no normals: the face's from the world
// position's screen derivatives, turned toward the eye (the scans' winding isn't reliable).
fn glow_light(world: vec3<f32>, face: vec3<f32>) -> vec3<f32> {
    var sum = vec3<f32>(0.0);
    let count = i32(frame.screen.z);
    for (var k = 0; k < 8; k++) {
        if k >= count {
            break;
        }
        let at = frame.lights[2 * k];
        let rgb = frame.lights[2 * k + 1].rgb;
        let d = at.xyz - world;
        let d2 = dot(d, d);
        let r2 = at.w * at.w;
        let window = clamp(1.0 - (d2 * d2) / (r2 * r2), 0.0, 1.0);
        let ndl = max(dot(face, d * inverseSqrt(max(d2, 1e-6))), 0.0);
        sum += rgb * (ndl * window * window / (d2 + 1.0));
    }
    return sum;
}

@fragment
fn fs_room(i: RoomOut) -> @location(0) vec4<f32> {
    let tex = textureSample(room_tex, room_smp, i.uv);
    let c = cross(dpdx(i.world), dpdy(i.world));
    let n = c * inverseSqrt(max(dot(c, c), 1e-20));
    let face = select(-n, n, dot(n, frame.eye.xyz - i.world) >= 0.0);
    let lit = tex.rgb * (i.color.rgb * frame.look.y + glow_light(i.world, face));
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
    @location(5) center: vec3<f32>,
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
    o.center = center;
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
        if i.color.a > 0.5 && (abs(i.local.y) < 0.2 || abs(abs(i.local.y) - 0.55) < 0.07) {
            // linked: a dark collar and two thin bands – a locked bolt
            c = c * 0.22;
        }
    }
    c = c + i.color.rgb * rim * (0.4 + 0.6 * i.extra.y);
    // hover (extra.z 0..1): brighter with a white rim; a hand on it (1..2): lit fully in its own colour
    let hot = clamp(i.extra.z, 0.0, 1.0);
    let grip = clamp(i.extra.z - 1.0, 0.0, 1.0);
    c = c * (1.0 + 0.7 * hot) + vec3<f32>(1.0) * rim * hot * 0.9;
    c = mix(c, i.color.rgb * 1.8 + vec3<f32>(0.35), grip);
    return vec4<f32>(to_srgb(fogged(c, i.world)), 1.0);
}

// core, shells, bubbles: additive ghosts – the rim glows, the middle stays clear
@fragment
fn fs_ghost(i: SphereOut) -> @location(0) vec4<f32> {
    let f = fresnel(i.normal, i.world);
    let glow = 0.35 + 0.65 * i.extra.y;
    var a = i.color.a * (0.12 + 0.88 * f) * glow;
    if i.extra.x > 5.5 {
        // a button's ring or gauge, facing the eye: where on its outline (0 = the eye's right, counter-clockwise) and
        // how near the outline (0 centre, 1 rim)
        let d = normalize(i.center - frame.eye.xyz);
        let right = normalize(cross(d, vec3<f32>(0.0, 1.0, 0.0)));
        let up = cross(right, d);
        let n = normalize(i.local);
        let rp = sqrt(max(1.0 - dot(n, -d) * dot(n, -d), 0.0));
        let ang = atan2(dot(n, up), dot(n, right));
        let pi = 3.14159265;
        if i.extra.x > 6.5 {
            // the value ring (#188, always on): 270° from the lower left clockwise over the top to the lower right, lit
            // up to the knob's value (extra.w); a linked knob's swing band (extra.y..z) brighter, the value now a white
            // tick ((extra.x − 7) / 0.9). No range ends, no numbers. Thin and quiet at rest (alpha 0.5), wide and bright
            // under the hand (alpha → 1).
            let hand = clamp((i.color.a - 0.5) * 2.0, 0.0, 1.0);
            let u = fract((1.25 * pi - ang) / (2.0 * pi)) * 4.0 / 3.0;
            let on_arc = step(u, 1.0);
            let inner = mix(0.83, 0.76, hand);
            let band = smoothstep(inner, inner + 0.04, rp);
            let lit = step(u, i.extra.w);
            let swing = step(i.extra.y, u) * step(u, i.extra.z) * step(0.004, i.extra.z - i.extra.y);
            let live = clamp((i.extra.x - 7.0) / 0.9, 0.0, 1.0);
            let now = (1.0 - smoothstep(0.0, 0.03, abs(u - live))) * step(0.004, i.extra.z - i.extra.y);
            let head = 1.0 - smoothstep(0.0, 0.025, abs(u - i.extra.w));
            // the track dim, the value bright in the parameter's colour, the swing between, a white mark where it stands
            a = i.color.a * band * on_arc * (0.3 + 1.0 * lit + 0.55 * swing + 1.2 * head + 1.4 * now);
            let c = to_srgb(mix(i.color.rgb * (0.45 + 0.8 * lit + 0.3 * swing), vec3<f32>(1.0), 0.1 * lit + 0.5 * head + 0.7 * now)) * a;
            return vec4<f32>(c, a);
        }
        // the ring: thin; a linked button's dashed (a lock), turning slowly
        var dash = 1.0;
        if i.extra.w > 0.5 {
            dash = step(0.45, fract(ang * 12.0 / (2.0 * pi) + frame.look.x * 0.15));
        }
        a = i.color.a * smoothstep(0.9, 0.95, rp) * dash * 1.3;
        let c = to_srgb(mix(i.color.rgb, vec3<f32>(1.0), 0.3)) * a;
        return vec4<f32>(c, a);
    } else if i.extra.x > 4.5 {
        // the head's synthetic skin: clear in the middle, a cool rim, fine scan lines drifting down it; the energy
        // brightens it
        let scan = step(0.82, fract(i.world.y * 60.0 + frame.look.x * 0.4));
        a = i.color.a * (0.03 + 0.8 * pow(f, 1.6)) * glow * (0.7 + 0.6 * i.extra.w) + scan * 0.05 * f;
    } else if i.extra.x > 3.5 {
        // the core: a slow inner shimmer
        a = a * (0.8 + 0.2 * sin(frame.look.x * 1.7 + i.local.y * 6.0));
    } else if i.extra.x > 2.5 {
        // a bubble: water – a bright rim, a highlight
        let spec = pow(max(dot(normalize(i.normal), normalize(vec3<f32>(0.4, 0.8, 0.45))), 0.0), 24.0);
        a = i.color.a * (0.25 + 0.75 * f) * glow + spec;
    } else if i.extra.x > 1.5 {
        // a shell: it flashes while one of its buttons moves (what the button reaches)
        a = a * (1.0 + 2.5 * i.extra.z);
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
    // kinds: 0 a hose, 1 a tendril, 2 a neck cable (into the head), 3 a hose between beings, 4 + 0.9 × motion a
    // button's tether
    let k = i.data.w;
    let fade = exp(-distance(i.world, frame.eye.xyz) * frame.fog.a);
    if k > 3.5 {
        // the tether: the parameter's own hue (absolute), its value bright and steady; while it moves bands run from
        // the button (t 0) to the node
        let motion = clamp((k - 4.0) / 0.9, 0.0, 1.0);
        let base = hsv(i.data.z, 0.7, 1.0);
        let bands = 0.5 + 0.5 * sin((i.data.x * 6.0 - frame.look.x * 5.0) * 6.2831853);
        let ta = (0.45 + 0.55 * f) * (0.4 + 0.6 * glow) * (0.5 + 1.0 * i.data.y) * (1.0 + motion * (0.8 + 1.6 * bands));
        return vec4<f32>(to_srgb(base) * ta * fade, ta);
    }
    let tendril = k > 0.5 && k < 1.5;
    let neck = k > 1.5 && k < 2.5;
    var sat = 0.45;
    var body = 0.55;
    if tendril {
        sat = 0.25;
        body = 0.3;
    } else if neck {
        sat = 0.12;
        body = 0.7;
    } else if k > 2.5 {
        sat = 0.35;
        body = 0.45;
    }
    let base = hsv(frame.look.w + i.data.z, sat, 1.0);
    // the flow: bands running from the source (t 0) to the target (t 1) while something moves through
    let bands = 0.5 + 0.5 * sin((i.data.x * 14.0 - frame.look.x * 3.0) * 6.2831853);
    var a = (0.1 + 0.9 * f) * body * glow;
    a = a * (1.0 + i.data.y * (0.6 + 1.4 * bands));
    return vec4<f32>(to_srgb(base) * a * fade, a);
}

// ---- the heads (#188 phase 5): the portrait on a curved face plate over a machine layer ----

struct HeadOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) world: vec3<f32>,
    @location(2) normal: vec3<f32>,
    // alpha, layer, glow, hue
    @location(3) look: vec4<f32>,
    // energy, has a face, the plate's (u, v)
    @location(4) more: vec4<f32>,
    // the strain (the output limiter's red flush, #188)
    @location(5) strain: f32,
};

@vertex
fn vs_head(
    @location(0) grid: vec2<f32>,
    @location(1) center_size: vec4<f32>,
    @location(2) right_bulge: vec4<f32>,
    @location(3) up_alpha: vec4<f32>,
    @location(4) fwd_layer: vec4<f32>,
    @location(5) extra: vec4<f32>,
    @location(6) more: vec4<f32>,
) -> HeadOut {
    var o: HeadOut;
    let machine = fwd_layer.w < 0.5;
    // the machine layer sits behind the face and a little larger: a layered head, its parallax as the camera turns
    let s = center_size.w * select(1.0, 1.06, machine);
    let r2 = dot(grid, grid);
    let bulge = right_bulge.w;
    let z = (bulge * (1.0 - 0.5 * r2) + select(0.0, -0.35, machine)) * center_size.w;
    let right = right_bulge.xyz;
    let up = up_alpha.xyz;
    let fwd = fwd_layer.xyz;
    // a portrait's face sits in its upper half: the plate hangs lower so the face fills the skin, the shoulders below
    let world = center_size.xyz + right * grid.x * s + up * (grid.y - 0.38) * s + fwd * z;
    o.clip = frame.view_proj * vec4<f32>(world, 1.0);
    o.world = world;
    o.normal = normalize(fwd + (right * grid.x + up * grid.y) * bulge);
    o.uv = vec2<f32>(grid.x * 0.5 + 0.5, 0.5 - grid.y * 0.5);
    o.look = vec4<f32>(up_alpha.w, fwd_layer.w, extra.x, extra.y);
    o.more = vec4<f32>(extra.z, extra.w, grid.x, grid.y);
    o.strain = more.x;
    return o;
}

@fragment
fn fs_head(i: HeadOut) -> @location(0) vec4<f32> {
    let face = i.more.y > 0.5;
    let energy = i.more.x;
    let glow = 0.35 + 0.65 * i.look.z;
    let r = length(i.more.zw);
    let f = fresnel(i.normal, i.world);
    // without a face: a round visor
    let disc = 1.0 - smoothstep(0.8, 1.0, r);
    // no plate edges: the square fades out, and below the chin the shoulders dissolve into the neck's cables
    let g = i.more.zw;
    let edge = (1.0 - smoothstep(0.8, 1.0, max(abs(g.x), abs(g.y)))) * smoothstep(-0.5, 0.02, g.y);
    // sampled before any branch: derivatives need uniform control flow
    let tex = textureSample(room_tex, room_smp, i.uv);
    let blur = textureSampleBias(room_tex, room_smp, i.uv, 3.0);
    var c: vec3<f32>;
    var a: f32;
    if i.look.y < 0.5 {
        // the machine: the face's silhouette (blurred – a coarser mip) as a dark shell with circuit lines, lit by the
        // energy; plus a ring of jacks round the edge
        var sil = disc * 0.9;
        if face {
            sil = min(blur.a * 1.25, 1.0);
        }
        let cg = i.uv * 22.0;
        let cell = abs(fract(cg) - 0.5);
        let lines = step(0.46, max(cell.x, cell.y)) * step(0.5, fract(sin(dot(floor(cg), vec2<f32>(12.9898, 78.233))) * 43758.5453));
        let pulse = 0.5 + 0.5 * sin(frame.look.x * 2.0 - r * 9.0);
        let tint = hsv(i.look.w, 0.55, 1.0);
        c = vec3<f32>(0.015, 0.02, 0.035) + tint * lines * (0.25 + 0.9 * energy * pulse) * glow;
        c = c + tint * f * 0.3 * glow;
        a = sil * edge;
    } else {
        // the portrait: organic, translucent toward its rim (the machine shows through), a cool synthetic sheen, faint
        // scan lines; the energy lights it a little
        if face {
            c = tex.rgb;
            a = tex.a;
        } else {
            c = hsv(i.look.w, 0.35, 0.25);
            a = disc * 0.5;
        }
        let sheen = hsv(i.look.w - 0.05, 0.45, 1.0);
        let scan = 0.92 + 0.08 * step(0.5, fract(i.uv.y * 140.0 - frame.look.x * 0.8));
        c = (c * scan * (0.85 + 0.3 * energy)) + sheen * pow(f, 2.0) * 0.45 * glow;
        a = a * edge * i.look.x * (0.97 - 0.4 * f);
    }
    // overload (#188, owner 2026-10-31: "the head and avatar turn red-faced"): a red flush over the face and the machine
    // alike, strongest in the cheeks (the plate's middle), pulsing a little as it strains
    let flush = i.strain * (0.75 + 0.25 * sin(frame.look.x * 9.0)) * (1.0 - 0.4 * r);
    c = mix(c, c * vec3<f32>(1.25, 0.45, 0.4) + vec3<f32>(0.35, 0.02, 0.0), clamp(flush, 0.0, 1.0));
    if a < 0.04 {
        discard;
    }
    let shown = to_srgb(fogged(c, i.world));
    return vec4<f32>(shown * a, a);
}

// ---- the buttons' labels: a quad per character, a fixed size on the screen, always on top ----

struct LabelOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_label(
    @builtin(vertex_index) v: u32,
    @location(0) anchor: vec4<f32>,
    @location(1) rect: vec4<f32>,
    @location(2) uv: vec4<f32>,
    @location(3) color: vec4<f32>,
) -> LabelOut {
    var o: LabelOut;
    // two triangles: (0,0) (1,0) (0,1) · (1,0) (1,1) (0,1)
    var cx = array<f32, 6>(0.0, 1.0, 0.0, 1.0, 1.0, 0.0);
    var cy = array<f32, 6>(0.0, 0.0, 1.0, 0.0, 1.0, 1.0);
    let k = vec2<f32>(cx[v], cy[v]);
    let px = vec2<f32>(mix(rect.x, rect.z, k.x), mix(rect.y, rect.w, k.y));
    var clip = frame.view_proj * vec4<f32>(anchor.xyz, 1.0);
    // behind the eye: out of the picture
    if clip.w < 1e-3 {
        clip = vec4<f32>(2.0, 2.0, 2.0, 1.0);
    }
    // anchor.w = 1: on top of everything (a hovered knob's name among its neighbours), at the near plane
    clip = vec4<f32>(clip.xy + px * 2.0 / frame.screen.xy * clip.w, select(clip.z, 0.0, anchor.w > 0.5), clip.w);
    o.clip = clip;
    o.uv = vec2<f32>(mix(uv.x, uv.z, k.x), mix(uv.w, uv.y, k.y));
    o.color = color;
    return o;
}

@fragment
fn fs_label(i: LabelOut) -> @location(0) vec4<f32> {
    let a = textureSample(room_tex, room_smp, i.uv).a * i.color.a;
    return vec4<f32>(to_srgb(i.color.rgb) * a, a);
}
