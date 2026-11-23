import { Color, type Texture } from "three";
import { enabledPasses, type PassData, PASSES, resolvePass } from "../../types/index";
import type { MaskUsage, PassFactory, PostPass } from "./registry";

/**
 * The built-in `grade`: exposure, contrast, saturation and a tint. The layer is premultiplied: the colour is divided
 * by the alpha before grading and multiplied again after, so an edge keeps its coverage; a texel with alpha 0 (light
 * added by a bloom) is graded as it is. Contrast pivots on linear mid-grey. `reach`: where the image holds the
 * editor's desk or the look-around world too, `ar` keeps the grade off them with the mask (the AR layer's coverage),
 * `all` grades them with it. The phone's camera picture never reaches this buffer (the engine draws it into its own canvas under the
 * three.js canvas): for `all` the engine's camera renderer grades it with the same steps (`cameraGradeShader`).
 */
const FRAGMENT = /* glsl */ `
  uniform sampler2D tDiffuse;
  uniform float uExposure;
  uniform float uContrast;
  uniform float uSaturation;
  uniform vec3 uTint;
  uniform sampler2D tMask;
  uniform float uMasked;
  varying vec2 vUv;
  const float MID_GREY = 0.18;
  void main() {
    vec4 c = texture2D(tDiffuse, vUv);
    float a = c.a > 0.001 ? c.a : 1.0;
    vec3 s = c.rgb / a;
    s *= uExposure * uTint;
    s = (s - MID_GREY) * uContrast + MID_GREY;
    float luma = dot(s, vec3(0.2126, 0.7152, 0.0722));
    s = mix(vec3(luma), s, uSaturation);
    vec3 graded = max(s, 0.0) * a;
    float m = uMasked > 0.5 ? texture2D(tMask, vUv).a : 1.0;
    gl_FragColor = vec4(mix(c.rgb, graded, m), c.a);
  }
`;

/** The grade reaches the whole picture (the desk, the camera picture), not the AR layer alone */
const reachesAll = (params: Record<string, unknown>) => params.reach === "all";

export const gradePass: PassFactory = ({ params, shaderPass }) => {
  const initial = {
    uExposure: { value: Math.pow(2, params.exposure as number) },
    uContrast: { value: params.contrast as number },
    uSaturation: { value: params.saturation as number },
    uTint: { value: new Color(params.tint as string) },
    tMask: { value: null as Texture | null },
    uMasked: { value: 0 },
  };
  const pass = shaderPass(FRAGMENT, initial) as PostPass & MaskUsage;
  // The pass's own uniforms (three clones a shader object's): the live edit writes to these
  const uniforms = (pass as unknown as { uniforms: typeof initial }).uniforms;
  let all = reachesAll(params);
  let mask: Texture | null = null;
  const applyMask = () => {
    pass.usesMask = !all;
    uniforms.tMask.value = all ? null : mask;
    uniforms.uMasked.value = !all && mask ? 1 : 0;
  };
  pass.update = next => {
    all = reachesAll(next);
    applyMask();
    uniforms.uExposure.value = Math.pow(2, next.exposure as number);
    uniforms.uContrast.value = next.contrast as number;
    uniforms.uSaturation.value = next.saturation as number;
    uniforms.uTint.value.set(next.tint as string);
  };
  pass.setMask = next => {
    mask = next;
    applyMask();
  };
  applyMask();
  return pass;
};

export const GRADE_SPEC = PASSES.grade;

/** The resolved parameters of the passes that grade the whole picture (`grade` with `reach: all`, not off), in order */
export const cameraGrades = (passes: readonly PassData[]): Record<string, unknown>[] =>
  enabledPasses(passes).filter(pass => pass.type === "grade").map(pass => resolvePass(GRADE_SPEC, pass)).filter(reachesAll);

const glsl = (value: number) => (Number.isInteger(value) ? value.toFixed(1) : String(Math.round(value * 1e6) / 1e6));

/**
 * The fragment shader the engine's camera renderer (8th Wall `GlTextureRenderer`: `texUv`, `sampler`) draws the
 * camera picture with, for these grades in order – `null` for none (the engine's own shader). The picture is sRGB:
 * decoded to linear, graded with the steps of the AR layer's grade (the tint as three converts it), encoded again.
 * The values are constants of the source: a change builds the renderer's program again (rare – the content's passes).
 */
export const cameraGradeShader = (grades: readonly Record<string, unknown>[]): string | null => {
  if (!grades.length) return null;
  const steps = grades.map(grade => {
    const exposure = Math.pow(2, grade.exposure as number);
    const tint = new Color(grade.tint as string);
    const gain = [tint.r, tint.g, tint.b].map(channel => glsl(channel * exposure)).join(", ");
    return [
      `  s *= vec3(${gain});`,
      `  s = (s - MID_GREY) * ${glsl(grade.contrast as number)} + MID_GREY;`,
      `  s = max(mix(vec3(dot(s, LUMA)), s, ${glsl(grade.saturation as number)}), 0.0);`,
    ].join("\n");
  });
  return `precision mediump float;
varying vec2 texUv;
uniform sampler2D sampler;
const float MID_GREY = 0.18;
const vec3 LUMA = vec3(0.2126, 0.7152, 0.0722);
vec3 toLinear(vec3 c) { return mix(c / 12.92, pow((c + 0.055) / 1.055, vec3(2.4)), step(0.04045, c)); }
vec3 toSrgb(vec3 c) { return mix(c * 12.92, 1.055 * pow(c, vec3(1.0 / 2.4)) - 0.055, step(0.0031308, c)); }
void main() {
  vec4 c = texture2D(sampler, texUv);
  vec3 s = toLinear(c.rgb);
${steps.join("\n")}
  gl_FragColor = vec4(toSrgb(clamp(s, 0.0, 1.0)), c.a);
}
`;
};
