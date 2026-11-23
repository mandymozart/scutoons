/**
 * A `light` node (content schema 2.6): a light an author places like any node – under a group or a model it moves
 * with it. Three kinds: a **point** light shines in every direction from the node's origin, a **spot** light in a
 * cone and a **directional** light as parallel rays, both along the node's own −z (turn the node to aim it; the
 * default placement puts a light half a target width above the page, pointing down onto it). Ambient light is the
 * scene's, not a node. Lights cast no shadows.
 *
 * The values live in the node's `light` object; every parameter is defined once here – kind, default, range and a
 * description – so the editor's panel, the content build, the config guard and the engine read one definition
 * (as the filters and the components). A missing value takes the default. Intensity and distance are in the node's
 * units (target widths), whatever the target's printed size: the engine scales them to the world.
 */

import { type ComponentParamSpec, paramProblem } from "./components";
import { isObject } from "../guards/primitives";

export const LIGHT_KINDS = ["point", "spot", "directional"] as const;
export type LightKind = (typeof LIGHT_KINDS)[number];

export const LIGHT_PARAMS = {
  kind: {
    kind: "choice", options: LIGHT_KINDS, default: "point", label: "kind",
    description: "Point: in every direction from the node. Spot: a cone along the node's −z. Directional: parallel rays along the node's −z (like the sun).",
  },
  color: { kind: "color", default: "#ffffff", label: "color", description: "The light's colour." },
  intensity: {
    kind: "number", default: 0.25, min: 0, max: 10, step: 0.05, label: "intensity",
    description: "How bright, in the node's units: a point or spot light of 1 lights a surface one target width away fully (a white page turns white; more clips), half as far away four times as much. A directional light of 1 lights every surface fully.",
  },
  distance: {
    kind: "number", default: 0, min: 0, max: 5, step: 0.05, unit: "target widths", label: "distance",
    description: "How far a point or spot light reaches – 0 = no limit.",
  },
  decay: { kind: "number", default: 2, min: 0, max: 4, step: 0.1, label: "decay", description: "How fast a point or spot light dims with the distance (2 = physical)." },
  angle: { kind: "number", default: 30, min: 1, max: 90, step: 1, unit: "°", label: "cone angle", description: "The spot's cone: half the opening angle." },
  penumbra: { kind: "number", default: 0.1, min: 0, max: 1, step: 0.01, label: "soft edge", description: "The spot's soft edge: 0 = hard, 1 = the whole cone fades." },
} as const satisfies Record<string, ComponentParamSpec>;

export type LightParam = keyof typeof LIGHT_PARAMS;

/** A light as the content holds it: the parameters set (missing ones use the defaults) */
export interface LightData {
  kind?: LightKind;
  color?: string;
  intensity?: number;
  distance?: number;
  decay?: number;
  angle?: number;
  penumbra?: number;
}

/** A light with every parameter resolved: the value set, or the default */
export interface Light {
  kind: LightKind;
  color: string;
  intensity: number;
  distance: number;
  decay: number;
  angle: number;
  penumbra: number;
}

/** Problems of a node's `light` object (the content build, the guard) */
export const lightProblems = (raw: unknown, path: string): string[] => {
  if (!isObject(raw)) return [`${path}: expected { kind, color, intensity, … }`];
  const problems: string[] = [];
  for (const [name, value] of Object.entries(raw)) {
    const spec = (LIGHT_PARAMS as Record<string, ComponentParamSpec>)[name];
    if (!spec) {
      problems.push(`${path}.${name}: unknown light parameter (${Object.keys(LIGHT_PARAMS).join(", ")})`);
      continue;
    }
    const problem = paramProblem(spec, value);
    if (problem) problems.push(`${path}.${name}: ${problem}`);
  }
  return problems;
};

/** The node's light with every parameter resolved to its value or default (an invalid value takes the default) */
export const resolveLight = (light?: LightData): Light => {
  const value = <K extends LightParam>(name: K): Light[K] => {
    const spec: ComponentParamSpec = LIGHT_PARAMS[name];
    const set = light?.[name];
    return (set !== undefined && !paramProblem(spec, set) ? set : spec.default) as Light[K];
  };
  return {
    kind: value("kind"), color: value("color"), intensity: value("intensity"), distance: value("distance"),
    decay: value("decay"), angle: value("angle"), penumbra: value("penumbra"),
  };
};
