import { Color, DoubleSide, ShaderMaterial, type Texture } from "three";
import { type FilterData, FILTERS, type FilterParamSpec, keyMode } from "../types/index";

/**
 * Chroma key filter for AR videos: one key color (e.g. neon green, purple or black) in the video becomes
 * transparent – no alpha channel needed in the video file (works for MP4/H.264 on iOS and Android).
 *
 * Algorithm: the widely used OBS / three.js-forum chroma key (Mugen87). Colors are compared by their
 * chroma (UV of YUV), not RGB, so darker and lighter shades of the key color are removed too.
 * A neutral key color (black, grey, white) has no chroma – comparing chroma would remove every grey
 * too. For those the key compares brightness instead ("luma" mode, e.g. a video on pure black).
 *
 * Content: `entity.filters: [{ type: chromaKey, color, mode?, threshold?, softness?, spill?, opacity? }]`.
 * The parameters (kinds, defaults, ranges, descriptions) are defined once in shared/types/filters.ts –
 * the shader's uniforms are built from there.
 */

const SPEC = FILTERS.chromaKey.params;

/** chroma: compare color tone (green, purple, …) · luma: compare brightness (black, white) */
export type ChromaKeyMode = "chroma" | "luma";

/** All parameters of a chroma key, defaults filled in and `mode` resolved */
export interface ChromaKey {
  color: string;
  mode: ChromaKeyMode;
  threshold: number;
  softness: number;
  spill: number;
  opacity: number;
}

export { keyMode };

type NumberParam = Extract<FilterParamSpec, { kind: "number" }>;

/** A number parameter: the value clamped to its range, else its default (luma mode has own defaults) */
const numberParam = (spec: NumberParam, value: unknown, mode: ChromaKeyMode): number => {
  if (typeof value === "number" && Number.isFinite(value)) return Math.min(spec.max, Math.max(spec.min, value));
  return mode === "luma" && spec.lumaDefault !== undefined ? spec.lumaDefault : spec.default;
};

/** The entity's chroma key filter with all parameters, or undefined when it has none */
export const parseChromaKey = (filters?: readonly FilterData[]): ChromaKey | undefined => {
  const filter = filters?.find(f => f.type === "chromaKey");
  if (!filter) return undefined;
  const color = typeof filter.color === "string" ? filter.color : SPEC.color.default;
  const mode: ChromaKeyMode = filter.mode === "chroma" || filter.mode === "luma" ? filter.mode : keyMode(color);
  return {
    color,
    mode,
    threshold: numberParam(SPEC.threshold, filter.threshold, mode),
    softness: numberParam(SPEC.softness, filter.softness, mode),
    spill: numberParam(SPEC.spill, filter.spill, mode),
    opacity: numberParam(SPEC.opacity, filter.opacity, mode),
  };
};

const vertexShader = /* glsl */ `
varying vec2 vUv;
void main() {
  vUv = uv;
  gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
}
`;

const fragmentShader = /* glsl */ `
uniform sampler2D src;
uniform vec3 color;
uniform float threshold;
uniform float softness;
uniform float spill;
uniform float keyOpacity;
uniform float luma;
varying vec2 vUv;

vec2 RGBtoUV(vec3 rgb) {
  return vec2(
    rgb.r * -0.169 + rgb.g * -0.331 + rgb.b *  0.5   + 0.5,
    rgb.r *  0.5   + rgb.g * -0.419 + rgb.b * -0.081 + 0.5
  );
}

// Perceptual brightness – the samples are linear (color management), black and dark grey would be too close
float brightness(vec3 linearRgb) {
  return dot(pow(max(linearRgb, 0.0), vec3(1.0 / 2.2)), vec3(0.2126, 0.7152, 0.0722));
}

void main() {
  vec4 rgba = texture2D(src, vUv);
  float keyDistance = luma > 0.5
    ? abs(brightness(rgba.rgb) - brightness(color))
    : distance(RGBtoUV(rgba.rgb), RGBtoUV(color));

  float baseMask = keyDistance - threshold;
  rgba.a = pow(clamp(baseMask / softness, 0.0, 1.0), 1.5) * keyOpacity;

  // Spill (tint of the key color on edges) only exists for colored keys
  float spillValue = luma > 0.5 ? 1.0 : pow(clamp(baseMask / spill, 0.0, 1.0), 1.5);
  float desaturated = clamp(rgba.r * 0.2126 + rgba.g * 0.7152 + rgba.b * 0.0722, 0.0, 1.0);
  rgba.rgb = mix(vec3(desaturated), rgba.rgb, spillValue);

  gl_FragColor = rgba;
  #include <colorspace_fragment>
}
`;

/**
 * Material of a keyed video plane. Uniforms: the video texture (`src`, sRGB – samples are linear), the
 * key color (a THREE.Color, linear with color management like the samples) and one uniform per number
 * parameter of the filter definition (`opacity` is `keyOpacity`); `mode` is resolved in JS → `luma`.
 */
export const chromaKeyMaterial = (texture: Texture | null, key: ChromaKey): ShaderMaterial => {
  const material = new ShaderMaterial({
    uniforms: {
      src: { value: texture },
      color: { value: new Color() },
      luma: { value: 0 },
      threshold: { value: 0 },
      softness: { value: 0 },
      spill: { value: 0 },
      keyOpacity: { value: 1 },
    },
    vertexShader,
    fragmentShader,
    transparent: true,
    side: DoubleSide,
  });
  setChromaKey(material, key);
  return material;
};

/** Put a chroma key's values into a keyed video material's uniforms */
export const setChromaKey = (material: ShaderMaterial, key: ChromaKey): void => {
  const u = material.uniforms;
  (u.color.value as Color).set(key.color);
  u.luma.value = key.mode === "luma" ? 1 : 0;
  u.threshold.value = key.threshold;
  u.softness.value = key.softness;
  u.spill.value = key.spill;
  u.keyOpacity.value = key.opacity;
};
