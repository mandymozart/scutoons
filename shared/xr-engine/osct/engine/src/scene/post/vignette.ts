import { Color } from "three";
import { PASSES } from "../../types/index";
import type { PassFactory, PostPass } from "./registry";

/**
 * The built-in `vignette`: the edges fade to a colour. The AR layer is premultiplied colour over alpha, so a fade
 * of strength v to the colour K is `rgb (1 − v) + K v` with alpha `a (1 − v) + v`: composited over the camera picture
 * that is the whole picture faded to K – the camera picture darkens with the layer.
 */
const FRAGMENT = /* glsl */ `
  uniform sampler2D tDiffuse;
  uniform float uStrength;
  uniform float uRadius;
  uniform float uSoftness;
  uniform vec3 uColor;
  varying vec2 vUv;
  void main() {
    vec4 c = texture2D(tDiffuse, vUv);
    float d = length((vUv - 0.5) * 2.0);
    float v = smoothstep(uRadius, uRadius + uSoftness, d) * uStrength;
    gl_FragColor = vec4(c.rgb * (1.0 - v) + uColor * v, c.a * (1.0 - v) + v);
  }
`;

export const vignettePass: PassFactory = ({ params, shaderPass }) => {
  const initial = {
    uStrength: { value: params.strength as number },
    uRadius: { value: params.radius as number },
    uSoftness: { value: params.softness as number },
    uColor: { value: new Color(params.color as string) },
  };
  const pass = shaderPass(FRAGMENT, initial) as PostPass;
  // The pass's own uniforms (three clones a shader object's): the live edit writes to these
  const uniforms = (pass as unknown as { uniforms: typeof initial }).uniforms;
  pass.update = next => {
    uniforms.uStrength.value = next.strength as number;
    uniforms.uRadius.value = next.radius as number;
    uniforms.uSoftness.value = next.softness as number;
    uniforms.uColor.value.set(next.color as string);
  };
  return pass;
};

export const VIGNETTE_SPEC = PASSES.vignette;
