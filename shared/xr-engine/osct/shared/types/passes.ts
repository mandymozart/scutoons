/**
 * Post-processing passes (content schema 2.5): effects over the whole rendered AR image – a bloom, a vignette, a colour
 * grade, a shader of the experience's own. A scene names them as `passes: [{ type, ...parameters }]`, run in order on
 * the image the scene rendered, before it lies over the camera picture; the experience's `passes` are the default for
 * every scene without a list of its own (an empty list turns them off). Like the components, every built-in type
 * is defined once here – its parameters with kind, default, range and a description – so the editor's rows, the
 * content build, the config guard and the engine read one definition. A plugin's pass (`registerPass`) brings its
 * own spec: since 2.21 a `passes` entry may name it where a declared plugin recorded it (`registered.passes`, schema
 * 2.14) – its parameters checked against that spec like a built-in pass's – and the engine builds it from the plugin's
 * factory in its place in the list; a 2.20 app refuses such a type. Since 2.22 a pass may be switched off
 * (`enabled: false`, #233): it stays in the list with its values, the chain skips it; a 2.21 app ignores the field and
 * so runs the pass.
 *
 * Adding a built-in pass: define it in PASSES, implement it in the engine (`engine/src/scene/post/`), document it in
 * docs/content.md.
 */

import { type ComponentParamSpec, paramProblems, type SpecLogic } from "./components";
import { isObject } from "../guards/primitives";

/** Schema 5.1: `actions`, `events`, `uses` (`SpecLogic`) as a component's */
export interface PassSpec extends SpecLogic {
  description: string;
  params: Record<string, ComponentParamSpec>;
}

export const PASSES = {
  /** Bright parts glow: the part of the image above a threshold is blurred and added back as light */
  bloom: {
    description: "Bright parts of the image glow – blurred and added back as light over everything, the camera picture included.",
    params: {
      strength: { kind: "number", default: 1, min: 0, max: 3, step: 0.05, description: "How much light the glow adds." },
      radius: { kind: "number", default: 0.4, min: 0, max: 1, step: 0.05, description: "How far the glow spreads." },
      threshold: { kind: "number", default: 0.85, min: 0, max: 1, step: 0.01, description: "Brightness from which a pixel glows (1 = only brighter than white)." },
    },
  },
  /** The edges of the picture darken (or take a colour) – the camera picture included */
  vignette: {
    description: "The edges of the picture fade to a colour (black by default) – the AR layer and the camera picture alike.",
    params: {
      strength: { kind: "number", default: 0.6, min: 0, max: 1, step: 0.01, description: "How dark the edges get (1 = fully the colour)." },
      radius: { kind: "number", default: 0.6, min: 0, max: 1.5, step: 0.01, description: "Distance from the centre where the fade begins (1 = the sides)." },
      softness: { kind: "number", default: 0.6, min: 0.01, max: 1.5, step: 0.01, description: "How wide the fade is." },
      color: { kind: "color", default: "#000000", description: "The colour the edges fade to." },
    },
  },
  /**
   * Colour grading – of the AR layer alone (`reach: ar`) or of the whole picture (`reach: all`, schema 2.9): in the
   * editor the desk too, on the phone the camera picture (the engine's camera renderer draws it with the same grade)
   */
  grade: {
    description: "Colour grading – exposure, contrast, saturation and a tint. Of the AR layer alone, or of the whole picture with the camera picture.",
    params: {
      reach: {
        kind: "choice", options: ["ar", "all"], default: "ar",
        description: "What is graded: ar = the AR layer alone (the camera picture keeps its colours), all = the whole picture, the camera picture included.",
      },
      exposure: { kind: "number", default: 0, min: -2, max: 2, step: 0.05, unit: "EV", description: "Brighter (+) or darker (−) in stops." },
      contrast: { kind: "number", default: 1, min: 0, max: 2, step: 0.05, description: "1 = as rendered, 0 = flat grey, 2 = hard." },
      saturation: { kind: "number", default: 1, min: 0, max: 2, step: 0.05, description: "1 = as rendered, 0 = black and white." },
      tint: { kind: "color", default: "#ffffff", description: "Multiplied into the colours (white = none)." },
    },
  },
} as const satisfies Record<string, PassSpec>;

export type BuiltInPassType = keyof typeof PASSES;
export const PASS_TYPES = Object.keys(PASSES) as BuiltInPassType[];
export const isBuiltInPassType = (value: unknown): value is BuiltInPassType => typeof value === "string" && value in PASSES;

/**
 * A pass as the content holds it: its type, whether it runs and the parameters set (missing ones use the spec's
 * defaults)
 */
export interface PassData {
  type: string;
  /** Optional (schema 2.22, #233): `false` switches the pass off – it keeps its place and values, the chain skips it */
  enabled?: boolean;
  [param: string]: unknown;
}

/** The fields of a pass that are no parameter of its spec */
export const PASS_FIELDS = ["type", "enabled"] as const;

/** The pass runs: anything but `enabled: false` */
export const isPassEnabled = (pass: Pick<PassData, "enabled">): boolean => pass.enabled !== false;

/** The passes of a list that run, in order – what the chain builds and the phone budget counts */
export const enabledPasses = <T extends Pick<PassData, "enabled">>(passes: readonly T[]): T[] => passes.filter(isPassEnabled);

export interface PassCheckOptions {
  /** The spec of a type that is not built in (a plugin's) – `undefined` = unknown */
  spec?: (type: string) => PassSpec | undefined;
  /** The plugin pass types known – named in the message about an unknown type */
  pluginTypes?: readonly string[];
  /** What an unknown type is: refused (the default – nothing would render it) or allowed (a plugin registers it at load) */
  unknownTypes?: "refuse" | "allow";
  /**
   * What a parameter the spec does not have is: refused (the default – the build, the API) or ignored (the app's
   * guard since 2.9: a parameter a newer MINOR adds is left to the newer app)
   */
  unknownParams?: "refuse" | "ignore";
}

/** The spec of a pass type: built in, or from `options.spec` */
export const passSpec = (type: string, options: PassCheckOptions = {}): PassSpec | undefined =>
  isBuiltInPassType(type) ? PASSES[type] : options.spec?.(type);

/** Problems of a `passes` list (the content build, the guard, the API) */
export const passProblems = (raw: unknown, path: string, options: PassCheckOptions = {}): string[] => {
  if (!Array.isArray(raw)) return [`${path}: expected a list of passes`];
  const problems: string[] = [];
  raw.forEach((pass, i) => {
    const at = `${path}[${i}]`;
    if (!isObject(pass)) {
      problems.push(`${at}: expected { type, ...parameters }`);
      return;
    }
    const { type, enabled, ...params } = pass;
    if (typeof type !== "string" || !type) {
      problems.push(`${at}.type: expected a pass type (${PASS_TYPES.join(", ")})`);
      return;
    }
    if (enabled !== undefined && typeof enabled !== "boolean") problems.push(`${at}.enabled: expected true or false, got ${JSON.stringify(enabled)}`);
    const spec = passSpec(type, options);
    if (!spec) {
      const plugins = options.pluginTypes?.length ? ` or a pass of the experience's plugins (${options.pluginTypes.join(", ")})` : "";
      if (options.unknownTypes !== "allow") problems.push(`${at}.type: "${type}" is not one of ${PASS_TYPES.join(", ")}${plugins}`);
      return;
    }
    const known = options.unknownParams === "ignore" ? Object.fromEntries(Object.entries(params).filter(([name]) => name in spec.params)) : params;
    problems.push(...paramProblems(spec.params, known, at, type));
  });
  return problems;
};

/** A pass's parameters with every one resolved to its value or the spec's default */
export const resolvePass = (spec: PassSpec, pass: PassData): Record<string, unknown> => {
  const resolved: Record<string, unknown> = {};
  for (const [name, param] of Object.entries(spec.params)) {
    resolved[name] = pass[name] !== undefined ? pass[name] : param.default;
  }
  return resolved;
};

/**
 * The phone budget of a chain (#86, agents/MEMORY.md 2026-10-29): every pass is at least one full-screen
 * render-to-texture plus one full-screen shader – a plugin's pass counts like a built-in one (its feedback buffers come
 * on top) – and the bloom is several (five blur levels at half resolution). Up to one bloom and three passes in all run on
 * a phone; more is a warning (the content build, the editor), never an error – the S22 measurement is still open. A
 * pass that is off costs nothing and is not counted.
 */
export const PASS_BUDGET = { passes: 3, blooms: 1 } as const;

/** What a `passes` list asks beyond the phone budget, in words; null within it */
export const passBudgetWarning = (list: readonly PassData[]): string | null => {
  const passes = enabledPasses(list);
  const blooms = passes.filter(pass => pass.type === "bloom").length;
  const over = [
    ...(passes.length > PASS_BUDGET.passes ? [`${passes.length} passes (the phone budget is ${PASS_BUDGET.passes})`] : []),
    ...(blooms > PASS_BUDGET.blooms ? [`${blooms} blooms (one fits)`] : []),
  ];
  return over.length ? `${over.join(" and ")} – every pass is a full-screen render on the phone` : null;
};
