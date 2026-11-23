import { Matrix3, Matrix4, NoBlending, ShaderMaterial, Vector2, Vector3 } from "three";

/**
 * Sky test on the camera sample (raw sRGB): clear blue, or bright neutral grey (overcast) – light façades are
 * warm and stay real – and smooth (leaves, edges and texture are not sky). Five taps in a cross.
 */
const SKY_KEY = /* glsl */ `
  float skyColour(vec3 c) {
    float lum = dot(c, vec3(0.299, 0.587, 0.114));
    float hi = max(c.r, max(c.g, c.b));
    float lo = min(c.r, min(c.g, c.b));
    float saturation = (hi - lo) / (hi + 0.001);
    float blue = smoothstep(0.03, 0.14, c.b - c.r) * smoothstep(0.28, 0.5, lum) * (1.0 - smoothstep(0.02, 0.12, c.g - c.b));
    float grey = smoothstep(0.6, 0.8, lum) * (1.0 - smoothstep(0.1, 0.24, saturation)) * smoothstep(-0.035, 0.0, c.b - c.r);
    return max(blue, grey);
  }

  float skyMask(vec2 uv) {
    vec2 cam = (uv - 0.5) * uCover + 0.5;
    vec2 d = uTexel * 1.5;
    vec3 c0 = texture2D(tCamera, cam).rgb;
    vec3 c1 = texture2D(tCamera, cam + vec2(d.x, 0.0)).rgb;
    vec3 c2 = texture2D(tCamera, cam - vec2(d.x, 0.0)).rgb;
    vec3 c3 = texture2D(tCamera, cam + vec2(0.0, d.y)).rgb;
    vec3 c4 = texture2D(tCamera, cam - vec2(0.0, d.y)).rgb;
    float score = (skyColour(c0) + skyColour(c1) + skyColour(c2) + skyColour(c3) + skyColour(c4)) / 5.0;
    vec3 w = vec3(0.299, 0.587, 0.114);
    float l0 = dot(c0, w);
    float edge = abs(dot(c1, w) - l0) + abs(dot(c2, w) - l0) + abs(dot(c3, w) - l0) + abs(dot(c4, w) - l0);
    return score * (1.0 - smoothstep(0.06, 0.2, edge));
  }
`;


/**
 * The experience's world (no built-in world since the café went – Tilman, 2026-11-18): the sky image where the
 * environment names one (equirectangular, its centre towards the surfaces, its top edge straight up), over it the world
 * model's cube (baked once from the reader's eye, transparent where the model has nothing). The alpha says where there is
 * anything: without a sky image or a world model nothing is drawn around the surfaces.
 */
const WORLD = /* glsl */ `
  const float PI = 3.141592653589793;

  vec3 skyImage(vec3 dir) {
    vec2 uv = vec2(atan(dir.x, -dir.z) / (2.0 * PI) + 0.5, asin(clamp(dir.y, -1.0, 1.0)) / PI + 0.5);
    return texture2D(tSky, uv).rgb;
  }

  vec4 worldColour(vec3 dir) {
    vec4 colour = uSkyImage > 0.5 ? vec4(skyImage(dir), 1.0) : vec4(0.0);
    if (uWorldModel > 0.5) {
      vec4 world = textureCube(tWorld, dir);
      colour = vec4(mix(colour.rgb, world.rgb / max(world.a, 1e-4), world.a), max(colour.a, world.a));
    }
    return colour;
  }
`;

/**
 * Per pixel: the view ray in the world (from the inverse projection and the phone's orientation), then
 * - world alpha = presence × opacity, cleared in the surfaces' window, below `uFloor` (table and surfaces stay real) and
 *   above ~20–45° (open roof);
 * - sky alpha = the sky test above the horizon.
 * Output is premultiplied (the canvas lies over the camera canvas); pixels with neither stay transparent at once.
 */
const COMPOSITE = /* glsl */ `
  void main() {
    vec4 point = uInverseProjection * vec4(vUv * 2.0 - 1.0, 1.0, 1.0);
    vec3 ray = normalize(uRotation * (point.xyz / point.w));
    float elevation = asin(clamp(ray.y, -1.0, 1.0));

    float outside = smoothstep(uWindow.x, uWindow.y, acos(clamp(dot(ray, uAnchor), -1.0, 1.0)));
    float roof = 1.0 - smoothstep(0.35, 0.8, elevation);
    float floorClear = smoothstep(uFloor.x, uFloor.y, elevation);
    float world = uPresence * uOpacity * outside * roof * floorClear;
    float sky = uSky > 0.0 && elevation > 0.03 ? uSky * smoothstep(0.03, 0.3, elevation) * skyMask(vUv) : 0.0;

    float alpha = max(world, sky);
    if (alpha < 0.002) {
      gl_FragColor = vec4(0.0);
      return;
    }
    float yc = cos(uYaw);
    float ys = sin(uYaw);
    vec4 found = worldColour(vec3(yc * ray.x - ys * ray.z, ray.y, ys * ray.x + yc * ray.z));
    alpha *= found.a;
    if (alpha < 0.002) {
      gl_FragColor = vec4(0.0);
      return;
    }
    gl_FragColor = vec4(found.rgb, 1.0);
    #include <colorspace_fragment>
    gl_FragColor = vec4(gl_FragColor.rgb * alpha, alpha);
  }
`;

/**
 * The look-around pass: one full-screen material (`FullScreenQuad`), drawn into the cleared screen or, while passes
 * run, into the post chain's buffer (linear there: three picks the output colour space by the render target)
 */
export const lookAroundMaterial = (): ShaderMaterial => new ShaderMaterial({
  uniforms: {
    uTime: { value: 0 },
    uYaw: { value: 0 },
    tCamera: { value: null },
    uCover: { value: new Vector2(1, 1) },
    uTexel: { value: new Vector2(0.01, 0.01) },
    uInverseProjection: { value: new Matrix4() },
    uRotation: { value: new Matrix3() },
    uAnchor: { value: new Vector3(0, -1, 0) },
    uWindow: { value: new Vector2(0.3, 0.6) },
    uFloor: { value: new Vector2(-0.7, -0.44) },
    uPresence: { value: 0 },
    uOpacity: { value: 1 },
    uSky: { value: 0 },
    tSky: { value: null },
    uSkyImage: { value: 0 },
    tWorld: { value: null },
    uWorldModel: { value: 0 },
  },
  vertexShader: /* glsl */ `
    varying vec2 vUv;
    void main() {
      vUv = uv;
      gl_Position = vec4(position.xy, 0.0, 1.0);
    }
  `,
  fragmentShader: /* glsl */ `
    uniform float uTime;
    uniform float uYaw;
    uniform sampler2D tCamera;
    uniform vec2 uCover;
    uniform vec2 uTexel;
    uniform mat4 uInverseProjection;
    uniform mat3 uRotation;
    uniform vec3 uAnchor;
    uniform vec2 uWindow;
    uniform vec2 uFloor;
    uniform float uPresence;
    uniform float uOpacity;
    uniform float uSky;
    uniform sampler2D tSky;
    uniform float uSkyImage;
    uniform samplerCube tWorld;
    uniform float uWorldModel;
    varying vec2 vUv;
    ${SKY_KEY}
    ${WORLD}
    ${COMPOSITE}
  `,
  blending: NoBlending,
  depthTest: false,
  depthWrite: false,
});
