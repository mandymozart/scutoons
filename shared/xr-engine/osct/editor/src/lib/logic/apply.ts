import {
  BEHAVIOUR_CORE_EVENTS, BEHAVIOUR_NODE_ACTIONS, BEHAVIOUR_TYPE, behaviourData, behaviourRules, isBehaviourNodeEvent,
  type BehaviourAction, type BehaviourRule, type ComponentData, type EntityData, type LogicCanvas, type LogicCanvasGroup, type LogicCanvasNote,
} from "@engine/types";
import { CAMEL_CASE_NAME } from "@shared/guards/primitives";
import { expressionVariables } from "@shared/text/expressions";
import { addressProblem, nodeAddress, parseAddress, sceneAddress } from "@shared/types/address";
import type { VariableInfo, VariableType } from "@shared/types/variables";
import { appendCondition, literalOf, newAction, pathOfName, removeConditionTerms, rulesOf, setValueProblems, typeWord, whenProblems, withAction, withRules } from "#lib/behaviour-edit.js";
import { nodeLabel } from "#lib/tree.js";
import { nodeAt, withNode } from "#lib/values.js";
import { NOTE_BESIDE, flowOf, noteView } from "./flow";
import { GRID, GROUP_COLOURS, snap } from "./signals";
import { conditionWords, experienceProblems, logicRuleLabel, logicTypes, project, type LogicContent, type ProjectOptions } from "./project";
import { typeTakes, typeWords } from "./spec-ports";
import type { CanvasEdit, Context, GraphEdge, GraphNode, Home, Op, PortSpec, PortType, Problem, RulePlace, RuleRef, View } from "./types";

/**
 * `apply(op, docs)` (#334 L-T4, L-T5b, research §5.1, §5.5): an edit of the logic as an **op on the documents** – pure, no
 * store, no DOM. It returns the new documents with what happened **in words** (the panel's status line), the trees it
 * changed (the panel hands them to the stage, which saves them like the Inspector's edits), the scene and experience rules it
 * changed (the panel saves them with `PUT …/scenes/{sid}` / `PUT …/logic`) and the experience's `problems` after it; or a
 * **refusal with its reason**. Rules live on a node (`Home.doc` "entity", a `behaviour` component), on a scene or on the
 * experience (content schema 5.0). The rule grammar is `behaviour-edit.ts`'s and the shared contract's – nothing here
 * writes a rule another way, and every expression it writes is type-checked first (B8).
 *
 * `connect` follows the ports of the grammar (`registry.ts`): an event's outlet → a rule's `on` writes the event's name into
 * `on` (the bus stays by name, L6); a variable → a rule's `when` adds a term to its condition (a boolean variable its name,
 * any other `name == <its default or a typed placeholder>`, joined with `&&`); a rule's `do` → a node, a variable, an event
 * or a scene box adds the matching action (show / hide / play / pause / stop, set, emit, goto) – from an action's own
 * `do:<i>` it re-aims that action. A scene's rule names a node of its own scene's targets by address. `disconnect` removes
 * what a wire stands for – a cut `on` leaves the rule listening to nothing, a problem until an event is wired to it again
 * (Tilman, 2026-11-20); a cut `when` takes the variable's terms out of a top-level `&&` / `||` chain (anything deeper is
 * the Inspector's); a wire that only joins by name (`home: null`) removes the name from the listening side here, and says
 * so. Ports and wires are named by the level's view (`project`), so `connect` and `disconnect` take the level (`context`).
 * The group level is read-only; the experience level edits the experience's rules only.
 *
 * **Ports from the specs** (L-T6, §5.9, B8): connecting is strictly typed (`typedRefusal`) – a value inlet takes only its
 * type (an integer where a number goes), a refusal says why ("volume takes a number 0–1, not a text (mood)"). A wire still
 * needs a field to land in (L1): a rule's do into a node's **action inlet** (`act:play`) writes that verb on the node; a
 * node's **event outlet** (`ev:ended`) into a rule's `on` makes the rule listen to that event by name; a **parameter inlet** has
 * no rule action yet, and a **file inlet** waits for file nodes (L-T12) – both are refused with the reason. The canvas ops
 * change the level's canvas (the scene's `LogicCanvas`, or the experience's): `expose` and `hideUnused` its exposed ports,
 * and (L-T7, schema 5.2) `place` / `tidy` its pinned boxes, `fold` a box to its head, `group` / `renameGroup` /
 * `groupColour` / `foldGroup` / `ungroup` the deck's groups – saved like a rule's edit, so History lists them and Undo takes
 * them back. **Notes** (schema 5.3, the deck's `Note` / `NoteText`): `note` adds an empty note – free at a point, or beside a
 * box, by its author – `noteText` writes its text (empty removes it), `place` with `note:<id>` moves it. A note is no logic:
 * no op but these reads it. When an op takes away the box a note stands beside (a rule removed, a variable nothing names any
 * more), the note stays – free, where it last stood (`keepNotes`); a removed rule's later siblings keep their notes.
 */

/** A tree an op changed: the target's whole new entity */
export interface TreeChange {
  scene: string;
  target: string;
  entity: EntityData;
}

/** A scene's or the experience's rules an op changed: the whole new list */
export type LogicChange = { doc: "scene"; scene: string; rules: BehaviourRule[] } | { doc: "experience"; rules: BehaviourRule[] };

/** A scene's or the experience's canvas an op changed (undefined: it holds nothing any more) */
export type CanvasChange = { doc: "scene"; scene: string; canvas: LogicCanvas | undefined } | { doc: "experience"; canvas: LogicCanvas | undefined };

export type OpResult =
  | { ok: true; content: LogicContent; changed: TreeChange[]; logic: LogicChange[]; canvas: CanvasChange[]; words: string; problems: Problem[] }
  | { ok: false; reason: string };

const CORE = BEHAVIOUR_CORE_EVENTS as readonly string[];
const isName = (name: string) => CAMEL_CASE_NAME.test(name);
const refuse = (reason: string): OpResult => ({ ok: false, reason });
const READ_ONLY = "The group level is read-only – open a scene or a target to edit its rules.";
const EXPERIENCE_ONLY = "Here only the experience's own rules are edited – open a scene or a target to edit its rules.";
const CODE = (plugin: string) => `The plugin ${plugin}'s code decides what it listens to, emits, reads and sets – change it in the Code tab.`;
const INSPECTOR = "edit the condition in the Inspector";

/** The experience's problems (the deck's "compile"): what the Logic panel lists under the graph */
export const problems = (content: LogicContent, options: ProjectOptions = {}): Problem[] => experienceProblems(content, options);

// ── The documents: a target's tree, a node's rules, a scene's rules, the experience's ─────────────────────────────

type EntityRef = Extract<RuleRef, { doc: "entity" }>;
type LogicRef = Exclude<RuleRef, { doc: "entity" }>;

/** A home without its part: the rule it names */
export const refOf = (home: RuleRef): RuleRef =>
  home.doc === "entity" ? { doc: "entity", target: home.target, path: home.path, component: home.component, rule: home.rule }
    : home.doc === "scene" ? { doc: "scene", scene: home.scene, rule: home.rule } : { doc: "experience", rule: home.rule };

/** Where a target sits in the content */
const findTarget = (content: LogicContent, target: string) => {
  for (const scene of content.scenes) {
    const found = scene.targets.find(t => t.id === target);
    if (found) return { scene, target: found };
  }
  return null;
};

/** The rules of a scene's or the experience's logic */
const logicRulesOf = (content: LogicContent, ref: LogicRef): readonly BehaviourRule[] | null => {
  if (ref.doc === "experience") return content.rules ?? [];
  const scene = content.scenes.find(s => s.id === ref.scene);
  return scene ? scene.rules ?? [] : null;
};

interface RuleAt {
  /** The scene it belongs to; null: the experience's */
  scene: string | null;
  rules: readonly BehaviourRule[];
  rule: BehaviourRule;
  /** A node's rule: its node */
  node?: EntityData;
}

/** The rule a ref names, with its list (and its node) – or null when there is none */
const ruleAt = (content: LogicContent, ref: RuleRef): RuleAt | null => {
  if (ref.doc !== "entity") {
    const rules = logicRulesOf(content, ref);
    const rule = rules?.[ref.rule];
    return rules && rule ? { scene: ref.doc === "scene" ? ref.scene : null, rules, rule } : null;
  }
  const found = findTarget(content, ref.target);
  const root = found?.target.entity;
  const node = root && nodeAt(root, ref.path);
  if (!found || !root || !node) return null;
  if (node.components?.[ref.component]?.type !== BEHAVIOUR_TYPE) return null;
  const rules = rulesOf(node, ref.component);
  const rule = rules[ref.rule];
  return rule ? { scene: found.scene.id, rules, rule, node } : null;
};

/** A rule's name as the canvas shows it: its node's label and its number in the target (the projection counts the same way), or `scene rule 2` */
export const ruleName = (content: LogicContent, ref: RuleRef | Omit<EntityRef, "doc">): string => {
  if ("doc" in ref && ref.doc !== "entity") return logicRuleLabel(ref);
  const entity = ref as Omit<EntityRef, "doc">;
  const root = findTarget(content, entity.target)?.target.entity;
  if (!root) return "the rule";
  let n = 0;
  const visit = (node: EntityData, path: number[]): string | null => {
    for (const [c, component] of (node.components ?? []).entries()) {
      if (component.type !== BEHAVIOUR_TYPE) continue;
      for (let r = 0; r < behaviourRules(component).length; r++) {
        n++;
        if (c === entity.component && r === entity.rule && path.join(".") === entity.path.join(".")) return `${nodeLabel(node)} · rule ${n}`;
      }
    }
    for (const [i, child] of (node.children ?? []).entries()) {
      const hit = visit(child, [...path, i]);
      if (hit) return hit;
    }
    return null;
  };
  return visit(root, []) ?? "the rule";
};

/** Edits on a working copy of the content; each tree and each rules list changed is kept once, its newest form */
class Draft {
  changed = new Map<string, TreeChange>();
  logic = new Map<string, LogicChange>();
  canvas = new Map<string, CanvasChange>();
  /** The variables' types before the op (declared, else inferred) */
  types: Map<string, VariableInfo>;
  constructor(public content: LogicContent) {
    this.types = logicTypes(content);
  }

  typeOf = (name: string) => this.types.get(name)?.type;

  /** Replace a node's rules (its behaviour component at `component`, or a new one when `component` is past its components) */
  setRules(target: string, path: number[], component: number, rules: readonly BehaviourRule[]): void {
    const found = findTarget(this.content, target);
    const root = found?.target.entity;
    const node = root && nodeAt(root, path);
    if (!found || !root || !node) return;
    const nextNode = component >= (node.components?.length ?? 0)
      ? { ...node, components: [...(node.components ?? []), behaviourData(rules) as ComponentData] }
      : withRules(node, component, rules);
    const entity = withNode(root, path, nextNode);
    this.content = {
      ...this.content,
      scenes: this.content.scenes.map(s => (s.id !== found.scene.id ? s : { ...s, targets: s.targets.map(t => (t.id === target ? { ...t, entity } : t)) })),
    };
    this.changed.set(target, { scene: found.scene.id, target, entity });
  }

  /** Replace a scene's or the experience's rules */
  setLogic(ref: { doc: "scene"; scene: string } | { doc: "experience" }, rules: BehaviourRule[]): void {
    if (ref.doc === "experience") {
      this.content = { ...this.content, rules };
      this.logic.set("experience", { doc: "experience", rules });
    } else {
      this.content = { ...this.content, scenes: this.content.scenes.map(s => (s.id === ref.scene ? { ...s, rules } : s)) };
      this.logic.set(`scene:${ref.scene}`, { doc: "scene", scene: ref.scene, rules });
    }
  }

  /** Replace a scene's or the experience's canvas */
  setCanvas(ref: { doc: "scene"; scene: string } | { doc: "experience" }, canvas: LogicCanvas | undefined): void {
    if (ref.doc === "experience") {
      this.content = { ...this.content, canvas };
      this.canvas.set("experience", { doc: "experience", canvas });
    } else {
      this.content = { ...this.content, scenes: this.content.scenes.map(s => (s.id === ref.scene ? { ...s, canvas } : s)) };
      this.canvas.set(`scene:${ref.scene}`, { doc: "scene", scene: ref.scene, canvas });
    }
  }

  setRule(ref: RuleRef, rule: BehaviourRule | undefined): void {
    const at = ruleAt(this.content, ref);
    if (!at) return;
    const rules = rule ? at.rules.map((r, i) => (i === ref.rule ? rule : r)) : at.rules.filter((_, i) => i !== ref.rule);
    if (ref.doc === "entity") this.setRules(ref.target, ref.path, ref.component, rules);
    else this.setLogic(ref, rules);
  }

  done(words: string, options: ProjectOptions): OpResult {
    return {
      ok: true, content: this.content, changed: [...this.changed.values()], logic: [...this.logic.values()], canvas: [...this.canvas.values()], words,
      problems: problems(this.content, options),
    };
  }
}

// ── Words ────────────────────────────────────────────────────────────────────────────────────────────────

/** An action in words: `show lamp`, `hide its own node`, `set taps = taps + 1 (scene)`, `emit doorOpened`, `goto scene:epilogue` */
export const actionWords = (action: BehaviourAction): string => {
  switch (action.action) {
    case "set": return `set ${action.name || "?"} = ${action.value ?? "unset"}${action.scope ? ` (${action.scope})` : ""}`;
    case "emit": return `emit ${action.event || "?"}`;
    case "goto": return `goto ${action.address || "?"}`;
    case "play": return `play ${action.clip || "its clip"}${action.node ? ` on ${action.node}` : ""}`;
    default: return `${action.action} ${action.node || "its own node"}`;
  }
};

// ── Checks the grammar makes ────────────────────────────────────────────────────────────────────────────────

/** Why an action cannot be written in this rule (null: it can) */
const actionProblem = (draft: Draft, ref: RuleRef, at: RuleAt, action: BehaviourAction): string | null => {
  switch (action.action) {
    case "set": {
      if (!isName(action.name)) return `${action.name ? `"${action.name}"` : "An empty name"} is no variable name – camelCase letters and digits.`;
      const wrong = setValueProblems(action.value ?? "", action.name, draft.typeOf);
      return wrong.length ? `set ${action.name} = ${action.value}: ${wrong[0]}.` : null;
    }
    case "emit":
      if (!isName(action.event)) return `${action.event ? `"${action.event}"` : "An empty name"} is no event name – camelCase letters and digits.`;
      return CORE.includes(action.event) ? `${action.event} is the engine's own event – a rule listens to it, never emits it.` : null;
    case "goto": {
      const problem = addressProblem(action.address, ["scene"]);
      if (problem) return `goto: ${problem}.`;
      const id = parseAddress(action.address)!.id;
      return draft.content.scenes.some(s => s.id === id) ? null : `The experience has no scene ${id}.`;
    }
    default: {
      if (ref.doc === "entity") {
        return pathOfName(at.node!, action.node) ? null : `No node named ${action.node} under ${nodeLabel(at.node!)} – ${action.action} reaches only its own subtree.`;
      }
      const problem = addressProblem(action.node, ["target"]);
      if (problem) return `A ${ref.doc}'s rule names a node by address – ${problem}.`;
      const address = parseAddress(action.node) as { id: string; node: string };
      const found = findTarget(draft.content, address.id);
      if (!found) return `There is no target ${address.id}.`;
      if (ref.doc === "scene" && found.scene.id !== ref.scene) return `The target ${address.id} is in another scene – a scene's rule acts on its own scene's targets.`;
      return found.target.entity && pathOfName(found.target.entity, address.node) ? null : `No node named ${address.node} on the target ${address.id}.`;
    }
  }
};

// ── The ops ───────────────────────────────────────────────────────────────────────────────────────────────

/** Why a rule cannot be edited at this level (null: it can) */
const outside = (content: LogicContent, context: Context, ref: RuleRef | RulePlace): string | null => {
  if (context.level === "group") return READ_ONLY;
  if (ref.doc === "experience") return context.level === "experience" ? null : "The experience's rules are edited on the experience level of the Logic view, or in its Inspector.";
  if (context.level === "experience") return EXPERIENCE_ONLY;
  if (ref.doc === "scene") return ref.scene === context.scene ? null : "That scene's rules are edited on its own level – open it there.";
  const found = findTarget(content, ref.target);
  if (!found) return `There is no target ${ref.target}.`;
  return found.scene.id === context.scene ? null : `The target ${ref.target} is in another scene – open it there to edit its rules.`;
};

/** The first event a scene's or the experience's new rule listens to: when the scene opens */
const LOGIC_FIRST_EVENT = "sceneChanged";

/**
 * The event a new rule listens to when nothing names one (#334): a node's rule its node's `found` (the target is seen), a
 * scene's or the experience's rule `sceneChanged` (the scene opens) – the add menu's action or condition chosen with no
 * rule selected makes a rule on this event
 */
export const firstEvent = (at: RulePlace): string => (at.doc === "entity" ? "found" : LOGIC_FIRST_EVENT);

type NewRule = { on?: string; when?: string | null; do?: BehaviourAction[] };

const addRule = (draft: Draft, at: RulePlace, fresh: NewRule, options: ProjectOptions): OpResult => {
  const event = fresh.on ?? firstEvent(at);
  const when = fresh.when?.trim() || null;
  const actions = fresh.do ?? [];
  let ref: RuleRef;
  if (at.doc !== "entity") {
    if (!isName(event)) return refuse(`"${event}" is no event – a scene event's name (camelCase) or a core event.`);
    if (isBehaviourNodeEvent(event)) return refuse(`${event} is a node's own event – a ${at.doc}'s rule listens to the bus: a scene event or a core event.`);
    const rules = at.doc === "experience" ? draft.content.rules ?? [] : draft.content.scenes.find(s => s.id === at.scene)?.rules ?? (draft.content.scenes.some(s => s.id === at.scene) ? [] : undefined);
    if (!rules) return refuse(`There is no scene ${(at as { scene: string }).scene}.`);
    draft.setLogic(at, [...rules, { on: event, when: null, do: [] }]);
    ref = at.doc === "experience" ? { doc: "experience", rule: rules.length } : { doc: "scene", scene: at.scene, rule: rules.length };
  } else {
    if (!isName(event)) return refuse(`"${event}" is no event – found, lost, tap, unlocked or an event's name (camelCase).`);
    const root = findTarget(draft.content, at.target)?.target.entity;
    const node = root && nodeAt(root, at.path);
    if (!node) return refuse("That node is not in the target's tree any more.");
    const component = (node.components ?? []).findIndex(c => c.type === BEHAVIOUR_TYPE);
    const at2 = component < 0 ? (node.components?.length ?? 0) : component;
    const rules = component < 0 ? [] : rulesOf(node, component);
    draft.setRules(at.target, at.path, at2, [...rules, { on: event, when: null, do: [] }]);
    ref = { doc: "entity", target: at.target, path: at.path, component: at2, rule: rules.length };
  }
  const name = ref.doc === "entity" ? ruleName(draft.content, ref) : logicRuleLabel(ref);
  if (!when && !actions.length) return draft.done(`Added ${name}: on ${event}, nothing to do yet – connect its do to a node, a variable, an event or a scene.`, options);
  // A rule that comes with a condition and actions: each checked as a wire into it would be (B8)
  if (when) {
    const wrong = whenProblems(when, draft.typeOf);
    if (wrong.length) return refuse(`${when}: ${wrong[0]}.`);
  }
  const added = ruleAt(draft.content, ref)!;
  for (const action of actions) {
    const problem = actionProblem(draft, ref, added, action);
    if (problem) return refuse(problem);
  }
  draft.setRule(ref, { on: event, when, do: actions });
  const does = actions.length ? `, does ${actions.map(actionWords).join(", ")}` : ", nothing to do yet";
  return draft.done(`Added ${name}: on ${event}${when ? ` when ${conditionWords(when)}` : ""}${does} – wire another event to its on to change what it listens to.`, options);
};

/** The rule a rule box stands for: the box carries its home (an on may be empty, so no wire has to say it) */
const ruleOfBox = (view: View, box: string): RuleRef | null => {
  const home = view.nodes.find(n => n.id === box)?.home;
  return home ? refOf(home) : null;
};

/** The view's box by id; a stub or a box standing for something elsewhere is no end an op can use */
const boxOf = (view: View, id: string): GraphNode | null => view.nodes.find(n => n.id === id) ?? null;

const KIND_WORDS: Record<string, string> = {
  event: "an event", rule: "a rule", variable: "a variable", node: "a node", plugin: "a plugin", scene: "a scene", group: "a group", far: "something elsewhere",
};
const kindWord = (node: GraphNode) => KIND_WORDS[node.kind] ?? node.kind;

/** The scene event a box stands for; null for a node's own event (`found` of the door) */
const sceneEvent = (node: GraphNode) => (node.kind === "event" && node.id === `event:${node.label}` ? node.label : null);

/** The term a variable wired into a condition adds: a boolean (or untyped) its name, any other `name == <literal>` */
const conditionTerm = (draft: Draft, name: string): { term: string; note: string } => {
  const info = draft.types.get(name);
  if (!info || info.type === "boolean") return { term: name, note: "" };
  const literal = literalOf(info);
  return { term: `${name} == ${literal}`, note: ` – ${name} is ${typeWord(info.type)}, so it is compared with ${literal}; change the value in the Inspector` };
};

/** A variable's type as a port's type */
const variableType = (type: VariableType): PortType => (type === "number" ? { kind: "number", min: -Infinity, max: Infinity } : type === "integer" ? { kind: "integer", min: -Infinity, max: Infinity } : { kind: type });

const VARIABLE_WORDS: Record<VariableType, string> = { number: "a number", integer: "a whole number", boolean: "true or false", string: "a text" };

/** One end of a wire: a box and its port */
export interface WireEnd {
  node: GraphNode;
  port: string;
}

/** The spec's port of an end, if it is one (`spec-ports.ts`) */
const specPort = (end: WireEnd, dir: "in" | "out"): PortSpec | undefined => end.node.ports.find(p => p.id === end.port && p.dir === dir && p.kind);

/**
 * Why a wire between a spec's port and anything cannot be (B8, L1) – or null when the types allow it (the documents decide
 * the rest). `from` is the outlet's end, `to` the inlet's; `typeOf` gives a variable's type. Cheap and pure: the canvas lights
 * the inlets that take a dragged wire with it, and `connect` refuses with its words.
 */
export const typedRefusal = (from: WireEnd, to: WireEnd, typeOf: (name: string) => VariableType | undefined): string | null => {
  const inlet = specPort(to, "in");
  const outlet = specPort(from, "out");
  if (!inlet && !outlet) return null;
  const isDo = from.node.kind === "rule" && (from.port === "do" || from.port.startsWith("do:"));
  const what = (end: WireEnd): string => {
    if (end.node.kind === "variable") {
      const type = typeOf(end.node.label);
      return `${type ? VARIABLE_WORDS[type] : "a value"} (${end.node.label})`;
    }
    if (end.node.kind === "rule") return end.port === "on" ? "a rule's on" : end.port === "when" ? "a rule's when" : "a rule's action";
    const port = end.node.ports.find(p => p.id === end.port);
    if (port?.kind === "event") return `an event (${port.label})`;
    return KIND_WORDS[end.node.kind] ?? end.node.kind;
  };
  if (inlet?.kind === "src") {
    return `${to.node.label}'s ${inlet.label} takes ${typeWords(inlet.type!)}, not ${what(from)} – files on the canvas come later; pick the file in the Inspector.`;
  }
  if (inlet?.kind === "param") {
    const type = inlet.type!;
    if (from.node.kind !== "variable") return `${inlet.label} takes ${typeWords(type)}, not ${what(from)}.`;
    const varType = typeOf(from.node.label);
    if (varType && !typeTakes(type, variableType(varType))) return `${inlet.label} takes ${typeWords(type)}, not ${what(from)}.`;
    return `${inlet.label} takes ${typeWords(type)} – no rule action sets a component's parameter yet; set it in the Inspector.`;
  }
  if (inlet?.kind === "action") {
    if (!isDo) return `${inlet.label} takes a rule's action, not ${what(from)}.`;
    return null;
  }
  if (outlet?.kind === "event") {
    if (to.node.kind === "rule" && to.port === "on") return null;
    return `${outlet.label} is an event – it goes into a rule's on, not ${what(to)}.`;
  }
  return null;
};

/** The verbs a node acts with in a rule (`play` … `hide`) */
const isNodeVerb = (verb: string | undefined): verb is (typeof BEHAVIOUR_NODE_ACTIONS)[number] => !!verb && (BEHAVIOUR_NODE_ACTIONS as readonly string[]).includes(verb);

/**
 * A wire at a plugin's pin (#334, Tilman 2026-11-21): its `emits` into a rule's on (the rule listens to one of its events), a
 * rule's do into its `listens` (the rule emits one of the events it listens to) – the event `asked` for, else its first. Anything
 * else at a plugin is its code's: null, so `connect` refuses with the Code tab.
 */
const pluginEvent = (a: GraphNode, aPort: string, b: GraphNode, bPort: string, asked: string | undefined): { event: string } | { refused: string } | null => {
  const plugin = a.kind === "plugin" ? a : b.kind === "plugin" ? b : null;
  if (!plugin) return null;
  const emits = plugin === a && aPort === "emits" && b.kind === "rule" && bPort === "on";
  const listens = plugin === b && bPort === "listens" && a.kind === "rule" && (aPort === "do" || aPort.startsWith("do:"));
  if (!emits && !listens) return { refused: CODE(plugin.label) };
  const names = (emits ? plugin.bus?.emits : plugin.bus?.listens) ?? [];
  if (!names.length) return { refused: `${plugin.label}'s code ${emits ? "emits" : "listens to"} no event a rule can ${emits ? "listen to" : "emit"} – change it in the Code tab.` };
  if (asked !== undefined && !names.includes(asked)) return { refused: `${plugin.label} ${emits ? "emits" : "listens to"} ${names.join(", ")} – not ${asked}.` };
  return { event: asked ?? names[0] };
};

const connect = (draft: Draft, view: View, context: Context, from: [string, string], to: [string, string], verb: BehaviourAction["action"] | undefined, options: ProjectOptions, asked?: string): OpResult => {
  let a = boxOf(view, from[0]);
  let b = boxOf(view, to[0]);
  if (!a || !b) return refuse("That end stands for something elsewhere – open its level to connect there.");
  let [aPort, bPort] = [from[1], to[1]];
  // A drag may start at an inlet: the wire still runs from the outlet to the inlet
  const isIn = (node: GraphNode, port: string) => node.ports.some(p => p.id === port && p.dir === "in");
  if (isIn(a, aPort) && !isIn(b, bPort)) [a, b, aPort, bPort] = [b, a, bPort, aPort];
  if (a.far || b.far) return refuse("That end stands for something elsewhere – open its level to connect there.");
  const fromPlugin = pluginEvent(a, aPort, b, bPort, asked);
  if (fromPlugin && "refused" in fromPlugin) return refuse(fromPlugin.refused);
  const busEvent = fromPlugin?.event ?? null;
  // A spec's port: strictly typed (B8), and only where a document has a field for it (L1)
  const typed = typedRefusal({ node: a, port: aPort }, { node: b, port: bPort }, draft.typeOf);
  if (typed) return refuse(typed);
  const declaredEvent = specPort({ node: a, port: aPort }, "out")?.kind === "event" ? a.ports.find(p => p.id === aPort)!.label : null;

  // An event or a variable into a rule: what it listens to, what its condition reads
  if (b.kind === "rule") {
    const ref = ruleOfBox(view, b.id);
    const at = ref && ruleAt(draft.content, ref);
    if (!ref || !at) return refuse("That rule is not in the content any more.");
    const why = outside(draft.content, context, ref);
    if (why) return refuse(why);
    const name = ruleName(draft.content, ref);
    if (bPort === "on") {
      if (!declaredEvent && !busEvent && (a.kind !== "event" || aPort !== "out")) return refuse(`A rule's on listens to an event – not ${kindWord(a)}.`);
      const event = declaredEvent ?? busEvent ?? sceneEvent(a);
      if (event === null) {
        if (ref.doc !== "entity") return refuse(`${a.label} is ${a.detail}'s own event – a ${ref.doc}'s rule listens to the bus: a scene event or a core event.`);
        const own = a.pick?.kind === "node" ? a.pick : null;
        if (!own || own.target !== ref.target || own.path.join(".") !== ref.path.join(".")) {
          return refuse(`${a.label} is ${a.detail}'s own event – a rule listens to the ${a.label} of its own node.`);
        }
      }
      const on = event ?? a.label;
      if (at.rule.on === on) return refuse(`${name} listens to ${on} already.`);
      draft.setRule(ref, { ...at.rule, on });
      return draft.done(`${name} now listens to ${on}${at.rule.on ? ` (was ${at.rule.on})` : ""}${event && !isBehaviourNodeEvent(event) ? " – by name, from whatever emits it" : ""}.`, options);
    }
    if (bPort === "when") {
      if (a.kind !== "variable" || aPort !== "get") return refuse(`A rule's when reads a variable – not ${kindWord(a)}.`);
      if (at.rule.when && expressionVariables(at.rule.when).includes(a.label)) return refuse(`${name} reads ${a.label} already – ${INSPECTOR}.`);
      const { term, note } = conditionTerm(draft, a.label);
      const when = appendCondition(at.rule.when, term);
      const wrong = whenProblems(when, draft.typeOf);
      if (wrong.length) return refuse(`${when}: ${wrong[0]} – ${INSPECTOR}.`);
      draft.setRule(ref, { ...at.rule, when });
      return draft.done(`${name} now runs only when ${when}${note}.`, options);
    }
    return refuse(`A rule takes an event at on and variables at when – a wire into ${bPort} is not one of them.`);
  }

  // A rule's do into a node, a variable, an event or a scene: an action
  if (a.kind === "rule" && (aPort === "do" || aPort.startsWith("do:"))) {
    const ref = ruleOfBox(view, a.id);
    const at = ref && ruleAt(draft.content, ref);
    if (!ref || !at) return refuse("That rule is not in the content any more.");
    const why = outside(draft.content, context, ref);
    if (why) return refuse(why);
    const name = ruleName(draft.content, ref);
    const index = aPort === "do" ? null : Number(aPort.slice(3));
    const old = index === null ? null : at.rule.do[index];
    if (index !== null && !old) return refuse(`${name} has no action ${index + 1}.`);
    const nodeVerb = (o: BehaviourAction | null) => !!o && "node" in o;
    let action: BehaviourAction;
    if (b.kind === "node" && b.id.startsWith("node:") && b.pick?.kind === "node") {
      const { target, path } = b.pick;
      let nodeName: string;
      if (ref.doc === "entity") {
        const inside = target === ref.target && ref.path.every((p, i) => path[i] === p);
        if (!inside) return refuse(`A rule acts on the nodes under its own node – ${b.label} is not under ${nodeLabel(at.node!)}.`);
        const relative = path.slice(ref.path.length);
        nodeName = relative.length ? nodeAt(at.node!, relative)?.name ?? "" : "";
        if (relative.length && !nodeName) return refuse(`${b.label} has no name – name it in the Inspector, then a rule can reach it.`);
        if (relative.length && pathOfName(at.node!, nodeName)?.join(".") !== relative.join(".")) {
          return refuse(`Another node named ${nodeName} comes first under ${nodeLabel(at.node!)} – a rule reaches the first; rename one of them.`);
        }
      } else {
        const found = findTarget(draft.content, target);
        const root = found?.target.entity;
        if (!found || !root) return refuse(`There is no target ${target}.`);
        if (ref.doc === "scene" && found.scene.id !== ref.scene) return refuse(`A scene's rule acts on its own scene's targets – ${target} is in another scene.`);
        const own = path.length ? nodeAt(root, path)?.name ?? "" : "";
        if (path.length && !own) return refuse(`${b.label} has no name – name it in the Inspector, then a rule can reach it.`);
        if (path.length && pathOfName(root, own)?.join(".") !== path.join(".")) {
          return refuse(`Another node named ${own} comes first on the target ${target} – a rule reaches the first; rename one of them.`);
        }
        nodeName = nodeAddress(target, own);
      }
      if (old && !nodeVerb(old)) return refuse(`Action ${index! + 1} of ${name} is ${old.action} – it does not act on a node. Drag from the rule's + to add one.`);
      // An action inlet (`act:play`) says the verb; the node's `in` keeps the action's own, else the one asked for, else show
      const inletVerb = bPort.startsWith("act:") ? bPort.slice(4) : null;
      if (inletVerb !== null && !isNodeVerb(inletVerb)) return refuse(`${b.label} has no verb ${inletVerb} a rule can write.`);
      if (old && "node" in old) action = inletVerb && inletVerb !== old.action ? { ...newAction(inletVerb), node: nodeName } as BehaviourAction : { ...old, node: nodeName };
      else {
        const v = inletVerb ?? (isNodeVerb(verb) ? verb : "show");
        action = { ...newAction(v), node: nodeName } as BehaviourAction;
      }
    } else if (b.kind === "variable" && bPort === "set") {
      if (old && old.action !== "set") return refuse(`Action ${index! + 1} of ${name} is ${old.action} – it does not set a variable.`);
      if (old?.action === "set") action = { ...old, name: b.label };
      else {
        const scope = draft.types.get(b.label)?.scope ? undefined : scopeOf(draft.content, b.label);
        action = { action: "set", name: b.label, value: literalOf(draft.types.get(b.label)), ...(scope ? { scope } : {}) };
      }
    } else if (b.kind === "event" && bPort === "in") {
      const event = sceneEvent(b);
      if (event === null) return refuse(`${b.label} is ${b.detail}'s own event – a rule emits scene events.`);
      if (CORE.includes(event)) return refuse(`${event} is the engine's own event – a rule listens to it, never emits it.`);
      if (old && old.action !== "emit") return refuse(`Action ${index! + 1} of ${name} is ${old.action} – it does not emit an event.`);
      action = { action: "emit", event };
    } else if (b.kind === "plugin" && busEvent) {
      if (old && old.action !== "emit") return refuse(`Action ${index! + 1} of ${name} is ${old.action} – it does not emit an event.`);
      action = { action: "emit", event: busEvent };
    } else if (b.kind === "scene" && b.pick?.kind === "scene" && (bPort === "arrives" || bPort === "in")) {
      if (old && old.action !== "goto") return refuse(`Action ${index! + 1} of ${name} is ${old.action} – it does not go to a scene.`);
      action = { action: "goto", address: sceneAddress(b.pick.scene) };
    } else if (b.kind === "scene" && bPort === "nodes") {
      return refuse(`Pick the node of ${b.label} in the Inspector – the experience level shows no nodes.`);
    } else {
      return refuse(`A rule's do reaches a node, a variable, an event or a scene – not ${kindWord(b)}.`);
    }
    const problem = actionProblem(draft, ref, at, action);
    if (problem) return refuse(problem);
    if (old && JSON.stringify(old) === JSON.stringify(action)) return refuse(`${name} does that already.`);
    draft.setRule(ref, withAction(at.rule, index ?? at.rule.do.length, action));
    return draft.done(old ? `${name}: ${actionWords(old)} → ${actionWords(action)}.` : `${name} now also does: ${actionWords(action)}${verb || bPort.startsWith("act:") ? "" : action.action === "show" ? " (pick another verb in the Inspector)" : ""}.`, options);
  }
  return refuse(`${a.label} (${kindWord(a).replace(/^an? /, "")}) → ${b.label} (${kindWord(b).replace(/^an? /, "")}): the rules have no such connection. A rule's on takes an event, its when a variable, its do reaches a node, a variable, an event or a scene.`);
};

/** The scope a variable is set in elsewhere (a new set reaches the same readers); undefined: none – the declared one, else scene */
const scopeOf = (content: LogicContent, name: string) => {
  const all = (node: EntityData): EntityData[] => [node, ...(node.children ?? []).flatMap(all)];
  const lists: BehaviourRule[][] = [[...(content.rules ?? [])], ...content.scenes.flatMap(scene => [
    [...(scene.rules ?? [])],
    ...scene.targets.flatMap(t => (t.entity ? all(t.entity) : []).flatMap(node => (node.components ?? []).filter(c => c.type === BEHAVIOUR_TYPE).map(c => behaviourRules(c)))),
  ])];
  for (const rules of lists) for (const rule of rules) for (const a of rule.do) if (a.action === "set" && a.name === name && a.scope) return a.scope;
  return undefined;
};

/** A condition without a variable's terms – or the refusal that sends the author to the Inspector */
const withoutVariable = (draft: Draft, ref: RuleRef, at: RuleAt, variable: string): string | null | { refused: string } => {
  const removed = removeConditionTerms(at.rule.when, variable);
  if ("refused" in removed) return { refused: `${ruleName(draft.content, ref)} reads ${variable} inside its condition (${at.rule.when}) – ${INSPECTOR}.` };
  return removed.when;
};

const disconnect = (draft: Draft, view: View, context: Context, id: string, options: ProjectOptions): OpResult => {
  const stub = view.stubs.find(s => s.edge.id === id);
  const edge: GraphEdge | undefined = view.edges.find(e => e.id === id) ?? stub?.edge;
  if (!edge) return refuse("That wire is not in the view any more.");
  const home = edge.home;
  if (home) {
    const ref = refOf(home);
    const why = outside(draft.content, context, ref);
    if (why) return refuse(why);
    const at = ruleAt(draft.content, ref);
    if (!at) return refuse("That rule is not in the content any more.");
    const name = ruleName(draft.content, ref);
    if (home.part === "on") {
      // Left empty, the rule listens to nothing until an event is wired to its on – a problem the panel lists (Tilman, 2026-11-20)
      draft.setRule(ref, { ...at.rule, on: "" });
      return draft.done(`${name} no longer listens to ${at.rule.on} – it listens to nothing until you wire an event to its on.`, options);
    }
    if (home.part === "when") {
      const variable = edge.from[0].replace(/^var:/, "");
      if (!at.rule.when || !expressionVariables(at.rule.when).includes(variable)) return refuse(`${name} does not read ${variable}.`);
      const when = withoutVariable(draft, ref, at, variable);
      if (when !== null && typeof when === "object") return refuse(when.refused);
      draft.setRule(ref, { ...at.rule, when });
      return draft.done(`${name} no longer reads ${variable} – it runs ${when ? `when ${when}` : "always"}.`, options);
    }
    const action = at.rule.do[home.index ?? 0];
    if (!action) return refuse(`${name} has no such action.`);
    draft.setRule(ref, withAction(at.rule, home.index ?? 0, undefined));
    return draft.done(`${name} no longer does: ${actionWords(action)}.`, options);
  }

  // A wire that joins by name: the name goes from the listening side, if that side is a rule here
  const ends = [edge.from[0], edge.to[0]];
  const plugin = ends.find(e => e.startsWith("plugin:")) ?? (/^far:plugin:/.test(ends.find(e => e.startsWith("far:")) ?? "") ? ends.find(e => e.startsWith("far:"))!.replace("far:", "") : null);
  if (plugin) return refuse(CODE(plugin.replace("plugin:", "")));
  if (edge.signal === "scene") return refuse("That goto is written in a scene's rules – open the scene (or the target) to remove it there.");
  const where = stub ? stub.label.replace(/^\S+ [←→] /, "") : "";
  if (stub?.side === "out") return refuse(`${edge.label} is ${where} – the listening side is there; open it to change it.`);
  const here = (e: GraphEdge) => !!e.home && !outside(draft.content, context, refOf(e.home));
  if (edge.signal === "value") {
    const box = `var:${edge.label}`;
    const reads = view.edges.filter(e => e.from[0] === box && e.home?.part === "when" && here(e));
    if (!reads.length) return refuse(`Nothing at this level reads ${edge.label}.`);
    const names: string[] = [];
    const done = new Set<string>();
    for (const read of reads) {
      const ref = refOf(read.home!);
      const key = JSON.stringify(ref);
      if (done.has(key)) continue;
      done.add(key);
      const at = ruleAt(draft.content, ref);
      if (!at) continue;
      const when = withoutVariable(draft, ref, at, edge.label);
      if (when !== null && typeof when === "object") return refuse(when.refused);
      names.push(ruleName(draft.content, ref));
      draft.setRule(ref, { ...at.rule, when });
    }
    return draft.done(`${names.join(", ")} no longer read${names.length === 1 ? "s" : ""} ${edge.label}. The wire joined by name: ${edge.label} is still ${where || "set where it was"} – only the read here went.`, options);
  }
  if (edge.signal === "event") {
    const box = `event:${edge.label}`;
    const listens = view.edges.filter(e => e.from[0] === box && e.home?.part === "on" && here(e));
    if (!listens.length) return refuse(`Nothing at this level listens to ${edge.label}.`);
    const names: string[] = [];
    for (const listen of listens) {
      const ref = refOf(listen.home!);
      const at = ruleAt(draft.content, ref);
      if (!at) continue;
      names.push(ruleName(draft.content, ref));
      draft.setRule(ref, { ...at.rule, on: "" });
    }
    return draft.done(`${names.join(", ")} no longer listen${names.length === 1 ? "s" : ""} to ${edge.label} – ${names.length === 1 ? "it listens to" : "they listen to"} nothing until you wire an event to ${names.length === 1 ? "its" : "their"} on. The wire joined by name: ${edge.label} is still ${where || "emitted where it was"}.`, options);
  }
  return refuse("That wire is drawn from a name, not written in a rule – nothing to remove here.");
};

/** The canvas a level keeps its exposed ports in: its scene's (the scene and entity levels) or the experience's */
const canvasHome = (context: Exclude<Context, { level: "group" }>): { doc: "scene"; scene: string } | { doc: "experience" } =>
  context.level === "experience" ? { doc: "experience" } : { doc: "scene", scene: context.scene };

/** A canvas without its empty fields; undefined when nothing is left */
const leanCanvas = (canvas: LogicCanvas): LogicCanvas | undefined => {
  const next: LogicCanvas = {};
  if (canvas.groups?.length) next.groups = canvas.groups;
  if (canvas.notes?.length) next.notes = canvas.notes;
  if (canvas.folds?.length) next.folds = canvas.folds;
  if (canvas.exposed && Object.keys(canvas.exposed).length) next.exposed = canvas.exposed;
  if (canvas.positions && Object.keys(canvas.positions).length) next.positions = canvas.positions;
  return Object.keys(next).length ? next : undefined;
};

/** A note's key in a `place` edit's positions */
const NOTE_KEY = "note:";
/** A second note beside the same box stands this far below the first */
const NOTE_STACK = 5 * GRID;
/** A group's name for the status line */
const groupName = (group: LogicCanvasGroup) => group.label || group.id;

/**
 * The canvas ops (`LogicCanvas` of the level's scene, or the experience's): `expose` / `hideUnused` – which ports the
 * level's boxes show beyond their wired ones (L-T6); `place` / `tidy` – boxes pinned over the layout on the 1rem grid, or
 * the pins dropped (L-T7); `fold` – a box to its head; `group`, `renameGroup` (empty: the group goes), `groupColour`,
 * `foldGroup` – the deck's groups (a box in one group at most, no nesting)
 */
const canvasEdit = (draft: Draft, context: Exclude<Context, { level: "group" }>, edit: CanvasEdit, options: ProjectOptions): OpResult => {
  const home = canvasHome(context);
  if (home.doc === "scene" && !draft.content.scenes.some(s => s.id === home.scene)) return refuse(`There is no scene ${home.scene}.`);
  const canvas: LogicCanvas = (home.doc === "experience" ? draft.content.canvas : draft.content.scenes.find(s => s.id === home.scene)?.canvas) ?? {};
  const view = project(draft.content, context, options);
  const boxOf = (id: string) => view.nodes.find(n => n.id === id);
  const groups = canvas.groups ?? [];
  const groupOf = (id: string) => groups.find(g => g.id === id);
  /** The canvas's notes; the level's own are those of its view (a scene's canvas holds its targets' views too) */
  const notesNow = canvas.notes ?? [];
  const here = noteView(context);
  const noteOf = (id: string) => notesNow.find(n => n.id === id && n.view === here);
  const save = (next: LogicCanvas, words: string) => {
    draft.setCanvas(home, leanCanvas(next));
    return draft.done(words, options);
  };
  switch (edit.edit) {
    case "expose":
    case "hideUnused": {
      const exposed: Record<string, string[]> = { ...(canvas.exposed ?? {}) };
      let words: string;
      if (edit.edit === "expose") {
        const box = boxOf(edit.box);
        if (!box) return refuse("That box is not at this level.");
        const ports = edit.ports.map(id => box.ports.find(p => p.id === id && p.kind));
        const missing = edit.ports.find((_, i) => !ports[i]);
        if (missing !== undefined) return refuse(`${box.label} has no port ${missing}.`);
        const fresh = edit.ports.filter(id => !(exposed[box.id] ?? []).includes(id));
        if (!fresh.length) return refuse(`${box.label} shows ${edit.ports.length === 1 ? "that port" : "those ports"} already.`);
        exposed[box.id] = [...(exposed[box.id] ?? []), ...fresh];
        words = `${box.label} shows ${ports.map(p => p!.label).join(", ")} – unused, it hides again with Hide unused ports.`;
      } else {
        const boxes = edit.box ? view.nodes.filter(n => n.id === edit.box) : view.nodes;
        if (edit.box && !boxes.length) return refuse("That box is not at this level.");
        const cleared = boxes.filter(n => exposed[n.id]?.length);
        if (!cleared.length) return refuse(edit.box ? `${boxes[0].label} shows only its wired ports already.` : "Every box here shows only its wired ports already.");
        cleared.forEach(n => delete exposed[n.id]);
        words = edit.box ? `${boxes[0].label} shows only its wired ports again.` : `${cleared.length === 1 ? "One box shows" : `${cleared.length} boxes show`} only their wired ports again.`;
      }
      return save({ ...canvas, exposed }, words);
    }
    case "place": {
      const all = Object.keys(edit.positions);
      if (!all.length) return refuse("Nothing was moved.");
      const noteIds = all.filter(id => id.startsWith(NOTE_KEY));
      const ids = all.filter(id => !id.startsWith(NOTE_KEY));
      const away = ids.find(id => !boxOf(id));
      if (away) return refuse("That box is not at this level – only the level's own boxes are placed.");
      if (noteIds.some(id => !noteOf(id.slice(NOTE_KEY.length)))) return refuse("That note is not here any more.");
      const wrong = all.find(id => !Number.isFinite(edit.positions[id].x) || !Number.isFinite(edit.positions[id].y));
      if (wrong) return refuse(`${boxOf(wrong)?.label ?? "The note"}: no place to pin it to.`);
      const positions = { ...(canvas.positions ?? {}) };
      for (const id of ids) positions[id] = { x: snap(edit.positions[id].x), y: snap(edit.positions[id].y) };
      const moved = new Map(noteIds.map(id => [id.slice(NOTE_KEY.length), edit.positions[id]]));
      const notes = notesNow.map(n => (moved.has(n.id) && n.view === here ? { ...n, x: snap(moved.get(n.id)!.x), y: snap(moved.get(n.id)!.y) } : n));
      const names = ids.map(id => boxOf(id)!.label);
      const words = !ids.length ? `${noteIds.length === 1 ? "The note" : `${noteIds.length} notes`} moved.`
        : `${names.length === 1 ? names[0] : `${names.length} boxes`} pinned where you let go${noteIds.length ? ` (and ${noteIds.length === 1 ? "a note" : `${noteIds.length} notes`} moved)` : ""} – Tidy lays the level out again.`;
      return save({ ...canvas, positions, ...(noteIds.length ? { notes } : {}) }, words);
    }
    case "tidy": {
      const positions = { ...(canvas.positions ?? {}) };
      const pinned = view.nodes.filter(n => positions[n.id]);
      if (!pinned.length) return refuse("Nothing is pinned at this level – the layout places every box already.");
      pinned.forEach(n => delete positions[n.id]);
      return save({ ...canvas, positions }, `${pinned.length === 1 ? "One box" : `${pinned.length} boxes`} back in the layout.`);
    }
    case "fold": {
      const box = boxOf(edit.box);
      if (!box) return refuse("That box is not at this level.");
      const folds = new Set(canvas.folds ?? []);
      if (folds.has(box.id) === edit.folded) return refuse(`${box.label} is ${edit.folded ? "folded" : "open"} already.`);
      if (edit.folded) folds.add(box.id);
      else folds.delete(box.id);
      return save({ ...canvas, folds: [...folds] }, `${box.label} ${edit.folded ? "folded to its head" : "open again"}.`);
    }
    case "group": {
      const ids = [...new Set(edit.boxes)];
      if (!ids.length) return refuse("Select the boxes to group first (Shift adds to the selection).");
      const away = ids.find(id => !boxOf(id));
      if (away) return refuse("Only the level's own boxes group – not a stub or a folded group.");
      // A box leaves its old group; a group left without boxes goes
      const left = groups.map(g => ({ ...g, members: g.members.filter(m => !ids.includes(m)) })).filter(g => g.members.length);
      let n = 1;
      while (left.some(g => g.id === `group-${n}`)) n++;
      const usedColours = new Set(left.map(g => g.colour));
      const colour = GROUP_COLOURS.find(c => !usedColours.has(c)) ?? GROUP_COLOURS[left.length % GROUP_COLOURS.length];
      const group: LogicCanvasGroup = { id: `group-${n}`, label: `group ${n}`, members: ids, colour };
      return save({ ...canvas, groups: [...left, group] }, `${ids.length === 1 ? boxOf(ids[0])!.label : `${ids.length} boxes`} grouped as ${group.label} – click its name to rename it.`);
    }
    case "renameGroup": {
      const group = groupOf(edit.group);
      if (!group) return refuse("That group is not there any more.");
      const label = edit.label.trim();
      if (!label) return save({ ...canvas, groups: groups.filter(g => g !== group) }, `${groupName(group)} removed – its boxes stay where they are.`);
      if (label === group.label) return refuse(`${label} is its name already.`);
      return save({ ...canvas, groups: groups.map(g => (g === group ? { ...g, label } : g)) }, `${groupName(group)} renamed to ${label}.`);
    }
    case "ungroup": {
      const gone = groups.filter(g => edit.groups.includes(g.id));
      if (!gone.length) return refuse(edit.groups.length ? "That group is not there any more." : "Select boxes of a group first – Ctrl+Shift+G ungroups their groups.");
      return save({ ...canvas, groups: groups.filter(g => !gone.includes(g)) },
        `${gone.length === 1 ? groupName(gone[0]) : `${gone.length} groups`} ungrouped – the boxes stay where they are.`);
    }
    case "note": {
      const box = edit.box === undefined ? undefined : boxOf(edit.box);
      if (edit.box !== undefined && !box) return refuse("That box is not at this level – a note stands beside one of the level's boxes.");
      if (!box && !(edit.at && Number.isFinite(edit.at.x) && Number.isFinite(edit.at.y))) return refuse("Where should the note go? Right-click the canvas or a box.");
      let n = 1;
      while (notesNow.some(note => note.id === `note-${n}`)) n++;
      // Beside its box, to the right; a second note on the same box below the first
      const beside = box ? notesNow.filter(note => note.box === box.id && note.view === here).length : 0;
      const author = edit.author?.trim();
      const note: LogicCanvasNote = {
        id: `note-${n}`, text: "",
        ...(box ? { x: NOTE_BESIDE.x, y: NOTE_BESIDE.y + beside * NOTE_STACK } : { x: snap(edit.at!.x), y: snap(edit.at!.y) }),
        ...(author ? { author } : {}), ...(box ? { box: box.id } : {}), ...(here ? { view: here } : {}),
      };
      return save({ ...canvas, notes: [...notesNow, note] }, box ? `A note beside ${box.label} – type its text.` : "A note – type its text.");
    }
    case "noteText": {
      const note = noteOf(edit.note);
      if (!note) return refuse("That note is not here any more.");
      const text = edit.text.trim();
      if (!text) return save({ ...canvas, notes: notesNow.filter(n => n !== note) }, note.text ? "Note removed." : "The empty note is gone.");
      if (text === note.text) return refuse("The note says that already.");
      return save({ ...canvas, notes: notesNow.map(n => (n === note ? { ...n, text } : n)) }, note.text ? "Note changed." : "Note written.");
    }
    case "groupColour": {
      const group = groupOf(edit.group);
      if (!group) return refuse("That group is not there any more.");
      const at = GROUP_COLOURS.indexOf((group.colour ?? GROUP_COLOURS[groups.indexOf(group) % GROUP_COLOURS.length]) as (typeof GROUP_COLOURS)[number]);
      const colour = GROUP_COLOURS[(at + 1) % GROUP_COLOURS.length];
      return save({ ...canvas, groups: groups.map(g => (g === group ? { ...g, colour } : g)) }, `${groupName(group)} has its next colour.`);
    }
    case "foldGroup": {
      const group = groupOf(edit.group);
      if (!group) return refuse("That group is not there any more.");
      if (!!group.folded === edit.folded) return refuse(`${groupName(group)} is ${edit.folded ? "folded" : "open"} already.`);
      const { folded: _was, ...rest } = group;
      return save({ ...canvas, groups: groups.map(g => (g === group ? (edit.folded ? { ...rest, folded: true } : rest) : g)) },
        `${groupName(group)} ${edit.folded ? "folded to one pill – its wires end on it" : "open again"}.`);
    }
    default:
      return refuse("Canvas edits for free nodes are not here yet.");
  }
};

/**
 * The ports that take a wire dragged from `from` (`dir`: from an outlet or back from an inlet), by box id – each one the
 * strict `connect` would apply. A plugin has two pins (#334): its `emits` reaches a rule's on, a rule's do reaches its
 * `listens`; what it reads, sets and drives are display rows, no pins.
 */
export const takesWire = (content: LogicContent, context: Context, from: [string, string], dir: "in" | "out", options: ProjectOptions = {}): Map<string, Set<string>> => {
  const takes = new Map<string, Set<string>>();
  if (context.level === "group") return takes;
  const view = project(content, context, options);
  const start = view.nodes.find(n => n.id === from[0]);
  if (!start) return takes;
  const other = dir === "in" ? "out" : "in";
  for (const node of view.nodes) {
    if (node.id === from[0] || node.far) continue;
    for (const port of node.ports) {
      if (port.dir !== other) continue;
      const op: Op = dir === "out" ? { op: "connect", from, to: [node.id, port.id] } : { op: "connect", from: [node.id, port.id], to: from };
      if (applyOp(op, content, context, options).ok) takes.set(node.id, (takes.get(node.id) ?? new Set()).add(port.id));
    }
  }
  return takes;
};

/**
 * Pin the box an op created where it was asked for (`op.pin`, #334): a new rule's box – the rule box whose home is new –
 * else the first box new to the level (a variable, an event a rule now names). Merged into the op's result, so the canvas
 * saves with the rules.
 */
const pinNew = (result: Extract<OpResult, { ok: true }>, op: Op, before: LogicContent, context: Exclude<Context, { level: "group" }>, options: ProjectOptions): OpResult => {
  const pin = op.pin!;
  if (!Number.isFinite(pin.x) || !Number.isFinite(pin.y)) return result;
  const was = project(before, context, options);
  const now = project(result.content, context, options);
  const homes = new Set(was.nodes.flatMap(n => (n.kind === "rule" && n.home ? [JSON.stringify(refOf(n.home))] : [])));
  const ids = new Set(was.nodes.map(n => n.id));
  const box = op.op === "addRule"
    ? now.nodes.find(n => n.kind === "rule" && n.home && !homes.has(JSON.stringify(refOf(n.home))))
    : now.nodes.find(n => !ids.has(n.id) && n.kind !== "rule" && !n.far);
  if (!box) return result;
  const home = canvasHome(context);
  const canvas: LogicCanvas = (home.doc === "experience" ? result.content.canvas : result.content.scenes.find(s => s.id === home.scene)?.canvas) ?? {};
  const draft = new Draft(result.content);
  draft.setCanvas(home, leanCanvas({ ...canvas, positions: { ...(canvas.positions ?? {}), [box.id]: { x: snap(pin.x), y: snap(pin.y) } } }));
  const key = (c: CanvasChange) => (c.doc === "experience" ? "experience" : c.scene);
  const fresh = [...draft.canvas.values()];
  return { ...result, content: draft.content, canvas: [...result.canvas.filter(c => !fresh.some(f => key(f) === key(c))), ...fresh] };
};

/** Two rules in one list (the same component, the same scene, the experience) */
const sameList = (x: RuleRef, y: RuleRef) => JSON.stringify({ ...x, rule: 0 }) === JSON.stringify({ ...y, rule: 0 });

/**
 * The notes beside boxes after an op on the documents (schema 5.3): a box id that moved with it follows (a removed rule's
 * later siblings – rule boxes are numbered), and a note whose box went stays free where it last stood. Only the level's own
 * view; merged into the op's result, so the canvas saves with the rules.
 */
const keepNotes = (result: Extract<OpResult, { ok: true }>, op: Op, before: LogicContent, context: Exclude<Context, { level: "group" }>, options: ProjectOptions): OpResult => {
  const home = canvasHome(context);
  const canvasIn = (c: LogicContent) => (home.doc === "experience" ? c.canvas : c.scenes.find(s => s.id === home.scene)?.canvas);
  const canvas = canvasIn(result.content);
  const at = noteView(context);
  if (!canvas?.notes?.some(n => n.box && n.view === at)) return result;
  const was = project(before, context, options);
  const now = project(result.content, context, options);
  const nowIds = new Set(now.nodes.map(n => n.id));
  const ruleNow = new Map(now.nodes.flatMap(n => (n.kind === "rule" && n.home ? [[JSON.stringify(refOf(n.home)), n.id] as const] : [])));
  const removed = op.op === "removeRule" ? refOf(op.home) : null;
  const follow = (ref: RuleRef): RuleRef | null =>
    !removed || !sameList(ref, removed) || ref.rule < removed.rule ? ref : ref.rule === removed.rule ? null : { ...ref, rule: ref.rule - 1 };
  let flow: ReturnType<typeof flowOf> | null = null;
  let changed = false;
  const notes = canvas.notes.map(note => {
    const old = note.box && note.view === at ? was.nodes.find(n => n.id === note.box) : undefined;
    if (!old) return note;
    let id: string | undefined;
    if (old.kind === "rule" && old.home) {
      const ref = follow(refOf(old.home));
      id = ref ? ruleNow.get(JSON.stringify(ref)) : undefined;
    } else id = nowIds.has(old.id) ? old.id : undefined;
    if (id === note.box) return note;
    changed = true;
    if (id) return { ...note, box: id };
    flow ??= flowOf(was, canvasIn(before));
    const box = flow.boxes.find(b => b.id === note.box || b.pill?.members.includes(note.box!));
    const { box: _gone, ...free } = note;
    return box ? { ...free, x: snap(box.x + note.x), y: snap(box.y + note.y) } : free;
  });
  if (!changed) return result;
  const draft = new Draft(result.content);
  draft.setCanvas(home, leanCanvas({ ...canvas, notes }));
  const key = (c: CanvasChange) => (c.doc === "experience" ? "experience" : c.scene);
  const fresh = [...draft.canvas.values()];
  return { ...result, content: draft.content, canvas: [...result.canvas.filter(c => !fresh.some(f => key(f) === key(c))), ...fresh] };
};

/** Apply an op to the documents, at the level `context` (see the module's comment); `op.pin` pins the box it creates */
export const apply = (op: Op, content: LogicContent, context: Context, options: ProjectOptions = {}): OpResult => {
  const result = applyOp(op, content, context, options);
  if (!result.ok || op.op === "canvas" || context.level === "group") return result;
  const pinned = op.pin ? pinNew(result, op, content, context, options) : result;
  return pinned.ok ? keepNotes(pinned, op, content, context, options) : pinned;
};

const applyOp = (op: Op, content: LogicContent, context: Context, options: ProjectOptions = {}): OpResult => {
  if (context.level === "group") return refuse(READ_ONLY);
  const draft = new Draft(content);
  switch (op.op) {
    case "canvas":
      return canvasEdit(draft, context, op.edit, options);
    case "addRule": {
      const why = outside(content, context, op.at);
      return why ? refuse(why) : addRule(draft, op.at, op, options);
    }
    case "connect":
    case "disconnect": {
      const view = project(content, context, options);
      return op.op === "connect" ? connect(draft, view, context, op.from, op.to, op.verb, options, op.event) : disconnect(draft, view, context, op.edge, options);
    }
    default: {
      const ref = refOf(op.home);
      const why = outside(content, context, ref);
      if (why) return refuse(why);
      const at = ruleAt(content, ref);
      if (!at) return refuse("That rule is not in the content any more.");
      const name = ruleName(content, ref);
      switch (op.op) {
        case "setEvent": {
          const on = op.on.trim();
          if (!isName(on)) return refuse(`"${on}" is no event – an event's name (camelCase).`);
          if (ref.doc !== "entity" && isBehaviourNodeEvent(on)) return refuse(`${on} is a node's own event – a ${ref.doc}'s rule listens to the bus: a scene event or a core event.`);
          if (at.rule.on === on) return refuse(`${name} listens to ${on} already.`);
          draft.setRule(ref, { ...at.rule, on });
          return draft.done(`${name} now listens to ${on}${at.rule.on ? ` (was ${at.rule.on})` : ""}.`, options);
        }
        case "removeRule":
          draft.setRule(ref, undefined);
          return draft.done(`Removed ${name} (on ${at.rule.on || "nothing"}${at.rule.do.length ? `, ${at.rule.do.map(actionWords).join(", ")}` : ""}).`, options);
        case "setCondition": {
          const when = op.when?.trim() || null;
          const wrong = when ? whenProblems(when, draft.typeOf) : [];
          if (wrong.length) return refuse(`${when}: ${wrong[0]}.`);
          draft.setRule(ref, { ...at.rule, when });
          return draft.done(`${name} now runs ${when ? `only when ${conditionWords(when)}` : "always"}.`, options);
        }
        case "addAction": {
          const problem = actionProblem(draft, ref, at, op.action);
          if (problem) return refuse(problem);
          draft.setRule(ref, withAction(at.rule, at.rule.do.length, op.action));
          return draft.done(`${name} now also does: ${actionWords(op.action)}.`, options);
        }
        case "removeAction": {
          const action = at.rule.do[op.home.index ?? -1];
          if (!action) return refuse(`${name} has no action ${(op.home.index ?? -1) + 1}.`);
          draft.setRule(ref, withAction(at.rule, op.home.index!, undefined));
          return draft.done(`${name} no longer does: ${actionWords(action)}.`, options);
        }
      }
    }
  }
};
