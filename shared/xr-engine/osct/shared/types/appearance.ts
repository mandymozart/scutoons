/**
 * How an entity appears and disappears (content schema 2.29): the experience names a **look** for each of three moments –
 *
 *   experience:
 *     appearance:
 *       reveal: { type: inflateDissolve }           # the first find of a target (the entry is unlocked with it)
 *       found:  { type: printLift, duration: 900 }  # every later find
 *       lost:   { type: fade }                      # the page is lost
 *
 * A look is `{ type, ...parameters }` like a pass: a built-in type, defined once here with its parameters (kind,
 * default, range, description) so the editor's rows, the content build, the config guard and the engine read one
 * definition – or a plugin's own (`registerAppearance`), where a declared plugin recorded it (`registered.appearances`).
 * A moment without a look takes `found`'s, and without that the default; `lost` plays its look rewound, in the look's
 * `outDuration`. An experience without `appearance` plays the default look at every moment.
 *
 * An entity may carry its own (schema 2.30): the components `reveal`, `found` and `lost` on a node, each
 * `{ type: reveal, look: printLift, duration: 900 }` – `look` is the look's type (empty = the experience's), the other keys its
 * parameters. They win over the experience's: a node's moment plays its own look, else its `found`'s, else (the root) the experience's
 * (`entityMomentAppearance`). Any node may carry them: a node with a look of its own animates its subtree with it, apart
 * from the rest of the entity.
 *
 * The built-in looks share one set of parameters (the engine's animation, `engine/src/scene/celebration.ts`) and differ
 * in their defaults. The reveal sandbox (`sandbox/reveal`) names and measures the values the same way: a look tuned
 * there is the parameters as they are. Starts and ends are shares of the duration (0–1); the widths of the edge and
 * the glow are shares of the pattern (the noise's range).
 *
 * Adding a built-in look: add it to APPEARANCES with its values; a look that needs a new part adds the parameter to
 * PARAMS and the part to the engine. Document it in docs/content.md.
 */

import type { ComponentData, ComponentParamSpec, SpecLogic } from "./components";
import { paramProblems } from "./components";
import { isObject } from "../guards/primitives";

/** The moments an entity appears or disappears at */
export const MOMENTS = ["reveal", "found", "lost"] as const;
export type Moment = (typeof MOMENTS)[number];
export const isMoment = (value: unknown): value is Moment => typeof value === "string" && (MOMENTS as readonly string[]).includes(value);

/** Schema 5.1: `actions`, `events`, `uses` (`SpecLogic`) as a component's */
export interface AppearanceSpec extends SpecLogic {
  description: string;
  params: Record<string, ComponentParamSpec>;
}

/** A look as the content holds it: its type and the parameters set (missing ones use the spec's defaults) */
export interface AppearanceData {
  type: string;
  [param: string]: unknown;
}

/** The experience's looks, by moment */
export type ExperienceAppearance = Partial<Record<Moment, AppearanceData>>;

/** The values of a built-in look, as the engine reads them */
export interface Appearance {
  /** ms to appear, and to disappear (the same, rewound) */
  duration: number;
  outDuration: number;
  /** `noise`: a front moves through 3D noise and what it has not reached is not drawn; `none`: everything is drawn */
  mask: "none" | "noise";
  maskStart: number;
  maskEnd: number;
  waves: number;
  edgeStart: number;
  edgeEnd: number;
  edgeColor: string;
  edgeIntensity: number;
  glow: number;
  glowWidth: number;
  glowColor: string;
  heightFrom: number;
  scaleFrom: number;
  spin: number;
  shapeStart: number;
  shapeEnd: number;
  shapeEase: ShapeEasing;
  fade: number;
  tint: number;
  tintColor: string;
  tintEnd: number;
  sparks: number;
  sparkColor: string;
  sparkSize: number;
  sparkDistance: number;
  bloom: number;
}

/** How a shape eases to its end */
export const SHAPE_EASINGS = ["linear", "smooth", "out", "inOut", "in"] as const;
export type ShapeEasing = (typeof SHAPE_EASINGS)[number];

const share = { kind: "number", min: 0, max: 1, step: 0.01 } as const;

/** The parameters of every built-in look, with the values of a look that only is there (nothing moves, nothing fades) */
const PARAMS = {
  duration: { kind: "number", default: 600, min: 50, max: 10000, step: 10, unit: "ms", label: "appear", description: "How long it takes to appear." },
  outDuration: { kind: "number", default: 400, min: 50, max: 10000, step: 10, unit: "ms", label: "disappear", description: "How long it takes to disappear: the same animation, rewound." },
  mask: {
    kind: "choice", options: ["none", "noise"], default: "none",
    description: "noise = a front moves through 3D noise and what it has not reached is not drawn yet (the dissolve); none = everything is drawn.",
  },
  maskStart: { ...share, default: 0, label: "mask start", description: "Share of the time at which the front starts." },
  maskEnd: { ...share, default: 1, label: "mask end", description: "Share of the time at which the front has passed everything." },
  waves: { kind: "number", default: 3, min: 0.5, max: 12, step: 0.1, description: "Noise waves across the object." },
  edgeStart: { kind: "number", default: 0, min: 0, max: 0.5, step: 0.005, label: "edge from", description: "Width of the coloured edge at the front when it starts (0 = none)." },
  edgeEnd: { kind: "number", default: 0, min: 0, max: 0.5, step: 0.005, label: "edge to", description: "Width of the edge when the front has passed." },
  edgeColor: { kind: "color", default: "#f3cc94", label: "edge colour", description: "The edge's colour." },
  edgeIntensity: { kind: "number", default: 1, min: 0, max: 4, step: 0.05, label: "edge intensity", description: "Multiplies the edge's colour; above 1 it is brighter than white and blooms." },
  glow: { ...share, default: 0, description: "How strongly the band behind the edge shines in the glow colour (0 = none)." },
  glowWidth: { ...share, default: 0.2, label: "glow width", description: "Width of the shining band behind the edge." },
  glowColor: { kind: "color", default: "#f3cc94", label: "glow colour", description: "The glow's colour." },
  heightFrom: { ...share, default: 1, label: "height from", description: "Height along the page normal at the start, as a share of the real one: 0 = flat on the page (the inflate), 1 = unchanged." },
  scaleFrom: { kind: "number", default: 1, min: 0, max: 2, step: 0.01, label: "scale from", description: "Size at the start, as a share of the real one (1 = unchanged)." },
  spin: { kind: "number", default: 0, min: -2, max: 2, step: 0.05, unit: "turns", description: "Turns around the page normal on the way in." },
  shapeStart: { ...share, default: 0, label: "shape start", description: "Share of the time at which the shape starts to change." },
  shapeEnd: { ...share, default: 1, label: "shape end", description: "Share of the time at which the entity has its real shape." },
  shapeEase: { kind: "choice", options: [...SHAPE_EASINGS], default: "out", label: "shape easing", description: "How the shape eases to its end." },
  fade: { ...share, default: 0, description: "Share of the time the opacity takes from nothing to full (0 = opaque from the start, 1 = a fade over the whole time)." },
  tint: { ...share, default: 0, description: "How much of the tint colour lies over the surface at the start (0 = none)." },
  tintColor: { kind: "color", default: "#f5e7c8", label: "tint colour", description: "The tint's colour." },
  tintEnd: { ...share, default: 0.6, label: "tint gone at", description: "Share of the time at which the tint is gone." },
  sparks: { kind: "number", default: 0, min: 0, max: 2000, step: 10, description: "Sparks flying from the surface (0 = none)." },
  sparkColor: { kind: "color", default: "#f3cc94", label: "spark colour", description: "The sparks' colour." },
  sparkSize: { kind: "number", default: 0.05, min: 0.005, max: 0.3, step: 0.005, unit: "widths", label: "spark size", description: "A spark's size in target widths." },
  sparkDistance: { kind: "number", default: 0.3, min: 0, max: 2, step: 0.01, unit: "widths", label: "spark flight", description: "How far a spark flies, in target widths." },
  bloom: { kind: "number", default: 0, min: 0, max: 2, step: 0.05, description: "Bloom over the whole picture while it appears (0 = none)." },
} as const satisfies Record<keyof Appearance, ComponentParamSpec>;

/** A built-in look: the shared parameters with this look's values as their defaults */
const look = (description: string, values: Partial<Appearance>): AppearanceSpec => ({
  description,
  params: Object.fromEntries(Object.entries(PARAMS).map(([name, param]) =>
    [name, name in values ? { ...param, default: values[name as keyof Appearance] } : param])) as Record<string, ComponentParamSpec>,
});

export const APPEARANCES = {
  /** The default: the tuned look of the reveal sandbox */
  inflateDissolve: look(
    "Flat and small on the page, the entity rises, grows and turns half around under a noise dissolve with a gold edge and sparks.",
    {
      duration: 3470, outDuration: 600, mask: "noise", maskEnd: 0.75, edgeStart: 0.06, edgeEnd: 0.015, edgeIntensity: 0.6, glow: 0.6, glowWidth: 0.2,
      heightFrom: 0, scaleFrom: 0, spin: -0.5, shapeEnd: 0.8, sparks: 300,
    },
  ),
  printLift: look(
    "A pale print on the paper lifts off the page and takes its colours.",
    { duration: 1100, outDuration: 600, heightFrom: 0, shapeStart: 0.2, shapeEase: "inOut", fade: 0.15, tint: 0.85, tintEnd: 0.7 },
  ),
  fade: look("Fades in, fades out.", { duration: 600, outDuration: 400, fade: 1 }),
} satisfies Record<string, AppearanceSpec>;

export type BuiltInAppearanceType = keyof typeof APPEARANCES;
export const APPEARANCE_TYPES = Object.keys(APPEARANCES) as BuiltInAppearanceType[];
export const isBuiltInAppearanceType = (value: unknown): value is BuiltInAppearanceType => typeof value === "string" && value in APPEARANCES;

/** The look an experience without `appearance` plays at every moment */
export const DEFAULT_APPEARANCE_TYPE: BuiltInAppearanceType = "inflateDissolve";

export interface AppearanceCheckOptions {
  /** The spec of a type that is not built in (a plugin's) – `undefined` = unknown */
  spec?: (type: string) => AppearanceSpec | undefined;
  /** The plugin looks known – named in the message about an unknown type */
  pluginTypes?: readonly string[];
  /** What an unknown type is: refused (the default – nothing would play it) or allowed (a plugin registers it at load) */
  unknownTypes?: "refuse" | "allow";
  /**
   * What a moment or a parameter the contract does not have is: refused (the default – the build, the API) or ignored
   * (the app's guard: what a newer MINOR adds is left to the newer app)
   */
  unknown?: "refuse" | "ignore";
}

/** The looks plugins registered (type → spec), as the components' registry is: the engine's plugin API and the content build fill it */
const registered = new Map<string, AppearanceSpec>();

export const registerAppearanceSpec = (type: string, spec: AppearanceSpec): void => {
  registered.set(type, spec);
};

export const unregisterAppearanceSpec = (type: string): void => {
  registered.delete(type);
};

/** The spec of a look's type: built in, from `options.spec` (the specs declared plugins recorded) or registered by a plugin */
export const appearanceSpec = (type: string, options: AppearanceCheckOptions = {}): AppearanceSpec | undefined =>
  isBuiltInAppearanceType(type) ? APPEARANCES[type] : options.spec?.(type) ?? registered.get(type);

/** Problems of an experience's `appearance` (the content build, the guard, the API) */
export const appearanceProblems = (raw: unknown, path: string, options: AppearanceCheckOptions = {}): string[] => {
  if (!isObject(raw)) return [`${path}: expected { ${MOMENTS.join("?, ")}? }, each { type, ...parameters }`];
  const problems: string[] = [];
  for (const [moment, value] of Object.entries(raw)) {
    const at = `${path}.${moment}`;
    if (!(MOMENTS as readonly string[]).includes(moment)) {
      if (options.unknown !== "ignore") problems.push(`${at}: not a moment (${MOMENTS.join(", ")})`);
      continue;
    }
    if (!isObject(value)) {
      problems.push(`${at}: expected { type, ...parameters }`);
      continue;
    }
    const { type, ...params } = value;
    if (typeof type !== "string" || !type) {
      problems.push(`${at}.type: expected a look (${APPEARANCE_TYPES.join(", ")})`);
      continue;
    }
    const spec = appearanceSpec(type, options);
    if (!spec) {
      const plugins = options.pluginTypes?.length ? ` or a look of the experience's plugins (${options.pluginTypes.join(", ")})` : "";
      if (options.unknownTypes !== "allow") problems.push(`${at}.type: "${type}" is not one of ${APPEARANCE_TYPES.join(", ")}${plugins}`);
      continue;
    }
    const known = options.unknown === "ignore" ? Object.fromEntries(Object.entries(params).filter(([name]) => name in spec.params)) : params;
    problems.push(...paramProblems(spec.params, known, at, type));
  }
  return problems;
};

/** A look's parameters with every one resolved to its value or the spec's default */
export const resolveAppearance = (spec: AppearanceSpec, data: AppearanceData): Record<string, unknown> => {
  const resolved: Record<string, unknown> = {};
  for (const [name, param] of Object.entries(spec.params)) {
    resolved[name] = data[name] !== undefined ? data[name] : param.default;
  }
  return resolved;
};

/** The look a moment plays: its own, else `found`'s, else the default */
export const momentAppearance = (experience: { appearance?: ExperienceAppearance } | null | undefined, moment: Moment): AppearanceData =>
  experience?.appearance?.[moment] ?? experience?.appearance?.found ?? { type: DEFAULT_APPEARANCE_TYPE };

/**
 * Problems of the parameters of a moment's component (`reveal`, `found`, `lost` – schema 2.30), `params` being the
 * component's keys but `type`: `look` names the look ("" = the experience's), the rest are its parameters, checked against its spec
 * like an experience's look
 */
export const momentComponentProblems = (params: Record<string, unknown>, at: string, options: AppearanceCheckOptions = {}): string[] => {
  const { look, ...values } = params;
  if (look === undefined || look === "") {
    const names = Object.keys(values);
    return names.length ? [`${at}.look: name a look – ${names.join(", ")} are its parameters (${APPEARANCE_TYPES.join(", ")}${options.pluginTypes?.length ? `, ${options.pluginTypes.join(", ")}` : ""})`] : [];
  }
  if (typeof look !== "string") return [`${at}.look: expected a look (${APPEARANCE_TYPES.join(", ")}) or "" for the experience's, got ${JSON.stringify(look)}`];
  const spec = appearanceSpec(look, options);
  if (!spec) {
    if (options.unknownTypes === "allow") return [];
    const plugins = options.pluginTypes?.length ? ` or a look of the experience's plugins (${options.pluginTypes.join(", ")})` : "";
    return [`${at}.look: "${look}" is not one of ${APPEARANCE_TYPES.join(", ")}${plugins}`];
  }
  const known = options.unknown === "ignore" ? Object.fromEntries(Object.entries(values).filter(([name]) => name in spec.params)) : values;
  return paramProblems(spec.params, known, at, look);
};

/** The look a moment's component names: `{ type: look, ...parameters }`; none when it names none (the experience's look plays) */
export const momentComponentLook = (entity: { components?: readonly ComponentData[] } | null | undefined, moment: Moment): AppearanceData | undefined => {
  const component = entity?.components?.find(c => c.type === moment);
  if (!component || typeof component.look !== "string" || !component.look) return undefined;
  const { type: _type, look, ...params } = component;
  return { type: look, ...params };
};

/**
 * The look a node names for a moment with its own components: the moment's, else its `found`'s (for `reveal` and `lost`);
 * none when it names none
 */
export const nodeMomentLook = (node: { components?: readonly ComponentData[] } | null | undefined, moment: Moment): AppearanceData | undefined =>
  momentComponentLook(node, moment) ?? (moment !== "found" ? momentComponentLook(node, "found") : undefined);

/**
 * The look an entity's root node plays at a moment: its own look (`nodeMomentLook`), else the experience's (`momentAppearance`:
 * the experience's moment, its `found`, the default). A child node with a look of its own animates apart, with that one
 */
export const entityMomentAppearance = (
  entity: { components?: readonly ComponentData[] } | null | undefined,
  experience: { appearance?: ExperienceAppearance } | null | undefined,
  moment: Moment,
): AppearanceData =>
  nodeMomentLook(entity, moment) ?? momentAppearance(experience, moment);

/**
 * How long a moment's built-in look takes, in ms (`lost`: its `outDuration`); undefined for a plugin's look, whose
 * duration only its code knows. With `entity`, the look it plays (`entityMomentAppearance`)
 */
export const momentDuration = (
  experience: { appearance?: ExperienceAppearance } | null | undefined,
  moment: Moment,
  entity?: { components?: readonly ComponentData[] } | null,
): number | undefined => {
  const data = entityMomentAppearance(entity, experience, moment);
  if (!isBuiltInAppearanceType(data.type)) return undefined;
  const value = resolveAppearance(APPEARANCES[data.type], data)[moment === "lost" ? "outDuration" : "duration"];
  return typeof value === "number" && Number.isFinite(value) ? value : undefined;
};
