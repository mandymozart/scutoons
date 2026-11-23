import {
  BEHAVIOUR_CORE_EVENTS, BEHAVIOUR_NODE_ACTIONS, BEHAVIOUR_NODE_EVENTS, type BehaviourAction, type BehaviourRule, type EntityData,
} from "@engine/types";
import { CAMEL_CASE_NAME } from "@shared/guards/primitives";
import { expressionVariables } from "@shared/text/expressions";
import { nodeAddress, sceneAddress } from "@shared/types/address";
import { appendCondition, experienceEvents, experienceVariables, literalOf, newAction, rulesOf, subtreeNames } from "#lib/behaviour-edit.js";
import { nodeLabel } from "#lib/tree.js";
import { nodeAt } from "#lib/values.js";
import { firstEvent, refOf, ruleName } from "./apply";
import { logicTypes, type LogicContent } from "./project";
import { coreRegistry, type NodeRegistry } from "./registry";
import type { DeckSignal } from "./signals";
import { typeWords } from "./spec-ports";
import type { Context, GraphNode, Home, NodeFamily, NodeOwner, Op, RulePlace, RuleRef } from "./types";

/**
 * **Which node types fit here** (#334 L-T4, L-T5b): the one list the Logic canvas's add menu (a right click on the empty
 * canvas, the panel's +) and the Library's *Nodes* read, so both offer the same things. `palette` is every type of the
 * registry with what adding one does (or why it is not added by hand – a node comes from the tree, a plugin from the Code
 * tab); `addChoices` turns **every** addable type into concrete ops (`apply.ts`), grouped by family under a header
 * (Events, Conditions, Actions, then the types not added here as grey lines):
 * - an event: a new rule listening to it, on the selected node, else the scene or the experience;
 * - a condition or an action: added to the selected rule – or, with no rule selected, a **new rule** on the same place that
 *   listens to its first event (`firstEvent`: a node's rule `found`, a scene's or the experience's `sceneChanged`) and carries it.
 * A typed `emit fooBar` / `set fooBar` / `when fooBar` / `rule on fooBar` that names nothing yet is offered as a new name.
 * Values are typed (B7): a set or a compared variable takes its declared default, else a placeholder of its type. An entry
 * names the thing chosen (`keyRattled`, `goto 3–4`, `when hasKey`); the words people type for it (`rule`, `on`) still find it.
 *
 * `wireChoices` (#334): a wire dragged from a port and let go on the empty canvas – what can be created **and wired** to
 * that port, by the same rules as the strict connect: a rule's `+` → an action, its `on` → an event, its `when` → a
 * condition; an event's outlet → a rule listening to it; a variable → a rule reading or setting it; a node's verb inlet → a rule
 * acting on it. A value inlet no rule action writes yet stays a grey line with the reason.
 */

export interface PaletteEntry {
  kind: string;
  family: NodeFamily;
  owner: NodeOwner;
  label: string;
  /** What adding one does, or why it is not added by hand */
  detail: string;
  /** It can be added on the canvas (by its menu or a drop) */
  addable: boolean;
}

const ADDS: Record<string, string> = {
  rule: "A rule on the current node, the scene or the experience: an event, a condition, actions",
  event: "A rule that listens to an event",
  variable: "A term of the selected rule's condition on a variable – or a new rule with it",
  play: "The selected rule plays a clip – or a new rule does", pause: "The selected rule pauses a clip – or a new rule does",
  stop: "The selected rule stops a clip – or a new rule does",
  show: "The selected rule shows a node – or a new rule does", hide: "The selected rule hides a node – or a new rule does",
  set: "The selected rule sets a variable – or a new rule does", emit: "The selected rule emits an event – or a new rule does",
  goto: "The selected rule goes to a scene – or a new rule does",
};
const NOT_BY_HAND: Record<string, string> = {
  node: "Nodes come from the tree – add one there",
  plugin: "Plugins come from the Code tab",
  scene: "Scenes come from the Scenes panel",
  group: "Groups come from the Scenes panel",
  far: "Drawn for something at another level",
};
const LABELS: Record<string, string> = { rule: "Rule", event: "Event", variable: "Variable", node: "Node", plugin: "Plugin", scene: "Scene", group: "Group", far: "Elsewhere" };

/** Every node type of the registry, the addable ones first, with what adding one does */
export const palette = (registry: NodeRegistry = coreRegistry): PaletteEntry[] => {
  const entries = registry.all().map((t): PaletteEntry => {
    const core = t.owner === "core";
    const addable = core && t.kind in ADDS;
    const detail = addable ? ADDS[t.kind] : core ? NOT_BY_HAND[t.kind] ?? "Drawn by the view" : `Registered by ${t.owner} – custom nodes run later`;
    return { kind: t.kind, family: t.family, owner: t.owner, label: LABELS[t.kind] ?? t.kind, detail, addable };
  });
  return [...entries.filter(e => e.addable), ...entries.filter(e => !e.addable)];
};

/** What the menu offers: one op, or a line that says why nothing is offered */
export interface AddChoice {
  key: string;
  /** The node type it adds */
  kind: string;
  label: string;
  detail: string;
  op: Op | null;
  /** Why it cannot be chosen (shown, not chosen) */
  disabled?: string;
  /** The header it is listed under (Events, Conditions, Actions, …) */
  group: string;
  /** More words the filter finds it by (`rule on` for an event) */
  terms?: string;
  /** Its type as the canvas legend shows it: the signal's pin and colour, and a word (event, vote, scene, rule …) */
  signal?: DeckSignal;
  type?: string;
}

export type { RuleRef };

export interface AddContext {
  content: LogicContent;
  context: Context;
  /** Where a new rule goes (the selection's node, else the entity level's own, else the scene or the experience); null: none */
  place: RulePlace | null;
  /** The rule selected on the canvas; null: none */
  rule: RuleRef | null;
  /** Variables and events seen on the stage (a plugin's too), besides the rules' own */
  liveVariables?: readonly string[];
  liveEvents?: readonly string[];
}

const CORE = BEHAVIOUR_CORE_EVENTS as readonly string[];
const NODE_VERBS = BEHAVIOUR_NODE_ACTIONS as readonly string[];
const ACTION_KINDS = ["play", "pause", "stop", "show", "hide", "set", "emit", "goto"];
const trees = (content: LogicContent): EntityData[] => content.scenes.flatMap(s => s.targets.flatMap(t => (t.entity ? [t.entity] : [])));
const logicRules = (content: LogicContent): BehaviourRule[] => [...(content.rules ?? []), ...content.scenes.flatMap(s => s.rules ?? [])];
const nodeOf = (content: LogicContent, target: string, path: number[]) => {
  for (const scene of content.scenes) {
    const root = scene.targets.find(t => t.id === target)?.entity;
    if (root) return nodeAt(root, path);
  }
  return undefined;
};
const home = (rule: RuleRef, part: "when" | "do"): Home => ({ ...rule, part });

/** The rule a ref names (undefined: not there) */
const ruleOf = (content: LogicContent, ref: RuleRef): BehaviourRule | undefined => {
  if (ref.doc === "experience") return content.rules?.[ref.rule];
  if (ref.doc === "scene") return content.scenes.find(s => s.id === ref.scene)?.rules?.[ref.rule];
  const node = nodeOf(content, ref.target, ref.path);
  return node ? rulesOf(node, ref.component)[ref.rule] : undefined;
};

/** A rule's place in words: a node's label, `the scene`, `the experience` */
const whereWords = (content: LogicContent, place: RulePlace): string => {
  if (place.doc === "experience") return "the experience";
  if (place.doc === "scene") return "the scene";
  const node = nodeOf(content, place.target, place.path);
  // A target's unnamed entity is called by its target
  return node ? node.name?.trim() || (place.path.length ? nodeLabel(node) : place.target) : place.target;
};

/** The family header of a type */
const FAMILY_HEADS: Record<string, string> = { rule: "Events", event: "Events", variable: "Conditions" };
const familyOf = (kind: string) => FAMILY_HEADS[kind] ?? (ACTION_KINDS.includes(kind) ? "Actions" : "Not added here");
/** The header of a family here: what choosing one of it makes (a new rule, or an edit of the selected one) */
const headOf = (kind: string, a: AddContext): string => {
  const family = familyOf(kind);
  if (family === "Not added here") return family;
  if (family === "Events") return a.place?.doc === "entity" ? `Events – a new rule on ${whereWords(a.content, a.place)} listens to it` : "Events – a new rule listens to it";
  if (a.rule) return `${family} – ${ruleName(a.content, a.rule)}`;
  return a.place ? `${family} – a new rule on ${firstEvent(a.place)}` : family;
};

/** Why a type does not fit here (null: it fits) */
export const fitsHere = (kind: string, a: Pick<AddContext, "context" | "place" | "rule">, registry: NodeRegistry = coreRegistry): string | null => {
  const entry = palette(registry).find(e => e.kind === kind);
  if (!entry) return `No node type ${kind}`;
  if (!entry.addable) return entry.detail;
  if (a.context.level === "group") return "This level is read-only – open a scene or a target";
  if (!a.rule && !a.place) return "Select a node first (here or in the tree)";
  const doc = a.rule?.doc ?? a.place?.doc;
  if (doc === "experience" && NODE_VERBS.includes(kind)) return "Pick the node of an experience rule in its Inspector";
  return null;
};

/** The variables and events the experience knows, and the variables' types */
const known = (a: AddContext) => {
  const extra = logicRules(a.content);
  const variables = [...new Set([...experienceVariables(trees(a.content), a.liveVariables, extra), ...Object.keys(a.content.variables ?? {})])].sort();
  const events = experienceEvents(trees(a.content), a.liveEvents, extra);
  return { variables, events, sceneEvents: events.filter(e => !CORE.includes(e)), types: logicTypes(a.content) };
};
/** A variable's signal: true / false is a vote, any other value a control */
const varSignal = (info: { type: string } | undefined): DeckSignal => (!info || info.type === "boolean" ? "vote" : "control");
/** What an action carries, as the legend draws it */
const actionSignal = (action: BehaviourAction, types: ReadonlyMap<string, { type: string }>): DeckSignal =>
  action.action === "set" ? varSignal(types.get(action.name)) : action.action === "emit" ? "event" : action.action === "goto" ? "scene" : "address";
const eventKind = (on: string) => (CORE.includes(on) ? "core event" : (BEHAVIOUR_NODE_EVENTS as readonly string[]).includes(on) ? "the node's own event" : "scene event");

/** The node actions a rule at a place can write: by address for a scene's rule, by name under its node for a node's */
const nodeActions = (a: AddContext, kind: string, place: RulePlace): Array<{ action: BehaviourAction; label: string; detail: string }> => {
  if (place.doc === "scene") {
    const scene = a.content.scenes.find(s => s.id === place.scene);
    return (scene?.targets ?? []).flatMap(t => (t.entity ? ["", ...subtreeNames(t.entity).filter(n => n !== t.entity!.name)].map(name => {
      const address = nodeAddress(t.id, name);
      return { action: { ...newAction(kind as "show"), node: address } as BehaviourAction, label: `${kind} ${address}`, detail: name ? `in ${t.id}` : `entity of ${t.id}` };
    }) : []));
  }
  if (place.doc !== "entity") return [];
  const node = nodeOf(a.content, place.target, place.path);
  if (!node) return [];
  return ["", ...subtreeNames(node).filter(n => n !== node.name)].map(name => ({
    action: { ...newAction(kind as "show"), node: name } as BehaviourAction, label: `${kind} ${name || "its own node"}`, detail: name ? `under ${nodeLabel(node)}` : nodeLabel(node),
  }));
};

/** The choices of one type at this level, for this selection (none when it does not fit) */
const choicesOf = (kind: string, a: AddContext): AddChoice[] => {
  const { variables, events, sceneEvents, types } = known(a);
  const group = headOf(kind, a);
  if (kind === "rule" || kind === "event") {
    const place = a.place!;
    const ons = kind === "event" ? sceneEvents : place.doc === "entity" ? [...BEHAVIOUR_NODE_EVENTS, ...events] : events;
    return ons.map(on => ({
      key: `${kind}:${on}`, kind, label: on, detail: eventKind(on), group, terms: "rule on event", signal: "event", type: "event", op: { op: "addRule", at: place, on },
    }));
  }
  const rule = a.rule;
  const current = rule ? ruleOf(a.content, rule) : undefined;
  if (rule && !current) return [];
  const where: RulePlace = rule ?? a.place!;
  if (kind === "variable") {
    const read = new Set(current?.when ? expressionVariables(current.when) : []);
    return variables.filter(v => !read.has(v)).map(name => {
      const info = types.get(name);
      const term = !info || info.type === "boolean" ? name : `${name} == ${literalOf(info)}`;
      const op: Op = rule ? { op: "setCondition", home: home(rule, "when"), when: appendCondition(current!.when, term) } : { op: "addRule", at: where, when: term };
      return {
        key: `when:${name}`, kind, label: `when ${term}`, detail: current?.when ? `and ${current.when}` : info?.type ?? "untyped", group, terms: "rule when variable",
        signal: varSignal(info), type: "condition", op,
      };
    });
  }
  const add = (action: BehaviourAction, label: string, detail: string): AddChoice => ({
    key: `${kind}:${label}`, kind, label, detail, group, terms: "rule do verb", signal: actionSignal(action, types), type: "action",
    op: rule ? { op: "addAction", home: home(rule, "do"), action } : { op: "addRule", at: where, do: [action] },
  });
  if (kind === "set") return variables.map(name => add({ action: "set", name, value: literalOf(types.get(name)) }, `set ${name} = ${literalOf(types.get(name))}`, types.get(name)?.type ?? "untyped"));
  if (kind === "emit") return sceneEvents.map(event => add({ action: "emit", event }, `emit ${event}`, "a scene event"));
  if (kind === "goto") return a.content.scenes.map(scene => add({ action: "goto", address: sceneAddress(scene.id) }, `goto ${scene.label}`, sceneAddress(scene.id)));
  return nodeActions(a, kind, where).map(n => add(n.action, n.label, n.detail));
};

/** A typed new name: `emit fooBar`, `set fooBar`, `when fooBar`, `rule on fooBar` (or `on fooBar`) */
const typed = (query: string, a: AddContext): AddChoice | null => {
  const m = /^\s*(emit|set|when|rule on|on)\s+(\S+)\s*$/i.exec(query);
  if (!m || !CAMEL_CASE_NAME.test(m[2])) return null;
  const [word, name] = [m[1].toLowerCase(), m[2]];
  const kind = word === "rule on" || word === "on" ? "rule" : word === "when" ? "variable" : word;
  if (fitsHere(kind, a)) return null;
  const group = headOf(kind, a);
  if (kind === "rule") return { key: `new:${query}`, kind, label: name, detail: "a new event", group, terms: "rule on event", signal: "event", type: "new event", op: { op: "addRule", at: a.place!, on: name } };
  const rule = a.rule;
  const current = rule ? ruleOf(a.content, rule) : undefined;
  if (kind === "variable") {
    const op: Op = rule ? { op: "setCondition", home: home(rule, "when"), when: appendCondition(current?.when ?? null, name) } : { op: "addRule", at: a.place!, when: name };
    return { key: `new:${query}`, kind, label: `when ${name}`, detail: "a new variable", group, signal: "vote", type: "new variable", op };
  }
  const action: BehaviourAction = kind === "set" ? { action: "set", name, value: "true" } : { action: "emit", event: name };
  const op: Op = rule ? { op: "addAction", home: home(rule, "do"), action } : { op: "addRule", at: a.place!, do: [action] };
  return {
    key: `new:${query}`, kind, label: kind === "set" ? `set ${name} = true` : `emit ${name}`, detail: kind === "set" ? "a new variable" : "a new event", group,
    signal: kind === "set" ? "vote" : "event", type: kind === "set" ? "new variable" : "new event", op,
  };
};

const matcher = (query: string) => {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  return (c: AddChoice) => words.every(w => `${c.label} ${c.detail} ${c.terms ?? ""}`.toLowerCase().includes(w));
};
/** The same op is offered once (rule and event both offer `bellRang`); a grey line only when it matches what is typed */
const tidy = (out: AddChoice[], match: (c: AddChoice) => boolean) =>
  out.filter((c, i) => (c.op ? out.findIndex(o => o.op && o.label === c.label && o.group === c.group) === i : match(c)));

/**
 * What can be added here: the choices of **every** addable type (or of `kind` alone – a type dropped from the Library), in
 * the palette's order under their family's header, filtered by `query` (any word of the label); a type refused here is one
 * grey line saying why, and the full list ends with the types made elsewhere (a node in the tree, a plugin in Code, …)
 */
export const addChoices = (a: AddContext, { kind = null as string | null, query = "", registry = coreRegistry } = {}): AddChoice[] => {
  const match = matcher(query);
  const out: AddChoice[] = [];
  for (const entry of palette(registry).filter(e => (kind ? e.kind === kind : e.addable))) {
    const why = fitsHere(entry.kind, a, registry);
    if (why) {
      const label = entry.label.toLowerCase();
      // Types refused for one reason share a line: "play, pause, … – Pick the node of an experience rule in its Inspector"
      const same = out.find(c => c.disabled === why);
      if (same) same.label = `${same.label}, ${label}`;
      else out.push({ key: `no:${entry.kind}`, kind: entry.kind, label, detail: why, op: null, disabled: why, group: familyOf(entry.kind) });
      continue;
    }
    out.push(...choicesOf(entry.kind, a).filter(match));
  }
  // The types made elsewhere, as grey lines – the list shows every type there is
  if (!kind) {
    for (const entry of palette(registry).filter(e => !e.addable && e.kind !== "far")) {
      out.push({ key: `info:${entry.kind}`, kind: entry.kind, label: entry.label.toLowerCase(), detail: entry.detail, op: null, disabled: entry.detail, group: "Not added here" });
    }
  }
  const fresh = typed(query, a);
  if (fresh && !out.some(c => c.op && c.label === fresh.label) && (!kind || fresh.kind === kind || (kind === "event" && fresh.kind === "rule"))) out.unshift(fresh);
  return tidy(out, match);
};

// ── A wire let go on the empty canvas (#334) ──

/** The port a wire was dragged from, and which way: `out` from an outlet, `in` back from an inlet */
export interface WireFrom {
  node: GraphNode;
  port: string;
  dir: "in" | "out";
}

/** Where a new rule wired to a box may go: the place (the selected node, …), and the scene's own rules beside it */
const rulePlaces = (a: AddContext): RulePlace[] => {
  const places: RulePlace[] = a.place ? [a.place] : [];
  if ((a.context.level === "scene" || a.context.level === "entity") && !places.some(p => p.doc === "scene")) places.push({ doc: "scene", scene: a.context.scene });
  return places;
};

const info = (key: string, label: string, why: string, group = "Not made from here"): AddChoice => ({ key, kind: "info", label, detail: why, op: null, disabled: why, group });

const wireAll = (a: AddContext, from: WireFrom, query: string): AddChoice[] => {
  const { node, port, dir } = from;
  if (a.context.level === "group") return [info("read-only", "nothing", "This level is read-only – open a scene or a target")];
  if (node.far) return [info("far", node.label, "That box stands for something elsewhere – open its level to connect there")];
  const { variables, events, sceneEvents, types } = known(a);
  const term = (name: string) => {
    const t = types.get(name);
    return !t || t.type === "boolean" ? name : `${name} == ${literalOf(t)}`;
  };
  /** A new rule at each place, made by `rule(place)` */
  const newRules = (kind: string, label: string, group: string, rule: (p: RulePlace) => Omit<Extract<Op, { op: "addRule" }>, "op" | "at">, places = rulePlaces(a)): AddChoice[] =>
    places.map(p => {
      const made = rule(p);
      const on = made.on ?? firstEvent(p);
      const signal: DeckSignal = made.do?.length ? actionSignal(made.do[0], types) : made.when ? "vote" : "event";
      return {
        key: `${kind}:${label}:${JSON.stringify(p)}`, kind, label: places.length > 1 ? `${label} · ${whereWords(a.content, p)}` : label, group, terms: "rule new", signal,
        type: p.doc === "entity" ? "node rule" : `${p.doc} rule`,
        detail: `a new rule on ${on} · on ${whereWords(a.content, p)}`, op: { op: "addRule", at: p, ...made },
      };
    });

  // A plugin's two pins (#334): a rule on each event it emits, a rule emitting each event it listens to – its rows make nothing
  if (node.kind === "plugin") {
    const code = info("plugin", node.label, `The plugin ${node.label}'s code decides what it reads, sets and drives – change it in the Code tab`);
    if (dir === "out" && port === "emits" && node.bus?.emits.length) {
      return node.bus.emits.flatMap(name => newRules("rule", `rule on ${name}`, `Rules – listening to ${node.label}'s event`, () => ({ on: name })));
    }
    if (dir === "in" && port === "listens" && node.bus?.listens.length) {
      return node.bus.listens.flatMap(name => newRules("emit", `emit ${name}`, `Rules – emitting what ${node.label} listens to`, () => ({ do: [{ action: "emit", event: name }] })));
    }
    return [code];
  }

  if (node.kind === "rule" && node.home) {
    const ref = refOf(node.home);
    const withRule = { ...a, rule: ref };
    if (dir === "out" && port === "do") {
      const out = ACTION_KINDS.flatMap(kind => (fitsHere(kind, withRule) ? [] : choicesOf(kind, withRule)));
      const fresh = /^\s*(emit|set)\s/i.test(query) ? typed(query, withRule) : null;
      if (NODE_VERBS.some(v => fitsHere(v, withRule))) out.push(info("no:verbs", NODE_VERBS.join(", "), fitsHere("show", withRule)!, headOf("show", withRule)));
      return fresh ? [fresh, ...out] : out;
    }
    if (dir === "in" && port === "when") {
      const fresh = /^\s*when\s/i.test(query) ? typed(query, withRule) : null;
      const out = choicesOf("variable", withRule);
      return fresh ? [fresh, ...out] : out;
    }
    if (dir === "in" && port === "on") {
      const current = ruleOf(a.content, ref);
      const group = `Events – ${ruleName(a.content, ref)} listens to it`;
      const ons = ref.doc === "entity" ? [...BEHAVIOUR_NODE_EVENTS, ...events] : events;
      const out: AddChoice[] = ons.filter(on => on !== current?.on).map(on => ({
        key: `on:${on}`, kind: "event", label: on, detail: eventKind(on), group, terms: "rule on event", signal: "event", type: "event", op: { op: "setEvent", home: { ...ref, part: "on" }, on },
      }));
      const m = /^\s*(?:rule on|on)?\s*(\S+)\s*$/i.exec(query);
      if (m && CAMEL_CASE_NAME.test(m[1]) && !ons.includes(m[1])) {
        out.unshift({ key: `new:${m[1]}`, kind: "event", label: m[1], detail: "a new event", group, signal: "event", type: "new event", op: { op: "setEvent", home: { ...ref, part: "on" }, on: m[1] } });
      }
      return out;
    }
    return [info("do:i", node.label, "This outlet re-aims its action – drop it on a box, or drag from the rule's + to add one")];
  }

  if (node.kind === "event") {
    const name = node.label;
    const own = node.pick?.kind === "node" && node.id !== `event:${name}` ? node.pick : null;
    if (dir === "out") {
      const places: RulePlace[] = own ? [{ doc: "entity", target: own.target, path: own.path }] : rulePlaces(a);
      return newRules("rule", `rule on ${name}`, "Rules – listening to it", () => ({ on: name }), places);
    }
    if (own) return [info("own", name, `${name} is ${node.detail}'s own event – the engine raises it, a rule listens to it`)];
    if (CORE.includes(name)) return [info("core", name, `${name} is the engine's own event – a rule listens to it, never emits it`)];
    return newRules("emit", `emit ${name}`, "Rules – emitting it", () => ({ do: [{ action: "emit", event: name }] }));
  }

  if (node.kind === "variable") {
    const name = node.label;
    if (dir === "out") return newRules("variable", `when ${term(name)}`, "Rules – reading it", () => ({ when: term(name) }));
    return newRules("set", `set ${name} = ${literalOf(types.get(name))}`, "Rules – setting it", () => ({ do: [{ action: "set", name, value: literalOf(types.get(name)) }] }));
  }

  if (node.kind === "node" && node.pick?.kind === "node") {
    const { target, path } = node.pick;
    const spec = node.ports.find(p => p.id === port && p.dir === dir);
    if (dir === "out" && spec?.kind === "event") {
      const places: RulePlace[] = [{ doc: "entity", target, path }, ...rulePlaces(a).filter(p => p.doc === "scene")];
      return newRules("rule", `rule on ${spec.label}`, "Rules – listening to it", () => ({ on: spec.label }), places);
    }
    if (dir === "in" && spec?.kind === "param") {
      return [info("param", spec.label, `${spec.label} takes ${typeWords(spec.type!)} – no rule action sets a component's parameter yet; set it in the Inspector`)];
    }
    if (dir === "in" && spec?.kind === "src") {
      return [info("src", spec.label, `${spec.label} takes ${typeWords(spec.type!)} – files on the canvas come later; pick the file in the Inspector`)];
    }
    if (dir === "in" && (port === "in" || port.startsWith("act:"))) {
      const verb = port.startsWith("act:") ? port.slice(4) : null;
      if (verb !== null && !NODE_VERBS.includes(verb)) return [info("verb", verb, `${node.label} has no verb ${verb} a rule can write`)];
      const verbs = verb ? [verb] : NODE_VERBS;
      const root = a.content.scenes.flatMap(s => s.targets).find(t => t.id === target)?.entity;
      const own = path.length && root ? nodeAt(root, path)?.name ?? "" : "";
      const scenePlace = rulePlaces(a).find(p => p.doc === "scene");
      // Its own rule acts on its own node; a scene's rule names it by address – a node without a name only its own rules reach
      const places: RulePlace[] = [{ doc: "entity", target, path }, ...(scenePlace && (!path.length || own) ? [scenePlace] : [])];
      return verbs.flatMap(v => newRules(v, `${v} ${node.label}`, "Rules – acting on it",
        p => ({ do: [{ ...newAction(v as "show"), node: p.doc === "scene" ? nodeAddress(target, own) : "" } as BehaviourAction] }), places));
    }
    return [info("port", node.label, "Nothing is made from this port – drop the wire on a box that takes it")];
  }

  if (node.kind === "scene" && node.pick?.kind === "scene" && dir === "in" && (port === "arrives" || port === "in") && a.context.level === "experience") {
    const address = sceneAddress(node.pick.scene);
    return newRules("goto", `goto ${node.label}`, "Rules – going there", () => ({ do: [{ action: "goto", address }] }), [{ doc: "experience" }]);
  }
  void variables;
  void sceneEvents;
  return [info("none", node.label, "Nothing is made from this port – drop the wire on a box that takes it")];
};

/** What a wire let go on the empty canvas can make and wire to its port, filtered by `query` (see the module's comment) */
export const wireChoices = (a: AddContext, from: WireFrom, { query = "" } = {}): AddChoice[] => {
  const match = matcher(query);
  const out = wireAll(a, from, query);
  return tidy(out.filter(c => !c.op || match(c) || c.key.startsWith("new:")), match);
};
