import * as THREE from "three";
import type { Vector2 } from "three";
import type { Pass } from "three/examples/jsm/postprocessing/Pass.js";
import { ShaderPass } from "three/examples/jsm/postprocessing/ShaderPass.js";
import { type PassData, type PassSpec, resolvePass } from "../../types/index";

/**
 * The pass registry: every post-processing type the engine can run – the built-in ones (`./bloom`, `./vignette`,
 * `./grade`, registered by `./index`) and a plugin's own, registered through the plugin API's `registerPass` with
 * the plugin as the owner. The content names passes by type (`passes: [{ type, …params }]`, `shared/types/passes.ts`) –
 * a plugin's too, where a declared plugin recorded it; `PostChain` builds them from here, in
 * the content's order, and skips one that cannot be built (logged).
 *
 * A pass is a three.js `Pass` (`render(renderer, writeBuffer, readBuffer)` over the composer's buffers): the read
 * buffer holds the AR layer as the scene rendered it – premultiplied colour over alpha, in linear RGB, nothing of
 * the camera picture, which lies under the canvas (in the editor the stage's desk stands in for it and is part of
 * the buffer: the passes shade it like the camera picture, a grade with `reach: ar` is limited to the AR layer by `setMask`;
 * the look-around world and sky, while drawn, lie in the buffer under the AR layer the same way).
 * A pass that writes alpha 1 everywhere would cover the camera picture with black: a pass keeps the texel's alpha unless it means to shade the camera picture too (a vignette
 * raises the alpha where it darkens – the picture below shows through less).
 */

/** What a pass factory gets */
export interface PassContext<P = Record<string, unknown>> {
  /**
   * The pass's parameters with the spec's defaults filled in – a `texture` parameter as a three.js texture (a Library
   * picture, a procedural texture of a plugin) or null for none
   */
  params: P;
  /** The app's three.js (one instance – a plugin never imports three itself) */
  three: typeof THREE;
  /** three's `ShaderPass` (`new ShaderPass({ uniforms, vertexShader, fragmentShader })`) */
  ShaderPass: typeof ShaderPass;
  /** The canvas size in CSS px and the renderer's pixel ratio */
  size: Vector2;
  pixelRatio: number;
  /**
   * A full-screen shader pass from a fragment shader: `tDiffuse` (the image so far) and `vUv` are defined, the
   * vertex shader is the standard one; `uniforms` are added to it
   */
  shaderPass(fragmentShader: string, uniforms?: Record<string, { value: unknown }>): ShaderPass;
}

/**
 * A registered pass: three's `Pass`, optionally with `update` for new parameter values in place (the editor's live edit).
 * `render` gets the seconds since the last frame as its fourth argument (an animated grain counts its time with it). A
 * feedback buffer the factory makes (`api.feedback`) belongs to the pass: it steps while the pass runs and
 * goes with it.
 */
export interface PostPass extends Pass {
  update?(params: Record<string, unknown>): void;
  /**
   * The AR layer's coverage (its alpha) while the image holds more than the AR layer – the editor's stage, where the
   * desk stands in for the camera picture, or the look-around world and sky under the scene; `null` while the image
   * is the AR layer alone. A pass that reaches the AR layer only (the grade with `reach: ar`) limits itself to it with
   * this.
   */
  setMask?(mask: THREE.Texture | null): void;
}

/**
 * Engine-internal, not part of the plugin API: whether a pass uses the mask with its current parameters (the built-in
 * grade with `reach: all` does not). The chain draws the mask – one more render of the AR layer – only while a pass
 * uses it; a pass with `setMask` and without this flag (a plugin's) is always given the mask.
 */
export interface MaskUsage {
  usesMask?: boolean;
}

export type PassFactory<P = Record<string, unknown>> = (context: PassContext<P>) => PostPass;

export interface PassRegistration {
  owner: string;
  spec: PassSpec;
  factory: PassFactory;
}

const registry = new Map<string, PassRegistration>();

export const ENGINE_OWNER = "engine";

/**
 * Register a pass type: its spec (parameters with kind, default and range, the same shape as a component's) and the
 * factory that builds it. A type registered by another owner is refused; the same owner replaces its own.
 */
export const registerPass = (type: string, spec: PassSpec, factory: PassFactory, owner = ENGINE_OWNER): void => {
  const existing = registry.get(type);
  if (existing && existing.owner !== owner) throw new Error(`The pass type "${type}" is registered by ${existing.owner}`);
  registry.set(type, { owner, spec, factory });
};

/** Every pass type an owner registered leaves the registry (a plugin is uninstalled) */
export const unregisterPassesOf = (owner: string): void => {
  registry.forEach((registration, type) => {
    if (registration.owner === owner) registry.delete(type);
  });
};

export const passRegistration = (type: string): PassRegistration | undefined => registry.get(type);

export const registeredPassTypes = (): string[] => [...registry.keys()];

const VERTEX_SHADER = /* glsl */ `
  varying vec2 vUv;
  void main() {
    vUv = uv;
    gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
  }
`;

/** The context a factory gets for a pass of the content, with the parameters resolved against the spec */
export const passContext = (registration: PassRegistration, data: PassData, size: Vector2, pixelRatio: number): PassContext => ({
  params: resolvePass(registration.spec, data),
  three: THREE,
  ShaderPass,
  size,
  pixelRatio,
  shaderPass: (fragmentShader, uniforms = {}) =>
    // A material, not a shader object: the pass then keeps the caller's `uniforms` (a plain shader object is cloned,
    // and a later `update` writing to its own uniforms would change nothing on screen)
    new ShaderPass(new THREE.ShaderMaterial({ uniforms: { tDiffuse: { value: null }, ...uniforms }, vertexShader: VERTEX_SHADER, fragmentShader })),
});
