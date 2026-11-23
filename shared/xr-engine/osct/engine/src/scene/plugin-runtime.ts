import { valueFits, type VariableInfo } from "@shared/types/variables";
import { BEHAVIOUR_CORE_EVENTS, BEHAVIOUR_NAME_PATTERN, BEHAVIOUR_SCOPES, type BehaviourScope, type BehaviourValue, type Scene } from "../types/index";
import { getScene } from "../content/game-config";

/**
 * The plugin runtime behind `api.events`, `api.vars`, `api.progress` and `api.scene`
 * (docs/plugins.md "Across nodes and pages"): what lets an experience's code reach across nodes and pages.
 *
 * - **Events** have two owners (the concept's rule): *core events* describe the experience and are emitted by the scene
 *   only (`CORE_EVENTS` – a plugin that emits one gets an error); *scene events* are the content's own, any other
 *   name, emitted and listened to by plugins freely. One flat bus: every plugin on the stage listens to every event.
 * - **Variables** live in a scope: `scene` while the scene is the current one, `stage` while the scene exists,
 *   `session` per app start, `experience` persisted with the reader's progress (through the app's `ProgressReader`;
 *   in memory without one). A read without a scope looks outward – scene, stage, session, experience – and the innermost
 *   definition wins. Every write emits the core event `varChanged`.
 * - **Progress** is read-only for plugins and comes from the app (`ArSceneOptions.progress`): unlocked targets,
 *   consulted entries, completion – never the app's store itself.
 * - **The scene** is the engine's own content model of the current scene.
 *
 * Listeners are kept per plugin id so that uninstalling a plugin (the editor's Run) removes them.
 *
 * The vocabulary – the scopes, the core events' names, the name pattern and the value type – is the behaviour
 * grammar's (`shared/types/behaviour.ts`): a behaviour rule and a plugin speak the same words.
 */

/** A variable's value: a JSON scalar; `null` on a write deletes the variable */
export type VarValue = BehaviourValue;

export const VAR_SCOPES = BEHAVIOUR_SCOPES;
export type VarScope = BehaviourScope;

export const AR_STATES = ["running", "paused", "stopped"] as const;
export type ArStateName = (typeof AR_STATES)[number];

/** The core events: the scene emits them, plugins subscribe */
export interface CoreEvents {
  targetFound: { targetId: string };
  targetLost: { targetId: string };
  /** The first find of a target (the app's progress tells the scene through `celebrate`) */
  targetUnlocked: { targetId: string };
  targetTapped: { targetId: string };
  sceneChanged: { sceneId: string; previous: string | null };
  arState: { state: ArStateName };
  varChanged: { name: string; scope: VarScope; value: VarValue | undefined; previous: VarValue | undefined };
}

type CoreEventName = (typeof BEHAVIOUR_CORE_EVENTS)[number];
/** True when two unions hold the same members */
type Same<A, B> = [A] extends [B] ? ([B] extends [A] ? true : false) : false;

/** The core events' names: the shared list, which must name exactly the events `CoreEvents` has a payload for (else a type error) */
export const CORE_EVENTS: Same<keyof CoreEvents, CoreEventName> extends true ? ReadonlyArray<keyof CoreEvents> : never = BEHAVIOUR_CORE_EVENTS;

export const isCoreEvent = (name: string): name is keyof CoreEvents => (CORE_EVENTS as readonly string[]).includes(name);

/** A scene event's or a variable's name: like a component type (`doorOpened`) */
export const EVENT_NAME_PATTERN = BEHAVIOUR_NAME_PATTERN;

export type EventListener<P = unknown> = (payload: P) => void;

/**
 * What the app knows about the reader's progress (RULES #32: the engine never reads the app's store – the reader
 * passes an adapter over its history, the editor its simulation). Every method is optional: without one, nothing is
 * unlocked or consulted, and `experience` variables live in memory.
 */
export interface ProgressReader {
  unlocked?(targetId: string): boolean;
  consulted?(entryId: string): boolean;
  /** The persisted `experience` variables */
  variables?: {
    get(name: string): VarValue | undefined;
    set(name: string, value: VarValue | null): void;
    all(): Record<string, VarValue>;
  };
}

/** `api.events` */
export interface PluginEvents {
  /** Listen to a core event or a scene event; returns the unsubscribe (a plugin's listeners go when it is uninstalled) */
  on<E extends keyof CoreEvents>(event: E, listener: EventListener<CoreEvents[E]>): () => void;
  on(event: string, listener: EventListener): () => void;
  /** Emit a scene event (any name that is not a core event) to every plugin on the stage */
  emit(event: string, payload?: unknown): void;
}

/** `api.vars` */
export interface PluginVars {
  /** The value in `scope`, or without one the innermost definition (scene → stage → session → experience) */
  get(name: string, scope?: VarScope): VarValue | undefined;
  /** Set (or delete with `null`) a variable in a scope – `scene` by default */
  set(name: string, value: VarValue | null, scope?: VarScope): void;
  /** Listen to every change of a variable (in any scope) */
  on(name: string, listener: (value: VarValue | undefined, previous: VarValue | undefined, scope: VarScope) => void): () => void;
  /** Every variable of a scope */
  all(scope: VarScope): Record<string, VarValue>;
}

/** `api.progress` – read-only */
export interface PluginProgress {
  unlocked(targetId: string): boolean;
  consulted(entryId: string): boolean;
  /** Unlocked targets of a scene (the whole experience without an id) as 0–1; a scene without targets counts as complete */
  completion(sceneId?: string): number;
  counts(sceneId?: string): { unlocked: number; targets: number };
}

/** `api.scene` */
export interface PluginScene {
  /** The current scene as the engine knows it, null before the first scene */
  readonly current: Scene | null;
  get(sceneId: string): Scene | undefined;
}

export class PluginRuntimeError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "PluginRuntimeError";
  }
}

interface Listener { plugin: string; event: string; listener: EventListener; run: number }

/** Counts the installs: the listeners of one run of a plugin are told from the previous run's */
let runs = 0;
export const nextRun = (): number => ++runs;

/** The rules' typed variables (schema 5.0, B7): a value of another type is refused, a declared one reads as its default */
let variableTypes: ReadonlyMap<string, VariableInfo> = new Map();

/** The scene sets the content's variable types (`getVariableTypes`) */
export const setVariableTypes = (types: ReadonlyMap<string, VariableInfo>): void => {
  variableTypes = types;
};

/** A variable's type, scope and default when the content types it */
export const variableInfo = (name: string): VariableInfo | undefined => variableTypes.get(name);

const listeners = new Set<Listener>();
/** The editor's log: listens to every delivery without being a plugin */
const observers = new Set<(event: string, payload: unknown) => void>();
const scopes: Record<Exclude<VarScope, "experience">, Map<string, VarValue>> = { scene: new Map(), stage: new Map(), session: new Map() };
/** The `experience` scope without an app that persists it */
const experienceInMemory = new Map<string, VarValue>();
let progress: ProgressReader = {};
let currentSceneId: string | null = null;
let scenesAccessor: (id: string) => Scene | undefined = getScene;

const isVarValue = (value: unknown): value is VarValue =>
  typeof value === "string" || typeof value === "boolean" || (typeof value === "number" && Number.isFinite(value));

/** Deliver an event to every listener (a listener that throws is logged, the others still run) */
const deliver = (event: string, payload: unknown): void => {
  observers.forEach(observer => observer(event, payload));
  [...listeners].filter(l => l.event === event).forEach(({ listener, plugin }) => {
    try {
      listener(payload);
    } catch (error) {
      console.error(`[plugins] ${plugin}: the listener for "${event}" failed:`, error);
    }
  });
};

const experienceVars = () => progress.variables;

const readVar = (name: string, scope: VarScope): VarValue | undefined =>
  scope === "experience" ? (experienceVars() ? experienceVars()!.get(name) : experienceInMemory.get(name)) : scopes[scope].get(name);

const writeVar = (name: string, value: VarValue | null, scope: VarScope, who = "a rule"): void => {
  const typed = variableTypes.get(name);
  if (typed && value !== null && !valueFits(typed.type, value)) {
    console.warn(`[variables] ${who}: ${name} is ${typed.type === "integer" ? "an integer" : `a ${typed.type}`} (${typed.from}) – ${JSON.stringify(value)} refused, the value stays`);
    return;
  }
  const previous = readVar(name, scope);
  if (scope === "experience") {
    if (experienceVars()) experienceVars()!.set(name, value);
    else if (value === null) experienceInMemory.delete(name);
    else experienceInMemory.set(name, value);
  } else if (value === null) {
    scopes[scope].delete(name);
  } else {
    scopes[scope].set(name, value);
  }
  const next = value === null ? undefined : value;
  if (previous !== next) deliver("varChanged", { name, scope, value: next, previous } satisfies CoreEvents["varChanged"]);
};

/** The `events`, `vars`, `progress` and `scene` parts of a plugin's API, with its listeners kept under its id */
export const runtimeApi = (plugin: string, run = nextRun()): { events: PluginEvents; vars: PluginVars; progress: PluginProgress; scene: PluginScene } => {
  const on = (event: string, listener: EventListener): (() => void) => {
    if (typeof listener !== "function") throw new PluginRuntimeError(`${plugin}: events.on("${event}") needs a listener function`);
    if (!isCoreEvent(event) && !EVENT_NAME_PATTERN.test(event)) throw new PluginRuntimeError(`${plugin}: "${event}" is not an event name (camelCase letters and digits)`);
    const entry: Listener = { plugin, event, listener, run };
    listeners.add(entry);
    return () => listeners.delete(entry);
  };
  return {
    events: {
      on: on as PluginEvents["on"],
      emit: (event, payload) => {
        if (isCoreEvent(event)) throw new PluginRuntimeError(`${plugin}: "${event}" is a core event – the engine emits it, plugins subscribe`);
        if (!EVENT_NAME_PATTERN.test(event)) throw new PluginRuntimeError(`${plugin}: "${event}" is not an event name (camelCase letters and digits)`);
        deliver(event, payload);
      },
    },
    vars: {
      get: (name, scope) => {
        if (scope) return readVar(name, scope);
        for (const s of VAR_SCOPES) {
          const value = readVar(name, s);
          if (value !== undefined) return value;
        }
        // A declared variable that is not set reads as its default (schema 5.0, B7)
        return variableTypes.get(name)?.default;
      },
      set: (name, value, scope = "scene") => {
        if (!EVENT_NAME_PATTERN.test(name)) throw new PluginRuntimeError(`${plugin}: "${name}" is not a variable name (camelCase letters and digits)`);
        if (!VAR_SCOPES.includes(scope)) throw new PluginRuntimeError(`${plugin}: the scope of ${name} must be ${VAR_SCOPES.join(", ")}`);
        if (value !== null && !isVarValue(value)) throw new PluginRuntimeError(`${plugin}: ${name} takes a string, a number or a boolean (null deletes)`);
        writeVar(name, value, scope, plugin);
      },
      on: (name, listener) => on("varChanged", payload => {
        const change = payload as CoreEvents["varChanged"];
        if (change.name === name) listener(change.value, change.previous, change.scope);
      }),
      all: scope => scope === "experience" ? (experienceVars() ? experienceVars()!.all() : Object.fromEntries(experienceInMemory)) : Object.fromEntries(scopes[scope]),
    },
    progress: {
      unlocked: targetId => progress.unlocked?.(targetId) ?? false,
      consulted: entryId => progress.consulted?.(entryId) ?? false,
      counts: sceneId => progressCounts(sceneId),
      completion: sceneId => {
        const { unlocked, targets } = progressCounts(sceneId);
        return targets === 0 ? 1 : unlocked / targets;
      },
    },
    scene: {
      get current() {
        return currentSceneId ? scenesAccessor(currentSceneId) ?? null : null;
      },
      get: id => scenesAccessor(id),
    },
  };
};

const progressCounts = (sceneId?: string): { unlocked: number; targets: number } => {
  const scenes = sceneId ? [scenesAccessor(sceneId)].filter((s): s is Scene => !!s) : allScenes();
  const targets = scenes.flatMap(s => s.targets);
  return { unlocked: targets.filter(t => progress.unlocked?.(t.id) ?? false).length, targets: targets.length };
};

let allScenes: () => Scene[] = () => [];

// ── The scene's side ───────────────────────────────────────────────────────────────────────────

/** The app's progress for `api.progress` and the `experience` scope (the scene sets it from its options) */
export const setProgressReader = (reader: ProgressReader | undefined): void => {
  progress = reader ?? {};
};

/** Where scenes come from (the engine's content accessors; tests pass their own) */
export const setSceneSource = (get: (id: string) => Scene | undefined, all: () => Scene[]): void => {
  scenesAccessor = get;
  allScenes = all;
};

/** The scene emits a core event */
export const emitCoreEvent = <E extends keyof CoreEvents>(event: E, payload: CoreEvents[E]): void => deliver(event, payload);

/** The scene changed: its scope ends, the new one is current */
export const sceneChanged = (sceneId: string): void => {
  const previous = currentSceneId;
  currentSceneId = sceneId;
  scopes.scene.clear();
  deliver("sceneChanged", { sceneId, previous } satisfies CoreEvents["sceneChanged"]);
};

/** The scene was torn down: the stage and scene scopes end */
export const stageEnded = (): void => {
  currentSceneId = null;
  scopes.scene.clear();
  scopes.stage.clear();
};

/** Every listener of a plugin goes with it – or only those of other runs than `keepRun` (an install replacing the previous one), or only `onlyRun`'s (a failed install) */
export const dropPluginListeners = (plugin: string, runs: { keepRun?: number; onlyRun?: number } = {}): void => {
  [...listeners]
    .filter(l => l.plugin === plugin && (runs.keepRun === undefined || l.run !== runs.keepRun) && (runs.onlyRun === undefined || l.run === runs.onlyRun))
    .forEach(l => listeners.delete(l));
};

/** Every variable of every scope (the editor's panel, a reset) */
export const resetVars = (): void => {
  (Object.keys(scopes) as Array<keyof typeof scopes>).forEach(scope => scopes[scope].clear());
  experienceInMemory.clear();
  experienceVars() && Object.keys(experienceVars()!.all()).forEach(name => experienceVars()!.set(name, null));
};

/** A snapshot of every variable with its scope, innermost first (the editor's panel) */
export const allVars = (): Array<{ scope: VarScope; name: string; value: VarValue }> =>
  VAR_SCOPES.flatMap(scope => {
    const values = scope === "experience" ? (experienceVars() ? experienceVars()!.all() : Object.fromEntries(experienceInMemory)) : Object.fromEntries(scopes[scope]);
    return Object.entries(values).map(([name, value]) => ({ scope, name, value }));
  });

/**
 * A rule of a `behaviour` component fired (`behaviour.ts`): told to the observers only, as the pseudo-event `behaviour` –
 * the editor's log shows which rule ran and what it did; it is no event of the bus, plugins never listen to it
 */
export const noteBehaviour = (payload: BehaviourNote): void =>
  observers.forEach(observer => observer("behaviour", payload));

/**
 * What the editor's log listens to of a rule that fired: a node's rule names its target (and the node's name), a scene's or
 * the experience's rule (schema 5.0) its `home` (and the scene)
 */
export type BehaviourNote =
  | { targetId: string; node?: string; home?: undefined; rule: number; on: string; do: string }
  | { home: "scene" | "experience"; scene?: string; targetId?: undefined; rule: number; on: string; do: string };

/** Where a rule's `goto` goes (schema 5.0): the host's `ArSceneOptions.navigate`; none = a console note */
let navigator: ((address: string) => void) | undefined;

/** The scene sets the host's navigate from its options (none: a goto only logs) */
export const setNavigator = (navigate: ((address: string) => void) | undefined): void => {
  navigator = navigate;
};

/** A rule asks to go to an address (`scene:<id>`): the host resolves it – a host that gives no navigate only logs it */
export const navigateTo = (address: string, from: string): void => {
  if (!navigator) {
    console.info(`[behaviour] ${from}: goto ${address} – the host gives no navigate, nothing happens`);
    return;
  }
  try {
    navigator(address);
  } catch (error) {
    console.error(`[behaviour] ${from}: goto ${address} failed:`, error);
  }
};

/** Listen to every event of the stage without being a plugin (the editor's log); returns the unsubscribe */
export const observeEvents = (listener: (event: string, payload: unknown) => void): (() => void) => {
  observers.add(listener);
  return () => observers.delete(listener);
};
