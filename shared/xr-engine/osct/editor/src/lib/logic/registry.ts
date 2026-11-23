import { BEHAVIOUR_ACTIONS, type BehaviourAction, type BehaviourRule } from "@engine/types";
import type { NodeFamily, NodeType, PortSpec, Signal } from "./types";

/**
 * The **node type registry** of the logic model (#334, research §5.2, §5.8): every box of a `View` has a kind registered
 * here with its family, its ports and its owner. The core registers the families and OSCT's verbs; a title or a plugin can
 * register more (`owner` names it – a plugin's custom nodes come with L-T13, free canvas nodes with family `free` with
 * L-T7). **Ports come from the rule grammar, not drawn by hand**: a rule has an `on` inlet (event), a `when` inlet (value,
 * many) and one `do` outlet per action, its signal by verb (`set` → value, `emit` → event, `show` / `hide` / `play` /
 * `pause` / `stop` → node, `goto` → scene).
 */

/** The verbs the core registers: the grammar's (`BEHAVIOUR_ACTIONS`, `goto` among them since content schema 5.0) */
export const CORE_VERBS = BEHAVIOUR_ACTIONS;
export type CoreVerb = (typeof CORE_VERBS)[number];

/** What an action's outlet carries, by verb */
export const verbSignal = (verb: string): Signal =>
  verb === "set" ? "value" : verb === "emit" ? "event" : verb === "goto" ? "scene" : "node";

const port = (id: string, label: string, signal: Signal, dir: "in" | "out", many = false): PortSpec =>
  many ? { id, label, signal, dir, many } : { id, label, signal, dir };

/**
 * A rule's ports: `on`, `when`, `do:<i>` per action in the order of its actions, and `do` – the outlet a new action is
 * dragged from (to a node, a variable or an event: the action its target takes)
 */
export const rulePorts = (rule: Pick<BehaviourRule, "do">): PortSpec[] => [
  port("on", "on", "event", "in"),
  port("when", "when", "value", "in", true),
  ...rule.do.map((action, i) => port(`do:${i}`, action.action, verbSignal(action.action), "out")),
  port("do", "+", "node", "out", true),
];

/** The ports of a place that joins others by name: a scene at the experience level */
const placePorts = (): PortSpec[] => [
  port("listens", "listens", "event", "in", true), port("reads", "reads", "value", "in", true), port("arrives", "arrives", "scene", "in", true),
  port("emits", "emits", "event", "out", true), port("sets", "sets", "value", "out", true), port("goto", "goto", "scene", "out", true),
];

const type = <P>(kind: string, family: NodeFamily, ports: (params: P) => PortSpec[]): NodeType<P> => ({ kind, family, ports, owner: "core" });

/** The core's node types */
export const CORE_TYPES: NodeType<never>[] = [
  type("event", "event", () => [port("in", "", "event", "in", true), port("out", "", "event", "out", true)]),
  type<Pick<BehaviourRule, "do">>("rule", "rule", rulePorts),
  type("variable", "variable", () => [port("set", "set", "value", "in", true), port("get", "get", "value", "out", true)]),
  // A node: what acts on it (`in`), then the ports its specs give (`spec-ports.ts`, handed in by the projection)
  type<{ ports?: PortSpec[] } | undefined>("node", "entity", params => [port("in", "", "node", "in", true), ...(params?.ports ?? [])]),
  // A plugin's pins (#334, Tilman 2026-11-21): only what a wire can change – a rule listens to what it emits, a rule emits what it
  // listens to; what it reads, sets and drives are display rows (`GraphNode.rows`), no pins
  type<{ emits?: readonly string[]; listens?: readonly string[] } | undefined>("plugin", "plugin", p => [
    ...(p?.listens?.length ? [port("listens", "listens", "event", "in", true)] : []),
    ...(p?.emits?.length ? [port("emits", "emits", "event", "out", true)] : []),
  ]),
  // A scene box also takes the node actions of the experience's rules (on a node of one of its targets)
  type("scene", "scene", () => [...placePorts(), port("nodes", "nodes", "node", "in", true)]),
  type("group", "scene", () => []),
  type("far", "far", () => [port("in", "", "far", "in", true), port("out", "", "far", "out", true)]),
  ...CORE_VERBS.map(verb => type<BehaviourAction | void>(verb, "action", () => [port("in", "", "event", "in"), port("out", verb, verbSignal(verb), "out")])),
] as NodeType<never>[];

/** A set of node types by kind: the core's, then a title's or a plugin's (a kind is registered once) */
export class NodeRegistry {
  #types = new Map<string, NodeType<never>>();

  constructor(types: readonly NodeType<never>[] = CORE_TYPES) {
    types.forEach(t => this.register(t));
  }

  /** Add a type; a kind that is already registered is refused (a title or plugin cannot replace the core's) */
  register<P>(nodeType: NodeType<P>): void {
    const known = this.#types.get(nodeType.kind);
    if (known) throw new Error(`The logic node type ${nodeType.kind} is already registered by ${known.owner}`);
    this.#types.set(nodeType.kind, nodeType as NodeType<never>);
  }

  get<P = unknown>(kind: string): NodeType<P> | undefined {
    return this.#types.get(kind) as NodeType<P> | undefined;
  }

  /** A node's ports by its kind and params; none for a kind nobody registered */
  ports<P>(kind: string, params: P): PortSpec[] {
    return (this.#types.get(kind) as NodeType<P> | undefined)?.ports(params) ?? [];
  }

  all(): NodeType<never>[] {
    return [...this.#types.values()];
  }
}

/** The core registry the projection uses unless it is handed another */
export const coreRegistry = new NodeRegistry();
