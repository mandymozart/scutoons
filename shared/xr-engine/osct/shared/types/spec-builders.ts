/**
 * Typed spec builders (content schema 5.1, plugin API 3.1 – #334, agents/research/2026-11-20-logic-nodes-shared-concept.md
 * §5.9 "schema first, types inferred"): a plugin writes its parameters once, as runtime values, with small builders, and the
 * plugin API's generics infer the static types from them – the handler's `params.volume` is a `number`, a choice is the
 * union of its options, the declared `actions` and `events` are literal unions. The builders write **the same
 * `ComponentParamSpec` objects** a plugin writes by hand (`./components.ts`), so a hand-written spec keeps working unchanged
 * and nothing about them reaches the content but the plain object:
 *
 *   const p = api.p;
 *   api.registerComponent("hum", {
 *     description: "Hums while the page is found.",
 *     params: {
 *       volume: p.number({ min: 0, max: 1, default: 0.6, slider: [0, 0.8], description: "How loud." }),
 *       voice: p.choice(["low", "high"], { description: "Which voice." }),
 *     },
 *     actions: ["play", "pause"],
 *     events: ["ended"],
 *   }, ({ params, emit }) => ({ onAction: action => …, … }));   // params.volume: number, params.voice: "low" | "high"
 *
 * `min` / `max` are the hard limits (refused outside – the guard, the editor; the engine keeps its value), `slider` the soft
 * range of the editor's control. `p.integer` is a number of whole steps. `p.file(kind)` is the parameter of a file: `model`
 * a model parameter, `image` / `video` a texture parameter (a picture – a Library image or video, or a procedural texture).
 * Pure: no engine, no three.js – the engine hands the builders out as `api.p`, the content build's recording run the same.
 */

import type { ComponentParamSpec, ParamText } from "./components";

/** What every builder takes besides its own options: the help text and the row's few words */
export interface ParamTextOptions {
  description: string;
  label?: string;
}

export interface NumberOptions extends ParamTextOptions {
  /** The hard limits: a value outside is refused */
  min: number;
  max: number;
  /** Default: `min` */
  default?: number;
  step?: number;
  unit?: string;
  /** The soft range `[lo, hi]` the editor's control offers, inside min … max */
  slider?: readonly [number, number];
}

export type NumberParam = Extract<ComponentParamSpec, { kind: "number" }>;
export type BooleanParam = Extract<ComponentParamSpec, { kind: "boolean" }>;
export type StringParam = Extract<ComponentParamSpec, { kind: "string" }>;
export type ColorParam = Extract<ComponentParamSpec, { kind: "color" }>;
export type TextureParam = Extract<ComponentParamSpec, { kind: "texture" }>;
export type ModelParam = Extract<ComponentParamSpec, { kind: "model" }>;
/** A choice whose options stay their literal types (the handler sees their union) */
export type ChoiceParam<O extends readonly string[]> = ParamText & { kind: "choice"; options: O; default: O[number] };

/** The kinds of file `p.file` takes */
export type FileKind = "image" | "video" | "model";

/** The fields every builder writes: the description, and the label when there is one */
const text = (options: ParamTextOptions): ParamText => ({ description: options.description, ...(options.label !== undefined ? { label: options.label } : {}) });

/** The optional fields that are set (a spec never carries `undefined`) */
const set = <T extends Record<string, unknown>>(fields: T): Partial<T> =>
  Object.fromEntries(Object.entries(fields).filter(([, value]) => value !== undefined)) as Partial<T>;

const numberParam = (options: NumberOptions, integer: boolean): NumberParam => ({
  kind: "number",
  default: options.default ?? options.min,
  min: options.min,
  max: options.max,
  ...set({ step: options.step ?? (integer ? 1 : undefined), unit: options.unit, slider: options.slider }),
  ...(integer ? { integer: true } : {}),
  ...text(options),
});

/** The parameter builders (`api.p`) */
export const PARAM_BUILDERS = {
  /** A number with hard limits `min` … `max`, an optional soft `slider` range, step and unit */
  number: (options: NumberOptions): NumberParam => numberParam(options, false),
  /** A whole number (`integer: true`, step 1 unless given): min, max and default whole too */
  integer: (options: NumberOptions): NumberParam => numberParam(options, true),
  /** True or false (default false) */
  boolean: (options: ParamTextOptions & { default?: boolean }): BooleanParam => ({ kind: "boolean", default: options.default ?? false, ...text(options) }),
  /** A text (default "") */
  string: (options: ParamTextOptions & { default?: string }): StringParam => ({ kind: "string", default: options.default ?? "", ...text(options) }),
  /** One of the options (default the first); pass them `as const` or inline, and the handler sees their union */
  choice: <const O extends readonly [string, ...string[]]>(options: O, more: ParamTextOptions & { default?: O[number] }): ChoiceParam<O> =>
    ({ kind: "choice", options, default: more.default ?? options[0], ...text(more) }),
  /** A colour `#rrggbb` (default white) */
  color: (options: ParamTextOptions & { default?: string }): ColorParam => ({ kind: "color", default: options.default ?? "#ffffff", ...text(options) }),
  /** A picture: a Library image or video, a content file or a procedural texture (default "" = none) */
  texture: (options: ParamTextOptions & { default?: string }): TextureParam => ({ kind: "texture", default: options.default ?? "", ...text(options) }),
  /** A model file (default "" = none) – load it with `api.loadModel(value)` */
  model: (options: ParamTextOptions & { default?: string }): ModelParam => ({ kind: "model", default: options.default ?? "", ...text(options) }),
  /** A file: `model` → a model parameter, `image` / `video` → a texture parameter */
  file: <K extends FileKind>(kind: K, options: ParamTextOptions & { default?: string }): K extends "model" ? ModelParam : TextureParam =>
    ({ kind: kind === "model" ? "model" : "texture", default: options.default ?? "", ...text(options) }) as K extends "model" ? ModelParam : TextureParam,
};

export type ParamBuilders = typeof PARAM_BUILDERS;

// ── Types inferred from a spec ─────────────────────────────────────────────────────────────────────────

/**
 * The value a handler gets for one parameter: a number, true / false, a choice's union, a string (colour, text, model file)
 * – `Tex` for a texture (the engine hands a three.js texture or null)
 */
export type ParamValueOf<P, Tex = unknown> =
  P extends { kind: "number" } ? number
    : P extends { kind: "boolean" } ? boolean
      : P extends { kind: "choice"; options: readonly (infer O)[] } ? (string extends O ? string : O)
        : P extends { kind: "texture" } ? Tex
          : string;

/**
 * A handler's `params` for a spec: one typed field per parameter – and still indexable by any name (`params[name]` is
 * `unknown`), so code written against `Record<string, unknown>` keeps compiling. A spec typed only as `ComponentSpec` (its
 * parameter names unknown) gives `Record<string, unknown>`, as before the builders.
 */
export type ParamsOf<S, Tex = unknown> = S extends { params: infer P }
  ? string extends keyof P ? Record<string, unknown> : { -readonly [K in keyof P]: ParamValueOf<P[K], Tex> } & Record<string, unknown>
  : Record<string, unknown>;

/** The items of a declared list: its literal union, `string` for a list typed only as `string[]`, never without one */
type ItemsOf<L> = L extends readonly (infer A)[] ? A & string : never;

/** The literal union of a spec's declared `actions` (never without any) */
export type ActionsOf<S> = S extends { actions?: infer L } ? ItemsOf<L> : never;

/** The literal union of a spec's declared `events` (never without any) */
export type EventsOf<S> = S extends { events?: infer L } ? ItemsOf<L> : never;
