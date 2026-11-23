import {
  BEHAVIOUR_CORE_EVENTS, BEHAVIOUR_TYPE, behaviourRules, declaredUses, isBehaviourNodeEvent,
  type BehaviourAction, type BehaviourRule, type BehaviourScope, type EntityData, type GroupData, type LogicCanvas, type PluginData,
} from "@engine/types";
import { expressionVariables, textVariables } from "@shared/text/expressions";
import { parseAddress } from "@shared/types/address";
import { ruleTypeProblems, type RuleList, type VariableDeclaration, type VariableInfo, variableTypes } from "@shared/types/variables";
import { pathOfName } from "#lib/behaviour-edit.js";
import { nodeLabel } from "#lib/tree.js";
import type { PluginUses } from "./plugin-code";
import { coreRegistry, type NodeRegistry } from "./registry";
import { answersVerb, nodePorts, nodeSpecs } from "./spec-ports";
import type { Context, DisplayRow, GraphEdge, GraphNode, Home, LogicPick, Problem, Reference, RuleRef, Severity, Signal, Stub, View } from "./types";

/**
 * `project(content, context)` → `View` (#334, research §6): the logic of the experience as a graph **for one level**, computed
 * from the draft the editor already holds (the scenes with their targets' entity trees, entries and rules, the experience's
 * rules and variables, the groups, the plugins and what their code uses) – nothing is stored, nothing fetched.
 *
 * - **scene** (#239's panel): the events (a node's own `found` / `lost` / `tap` / `unlocked`, a scene event, a core event),
 *   the rules of the scene's `behaviour` components and of the scene's own `logic` (schema 5.0), the variables they read
 *   (their conditions' and set values' expressions) and set, the nodes they act on (a scene rule names them by address,
 *   `target:<id>/<name>`) and the plugins on its nodes or touching its rules' events and variables. Stubs: a variable set in
 *   another scene (or by the experience's rules) and read here (or the reverse), an event emitted elsewhere and listened to here
 *   (or the reverse), a **goto** to another scene ("→ Epilogue · goto") and one from another scene into this one.
 * - **entity**: the same for one target's tree (`path`: its subtree); stubs also to the scene's other targets and its rules.
 * - **group**: its scenes and the derived edges between them (events and variables joined by name, gotos); stubs to scenes
 *   outside the group and to plugins.
 * - **experience**: the groups (frames), the scenes in order, the plugins, the **experience's rules** (boxes, editable – they
 *   act on the scenes: a goto into a scene box, a node action onto the scene that holds the target), the variables that cross
 *   scenes (set in a scope wider than `scene`, or by a plugin), the core events the rules listen to, and the derived **goto**
 *   edges between scenes. An event joins two places (scenes, plugins) by one derived edge per pair; a variable joins them
 *   through its own box (set in A → `hasKey` → read in B). Nothing crosses it.
 *
 * Cross-scene joins follow the bus and the variables' scopes: a variable set with scope `scene` never reaches another scene;
 * a plugin's scope is unknown, so it counts as crossing; a set without a scope takes the variable's declared scope, else
 * `scene`. Entry texts read variables (`{hasKey}`, `shared/text/expressions.ts`).
 * **Problems** look at the whole experience: a variable read that nothing sets, an event listened to that nothing emits, an event
 * emitted that nothing listens to, a node an action names that is not there, a goto to a scene that is not there, a rule that listens to
 * nothing, and the rules' **type problems** (`ruleTypeProblems`, B7: a condition that is no true / false, a set of another
 * type than the variable's). While a plugin's code is unknown, the names a plugin may set or emit are not said.
 *
 * **Ports from the specs** (L-T6, §5.9 – `spec-ports.ts`): at the scene and entity levels a node's box carries the ports its
 * node type's, components' and material's specs give – its file, its parameters, the verbs it answers, the events it emits.
 * A rule's node action lands on the node's action inlet when the node answers that verb (else on its `in`); an event a
 * node's component declares is wired to the event's box when the level has one. The entity level draws every node of its
 * subtree that has something of its own (any type but a bare group, or a component or a material). A spec's port is
 * **hidden** until it is wired or the author exposed it (`LogicCanvas.exposed`, the scene's canvas – Blender's way). A
 * plugin's bus use is what its specs declare (`declaredUses`, schema 5.1), else what its code shows (`pluginCodeUses`).
 */

/** A target as the engine maps it (the entity ref resolved) */
export interface LogicTarget {
  id: string;
  entity?: EntityData;
}

/** An entry: its text may read variables */
export interface LogicEntry {
  id: string;
  title: string;
  body?: string;
}

/** A scene as the editor's draft holds it (`content.scenes`, the engine's `Scene`) */
export interface LogicScene {
  id: string;
  label: string;
  group?: string;
  targets: readonly LogicTarget[];
  entries?: readonly LogicEntry[];
  /** The scene's own rules (`SceneData.logic`, schema 5.0) */
  rules?: readonly BehaviourRule[];
  /** The scene's canvas (`SceneData.logic.canvas`): the ports exposed on its boxes (schema 5.1) */
  canvas?: LogicCanvas;
}

export interface LogicContent {
  /** The scenes in order */
  scenes: readonly LogicScene[];
  groups?: readonly GroupData[];
  /** The experience's declared plugins with what they recorded (`registered`) */
  plugins?: readonly PluginData[];
  /** What each plugin's code uses, by plugin id; a plugin missing here is unknown */
  code?: Readonly<Record<string, PluginUses>>;
  /** The experience's own rules (`ExperienceData.logic`, schema 5.0) */
  rules?: readonly BehaviourRule[];
  /** The declared variables (`ExperienceData.variables`, B7) */
  variables?: Readonly<Record<string, VariableDeclaration>>;
  /** The experience's canvas (`ExperienceData.logic.canvas`) */
  canvas?: LogicCanvas;
}

export interface ProjectOptions {
  /** The node types (ports by kind); the core's by default */
  registry?: NodeRegistry;
}

const CORE = BEHAVIOUR_CORE_EVENTS as readonly string[];
const BUILT_IN_COMPONENTS = [BEHAVIOUR_TYPE, "animation"];
const EMPTY_CANVAS = () => ({ nodes: [], wires: [] });

const pathKey = (target: string, path: readonly number[]) => `${target}/${path.join(".")}`;
const startsWith = (path: readonly number[], prefix: readonly number[]) => prefix.every((p, i) => path[i] === p);
const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;

/** Every node of a tree with its path, depth first */
const walk = (node: EntityData, path: number[] = []): Array<{ node: EntityData; path: number[] }> =>
  [{ node, path }, ...(node.children ?? []).flatMap((child, i) => walk(child, [...path, i]))];

/** A node's rules with where each sits: its component's index and its index there */
const rulesOfNode = (node: EntityData): Array<{ rule: BehaviourRule; component: number; index: number }> =>
  (node.components ?? []).flatMap((c, component) => (c.type === BEHAVIOUR_TYPE ? behaviourRules(c).map((rule, index) => ({ rule, component, index })) : []));

/** A condition in words: its expression, or `always` */
export const conditionWords = (when: string | null): string => when?.trim() || "always";

/** A set's label on its wire: `set = taps + 1`, `set = unset` */
export const setWords = (value: string | null): string => `set = ${value === null ? "unset" : value}`;

/** The id of a scene rule's box and of an experience rule's box */
export const sceneRuleBox = (scene: string, rule: number): string => `rule:scene:${scene}#${rule}`;
export const experienceRuleBox = (rule: number): string => `rule:experience#${rule}`;

/** A scene's or the experience's rule as the canvas names it */
export const logicRuleLabel = (ref: Exclude<RuleRef, { doc: "entity" }>): string =>
  ref.doc === "scene" ? `scene rule ${ref.rule + 1}` : `experience rule ${ref.rule + 1}`;

/**
 * What each plugin uses of the bus and the variables, by plugin id: what its specs declare (schema 5.1), else what its code
 * shows (`content.code`); a plugin in neither is unknown
 */
export const pluginUses = (content: LogicContent): Record<string, PluginUses> => {
  const uses: Record<string, PluginUses> = { ...(content.code ?? {}) };
  for (const plugin of content.plugins ?? []) {
    const declared = declaredUses(plugin.registered);
    if (declared) uses[plugin.id] = declared;
  }
  return uses;
};

/** Whether a plugin's bus use comes from its specs (true) or its code */
const declaresUses = (plugin: PluginData | undefined): boolean => !!plugin && declaredUses(plugin.registered) !== null;

/** A plugin box's detail: where what it uses comes from */
const pluginDetail = (content: LogicContent, id: string, uses: Record<string, PluginUses>): string =>
  declaresUses(content.plugins?.find(p => p.id === id)) ? "plugin · declared" : uses[id] ? "plugin" : "plugin – its code not read";

/** The plugin that registered a type (a component, a node type, a material), by what the plugins recorded */
const pluginOfType = (plugins: readonly PluginData[], kind: "components" | "nodeTypes" | "materials", type: string): string | null =>
  plugins.find(p => p.registered?.[kind]?.[type])?.id ?? null;

// ── Types (B7): declared or inferred, the rules checked against them ─────────────────────────────────────────────────

/** A list of rules with where it lives: the experience's (scene and target null), a scene's (target null), a node's behaviour */
export interface LogicRuleList extends RuleList {
  scene: string | null;
  target: string | null;
  /** Where in words: "the experience's rules", "Door · scene rules", "door (door)" */
  where: string;
}

/** Every list of rules in content order: the experience's, then per scene its own and its targets' behaviours */
export const logicRuleLists = (content: LogicContent): LogicRuleList[] => {
  const lists: LogicRuleList[] = [];
  if (content.rules?.length) lists.push({ rules: [...content.rules], path: "experience", scene: null, target: null, where: "the experience's rules" });
  for (const scene of content.scenes) {
    if (scene.rules?.length) lists.push({ rules: [...scene.rules], path: `scene:${scene.id}`, scene: scene.id, target: null, where: `${scene.label} · scene rules` });
    for (const target of scene.targets) {
      if (!target.entity) continue;
      for (const { node } of walk(target.entity)) {
        (node.components ?? []).forEach((c, i) => {
          if (c.type === BEHAVIOUR_TYPE) lists.push({ rules: behaviourRules(c), path: `${target.id}:${i}`, scene: scene.id, target: target.id, where: `${nodeLabel(node)} (${target.id})` });
        });
      }
    }
  }
  return lists;
};

/** Every variable's type: the declarations, else each one's first set (`shared/types/variables.ts`) */
export const logicTypes = (content: LogicContent): Map<string, VariableInfo> => variableTypes(content.variables, logicRuleLists(content));

/** One list's type problems by rule, in words: `when: …`, `action 2 (set tries): …` */
export const typeProblemsByRule = (rules: readonly BehaviourRule[], types: ReadonlyMap<string, VariableInfo>): Array<{ rule: number; message: string }> =>
  ruleTypeProblems([{ rules: [...rules], path: "R" }], types).flatMap(message => {
    const m = /^R\[(\d+)\]\.(?:when|do\[(\d+)\]\.set\.(\w+)): (.*)$/s.exec(message);
    if (!m) return [];
    const [, rule, action, name, rest] = m;
    const where = action === undefined ? "when" : `action ${Number(action) + 1} (set ${name})`;
    // The source of a variable's type names the content's path: in words, the declaration or "its first set"
    const words = rest.replace(/\((experience\.variables\.\w+)\)/, "(declared)").replace(/\(R\[\d+\][^)]*\)|\([\w:.]+\[\d+\][^)]*\)/, "(its first set)");
    return [{ rule: Number(rule), message: `${where}: ${words}` }];
  });

/** The type problems of some lists, each in words with its place (errors: the rule refuses the type) */
const typeProblemsOf = (lists: readonly LogicRuleList[], types: ReadonlyMap<string, VariableInfo>): Problem[] =>
  lists.flatMap(list => typeProblemsByRule(list.rules, types).map(p => ({
    message: `${list.where}, rule ${p.rule + 1}, ${p.message}`, severity: "error" as const, place: `${list.where}, rule ${p.rule + 1}`, short: p.message,
  })));

// ── Places and their uses: who sets, reads, emits and listens to what, across the experience ──────────────────────────────

/**
 * Where something is used: a target's tree, a scene's entry texts (`target` null), a scene's rules (`logic` "scene"), the
 * experience's rules (`logic` "experience", scene null), or a plugin's code
 */
interface Place {
  key: string;
  scene: string | null;
  target: string | null;
  plugin: string | null;
  logic: "scene" | "experience" | null;
}

type UseKind = "sets" | "reads" | "emits" | "listens";

interface Use {
  place: Place;
  kind: UseKind;
  name: string;
  /** A rule's `set` scope (its own, else the declared one, else scene); null: a plugin's (unknown) */
  scope?: BehaviourScope | null;
}

/** A join by name: a variable set in one place and read in another, an event emitted in one place and listened to in another */
interface Join {
  kind: "variable" | "event";
  name: string;
  from: Place;
  to: Place;
}

/** A goto written in a rule: from where, to which scene, the rule's do outlet (in the view of its own level) and its home */
interface Goto {
  place: Place;
  scene: string;
  home: Home;
}

const targetPlace = (scene: string, target: string): Place => ({ key: `target:${scene}/${target}`, scene, target, plugin: null, logic: null });
const textPlace = (scene: string): Place => ({ key: `text:${scene}`, scene, target: null, plugin: null, logic: null });
const sceneRulesPlace = (scene: string): Place => ({ key: `logic:${scene}`, scene, target: null, plugin: null, logic: "scene" });
const EXPERIENCE_PLACE: Place = { key: "logic:experience", scene: null, target: null, plugin: null, logic: "experience" };
const pluginPlace = (plugin: string): Place => ({ key: `plugin:${plugin}`, scene: null, target: null, plugin, logic: null });

/** The variables a rule reads: its condition's and its set values' */
const readsOf = (rule: BehaviourRule): string[] => [...new Set([
  ...(rule.when ? expressionVariables(rule.when) : []),
  ...rule.do.flatMap(a => (a.action === "set" && a.value ? expressionVariables(a.value) : [])),
])];

/** The scene a goto's address names (null: no scene address) */
const gotoScene = (action: BehaviourAction): string | null => {
  if (action.action !== "goto") return null;
  const address = parseAddress(action.address);
  return address?.scheme === "scene" ? address.id : null;
};

const usesOf = (content: LogicContent): Use[] => {
  const uses: Use[] = [];
  const declared = (name: string) => content.variables?.[name]?.scope;
  const ruleUses = (place: Place, rule: BehaviourRule) => {
    if (rule.on && !isBehaviourNodeEvent(rule.on)) uses.push({ place, kind: "listens", name: rule.on });
    for (const name of readsOf(rule)) uses.push({ place, kind: "reads", name });
    for (const action of rule.do) {
      if (action.action === "set") uses.push({ place, kind: "sets", name: action.name, scope: action.scope ?? declared(action.name) ?? "scene" });
      if (action.action === "emit") uses.push({ place, kind: "emits", name: action.event });
    }
  };
  for (const rule of content.rules ?? []) ruleUses(EXPERIENCE_PLACE, rule);
  for (const scene of content.scenes) {
    for (const rule of scene.rules ?? []) ruleUses(sceneRulesPlace(scene.id), rule);
    for (const target of scene.targets) {
      if (!target.entity) continue;
      const place = targetPlace(scene.id, target.id);
      for (const { node } of walk(target.entity)) for (const { rule } of rulesOfNode(node)) ruleUses(place, rule);
    }
    const place = textPlace(scene.id);
    for (const entry of scene.entries ?? []) {
      for (const name of entry.body ? textVariables(entry.body) : []) uses.push({ place, kind: "reads", name });
    }
  }
  for (const [plugin, code] of Object.entries(pluginUses(content))) {
    const place = pluginPlace(plugin);
    code.listens.forEach(name => uses.push({ place, kind: "listens", name }));
    code.emits.forEach(name => uses.push({ place, kind: "emits", name }));
    code.reads.forEach(name => uses.push({ place, kind: "reads", name }));
    code.sets.forEach(name => uses.push({ place, kind: "sets", name, scope: null }));
  }
  return uses;
};

/** Every goto of the experience: the experience's rules, the scenes' rules, the nodes' behaviours */
const gotosOf = (content: LogicContent): Goto[] => {
  const gotos: Goto[] = [];
  const add = (place: Place, rule: BehaviourRule, home: (a: number) => Home) =>
    rule.do.forEach((action, a) => {
      const scene = gotoScene(action);
      if (scene !== null) gotos.push({ place, scene, home: home(a) });
    });
  (content.rules ?? []).forEach((rule, r) => add(EXPERIENCE_PLACE, rule, a => ({ doc: "experience", rule: r, part: "do", index: a })));
  for (const scene of content.scenes) {
    (scene.rules ?? []).forEach((rule, r) => add(sceneRulesPlace(scene.id), rule, a => ({ doc: "scene", scene: scene.id, rule: r, part: "do", index: a })));
    for (const target of scene.targets) {
      if (!target.entity) continue;
      for (const { node, path } of walk(target.entity)) {
        for (const { rule, component, index } of rulesOfNode(node)) {
          add(targetPlace(scene.id, target.id), rule, a => ({ doc: "entity", target: target.id, path, component, rule: index, part: "do", index: a }));
        }
      }
    }
  }
  return gotos;
};

/** Every join between two different places; a variable set with scope `scene` joins only within its scene; core events are the engine's */
const joinsOf = (uses: readonly Use[]): Join[] => {
  const joins: Join[] = [];
  const seen = new Set<string>();
  const by = (kind: UseKind) => uses.filter(u => u.kind === kind);
  const pair = (kind: Join["kind"], from: Use, to: Use) => {
    if (from.name !== to.name || from.place.key === to.place.key) return;
    if (kind === "variable" && from.scope === "scene" && from.place.scene !== to.place.scene) return;
    const key = `${kind}|${from.name}|${from.place.key}|${to.place.key}`;
    if (seen.has(key)) return;
    seen.add(key);
    joins.push({ kind, name: from.name, from: from.place, to: to.place });
  };
  const reads = by("reads");
  const listens = by("listens").filter(u => !CORE.includes(u.name));
  by("sets").forEach(set => reads.forEach(read => pair("variable", set, read)));
  by("emits").forEach(emit => listens.forEach(listen => pair("event", emit, listen)));
  return joins;
};

/** The display rows in the order a card lists them */
const ROW_ORDER: readonly DisplayRow["id"][] = ["reads", "sets", "goto", "drives"];

/**
 * A plugin box's params: the events its pins stand for (`registry.ts` draws a pin only for a side it has), and the ones a
 * wire may pick – what it emits, and what it listens to that a rule can emit (no core event)
 */
const pluginBus = (used: PluginUses | undefined) => ({
  params: { emits: used?.emits ?? [], listens: used?.listens ?? [] },
  bus: { emits: [...(used?.emits ?? [])], listens: (used?.listens ?? []).filter(name => !CORE.includes(name)) },
});

// ── The graph of a set of targets: the scene and the entity levels ────────────────────────────────────────────────

/** Collects nodes and edges once each; edges with a home are kept each (an op needs every one), derived ones once */
class Graph {
  nodes = new Map<string, GraphNode>();
  edges: GraphEdge[] = [];
  problems: Problem[] = [];
  /** The gotos of the rules drawn here: a stub each at the scene and entity levels */
  gotos: Array<{ from: [string, string]; scene: string; home: Home }> = [];
  /** The display rows' references (#334): dotted lines from a plugin's row to the box it names */
  refs: Reference[] = [];
  #ids = new Set<string>();

  constructor(private registry: NodeRegistry) {}

  add(node: Omit<GraphNode, "ports">, params?: unknown): GraphNode {
    const known = this.nodes.get(node.id);
    if (known) return known;
    const full = { ...node, ports: this.registry.ports(node.kind, params) } as GraphNode;
    this.nodes.set(node.id, full);
    return full;
  }

  link(from: [string, string], to: [string, string], signal: Signal, label: string, home: Home | null, names?: readonly string[]): void {
    if (!home && this.edges.some(e => !e.home && e.from.join() === from.join() && e.to.join() === to.join() && e.label === label)) return;
    const base = `${from.join(":")}>${to.join(":")}`;
    let id = base;
    for (let n = 2; this.#ids.has(id); n++) id = `${base}~${n}`;
    this.#ids.add(id);
    this.edges.push({ id, from, to, signal, label, home, ...(names ? { names } : {}) });
  }

  /**
   * A name on a box's display row and the row's reference to the box it names (#334, Tilman 2026-11-21): a plugin's read,
   * set or driven type is a code fact – no wire, since a wire could not change it
   */
  refer(box: string, row: DisplayRow["id"], to: string, name: string, dir: Reference["dir"]): void {
    const node = this.nodes.get(box);
    if (!node) return;
    const rows = (node.rows ??= []);
    let known = rows.find(r => r.id === row);
    if (!known) {
      known = { id: row, names: [] };
      rows.push(known);
      rows.sort((x, y) => ROW_ORDER.indexOf(x.id) - ROW_ORDER.indexOf(y.id));
    }
    if (!known.names.includes(name)) known.names.push(name);
    const id = `ref:${box}:${row}>${to}`;
    if (!this.refs.some(r => r.id === id)) this.refs.push({ id, from: [box, row], to, dir });
  }

  /** A problem of a box: `short` is the message without the box's name (the issues bar shows the name beside it) */
  flag(node: GraphNode, message: string, severity: Severity, short?: string): void {
    if (!node.problem || (severity === "error" && node.problemSeverity === "warning")) {
      node.problem = message;
      node.problemSeverity = severity;
    }
    if (!this.problems.some(p => p.message === message)) this.problems.push({ message, severity, node: node.id, place: node.label, ...(short ? { short } : {}) });
  }

  eventNode(name: string): GraphNode {
    return this.add({ id: `event:${name}`, kind: "event", label: name, detail: CORE.includes(name) ? "core event" : "scene event", pick: null });
  }

  variableNode(name: string): GraphNode {
    return this.add({ id: `var:${name}`, kind: "variable", label: name, detail: "variable", pick: null });
  }

  /**
   * A rule's own wires: its `on` (a bus event – a node's own event is the caller's), the variables its condition reads
   * (`when`), and per action: a set to its variable (`set = taps + 1`), an emit to its event, a goto (a stub, or a scene
   * box with `scenes`), a node action through `nodeOf` (the box it acts on; null: no wire)
   */
  ruleWires(id: string, rule: BehaviourRule, home: (part: "on" | "when" | "do", index?: number) => Home, opts: {
    ownEvent?: (on: string) => GraphNode;
    nodeOf: (action: Extract<BehaviourAction, { node: string }>, a: number) => [string, string] | null;
    scenes?: ReadonlySet<string>;
    onSet?: (name: string, scope: string) => void;
  }): void {
    if (!rule.on) {
      // Its on was cut (or never set): the rule listens to nothing until an event is wired to it
      const box = this.nodes.get(id);
      if (box) this.flag(box, `${box.label} listens to nothing – wire an event to its on.`, "warning", "listens to nothing – wire an event to its on");
    } else if (isBehaviourNodeEvent(rule.on) && opts.ownEvent) {
      this.link([opts.ownEvent(rule.on).id, "out"], [id, "on"], "event", "on", home("on"));
    } else {
      this.link([this.eventNode(rule.on).id, "out"], [id, "on"], "event", "on", home("on"));
    }
    (rule.when ? expressionVariables(rule.when) : []).forEach((name, t) =>
      this.link([this.variableNode(name).id, "get"], [id, "when"], "value", "when", home("when", t)));
    rule.do.forEach((action, a) => {
      const out: [string, string] = [id, `do:${a}`];
      switch (action.action) {
        case "set":
          opts.onSet?.(action.name, action.scope ?? "");
          this.link(out, [this.variableNode(action.name).id, "set"], "value", setWords(action.value), home("do", a));
          break;
        case "emit":
          this.link(out, [this.eventNode(action.event).id, "in"], "event", "emit", home("do", a));
          break;
        case "goto": {
          const scene = gotoScene(action);
          if (scene !== null && opts.scenes?.has(scene)) this.link(out, [`scene:${scene}`, "arrives"], "scene", "goto", home("do", a));
          else if (scene !== null) this.gotos.push({ from: out, scene, home: home("do", a) });
          break;
        }
        default: {
          const to = opts.nodeOf(action, a);
          if (to) this.link(out, to, "node", action.action === "play" ? `play ${action.clip || "its clip"}` : action.action, home("do", a));
        }
      }
    });
  }
}

interface TargetsInput {
  scene: string;
  targets: ReadonlyArray<LogicTarget & { under?: number[] }>;
  /** Boxes that open the entity level (the scene level); none at the entity level, its nodes are leaves */
  enterEntities: boolean;
  /** The scene's own rules, drawn as boxes (the scene level) */
  sceneRules?: readonly BehaviourRule[];
}

/** The graph of a scene's targets (or one target's subtree) and its own rules: today's Logic panel (#239) */
const targetsGraph = (content: LogicContent, uses: readonly Use[], { scene, targets, enterEntities, sceneRules = [] }: TargetsInput, registry: NodeRegistry): Graph => {
  const plugins = content.plugins ?? [];
  const code = pluginUses(content);
  const g = new Graph(registry);

  const entityNode = (target: string, root: EntityData, path: number[]): GraphNode => {
    let node: EntityData | undefined = root;
    for (const i of path) node = node?.children?.[i];
    const enter: Context = { level: "entity", scene, target, path: [] };
    const picture = node?.type === "image" ? node.assets.find(a => a.assetType === "image")?.src : undefined;
    return g.add({
      id: `node:${pathKey(target, path)}`, kind: "node", label: node ? nodeLabel(node) : "?", detail: path.length ? `in ${target}` : `entity of ${target}`,
      pick: { kind: "node", target, path }, ...(enterEntities ? { enter } : {}), ...(picture ? { preview: picture } : {}),
    }, { ports: node ? nodePorts(node, plugins) : [] });
  };
  /** Where a node action lands: the node's action inlet when it answers the verb, else its `in` */
  const actionPort = (box: GraphNode, verb: string): [string, string] => [box.id, answersVerb(box.ports, verb) ? `act:${verb}` : "in"];
  const scopes = new Map<string, Set<string>>();
  const setIn = (name: string, scope: string) => scopes.set(name, (scopes.get(name) ?? new Set()).add(scope || content.variables?.[name]?.scope || "scene"));

  // The rules of every node of every target, with what they listen to, read, set, emit and act on
  const ruleVariables = new Set<string>();
  const ruleEvents = new Set<string>();
  const note = (rule: BehaviourRule) => {
    if (rule.on && !isBehaviourNodeEvent(rule.on)) ruleEvents.add(rule.on);
    readsOf(rule).forEach(v => ruleVariables.add(v));
    rule.do.forEach(a => (a.action === "set" ? ruleVariables.add(a.name) : a.action === "emit" ? ruleEvents.add(a.event) : undefined));
  };
  for (const target of targets) {
    if (!target.entity) continue;
    const root = target.entity;
    let i = -1;
    for (const { node, path } of walk(root)) {
      const inside = startsWith(path, target.under ?? []);
      const owner = nodeLabel(node);
      const pick: LogicPick = { kind: "node", target: target.id, path };
      for (const { rule, component, index } of rulesOfNode(node)) {
        i++;
        if (!inside) continue;
        note(rule);
        const n = i;
        const home = (part: "on" | "when" | "do", at?: number): Home =>
          ({ doc: "entity", target: target.id, path, component, rule: index, part, ...(at === undefined ? {} : { index: at }) });
        const id = `rule:${pathKey(target.id, path)}#${i}`;
        g.add({ id, kind: "rule", label: `${owner} · rule ${i + 1}`, detail: conditionWords(rule.when), pick, home: home("on") }, rule);
        g.ruleWires(id, rule, home, {
          ownEvent: on => g.add({ id: `event:${pathKey(target.id, path)}:${on}`, kind: "event", label: on, detail: owner, pick }),
          onSet: setIn,
          nodeOf: action => {
            const relative = pathOfName(node, action.node);
            if (relative) return actionPort(entityNode(target.id, root, [...path, ...relative]), action.action);
            const missing = g.add({ id: `missing:${pathKey(target.id, path)}:${action.node}`, kind: "node", label: action.node, detail: `not under ${owner}`, pick });
            g.flag(missing, `${owner}, rule ${n + 1}: ${action.action} names ${action.node} – no node of that name under ${owner}`, "error",
              `not under ${owner} (rule ${n + 1}, ${action.action})`);
            return [missing.id, "in"];
          },
        });
      }
    }
  }

  // The scene's own rules (schema 5.0): they listen to the bus and name nodes by address
  const sceneTargets = content.scenes.find(s => s.id === scene)?.targets ?? [];
  sceneRules.forEach((rule, r) => {
    note(rule);
    const id = sceneRuleBox(scene, r);
    const home = (part: "on" | "when" | "do", at?: number): Home => ({ doc: "scene", scene, rule: r, part, ...(at === undefined ? {} : { index: at }) });
    const label = logicRuleLabel({ doc: "scene", scene, rule: r });
    g.add({ id, kind: "rule", label, detail: conditionWords(rule.when), pick: { kind: "scene", scene }, home: home("on") }, rule);
    g.ruleWires(id, rule, home, {
      onSet: setIn,
      nodeOf: action => {
        const address = parseAddress(action.node);
        const target = address?.scheme === "target" ? sceneTargets.find(t => t.id === address.id) : undefined;
        const path = target?.entity && address?.scheme === "target" ? pathOfName(target.entity, address.node) : null;
        if (target?.entity && path) return actionPort(entityNode(target.id, target.entity, path), action.action);
        const missing = g.add({ id: `missing:scene:${action.node}`, kind: "node", label: action.node || "?", detail: "not in this scene", pick: { kind: "scene", scene } });
        const why = !address ? "not an address (target:<id>/<name>)" : !target ? "no such target in this scene" : "no node of that name in the target";
        const brief = !address ? "not an address" : !target ? "no such target here" : "no such node";
        g.flag(missing, `${label}: ${action.action} names ${action.node || "no node"} – ${why}`, "error", `${brief} (${label}, ${action.action})`);
        return [missing.id, "in"];
      },
    });
  });

  // The plugins: on a node here (their component, node type, material), or touching the rules' events / variables
  const drawn = new Set<string>();
  const pluginNode = (id: string) => {
    drawn.add(id);
    const { params, bus } = pluginBus(code[id]);
    return g.add({ id: `plugin:${id}`, kind: "plugin", label: id, detail: pluginDetail(content, id, code), pick: { kind: "plugin", plugin: id }, bus }, params);
  };
  // The entity level: every node of the subtree with something of its own has a box, wired or not (its ports to expose)
  if (!enterEntities) {
    for (const target of targets) {
      if (!target.entity) continue;
      for (const { node, path } of walk(target.entity)) {
        if (!startsWith(path, target.under ?? [])) continue;
        if (node.type !== "group" || nodeSpecs(node, plugins).length > 1 || node.material) entityNode(target.id, target.entity, path);
      }
    }
  }
  for (const target of targets) {
    if (!target.entity) continue;
    for (const { node, path } of walk(target.entity)) {
      if (!startsWith(path, target.under ?? [])) continue;
      const types: Array<[string | null, string]> = [
        ...(node.components ?? []).filter(c => !BUILT_IN_COMPONENTS.includes(c.type)).map((c): [string | null, string] => [pluginOfType(plugins, "components", c.type), c.type]),
        [pluginOfType(plugins, "nodeTypes", node.type), node.type],
        ...(node.material ? [[pluginOfType(plugins, "materials", node.material.type), node.material.type] as [string | null, string]] : []),
      ];
      for (const [plugin, type] of types) {
        if (plugin) g.refer(pluginNode(plugin).id, "drives", entityNode(target.id, target.entity, path).id, type, "out");
      }
    }
  }
  for (const plugin of plugins) {
    const used = code[plugin.id];
    if (!used) continue;
    const touches = [...used.listens, ...used.emits].some(e => ruleEvents.has(e)) || [...used.reads, ...used.sets].some(v => ruleVariables.has(v));
    if (!touches && !drawn.has(plugin.id)) continue;
    const id = pluginNode(plugin.id).id;
    used.listens.forEach(e => g.link([g.eventNode(e).id, "out"], [id, "listens"], "event", "listens", null));
    used.emits.forEach(e => g.link([id, "emits"], [g.eventNode(e).id, "in"], "event", "emits", null));
    // What it reads and sets: display rows with a reference each – a wire could not change its code
    used.reads.forEach(v => g.refer(id, "reads", g.variableNode(v).id, v, "in"));
    used.sets.forEach(v => g.refer(id, "sets", g.variableNode(v).id, v, "out"));
  }
  scopes.forEach((set, name) => { const node = g.nodes.get(`var:${name}`); if (node) node.detail = `variable · ${[...set].join(", ")}`; });
  // The events a node's components declare: wired to the event's box where the level has one (the bus joins them by name)
  for (const box of [...g.nodes.values()]) {
    for (const port of box.ports) {
      if (port.kind !== "event") continue;
      const event = g.nodes.get(`event:${port.label}`);
      if (event) g.link([box.id, port.id], [event.id, "in"], "event", "emits", null);
    }
  }

  // A goto to a scene the experience does not have
  for (const goto of g.gotos) {
    const box = g.nodes.get(goto.from[0]);
    if (box && !content.scenes.some(s => s.id === goto.scene)) g.flag(box, `${box.label}: goto scene:${goto.scene} – the experience has no scene ${goto.scene}`, "error", `goto: no scene ${goto.scene}`);
  }
  flagNames(g, content, uses);
  return g;
};

/** Problems across the experience – only when every plugin's code is known (a plugin may set or emit anything) */
const flagNames = (g: Graph, content: LogicContent, uses: readonly Use[]): void => {
  const plugins = content.plugins ?? [];
  const code = pluginUses(content);
  if (!plugins.every(p => code[p.id])) return;
  const ruleUse = (kind: UseKind) => new Set(uses.filter(u => u.kind === kind && (u.place.target !== null || u.place.plugin !== null || u.place.logic !== null)).map(u => u.name));
  const sets = ruleUse("sets");
  const emits = ruleUse("emits");
  const listens = ruleUse("listens");
  const declared = new Set(Object.keys(content.variables ?? {}));
  for (const node of g.nodes.values()) {
    const read = g.edges.some(e => e.from[0] === node.id) || g.refs.some(r => r.to === node.id && r.from[1] === "reads");
    if (node.kind === "variable" && read && !sets.has(node.label) && !declared.has(node.label)) {
      g.flag(node, `${node.label} is read, but nothing in the experience sets it`, "warning", "read, nothing sets it");
    }
    if (node.kind === "event" && node.id === `event:${node.label}` && !CORE.includes(node.label)) {
      if (g.edges.some(e => e.from[0] === node.id) && !emits.has(node.label)) g.flag(node, `${node.label} is listened to, but nothing in the experience emits it – its rules never fire`, "warning", "listened to, nothing emits it – never fires");
      else if (g.edges.some(e => e.to[0] === node.id) && !listens.has(node.label)) g.flag(node, `${node.label} is emitted, but nothing in the experience listens to it`, "warning", "emitted, nothing listens to it");
    }
  }
};

// ── Levels ───────────────────────────────────────────────────────────────────────────────────────────────────

const sceneOf = (content: LogicContent, id: string) => content.scenes.find(s => s.id === id);
const sceneLabel = (content: LogicContent, id: string) => sceneOf(content, id)?.label ?? id;
const groupOf = (content: LogicContent, scene: LogicScene | undefined) =>
  scene?.group ? content.groups?.find(g => g.id === scene.group) : undefined;

/** The levels from the experience down to `context`, root first: experience › group › scene › entity */
export const levelsOf = (content: LogicContent, context: Context): Context[] => {
  const levels: Context[] = [{ level: "experience" }];
  if (context.level === "experience") return levels;
  if (context.level === "group") return [...levels, context];
  const group = groupOf(content, sceneOf(content, context.scene));
  if (group) levels.push({ level: "group", group: group.id });
  levels.push({ level: "scene", scene: context.scene });
  if (context.level === "entity") {
    levels.push({ level: "entity", scene: context.scene, target: context.target, path: [] });
    if (context.path.length) levels.push(context);
  }
  return levels;
};

/** The words for a place outside the level, and the level that opens it */
const farOf = (content: LogicContent, place: Place, near: { scene: string }): { far: Context; words: string } => {
  if (place.plugin !== null) return { far: { level: "experience" }, words: `plugin ${place.plugin}` };
  if (place.logic === "experience") return { far: { level: "experience" }, words: "the experience's rules" };
  const scene = place.scene!;
  if (scene === near.scene && place.target !== null) return { far: { level: "entity", scene, target: place.target, path: [] }, words: `target ${place.target}` };
  const label = sceneLabel(content, scene);
  if (place.logic === "scene") return { far: { level: "scene", scene }, words: scene === near.scene ? "the scene's rules" : `the rules of ${label}` };
  return { far: { level: "scene", scene }, words: place.target === null ? `the texts of ${label}` : label };
};

const JOIN_WORDS = { variable: { in: "set in", out: "read in" }, event: { in: "emitted in", out: "listened to in" } } as const;
const JOIN_PORTS = { variable: { from: "sets", to: "reads", near: { in: "set", out: "get" } }, event: { from: "emits", to: "listens", near: { in: "in", out: "out" } } } as const;

/**
 * The stubs of a level whose boxes are events and variables (scene, entity): a join with one end here, one outside – drawn
 * at the box of its name, one per side, name and far end
 */
const nameStubs = (content: LogicContent, g: Graph, joins: readonly Join[], isNear: (p: Place) => boolean, near: { scene: string }): Stub[] => {
  const stubs: Stub[] = [];
  for (const join of joins) {
    const fromNear = isNear(join.from);
    const toNear = isNear(join.to);
    if (fromNear === toNear) continue;
    const side = toNear ? "in" : "out";
    const farPlace = toNear ? join.from : join.to;
    if (farPlace.plugin !== null) continue; // a plugin touching this level's names is drawn in it
    const box = g.nodes.get(join.kind === "variable" ? `var:${join.name}` : `event:${join.name}`);
    if (!box) continue; // only a text here reads it: texts are no boxes of this level
    const { far, words } = farOf(content, farPlace, near);
    const farId = `far:${farPlace.key}`;
    const ports = JOIN_PORTS[join.kind];
    const signal: Signal = join.kind === "variable" ? "value" : "event";
    const label = `${join.name} ${side === "in" ? "←" : "→"} ${JOIN_WORDS[join.kind][side]} ${words}`;
    if (stubs.some(s => s.label === label)) continue;
    const edge: GraphEdge = side === "in"
      ? { id: `stub:${farId}>${box.id}:${join.name}`, from: [farId, ports.from], to: [box.id, ports.near.in], signal, label: join.name, home: null, names: [join.name] }
      : { id: `stub:${box.id}>${farId}:${join.name}`, from: [box.id, ports.near.out], to: [farId, ports.to], signal, label: join.name, home: null, names: [join.name] };
    stubs.push({ edge, side, far, label });
  }
  return stubs;
};

/**
 * The goto stubs of the scene and entity levels: each goto of a rule drawn here, at the border, named by its scene
 * ("→ Epilogue · goto"); the edge keeps its rule's home, so disconnecting it removes the goto. A goto into this scene from
 * elsewhere arrives at the scene itself, which is no box here – the group and experience levels draw it.
 */
const gotoStubs = (content: LogicContent, g: Graph): Stub[] =>
  g.gotos.map(goto => ({
    edge: { id: `goto:${goto.from.join(":")}>${goto.scene}`, from: goto.from, to: [`far:scene:${goto.scene}`, "arrives"], signal: "scene", label: "goto", home: goto.home },
    side: "out", far: { level: "scene", scene: goto.scene }, label: `→ ${sceneLabel(content, goto.scene)} · goto`,
  }));

const sceneView = (content: LogicContent, context: Extract<Context, { level: "scene" | "entity" }>, uses: readonly Use[], registry: NodeRegistry): View => {
  const scene = sceneOf(content, context.scene);
  const levels = levelsOf(content, context);
  if (!scene) return { context, levels, nodes: [], edges: [], stubs: [], refs: [], problems: [], canvas: EMPTY_CANVAS() };
  const entity = context.level === "entity";
  const targets = entity ? scene.targets.filter(t => t.id === context.target).map(t => ({ ...t, under: context.path })) : scene.targets;
  const g = targetsGraph(content, uses, { scene: scene.id, targets, enterEntities: !entity, sceneRules: entity ? [] : scene.rules ?? [] }, registry);
  // The scene level holds its texts and its rules too (no boxes for the texts, no stubs); the entity level only its target
  const isNear = (p: Place) => p.scene === scene.id && (!entity || p.target === context.target);
  const stubs = [...nameStubs(content, g, joinsOf(uses), isNear, { scene: scene.id }), ...gotoStubs(content, g)];
  const lists = logicRuleLists(content).filter(l => l.scene === scene.id && (!entity || l.target === context.target));
  const problems = [...g.problems, ...typeProblemsOf(lists, logicTypes(content))];
  const nodes = hideUnused([...g.nodes.values()], [...g.edges, ...stubs.map(s => s.edge)], scene.canvas?.exposed ?? {});
  return { context, levels, nodes, edges: g.edges, stubs, refs: g.refs.filter(r => nodes.some(n => n.id === r.to)), problems, canvas: EMPTY_CANVAS() };
};

/**
 * The Blender way (§5.9.4): a spec's port is hidden unless a wire uses it or the author exposed it (`exposed`: box id → port
 * ids, the canvas's); the grammar's own ports always show
 */
export const hideUnused = (nodes: GraphNode[], edges: readonly GraphEdge[], exposed: Readonly<Record<string, readonly string[]>>): GraphNode[] =>
  nodes.map(node => {
    if (!node.ports.some(p => p.kind)) return node;
    const used = new Set(edges.flatMap(e => [e.from[0] === node.id ? `out:${e.from[1]}` : "", e.to[0] === node.id ? `in:${e.to[1]}` : ""]));
    const shown = new Set(exposed[node.id] ?? []);
    return { ...node, ports: node.ports.map(p => (p.kind && !used.has(`${p.dir}:${p.id}`) && !shown.has(p.id) ? { ...p, hidden: true } : p)) };
  });

/** A place as a box of the group and experience levels: a scene (rules and texts together) or a plugin; the experience's rules: none */
const boxOf = (place: Place) => (place.plugin !== null ? `plugin:${place.plugin}` : place.scene !== null ? `scene:${place.scene}` : null);

/** The derived edges between boxes: one per kind and pair, with a count – the deck's bundled wires */
const bundle = (joins: readonly Join[]) => {
  const pairs = new Map<string, { kind: Join["kind"]; from: string; to: string; fromPlace: Place; toPlace: Place; names: Set<string> }>();
  for (const join of joins) {
    const from = boxOf(join.from);
    const to = boxOf(join.to);
    if (!from || !to || from === to) continue;
    const key = `${join.kind}|${from}|${to}`;
    const pair = pairs.get(key) ?? { kind: join.kind, from, to, fromPlace: join.from, toPlace: join.to, names: new Set<string>() };
    pair.names.add(join.name);
    pairs.set(key, pair);
  }
  return [...pairs.values()].map(p => {
    const names = [...p.names].sort();
    const label = names.length === 1 ? names[0] : plural(names.length, p.kind);
    const ports = JOIN_PORTS[p.kind];
    const edge: GraphEdge = { id: `${p.kind}:${p.from}>${p.to}`, from: [p.from, ports.from], to: [p.to, ports.to], signal: p.kind === "variable" ? "value" : "event", label, home: null, names };
    return { ...p, names, edge };
  });
};

/** The derived goto edges between scenes: one per pair, with a count (the experience's own rules draw theirs) */
const gotoPairs = (gotos: readonly Goto[]) => {
  const pairs = new Map<string, { from: string; to: string; fromScene: string; toScene: string; n: number }>();
  for (const goto of gotos) {
    if (goto.place.scene === null || goto.place.scene === goto.scene) continue;
    const key = `${goto.place.scene}>${goto.scene}`;
    const pair = pairs.get(key) ?? { from: `scene:${goto.place.scene}`, to: `scene:${goto.scene}`, fromScene: goto.place.scene, toScene: goto.scene, n: 0 };
    pair.n++;
    pairs.set(key, pair);
  }
  return [...pairs.values()];
};

/** How many rules a scene holds: its targets' and its own */
const ruleCount = (scene: LogicScene) =>
  (scene.rules?.length ?? 0) + scene.targets.reduce((n, t) => n + (t.entity ? walk(t.entity).reduce((m, { node }) => m + rulesOfNode(node).length, 0) : 0), 0);

/** The problems of a set of scenes (each scene's, once), pointing at the scene's box – and `at` the box in the scene's level */
const scenesProblems = (content: LogicContent, scenes: readonly LogicScene[], uses: readonly Use[], registry: NodeRegistry): Problem[] => {
  const problems: Problem[] = [];
  for (const scene of scenes) {
    for (const p of targetsGraph(content, uses, { scene: scene.id, targets: scene.targets, enterEntities: false, sceneRules: scene.rules ?? [] }, registry).problems) {
      if (problems.some(q => q.message === p.message)) continue;
      problems.push({ ...p, node: `scene:${scene.id}`, ...(p.node ? { at: { context: { level: "scene" as const, scene: scene.id }, node: p.node } } : {}) });
    }
  }
  return problems;
};

/** Where a target is: its scene and its entity */
const targetOf = (content: LogicContent, id: string) => {
  for (const scene of content.scenes) {
    const target = scene.targets.find(t => t.id === id);
    if (target) return { scene, target };
  }
  return null;
};

/**
 * The experience's rules as boxes of a graph (the experience level): their events and variables, a goto into the scene box,
 * a node action onto the box of the scene that holds the target
 */
const experienceRules = (content: LogicContent, g: Graph): void => {
  const scenes = new Set(content.scenes.map(s => s.id));
  (content.rules ?? []).forEach((rule, r) => {
    const id = experienceRuleBox(r);
    const label = logicRuleLabel({ doc: "experience", rule: r });
    const home = (part: "on" | "when" | "do", at?: number): Home => ({ doc: "experience", rule: r, part, ...(at === undefined ? {} : { index: at }) });
    g.add({ id, kind: "rule", label, detail: conditionWords(rule.when), pick: { kind: "experience" }, home: home("on") }, rule);
    g.ruleWires(id, rule, home, {
      scenes,
      nodeOf: action => {
        const address = parseAddress(action.node);
        const found = address?.scheme === "target" ? targetOf(content, address.id) : null;
        const path = found?.target.entity && address?.scheme === "target" ? pathOfName(found.target.entity, address.node) : null;
        const box = g.nodes.get(id)!;
        if (!found) {
          const why = address ? "no such target in the experience" : "not an address (target:<id>/<name>)";
          g.flag(box, `${label}: ${action.action} names ${action.node || "no node"} – ${why}`, "error", `${action.action} ${action.node || "no node"}: ${why}`);
          return null;
        }
        if (!path) g.flag(box, `${label}: ${action.action} names ${action.node} – no node of that name on target ${found.target.id}`, "error", `${action.action} ${action.node}: not on target ${found.target.id}`);
        return [`scene:${found.scene.id}`, "nodes"];
      },
    });
    for (const goto of g.gotos.splice(0)) {
      g.flag(g.nodes.get(goto.from[0])!, `${label}: goto scene:${goto.scene} – the experience has no scene ${goto.scene}`, "error", `goto: no scene ${goto.scene}`);
    }
  });
};

const sceneBox = (g: Graph, scene: LogicScene, group: string | undefined) =>
  g.add({
    id: `scene:${scene.id}`, kind: "scene", label: scene.label, detail: ruleCount(scene) ? plural(ruleCount(scene), "rule") : "no rules",
    pick: { kind: "scene", scene: scene.id }, enter: { level: "scene", scene: scene.id }, ...(group ? { group: `group:${group}` } : {}),
  });

const groupView = (content: LogicContent, context: Extract<Context, { level: "group" }>, uses: readonly Use[], registry: NodeRegistry): View => {
  const g = new Graph(registry);
  const scenes = content.scenes.filter(s => s.group === context.group);
  scenes.forEach(scene => sceneBox(g, scene, undefined));
  const inside = (p: Place) => p.scene !== null && scenes.some(s => s.id === p.scene);
  const insideScene = (id: string) => scenes.some(s => s.id === id);
  const stubs: Stub[] = [];
  const joins = joinsOf(uses).filter(j => j.from.scene !== j.to.scene);
  for (const pair of bundle(joins)) {
    const fromIn = inside(pair.fromPlace);
    const toIn = inside(pair.toPlace);
    if (fromIn && toIn) g.link(pair.edge.from, pair.edge.to, pair.edge.signal, pair.edge.label, null, pair.names);
    else if (fromIn || toIn) {
      const side = toIn ? "in" : "out";
      const farPlace = toIn ? pair.fromPlace : pair.toPlace;
      const words = farPlace.plugin !== null ? `plugin ${farPlace.plugin}` : sceneLabel(content, farPlace.scene!);
      const far: Context = farPlace.plugin !== null ? { level: "experience" } : { level: "scene", scene: farPlace.scene! };
      stubs.push({ edge: pair.edge, side, far, label: `${pair.edge.label} ${side === "in" ? "←" : "→"} ${JOIN_WORDS[pair.kind][side]} ${words}` });
    }
  }
  for (const pair of gotoPairs(gotosOf(content))) {
    const [fromIn, toIn] = [insideScene(pair.fromScene), insideScene(pair.toScene)];
    const edge: GraphEdge = { id: `goto:${pair.from}>${pair.to}`, from: [pair.from, "goto"], to: [pair.to, "arrives"], signal: "scene", label: pair.n > 1 ? `goto · ${pair.n}` : "goto", home: null };
    if (fromIn && toIn) g.link(edge.from, edge.to, "scene", edge.label, null);
    else if (fromIn) stubs.push({ edge, side: "out", far: { level: "scene", scene: pair.toScene }, label: `→ ${sceneLabel(content, pair.toScene)} · goto` });
    else if (toIn) stubs.push({ edge, side: "in", far: { level: "scene", scene: pair.fromScene }, label: `← ${sceneLabel(content, pair.fromScene)} · goto` });
  }
  return {
    context, levels: levelsOf(content, context), nodes: [...g.nodes.values()], edges: g.edges, stubs, refs: g.refs,
    problems: scenesProblems(content, scenes, uses, registry), canvas: EMPTY_CANVAS(),
  };
};

const experienceView = (content: LogicContent, context: Context, uses: readonly Use[], registry: NodeRegistry): View => {
  const g = new Graph(registry);
  const code = pluginUses(content);
  const groups = new Set((content.groups ?? []).map(group => group.id));
  for (const group of content.groups ?? []) {
    const count = content.scenes.filter(s => s.group === group.id).length;
    g.add({ id: `group:${group.id}`, kind: "group", label: group.label, detail: plural(count, "scene"), pick: { kind: "group", group: group.id }, enter: { level: "group", group: group.id } });
  }
  content.scenes.forEach(scene => sceneBox(g, scene, scene.group && groups.has(scene.group) ? scene.group : undefined));
  for (const plugin of content.plugins ?? []) {
    const { params, bus } = pluginBus(code[plugin.id]);
    g.add({ id: `plugin:${plugin.id}`, kind: "plugin", label: plugin.id, detail: pluginDetail(content, plugin.id, code), pick: { kind: "plugin", plugin: plugin.id }, bus }, params);
  }

  // Variables that cross scenes: set in a wider scope than the scene's, or by a plugin – through their own box
  const crossing = new Map<string, Set<string>>();
  for (const use of uses) {
    if (use.kind === "sets" && use.scope !== "scene") crossing.set(use.name, (crossing.get(use.name) ?? new Set()).add(use.scope ?? "plugin"));
  }
  const counted = new Map<string, { from: [string, string]; to: [string, string]; word: string; n: number }>();
  const count = (from: [string, string], to: [string, string], word: string) => {
    const key = `${from.join(":")}>${to.join(":")}`;
    const known = counted.get(key) ?? { from, to, word, n: 0 };
    known.n++;
    counted.set(key, known);
  };
  for (const [name, scopes] of crossing) {
    g.add({ id: `var:${name}`, kind: "variable", label: name, detail: `variable · ${[...scopes].join(", ")}`, pick: null });
  }
  for (const use of uses) {
    const box = boxOf(use.place);
    if (!box) continue; // the experience's rules draw their own wires
    if ((use.kind === "sets" || use.kind === "reads") && crossing.has(use.name) && use.place.plugin !== null) {
      // A plugin's read or set: a display row and its reference (#334)
      g.refer(box, use.kind, `var:${use.name}`, use.name, use.kind === "reads" ? "in" : "out");
    } else if ((use.kind === "sets" || use.kind === "reads") && crossing.has(use.name)) {
      if (use.kind === "sets") count([box, "sets"], [`var:${use.name}`, "set"], "sets");
      else count([`var:${use.name}`, "get"], [box, "reads"], "reads");
    }
    if (use.kind === "listens" && CORE.includes(use.name)) {
      g.eventNode(use.name);
      count([`event:${use.name}`, "out"], [box, "listens"], "on");
    }
  }
  for (const { from, to, word, n } of counted.values()) {
    if (g.nodes.has(from[0]) && g.nodes.has(to[0])) g.link(from, to, from[0].startsWith("event:") ? "event" : "value", n > 1 ? `${word} · ${n}` : word, null);
  }

  // Events across places: one derived edge per pair (the bus joins them by name)
  const joins = joinsOf(uses).filter(j => j.kind === "event" && boxOf(j.from) !== boxOf(j.to));
  for (const pair of bundle(joins)) {
    if (g.nodes.has(pair.from) && g.nodes.has(pair.to)) g.link(pair.edge.from, pair.edge.to, "event", pair.edge.label, null, pair.names);
  }
  // Gotos between scenes (derived), then the experience's own rules with theirs (written – editable here)
  for (const pair of gotoPairs(gotosOf(content))) {
    if (g.nodes.has(pair.from) && g.nodes.has(pair.to)) g.link([pair.from, "goto"], [pair.to, "arrives"], "scene", pair.n > 1 ? `goto · ${pair.n}` : "goto", null);
  }
  experienceRules(content, g);
  flagNames(g, content, uses);
  const problems = [...g.problems, ...scenesProblems(content, content.scenes, uses, registry)];
  const lists = logicRuleLists(content);
  problems.push(...typeProblemsOf(lists, logicTypes(content)));
  return {
    context, levels: [{ level: "experience" }], nodes: [...g.nodes.values()], edges: g.edges, stubs: [], refs: g.refs,
    problems: problems.filter((p, i) => problems.findIndex(q => q.message === p.message) === i), canvas: EMPTY_CANVAS(),
  };
};

/** The problems of the whole experience, each once (`apply.ts`'s `problems`): the scenes', the experience's rules', the types' */
export const experienceProblems = (content: LogicContent, options: ProjectOptions = {}): Problem[] =>
  experienceView(content, { level: "experience" }, usesOf(content), options.registry ?? coreRegistry).problems;

/** The logic view of one level (see the module's comment) */
export const project = (content: LogicContent, context: Context, { registry = coreRegistry }: ProjectOptions = {}): View => {
  const uses = usesOf(content);
  switch (context.level) {
    case "experience": return experienceView(content, context, uses, registry);
    case "group": return groupView(content, context, uses, registry);
    default: return sceneView(content, context, uses, registry);
  }
};
