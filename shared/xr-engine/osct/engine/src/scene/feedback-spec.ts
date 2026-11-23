import type { Texture } from "three";

/**
 * The three-free part of the plugins' feedback buffers (`feedback.ts`): the options and the handle a
 * plugin sees, their checks, the prelude of a pass and the Shadertoy adapter – shared by the engine and the content
 * build's recording run (`scripts/src/build/plugins.ts`), which hands out a dummy handle.
 */

export const FEEDBACK_TYPES = ["half", "float", "byte"] as const;
export type FeedbackType = (typeof FEEDBACK_TYPES)[number];

/** The largest side of a feedback buffer, in pixels */
export const MAX_FEEDBACK_SIZE = 2048;
/** Channels a pass can read (`iChannel0…3`, as Shadertoy) */
export const MAX_FEEDBACK_CHANNELS = 4;

/** What a channel reads: the buffer's own previous frame, another buffer's current one, a texture, or nothing */
export type FeedbackChannel = "self" | Feedback | Texture | null;

export interface FeedbackOptions {
  /** Pixels across and down (1…2048) */
  width: number;
  height: number;
  /** GLSL ES 3.0 with `void main()` writing `gl_FragColor`; the prelude declares the `i…` uniforms and `vUv` */
  fragment: string;
  /** The plugin's own uniforms (`{ name: { value } }`), declared in its fragment – kept by reference, change `.value` */
  uniforms?: Record<string, { value: unknown }>;
  /** `iChannel0…3` in order; default `["self"]` (Buffer A reading itself) */
  channels?: FeedbackChannel[];
  /** The target's precision: `half` (default), `float` or `byte`; falls back where the device cannot render into it */
  type?: FeedbackType;
  /** Sampling of the buffer's texture: `linear` (default) or `nearest` */
  filter?: "linear" | "nearest";
  /** Outside 0…1: `clamp` (default) or `repeat` */
  wrap?: "clamp" | "repeat";
}

/** The handle `api.feedback` returns */
export interface Feedback {
  /** The buffer's current frame – the last step's output; with ping-pong another texture after every step */
  readonly texture: Texture;
  /** `{ value: texture }`, kept current by every step: put it into a ShaderMaterial's uniforms and it follows */
  readonly uniform: { value: Texture };
  /** Every uniform of the pass (the engine's `i…` and the plugin's own) */
  readonly uniforms: Record<string, { value: unknown }>;
  readonly width: number;
  readonly height: number;
  /** The precision it got (after a fallback) */
  readonly type: FeedbackType;
  /** Steps done (`iFrame` of the next step) */
  readonly frame: number;
  /** Stop stepping by hand (it also pauses by itself while its node is hidden) */
  paused: boolean;
  /** Start again: `iFrame` and `iTime` back to 0 (the pass's own start, e.g. `if (iFrame < 10)`, runs again) */
  reset(): void;
  /** Free its render targets and its material; it never steps again */
  dispose(): void;
}

/** What `stepFeedbacks` needs of the renderer (a stub in tests) */

/** The uniforms a pass reads, declared before its fragment */
export const feedbackPrelude = (channels: number): string => [
  "uniform float iTime;",
  "uniform float iTimeDelta;",
  "uniform int iFrame;",
  "uniform vec3 iResolution;",
  ...Array.from({ length: channels }, (_, i) => `uniform sampler2D iChannel${i};`),
  "varying vec2 vUv;",
  "",
].join("\n");

/**
 * A Shadertoy pass as a feedback buffer's fragment (`api.shadertoy`): its source unchanged, then a `main` that calls
 * `mainImage(color, vUv * iResolution.xy)` – the pixel's centre, Shadertoy's `fragCoord` – and writes the colour. The
 * uniforms (`iTime`, `iTimeDelta`, `iFrame`, `iResolution`, `iChannel0…3`) come from the buffer's prelude. Throws when
 * the source has no `mainImage`, or brings a `main` of its own.
 */
export const shadertoyFragment = (source: string): string => {
  if (typeof source !== "string" || !/\bvoid\s+mainImage\s*\(/.test(source)) throw new Error("a Shadertoy pass defines void mainImage(out vec4, in vec2)");
  if (/\bvoid\s+main\s*\(/.test(source)) throw new Error("a Shadertoy pass has no main() of its own – the adapter adds it");
  return `${source.trimEnd()}

void main() {
  vec4 shadertoyColor = vec4(0.0, 0.0, 0.0, 1.0);
  mainImage(shadertoyColor, vUv * iResolution.xy);
  gl_FragColor = shadertoyColor;
}
`;
};

/** Problems of the options, as sentences (empty: fine) */
export const feedbackProblems = (options: unknown): string[] => {
  if (typeof options !== "object" || options === null) return ["expected { width, height, fragment }"];
  const o = options as Partial<FeedbackOptions>;
  const problems: string[] = [];
  for (const side of ["width", "height"] as const) {
    const value = o[side];
    if (typeof value !== "number" || !Number.isInteger(value) || value < 1 || value > MAX_FEEDBACK_SIZE) problems.push(`${side} is a whole number of pixels, 1–${MAX_FEEDBACK_SIZE}`);
  }
  if (typeof o.fragment !== "string" || !o.fragment.trim()) problems.push("fragment is the GLSL of the pass");
  else if (!/\bvoid\s+main\s*\(/.test(o.fragment)) problems.push("the fragment has no void main() – a Shadertoy pass goes through api.shadertoy(source)");
  if (o.type !== undefined && !(FEEDBACK_TYPES as readonly string[]).includes(o.type)) problems.push(`type is ${FEEDBACK_TYPES.join(", ")}`);
  if (o.filter !== undefined && o.filter !== "linear" && o.filter !== "nearest") problems.push("filter is linear or nearest");
  if (o.wrap !== undefined && o.wrap !== "clamp" && o.wrap !== "repeat") problems.push("wrap is clamp or repeat");
  if (o.channels !== undefined && (!Array.isArray(o.channels) || o.channels.length > MAX_FEEDBACK_CHANNELS)) problems.push(`channels is a list of at most ${MAX_FEEDBACK_CHANNELS}`);
  if (o.uniforms !== undefined && (typeof o.uniforms !== "object" || o.uniforms === null)) problems.push("uniforms is { name: { value } }");
  return problems;
};

/**
 * The precision a device renders into: the asked one, else the next lower – float needs `EXT_color_buffer_float` (and,
 * filtered linearly, `OES_texture_float_linear`), half `EXT_color_buffer_half_float` or `EXT_color_buffer_float`
 */
export const supportedFeedbackType = (asked: FeedbackType, has: (extension: string) => boolean, linear = true): FeedbackType => {
  const half = has("EXT_color_buffer_float") || has("EXT_color_buffer_half_float");
  if (asked === "float") {
    if (has("EXT_color_buffer_float") && (!linear || has("OES_texture_float_linear"))) return "float";
    return half ? "half" : "byte";
  }
  if (asked === "half") return half ? "half" : "byte";
  return "byte";
};

