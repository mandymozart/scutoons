/**
 * Placement of an AR entity on its target: `entity.params` in the content –
 *
 *   entity:
 *     type: model
 *     src: racoon.glb
 *     params:
 *       rotation: [90, 45, 0]   # degrees: x tips it up (90 = standing), y turns it around its own
 *                               # vertical axis, z tilts it sideways (three.js Euler order XYZ)
 *       position: [0, 0.1, 0]   # in target widths
 *       scale: 0.5              # one number, or [x, y, z]
 *
 * Axes (the printed page = the target): x → right, y → towards the top of the page, z → out of the
 * page (towards the reader). 1 unit = the target image's width, origin in its centre.
 * Defaults per entity type: models stand **on the page** (rotated 90° around x: the model's up points
 * out of the page, its front towards the bottom edge – the reader); videos and images lie flat on it.
 * Every missing value takes the default, so `rotation: [90, 180, 0]` alone turns a model around.
 *
 * Checked by the content build and the config guard (`placementProblems`), applied by the engine
 * (`resolvePlacement`).
 */

import { isObject } from "../guards/primitives";

export type Vector3Data = [number, number, number];

export interface Placement {
  position: Vector3Data;
  /** Degrees, Euler order XYZ */
  rotation: Vector3Data;
  scale: Vector3Data;
}

export const PLACEMENT_KEYS = ["position", "rotation", "scale"] as const;
export type PlacementKey = (typeof PLACEMENT_KEYS)[number];

/** How a placement value is adjusted in tools (debug tune panel): slider range and step, unit shown */
export interface PlacementParamSpec {
  unit: string;
  /** Slider range – a typed value may lie outside it (the content accepts any finite number, scale > 0) */
  min: number;
  max: number;
  step: number;
  description: string;
}

/** The placement values with the names of three.js `Object3D` (`position.x`, `rotation.y`, `scale.z`) */
export const PLACEMENT_PARAMS: Record<PlacementKey, PlacementParamSpec> = {
  position: { unit: "target widths", min: -1, max: 1, step: 0.01, description: "x → right, y → top of the page, z → out of the page" },
  rotation: { unit: "°", min: -180, max: 180, step: 1, description: "Degrees, Euler order XYZ (three.js uses radians)" },
  scale: { unit: "×", min: 0.05, max: 3, step: 0.01, description: "Relative to the target's width" },
};

const FLAT: Placement = { position: [0, 0, 0], rotation: [0, 0, 0], scale: [1, 1, 1] };

/**
 * Defaults by node type (other types lie flat, size 1); a text lies a hair above its parent so it never flickers on
 * a plane; a light hangs half a target width above its parent, its −z (the spot's and the sun's direction) down onto it
 */
export const PLACEMENT_DEFAULTS: Record<string, Placement> = {
  model: { position: [0, 0, 0], rotation: [90, 0, 0], scale: [0.5, 0.5, 0.5] },
  video: FLAT,
  image: FLAT,
  group: FLAT,
  text: { position: [0, 0, 0.01], rotation: [0, 0, 0], scale: [1, 1, 1] },
  light: { position: [0, 0, 0.5], rotation: [0, 0, 0], scale: [1, 1, 1] },
};

const isVector = (value: unknown): value is Vector3Data =>
  Array.isArray(value) && value.length === 3 && value.every(v => typeof v === "number" && Number.isFinite(v));

/** Problems of the placement keys in `params` (other keys of params are not checked here) */
export const placementProblems = (params: unknown, path: string): string[] => {
  if (!isObject(params)) return [];
  const p = params;
  const problems: string[] = [];
  (["position", "rotation"] as const).forEach(key => {
    if (p[key] !== undefined && !isVector(p[key])) {
      problems.push(`${path}.${key}: expected three numbers [x, y, z], got ${JSON.stringify(p[key])}`);
    }
  });
  if (p.scale !== undefined) {
    const ok = isVector(p.scale) ? p.scale.every(v => v > 0) : typeof p.scale === "number" && p.scale > 0;
    if (!ok) problems.push(`${path}.scale: expected a number > 0 or three numbers [x, y, z] > 0, got ${JSON.stringify(p.scale)}`);
  }
  return problems;
};

/** The entity's placement: its params where valid, the type's defaults otherwise */
export const resolvePlacement = (type: string, params?: Record<string, unknown>): Placement => {
  const defaults = PLACEMENT_DEFAULTS[type] ?? FLAT;
  const scale = params?.scale;
  return {
    position: isVector(params?.position) ? params.position : defaults.position,
    rotation: isVector(params?.rotation) ? params.rotation : defaults.rotation,
    scale: isVector(scale) ? scale : typeof scale === "number" && scale > 0 ? [scale, scale, scale] : defaults.scale,
  };
};
