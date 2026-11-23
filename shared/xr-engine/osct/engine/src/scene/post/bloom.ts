import { CustomBlending, OneFactor, Vector2, ZeroFactor } from "three";
import { UnrealBloomPass } from "three/examples/jsm/postprocessing/UnrealBloomPass.js";
import { PASSES } from "../../types/index";
import type { PassFactory, PostPass } from "./registry";

/** The glow is computed at this fraction of the canvas resolution (a blur needs no more) */
const BLOOM_SCALE = 0.5;

/**
 * The built-in `bloom`: three's UnrealBloomPass over the AR layer. Its final blend adds the glow to the image and
 * leaves the alpha alone (three's own blend would set it to 1 – a black screen over the camera picture): the glow
 * over an empty part of the canvas is premultiplied colour over alpha 0, light added to the camera picture.
 */
export const bloomPass: PassFactory = ({ params, size, pixelRatio }) => {
  const resolution = new Vector2(size.x * pixelRatio * BLOOM_SCALE, size.y * pixelRatio * BLOOM_SCALE);
  const pass = new UnrealBloomPass(resolution, params.strength as number, params.radius as number, params.threshold as number) as UnrealBloomPass & PostPass;
  Object.assign(pass.blendMaterial, {
    blending: CustomBlending,
    blendSrc: OneFactor,
    blendDst: OneFactor,
    blendSrcAlpha: ZeroFactor,
    blendDstAlpha: OneFactor,
  });
  pass.blendMaterial.needsUpdate = true;
  const setSize = pass.setSize.bind(pass);
  pass.setSize = (width: number, height: number) => setSize(width * BLOOM_SCALE, height * BLOOM_SCALE);
  pass.update = next => {
    pass.strength = next.strength as number;
    pass.radius = next.radius as number;
    pass.threshold = next.threshold as number;
  };
  return pass;
};

export const BLOOM_SPEC = PASSES.bloom;
