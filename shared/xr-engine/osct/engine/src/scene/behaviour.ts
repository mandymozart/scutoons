import type { Object3D } from "three";
import {
  BEHAVIOUR_TYPE, type BehaviourAction, type ComponentData, type BehaviourRule, behaviourRules, describeActions, type EntityData,
  isBehaviourNodeEvent, type NodeTimeline, parseAddress, type Target,
} from "../types/index";
import { evaluateExpression, type Expr, exprMessage, isTruthy, parseExpression } from "@shared/text/expressions";
import { navigateTo, noteBehaviour, runtimeApi, type PluginVars, variableInfo } from "./plugin-runtime";

/**
 * Rules without code at runtime (grammar: `shared/types/behaviour.ts`): a node's `behaviour` component, a scene's and the
 * experience's `logic` (schema 5.0). Rules run on the plugin runtime's own mechanisms – the event bus and the variables
 * (`plugin-runtime.ts`), as a plugin's code would. A node's own events (`found`, `lost`, `tap`, `unlocked`) come from
 * the entity's hooks, `sceneChanged` for a scene's and the experience's rules from the scene once the new scene's nodes
 * are built (`LogicInstance.opened`), every other event from the bus.
 *
 * Rules of an event: every rule's condition (an expression over `api.vars`, `shared/text/expressions.ts`) is read first,
 * then the actions of the rules that matched run in their order – so a rule cannot switch on a later rule of the same
 * event (a toggle in two rules works). A rule's `emit` reaches the bus at once; a chain of rules emitting events is cut off
 * after `MAX_DEPTH` nested events (logged), so two rules that emit each other's event cannot hang the scene. A `goto` asks
 * the host to open the address (`ArSceneOptions.navigate`). Every rule that fires is told to the editor's log
 * (`noteBehaviour`), never to the bus. An action that cannot run (a node or a clip not there, a write the runtime refuses)
 * is logged and the next one runs; a condition that cannot be evaluated is false (logged once).
 */

/** How deep rules may set off rules through events before the chain is cut */
export const MAX_DEPTH = 8;
let depth = 0;

export interface BehaviourContext {
  node: EntityData;
  target: Target;
  /** The placement groups of the node's subtree by path relative to it (`""` = the node itself, `"1.0"`) */
  nodes: Record<string, Object3D>;
  /** The model timelines of the node's subtree by the same paths */
  timelines: Record<string, NodeTimeline>;
  /** The plugin parts that answer a verb (their spec's `actions`), by the same paths */
  actors?: Record<string, (verb: string) => boolean>;
}

export interface BehaviourInstance {
  onFound(): void;
  onLost(): void;
  onTap(): void;
  onUnlocked(): void;
  dispose(): void;
  /** New rules in place (the editor's live edit); false when the node has no behaviour any more */
  tune(node: EntityData): boolean;
}

/** The node's behaviour component, or undefined */
export const behaviourOf = (node: EntityData): ComponentData | undefined =>
  node.components?.find(component => component.type === BEHAVIOUR_TYPE);

/** Whether a variable reader passes a rule's condition (none = always; one that cannot be evaluated = never) */
export const passes = (when: string | Expr | null, vars: Pick<PluginVars, "get">): boolean => {
  if (when === null) return true;
  const result = evaluateExpression(when, name => vars.get(name));
  return result.ok && isTruthy(result.value);
};

/** The paths of the named nodes of a subtree, relative to its root – the first node of a name wins */
export const namedPaths = (node: EntityData): Map<string, string> => {
  const paths = new Map<string, string>();
  const walk = (n: EntityData, path: string) => {
    const name = n.name?.trim();
    if (name && !paths.has(name)) paths.set(name, path);
    (n.children ?? []).forEach((child, i) => walk(child, path ? `${path}.${i}` : `${i}`));
  };
  walk(node, "");
  return paths;
};

/** A node an action names: its placement group and its timeline – or why it is not there */
export type NodeLookup = { object?: Object3D; timeline?: NodeTimeline; act?: (verb: string) => boolean } | { missing: string };

/** What a list of rules needs from where it lives */
interface RuleHost {
  /** Who the warnings and the API's listeners are told under: `key`, `scene cover`, `experience` */
  label: string;
  api: ReturnType<typeof runtimeApi>;
  /** The node an action names (a subtree name, or an address); `what` for the messages ("the behaviour's node") */
  node(name: string): NodeLookup;
  /** What a rule's `emit` sends along */
  payload(): Record<string, unknown>;
  /** Tells the editor's log that a rule fired */
  note(rule: number, on: string, actions: string): void;
  /** The events the host delivers itself (`run`) – never subscribed on the bus */
  own(event: string): boolean;
}

interface CompiledRule {
  rule: BehaviourRule;
  index: number;
  /** The parsed condition; null = always; "bad" = it does not parse (never passes) */
  when: Expr | null | "bad";
}

/**
 * Where a list of rules lives on the stage, as the editor names it to run one of them by hand (#334): a node's behaviour
 * (its target and its node's name – "" for a node without one), the open scene's logic, the experience's
 */
export type RuleHome = { targetId: string; node: string } | { home: "scene"; scene: string } | { home: "experience" };

/** What running a rule by hand did: it ran; its condition was false (nothing ran); no such rule on the stage */
export type HandRun = "ran" | "held" | "missing";

/** The rule lists on the stage now (each one while it is built): `runRuleByHand` finds a rule by its home */
const onStage = new Set<{ home: () => RuleHome; one: (index: number) => HandRun }>();

const sameHome = (a: RuleHome, b: RuleHome): boolean =>
  "targetId" in a ? "targetId" in b && a.targetId === b.targetId && a.node === b.node
    : "targetId" in b ? false : a.home === b.home && (a.home === "experience" || (b.home === "scene" && a.scene === b.scene));

/**
 * Run one rule of a list on the stage as if its event had come (the editor's *Run this rule*, #334): its condition applies,
 * its actions run as the event's would (the log listens to `behaviour`, an `emit` reaches the bus), no other rule of that event
 * runs. `rule`: its index in its list (0 = the first). The first list of that home that has it answers.
 */
export const runRuleByHand = (home: RuleHome, rule: number): HandRun => {
  let result: HandRun = "missing";
  for (const list of onStage) {
    if (!sameHome(list.home(), home)) continue;
    result = list.one(rule);
    if (result !== "missing") break;
  }
  return result;
};

/** A list of rules on a host: `setRules` subscribes their bus events, `run` fires an event's matching rules, `one` one rule by hand */
const ruleRunner = (host: RuleHost) => {
  let rules: CompiledRule[] = [];
  let unsubscribe: Array<() => void> = [];
  const warned = new Set<string>();

  const warn = (message: string) => {
    if (warned.has(message)) return;
    warned.add(message);
    console.warn(`[behaviour] ${host.label}: ${message}`);
  };

  const lookup = (name: string, verb: string): Exclude<NodeLookup, { missing: string }> | undefined => {
    const found = host.node(name);
    if ("missing" in found) {
      warn(`cannot ${verb} – ${found.missing}`);
      return undefined;
    }
    return found;
  };

  const act = (action: BehaviourAction) => {
    switch (action.action) {
      case "play": case "pause": case "stop": {
        const found = lookup(action.node, action.action);
        if (!found) return;
        // A plugin part of the node that declares the verb listens to it (schema 5.1); the node's own clip plays as well
        const answered = found.act?.(action.action) ?? false;
        const timeline = found.timeline;
        if (!timeline) {
          if (!answered) warn(`${action.node || "the behaviour's node"} has no animation clip to ${action.action} (not a model with clips) and nothing on it answers ${action.action}`);
          return;
        }
        if (action.action === "play") {
          if (action.clip && !timeline.clips.some(clip => clip.name === action.clip)) return warn(`no clip "${action.clip}" (${timeline.clips.map(c => c.name).join(", ")})`);
          timeline.play(action.clip || undefined);
        } else if (action.action === "pause") {
          timeline.pause();
        } else {
          timeline.seek(0);
          timeline.pause();
        }
        return;
      }
      case "show": case "hide": {
        const found = lookup(action.node, action.action);
        if (found?.object) found.object.visible = action.action === "show";
        found?.act?.(action.action);
        return;
      }
      case "set": {
        // The value is an expression (schema 5.0, B2): `tries + 1`; its scope the action's, else the declared one, else scene
        const scope = action.scope ?? variableInfo(action.name)?.scope ?? "scene";
        if (action.value === null) return host.api.vars.set(action.name, null, scope);
        const result = evaluateExpression(action.value, name => host.api.vars.get(name));
        const error = result.problems.find(problem => problem.severity === "error");
        if (!result.ok || result.value === undefined) {
          return warn(`cannot set ${action.name} = ${action.value} – ${error ? exprMessage(error) : "a variable in it is not set"}`);
        }
        host.api.vars.set(action.name, result.value, scope);
        return;
      }
      case "emit":
        host.api.events.emit(action.event, host.payload());
        return;
      case "goto":
        if (!parseAddress(action.address)) return warn(`"${action.address}" is no address to go to (scene:<id>)`);
        navigateTo(action.address, host.label);
        return;
    }
  };

  const holds = (compiled: CompiledRule): boolean => {
    if (compiled.when === null) return true;
    if (compiled.when === "bad") return false;
    const result = evaluateExpression(compiled.when, name => host.api.vars.get(name));
    const error = result.problems.find(problem => problem.severity === "error");
    if (error) warn(`rule ${compiled.index + 1}: its condition ${JSON.stringify(compiled.rule.when)} cannot be evaluated – ${exprMessage(error)}`);
    return result.ok && isTruthy(result.value);
  };

  const run = (event: string) => fire(rules.filter(compiled => compiled.rule.on === event && holds(compiled)), event);

  /** One rule by hand (the editor's *Run this rule*, #334): as if its `on` came – its condition applies, no other rule runs */
  const one = (index: number): HandRun => {
    const compiled = rules.find(c => c.index === index);
    if (!compiled) return "missing";
    if (!holds(compiled)) return "held";
    fire([compiled], compiled.rule.on);
    return "ran";
  };

  const fire = (matched: CompiledRule[], event: string) => {
    if (!matched.length) return;
    if (depth >= MAX_DEPTH) return warn(`rules set off rules more than ${MAX_DEPTH} events deep – "${event}" stopped there`);
    depth++;
    try {
      matched.forEach(({ rule, index }) => {
        host.note(index + 1, event, describeActions(rule.do));
        rule.do.forEach(action => {
          try {
            act(action);
          } catch (error) {
            console.error(`[behaviour] ${host.label}: rule ${index + 1} (${event}) could not ${action.action}:`, error);
          }
        });
      });
    } finally {
      depth--;
    }
  };

  const dispose = () => {
    unsubscribe.forEach(off => off());
    unsubscribe = [];
  };

  const setRules = (next: readonly BehaviourRule[]) => {
    dispose();
    warned.clear();
    rules = next.map((rule, index) => {
      if (rule.when === null) return { rule, index, when: null };
      const parsed = parseExpression(rule.when);
      const error = parsed.problems.find(problem => problem.severity === "error");
      if (!parsed.expr || error) {
        warn(`rule ${index + 1}: its condition ${JSON.stringify(rule.when)} does not parse – ${error ? exprMessage(error) : "no expression"}; the rule never runs`);
        return { rule, index, when: "bad" as const };
      }
      return { rule, index, when: parsed.expr };
    });
    // An empty `on` listens to nothing (the editor's Logic view lists it as a problem)
    const events = [...new Set(next.map(rule => rule.on).filter(event => event && !host.own(event)))];
    unsubscribe = events.map(event => host.api.events.on(event, () => run(event)));
  };

  return { setRules, run, one, dispose };
};

/** The behaviour of a node (null when it carries none): its rules listen to the node's hooks and the bus */
export const applyBehaviour = (context: BehaviourContext): BehaviourInstance | null => {
  const component = behaviourOf(context.node);
  if (!component) return null;
  const { target, nodes, timelines, actors = {} } = context;
  let paths = new Map<string, string>();
  let ownName = "";

  const runner = ruleRunner({
    label: target.id,
    api: runtimeApi(`${BEHAVIOUR_TYPE}:${target.id}`),
    node: name => {
      if (!name) return { object: nodes[""], timeline: timelines[""], act: actors[""] };
      const path = paths.get(name);
      if (path === undefined) return { missing: `no node named "${name}" under ${ownName || "the behaviour's node"}` };
      return { object: nodes[path], timeline: timelines[path], act: actors[path] };
    },
    payload: () => ({ targetId: target.id, ...(ownName ? { node: ownName } : {}) }),
    note: (rule, on, actions) => noteBehaviour({ targetId: target.id, ...(ownName ? { node: ownName } : {}), rule, on, do: actions }),
    own: isBehaviourNodeEvent,
  });

  const listen = (node: EntityData) => {
    paths = namedPaths(node);
    ownName = node.name?.trim() ?? "";
    runner.setRules(behaviourRules(behaviourOf(node)));
  };
  listen(context.node);
  const entry = { home: (): RuleHome => ({ targetId: target.id, node: ownName }), one: runner.one };
  onStage.add(entry);

  return {
    onFound: () => runner.run("found"),
    onLost: () => runner.run("lost"),
    onTap: () => runner.run("tap"),
    onUnlocked: () => runner.run("unlocked"),
    dispose: () => {
      onStage.delete(entry);
      runner.dispose();
    },
    tune: node => {
      if (!behaviourOf(node)) return false;
      listen(node);
      return true;
    },
  };
};

// ── A scene's and the experience's logic (schema 5.0) ─────────────────────────────────────────────

export interface LogicContext {
  /** `scene`: live while its scene is open; `experience`: live for the whole session, across scene switches */
  home: "scene" | "experience";
  /** The scene of a scene's rules (the log and the warnings name it) */
  sceneId?: string;
  /**
   * The node of a target of the open scene by its name in the target's entity ("" = its root) – the scene answers from
   * the entities it built; `missing` says why there is none (the target is not in the open scene, no node of that name)
   */
  node(targetId: string, name: string): NodeLookup;
}

export interface LogicInstance {
  /**
   * The scene opened and its nodes are built: the rules on `sceneChanged` run (the bus delivered it before the nodes
   * existed, so these rules listen to it from here)
   */
  opened(): void;
  /** Other rules in place (the content changed) */
  tune(rules: readonly BehaviourRule[]): void;
  dispose(): void;
}

/** A scene's or the experience's rules: they listen to the bus and name nodes by address (`target:<targetId>/<node name>`) */
export const applyLogic = (rules: readonly BehaviourRule[], context: LogicContext): LogicInstance => {
  const label = context.home === "scene" ? `scene ${context.sceneId ?? ""}`.trim() : "experience";
  const runner = ruleRunner({
    label,
    api: runtimeApi(`logic:${label}`),
    node: address => {
      const at = parseAddress(address);
      if (at?.scheme !== "target") return { missing: `"${address}" is no node's address (target:<targetId>/<node name>)` };
      return context.node(at.id, at.node);
    },
    payload: () => ({ [context.home]: context.home === "scene" ? context.sceneId ?? "" : true }),
    note: (rule, on, actions) => noteBehaviour({ home: context.home, ...(context.sceneId ? { scene: context.sceneId } : {}), rule, on, do: actions }),
    own: event => event === "sceneChanged" || isBehaviourNodeEvent(event),
  });
  runner.setRules(rules);
  const home: RuleHome = context.home === "scene" ? { home: "scene", scene: context.sceneId ?? "" } : { home: "experience" };
  const entry = { home: () => home, one: runner.one };
  onStage.add(entry);
  return {
    opened: () => runner.run("sceneChanged"),
    tune: next => runner.setRules(next),
    dispose: () => {
      onStage.delete(entry);
      runner.dispose();
    },
  };
};
