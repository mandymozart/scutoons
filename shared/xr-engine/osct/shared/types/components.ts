/**
 * Components of a node (content schema 2.2, the first piece of the plugin API – agents/research/2026-10-28-entity-
 * components.md): a node carries `components: [{ type, ...parameters }]`, each applied to it by the engine's
 * registry. Like the filters, every component type is defined once – its parameters with kind, default, range
 * and a description, and the node types it applies to – so the editor's panel, the content build, the config guard
 * and the engine read one definition.
 *
 * Two kinds of definition share one registry (`componentSpec`): the **built-in** components in COMPONENTS (the
 * engine implements them in `engine/src/scene/entities.ts`), and the components and material layers a **plugin**
 * registers at runtime (`registerComponentSpec`, called by the engine's plugin API – `engine/src/scene/plugins.ts`).
 * A type nobody registered is refused by the build and the guard, unless the configuration declares plugins: then
 * the guard checks it against the specs the plugins recorded (schema 2.14, `PluginData.registered`) and refuses a type
 * none of them registers – or, when a plugin recorded nothing (an older build), lets it pass (`unknownTypes: "allow"`):
 * the plugin registers it when it loads.
 *
 * Adding a built-in component: define it in COMPONENTS, implement it in the engine, document it in docs/content.md.
 * The editor shows its rows without panel code. One built-in component is not made of parameters: `behaviour` (schema
 * 2.19) holds a list of rules, its grammar and checks are its own (`./behaviour.ts`) and the editor has a rule editor for it.
 * `reveal`, `found` and `lost` (schema 2.30) are the entity's own looks for the three moments (`./appearance.ts`): their
 * parameters are the chosen look's, so their grammar and checks are its own too.
 */

import type { NodeType } from "./game-config";
import { type AppearanceCheckOptions, isMoment, momentComponentProblems } from "./appearance";
import { BEHAVIOUR_TYPE, behaviourProblems } from "./behaviour";
import { CAMEL_CASE_NAME, isObject } from "../guards/primitives";

/**
 * A parameter of a spec (a component's, a material layer's, a pass's, a light's, a plugin node type's, a material's, a
 * procedural texture's): its kind with default and range, a `description` (the editor's help) and an optional `label`
 * (schema 2.15, plugin API 1.3) – the few words the editor's row shows. Without a label the row shows the name split
 * into words (`paramLabel`). A `texture` (schema 2.17, plugin API 1.4) takes a picture: a file (a Library image or
 * video, a file of the content) or a procedural texture a plugin registers (`registerTexture`) – its value is a
 * `TextureValue` (below); the engine hands the plugin's code a three.js texture. A `model` (schema 2.28, plugin API
 * 1.7) takes a model file: a Library model's URL or a `/assets/content/…` path (in the content's YAML a `.glb` / `.gltf`
 * next to the node, which the content build resolves to its path), `""` = none – its value is that string, and the
 * plugin's code loads it with `api.loadModel(value)`.
 *
 * A number's `min` and `max` are **hard** limits: the guard and the editor refuse a value outside them, the engine keeps the
 * value it had (a warning). Schema 5.1 (#334, plugin API 3.1): its optional `slider` `[lo, hi]` is the **soft** range – the
 * span the editor's control offers first, inside min … max – and `integer: true` takes whole numbers only (min, max and
 * default whole too). The plugin API's builders (`api.p`, `./spec-builders.ts`) write these objects.
 */
export type ComponentParamSpec = ParamText & (
  | { kind: "color"; default: string }
  | { kind: "number"; default: number; min: number; max: number; step?: number; unit?: string; slider?: readonly [number, number]; integer?: boolean }
  | { kind: "choice"; options: readonly string[]; default: string }
  | { kind: "boolean"; default: boolean }
  | { kind: "string"; default: string }
  | { kind: "texture"; default: string }
  | { kind: "model"; default: string }
);

/**
 * A texture parameter's value (schema 2.17): `""` = none; a procedural texture's type (`caustics` – camelCase, what a
 * plugin registers with `registerTexture`, with its defaults); `{ type, …parameters }` = a procedural texture with other
 * values; any other string = a file (a Library file's URL or a `/assets/content/…` path – in the content's YAML a file
 * next to the node, which the content build resolves to its path).
 */
export type TextureValue = string | { type: string; [param: string]: unknown };

/** A procedural texture's type: camelCase letters and digits (a file always has a dot or a slash) */
export const TEXTURE_TYPE_PATTERN = CAMEL_CASE_NAME;

const TEXTURE_VALUE_EXPECTED = "expected a texture: a file, a procedural texture's type or { type, …parameters }";

export const isTextureValue = (value: unknown): value is TextureValue =>
  typeof value === "string" ||
  (isObject(value) && typeof value.type === "string" && TEXTURE_TYPE_PATTERN.test(value.type));

/** What a texture value names: nothing, a file, or a procedural texture (its type and parameters) */
export type TextureSource =
  | { kind: "none" }
  | { kind: "file"; src: string }
  | { kind: "procedural"; type: string; params: Record<string, unknown> };

export const textureSource = (value: unknown): TextureSource => {
  if (typeof value === "string") {
    if (!value.trim()) return { kind: "none" };
    if (TEXTURE_TYPE_PATTERN.test(value)) return { kind: "procedural", type: value, params: {} };
    return { kind: "file", src: value };
  }
  if (typeof value === "object" && value !== null && isTextureValue(value)) {
    const { type, ...params } = value as { type: string; [param: string]: unknown };
    return { kind: "procedural", type, params };
  }
  return { kind: "none" };
};

const MODEL_VALUE_EXPECTED = "expected a model: a file (.glb, .gltf) or \"\" for none";

/** What every parameter says about itself, whatever its kind */
export interface ParamText {
  /**
   * The few words the editor's row shows (schema 2.15, plugin API 1.3), e.g. `"speed"` for `degreesPerSecond` – 1 to
   * 24 characters (`PARAM_LABEL_MAX`). Without one the row shows the name split into words ("degrees per second").
   */
  label?: string;
  /** What the parameter does – the row's help in the editor's Info view */
  description: string;
}

/** The longest label a parameter may carry (the editor's label column is narrow; the description is the help) */
export const PARAM_LABEL_MAX = 24;

/** A parameter's name as words: `degreesPerSecond` → "degrees per second", `playOn` → "play on", `uvScale2` → "uv scale 2" */
export const paramWords = (name: string): string =>
  name
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .replace(/([A-Z]+)([A-Z][a-z])/g, "$1 $2")
    .replace(/([a-zA-Z])([0-9])/g, "$1 $2")
    .toLowerCase();

/** What the editor's row shows for a parameter: its spec's label, else its name as words */
export const paramLabel = (name: string, spec?: { label?: string }): string => spec?.label?.trim() || paramWords(name);

/**
 * What a spec says beyond its parameters (schema 5.1, #334, plugin API 3.1) – what the editor's Logic view draws as ports:
 * - `actions`: the rule verbs it answers on its node (`play`, `pause`, `stop`, `show`, `hide` – `SPEC_ACTIONS`): a rule's
 *   `play: <node>` reaches the component's `onAction("play")` beside the node's own clip;
 * - `events`: the scene events it emits (`ended`) – through its context's `emit`, on the bus by name;
 * - `uses`: what its code listens to, emits, reads and sets of the bus and the variables, by name – the Logic view joins it to
 *   the rules (it reads the plugin's source only for a plugin that declares nothing).
 * The names are camelCase; an action is one of the node verbs, an event never a core event. An older app ignores the fields.
 */
export interface SpecLogic {
  actions?: readonly string[];
  events?: readonly string[];
  uses?: SpecUses;
}

/** A spec's bus use: event names it listens to and emits, variable names it reads and sets */
export interface SpecUses {
  listens?: readonly string[];
  emits?: readonly string[];
  reads?: readonly string[];
  sets?: readonly string[];
}

/** The rule verbs a spec may answer: the behaviour's node verbs (`./behaviour.ts` `BEHAVIOUR_NODE_ACTIONS`) */
export const SPEC_ACTIONS = ["play", "pause", "stop", "show", "hide"] as const;
export type SpecAction = (typeof SPEC_ACTIONS)[number];

/** The engine's core events (`./behaviour.ts` `BEHAVIOUR_CORE_EVENTS`): a spec listens to them, never emits them */
const CORE_EVENT_NAMES = ["targetFound", "targetLost", "targetUnlocked", "targetTapped", "sceneChanged", "arState", "varChanged"];
/** A node's own events (`./behaviour.ts` `BEHAVIOUR_NODE_EVENTS`): its rules', never a spec's to emit */
const NODE_EVENT_NAMES = ["found", "lost", "tap", "unlocked"];

/** Problems of a spec's `actions`, `events` and `uses` (any spec: a component's, a node type's, a pass's, a material's …) */
export const specLogicProblems = (type: string, spec: Record<string, unknown>): string[] => {
  const problems: string[] = [];
  const names = (value: unknown, at: string, check?: (name: string) => string | null) => {
    if (value === undefined) return;
    if (!Array.isArray(value)) {
      problems.push(`${at}: expected a list of names`);
      return;
    }
    const seen = new Set<string>();
    for (const name of value) {
      if (typeof name !== "string" || !CAMEL_CASE_NAME.test(name)) {
        problems.push(`${at}: ${JSON.stringify(name)} is no name (camelCase letters and digits)`);
        continue;
      }
      if (seen.has(name)) problems.push(`${at}: ${name} is listed twice`);
      seen.add(name);
      const problem = check?.(name);
      if (problem) problems.push(`${at}: ${problem}`);
    }
  };
  const notCore = (name: string) => (CORE_EVENT_NAMES.includes(name) ? `${name} is the engine's own event – a spec listens to it, never emits it` : null);
  names(spec.actions, `${type}.actions`, name => ((SPEC_ACTIONS as readonly string[]).includes(name) ? null : `${name} is no rule verb a node answers (${SPEC_ACTIONS.join(", ")})`));
  names(spec.events, `${type}.events`, name => notCore(name) ?? (NODE_EVENT_NAMES.includes(name) ? `${name} is a node's own event – a spec emits scene events` : null));
  if (spec.uses !== undefined) {
    if (!isObject(spec.uses)) problems.push(`${type}.uses: expected { listens?, emits?, reads?, sets? }`);
    else {
      const uses = spec.uses;
      Object.keys(uses).filter(key => !["listens", "emits", "reads", "sets"].includes(key)).forEach(key => problems.push(`${type}.uses.${key}: unknown field (listens, emits, reads, sets)`));
      names(uses.listens, `${type}.uses.listens`);
      names(uses.emits, `${type}.uses.emits`, notCore);
      names(uses.reads, `${type}.uses.reads`);
      names(uses.sets, `${type}.uses.sets`);
    }
  }
  return problems;
};

/** A spec's declared logic as plain lists (missing ones empty) */
export const specLogicOf = (spec: SpecLogic | undefined): { actions: string[]; events: string[]; listens: string[]; emits: string[]; reads: string[]; sets: string[] } => ({
  actions: [...(spec?.actions ?? [])], events: [...(spec?.events ?? [])],
  listens: [...(spec?.uses?.listens ?? [])], emits: [...(spec?.uses?.emits ?? [])], reads: [...(spec?.uses?.reads ?? [])], sets: [...(spec?.uses?.sets ?? [])],
});

/** Does a spec declare any logic (actions, events or bus use)? */
export const declaresLogic = (spec: SpecLogic | undefined): boolean =>
  !!spec && (spec.actions !== undefined || spec.events !== undefined || spec.uses !== undefined);

export interface ComponentSpec extends SpecLogic {
  description: string;
  /** The node types it applies to (none = every type) – built-in types or a plugin's (`registerNodeType`) */
  nodeTypes?: readonly NodeType[];
  params: Record<string, ComponentParamSpec>;
}

/** The `look` of a moment's component: the look's type, or "" for the experience's – what the other parameters belong to */
const MOMENT_LOOK = {
  kind: "string", default: "", label: "look",
  description: "The look this entity appears with – a built-in one (inflateDissolve, printLift, fade) or a plugin's – in place of the experience's. Empty = the experience's. The look's own parameters follow.",
} as const;

export const COMPONENTS = {
  /** A model's glTF animation: which clip plays, when it starts and stops, how it loops – the editor's timeline */
  animation: {
    description: "Plays one of the model's animation clips (glTF). Without this component every clip plays in a loop.",
    nodeTypes: ["model"],
    params: {
      clip: { kind: "string", default: "", label: "clip", description: "The clip's name – empty plays the first clip." },
      playOn: {
        kind: "choice", options: ["found", "tap", "never"], default: "found", label: "plays on",
        description: "When the clip starts: when the page is found, when the entity is tapped, or never (the editor's timeline only).",
      },
      stopOn: {
        kind: "choice", options: ["lost", "never"], default: "lost", label: "stops on",
        description: "When it stops: when the page is lost, or never (it keeps its time while the page is away).",
      },
      loop: { kind: "boolean", default: true, label: "loop", description: "Repeat the clip; off = play it once and hold the last frame." },
      speed: { kind: "number", default: 1, min: 0.1, max: 4, step: 0.1, unit: "×", label: "speed", description: "Playback speed." },
    },
  },
  /**
   * Rules without code (schema 2.19): on an event, when the variables match, do actions – its `rules` are no parameters
   * (no rows), their grammar is `./behaviour.ts`
   */
  /**
   * The entity's own look for a moment (schema 2.30, `./appearance.ts`), in place of the experience's `appearance` – on the
   * any node – a node with a look of its own animates its subtree with it. Not made of parameters of its own: after `look` come the look's parameters (a built-in look's or
   * a plugin's), checked against its spec.
   */
  reveal: {
    description: "How this entity appears the first time its target is found (the entry is unlocked with it): a look – built in or a plugin's – with its parameters, in place of the experience's. On any node: its subtree animates with it.",
    params: { look: MOMENT_LOOK },
  },
  found: {
    description: "How this entity appears every time its target is found after the first (and the first too, while it has no reveal): a look with its parameters, in place of the experience's. On any node: its subtree animates with it.",
    params: { look: MOMENT_LOOK },
  },
  lost: {
    description: "How this entity disappears when its target is lost: a look with its parameters – it plays rewound, in the look's disappear time – in place of the experience's. On any node: its subtree animates with it.",
    params: { look: MOMENT_LOOK },
  },
  behaviour: {
    description: "Rules without code: on an event (found, lost, tap, unlocked or a named event), when variables match, do actions – play, pause or stop a clip, show or hide a node, set a variable, emit an event.",
    params: {},
  },
} as const satisfies Record<string, ComponentSpec>;

/** The built-in component types */
export type ComponentType = keyof typeof COMPONENTS;
export const COMPONENT_TYPES = Object.keys(COMPONENTS) as ComponentType[];

/** A component type: `camelCase`, letters and digits */
export const COMPONENT_TYPE_PATTERN = CAMEL_CASE_NAME;

/** Components a plugin registered (type → spec); the built-in ones win on a clash */
const registered = new Map<string, ComponentSpec>();

/** A plugin's component or material layer: its spec joins the registry the panel, the build and the guard read */
export const registerComponentSpec = (type: string, spec: ComponentSpec): void => {
  registered.set(type, spec);
};

export const unregisterComponentSpec = (type: string): void => {
  registered.delete(type);
};

/** The definition of a component type – built-in or registered – or undefined */
export const componentSpec = (type: string): ComponentSpec | undefined =>
  (COMPONENTS as Record<string, ComponentSpec>)[type] ?? registered.get(type);

/** Every component type known right now: the built-in ones, then the registered ones */
export const componentTypes = (): string[] => [...COMPONENT_TYPES, ...[...registered.keys()].filter(type => !(type in COMPONENTS))];

/** A component as the content holds it: its type and the parameters set (missing ones use the defaults) */
export interface ComponentData {
  type: string;
  [param: string]: unknown;
}

/** Problems of the parameters set for a spec (a component, a pass): unknown names, wrong kinds, numbers out of range */
export const paramProblems = (specs: Record<string, ComponentParamSpec>, params: Record<string, unknown>, at: string, type: string): string[] => {
  const problems: string[] = [];
  for (const [name, value] of Object.entries(params)) {
    const param = specs[name];
    if (!param) {
      problems.push(`${at}.${name}: unknown parameter of ${type} (${Object.keys(specs).join(", ")})`);
    } else if (param.kind === "color" && !(typeof value === "string" && /^#(?:[0-9a-f]{3}|[0-9a-f]{6})$/i.test(value))) {
      problems.push(`${at}.${name}: expected a color "#rrggbb", got ${JSON.stringify(value)}`);
    } else if (param.kind === "choice" && !param.options.includes(value as string)) {
      problems.push(`${at}.${name}: expected one of ${param.options.join(", ")}, got ${JSON.stringify(value)}`);
    } else if (param.kind === "number" && numberProblem(param, value)) {
      problems.push(`${at}.${name}: ${numberProblem(param, value)}`);
    } else if (param.kind === "boolean" && typeof value !== "boolean") {
      problems.push(`${at}.${name}: expected true or false, got ${JSON.stringify(value)}`);
    } else if (param.kind === "string" && typeof value !== "string") {
      problems.push(`${at}.${name}: expected a string, got ${JSON.stringify(value)}`);
    } else if (param.kind === "texture" && !isTextureValue(value)) {
      problems.push(`${at}.${name}: ${TEXTURE_VALUE_EXPECTED}, got ${JSON.stringify(value)}`);
    } else if (param.kind === "model" && typeof value !== "string") {
      problems.push(`${at}.${name}: ${MODEL_VALUE_EXPECTED}, got ${JSON.stringify(value)}`);
    }
  }
  return problems;
};

/** Why a value breaks a number parameter's hard limits (min, max, whole), or null */
export const numberProblem = (param: Extract<ComponentParamSpec, { kind: "number" }>, value: unknown): string | null =>
  typeof value === "number" && value >= param.min && value <= param.max && (!param.integer || Number.isInteger(value))
    ? null
    : `expected ${param.integer ? "a whole number" : "a number"} from ${param.min} to ${param.max}, got ${JSON.stringify(value)}`;

const isColor = (value: unknown) => typeof value === "string" && /^#(?:[0-9a-f]{3}|[0-9a-f]{6})$/i.test(value);

/**
 * Problems of a spec a plugin registers (the editor's Run and the content build check it before the registry
 * takes it): the type's name, every parameter's kind and default
 */
export const componentSpecProblems = (type: string, spec: unknown): string[] => {
  const problems: string[] = [];
  if (!COMPONENT_TYPE_PATTERN.test(type)) problems.push(`"${type}" is not a component type (camelCase letters and digits, e.g. tint)`);
  if (type in COMPONENTS) problems.push(`"${type}" is a built-in component`);
  if (typeof spec !== "object" || spec === null) return [...problems, `${type}: the spec must be an object with description and params`];
  const s = spec as Record<string, unknown>;
  if (typeof s.description !== "string") problems.push(`${type}: description must be a string`);
  if (s.nodeTypes !== undefined && !(Array.isArray(s.nodeTypes) && s.nodeTypes.every(t => typeof t === "string"))) problems.push(`${type}: nodeTypes must be a list of node types`);
  problems.push(...specLogicProblems(type, s));
  if (!isObject(s.params)) return [...problems, `${type}: params must be an object`];
  return [...problems, ...paramSpecProblems(type, s.params)];
};

/** Problems of a spec's parameters (a component's, a layer's, a pass's, a plugin node type's): names, kinds, defaults */
export const paramSpecProblems = (type: string, params: Record<string, unknown>): string[] => {
  const problems: string[] = [];
  for (const [name, raw] of Object.entries(params)) {
    const at = `${type}.${name}`;
    if (!CAMEL_CASE_NAME.test(name)) problems.push(`${at}: a parameter name is camelCase letters and digits`);
    const p = raw as Partial<Record<string, unknown>> | null;
    if (typeof p !== "object" || p === null) {
      problems.push(`${at}: expected { kind, default, description }`);
      continue;
    }
    if (typeof p.description !== "string") problems.push(`${at}: description must be a string`);
    if (p.label !== undefined && !(typeof p.label === "string" && p.label.trim() && p.label.length <= PARAM_LABEL_MAX)) {
      problems.push(`${at}: label must be a short text (1–${PARAM_LABEL_MAX} characters)`);
    }
    switch (p.kind) {
      case "color": if (!isColor(p.default)) problems.push(`${at}: default must be a color "#rrggbb"`); break;
      case "number": {
        if (typeof p.default !== "number" || typeof p.min !== "number" || typeof p.max !== "number" || !(p.min <= p.default && p.default <= p.max)) {
          problems.push(`${at}: a number needs min ≤ default ≤ max`);
          break;
        }
        const [min, max, slider] = [p.min, p.max, p.slider];
        if (slider !== undefined && !(Array.isArray(slider) && slider.length === 2 && slider.every(n => typeof n === "number") && min <= slider[0] && slider[0] < slider[1] && slider[1] <= max)) {
          problems.push(`${at}: slider is the soft range [lo, hi] with min ≤ lo < hi ≤ max`);
        }
        if (p.integer !== undefined && typeof p.integer !== "boolean") problems.push(`${at}: integer must be true or false`);
        if (p.integer === true && ![min, max, p.default].every(n => Number.isInteger(n))) problems.push(`${at}: an integer needs whole min, max and default`);
        break;
      }
      case "choice":
        if (!(Array.isArray(p.options) && p.options.length && p.options.every(o => typeof o === "string") && p.options.includes(p.default as string))) {
          problems.push(`${at}: a choice needs options (strings) and a default among them`);
        }
        break;
      case "boolean": if (typeof p.default !== "boolean") problems.push(`${at}: default must be true or false`); break;
      case "string": if (typeof p.default !== "string") problems.push(`${at}: default must be a string`); break;
      case "texture": if (typeof p.default !== "string") problems.push(`${at}: a texture's default is a string (empty = none, or a procedural texture's type)`); break;
      case "model": if (typeof p.default !== "string") problems.push(`${at}: a model's default is a string (empty = none, or a file)`); break;
      default: problems.push(`${at}: kind must be color, number, choice, boolean, string, texture or model`);
    }
  }
  return problems;
};

export interface ComponentCheckOptions {
  /**
   * What to do with a type the registry does not know: refuse it (the default – the build, a configuration without
   * plugins), or allow it with its parameters unchecked (a configuration that declares plugins registers it at load)
   */
  unknownTypes?: "refuse" | "allow";
  /**
   * Specs beyond the registry's – the ones the declared plugins recorded (schema 2.14, `PluginData.registered`): a
   * type found here is checked like a registered one
   */
  specs?: ReadonlyMap<string, ComponentSpec>;
  /** How a moment's look is checked (`reveal`, `found`, `lost` – schema 2.30): the plugin looks recorded, what an unknown one is */
  appearance?: AppearanceCheckOptions;
}

/** Problems of a node's `components` list (the content build, the guard); `nodeType` checks that each applies */
export const componentProblems = (raw: unknown, path: string, nodeType?: string, options: ComponentCheckOptions = {}): string[] => {
  if (!Array.isArray(raw)) return [`${path}: expected a list of components`];
  const problems: string[] = [];
  raw.forEach((component, i) => {
    const at = `${path}[${i}]`;
    if (!isObject(component)) {
      problems.push(`${at}: expected { type, ...parameters }`);
      return;
    }
    const { type, ...params } = component;
    const spec = typeof type === "string" ? componentSpec(type) ?? options.specs?.get(type) : undefined;
    if (!spec) {
      if (options.unknownTypes === "allow" && typeof type === "string" && COMPONENT_TYPE_PATTERN.test(type)) return;
      const known = [...new Set([...componentTypes(), ...(options.specs?.keys() ?? [])])];
      problems.push(`${at}.type: "${type}" is not one of ${known.join(", ")}`);
      return;
    }
    if (nodeType && spec.nodeTypes && !(spec.nodeTypes as readonly string[]).includes(nodeType)) {
      problems.push(`${at}: ${type} applies to ${spec.nodeTypes.join(", ")} nodes, not to a ${nodeType}`);
    }
    // The behaviour's rules are no parameters: their own grammar
    if (type === BEHAVIOUR_TYPE) {
      problems.push(...behaviourProblems(params, at));
      return;
    }
    // A moment's parameters are its look's: checked against the look's spec
    if (isMoment(type)) {
      problems.push(...momentComponentProblems(params, at, options.appearance));
      return;
    }
    for (const [name, value] of Object.entries(params)) {
      const param = spec.params[name];
      if (!param) {
        problems.push(`${at}.${name}: unknown parameter of ${type} (${Object.keys(spec.params).join(", ")})`);
        continue;
      }
      const problem = paramProblem(param, value);
      if (problem) problems.push(`${at}.${name}: ${problem}`);
    }
  });
  return problems;
};

/** Why a value does not fit a parameter's spec (a component's, a light's), or null when it does */
export const paramProblem = (param: ComponentParamSpec, value: unknown): string | null => {
  if (param.kind === "color" && !isColor(value)) return `expected a color "#rrggbb", got ${JSON.stringify(value)}`;
  if (param.kind === "choice" && !param.options.includes(value as string)) {
    return `expected one of ${param.options.join(", ")}, got ${JSON.stringify(value)}`;
  }
  if (param.kind === "number") {
    const problem = numberProblem(param, value);
    if (problem) return problem;
  }
  if (param.kind === "boolean" && typeof value !== "boolean") return `expected true or false, got ${JSON.stringify(value)}`;
  if (param.kind === "string" && typeof value !== "string") return `expected a string, got ${JSON.stringify(value)}`;
  if (param.kind === "texture" && !isTextureValue(value)) return `${TEXTURE_VALUE_EXPECTED}, got ${JSON.stringify(value)}`;
  if (param.kind === "model" && typeof value !== "string") return `${MODEL_VALUE_EXPECTED}, got ${JSON.stringify(value)}`;
  return null;
};

/** A component's parameters with every one resolved to its value or the spec's default */
export const resolveParams = (spec: ComponentSpec, component: ComponentData): Record<string, unknown> => {
  const resolved: Record<string, unknown> = {};
  for (const [name, param] of Object.entries(spec.params)) {
    resolved[name] = component[name] !== undefined ? component[name] : param.default;
  }
  return resolved;
};

/** The component of a type on a node, with every parameter resolved to its value or default */
export const resolveComponent = <T extends string>(components: ComponentData[] | undefined, type: T):
  (Record<string, unknown> & { type: T }) | undefined => {
  const found = components?.find(c => c.type === type);
  const spec = componentSpec(type);
  if (!found || !spec) return undefined;
  return { ...resolveParams(spec, found), type };
};
