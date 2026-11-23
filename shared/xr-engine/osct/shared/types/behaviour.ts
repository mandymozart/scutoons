/**
 * The grammar of rules without code: the built-in `behaviour` component (content schema 2.19), a scene's and the
 * experience's `logic` (schema 5.0, `./logic.ts`). Each rule names an event it reacts to (`on`), optionally a condition
 * on variables (`when`, an expression), and the actions it then runs (`do`):
 *
 *   components:
 *     - type: behaviour
 *       rules:
 *         - on: tap
 *           do:
 *             - play: rattle
 *             - set: { hasKey: true, tries: "tries + 1", mood: '"calm"' }
 *               scope: experience
 *             - emit: keyRattled
 *         - on: found
 *           when: hasKey && tries > 2
 *           do: { hide: hint }
 *         - on: found
 *           when: chapterDone
 *           do: { goto: "scene:epilogue" }
 *
 * **Where rules live** (`RuleHome`): on a node (`entity` – the behaviour component; its actions name nodes of its subtree),
 * on a scene (`scene` – live while the scene is open) or on the experience (`experience` – live for the whole session,
 * across scene switches). Scene and experience rules name nodes by address (`target:<targetId>/<node name>`, `./address.ts`).
 * **Events**: a node's rules listen to the node's own `found`, `lost`, `tap` and `unlocked` (the target's first find), or any
 * other name – an event of the engine's bus (a scene event a plugin or another rule emits, or a core event such as
 * `targetFound`). A scene's and the experience's rules listen to the bus only: scene events and the core events
 * (`BEHAVIOUR_CORE_EVENTS`) – never a node's own four, which belong to one node (`targetFound` with a target's id is the
 * scene-wide one). An empty `on` listens to nothing and is a problem (the editor can leave it empty while an author rewires).
 * **Conditions** (schema 5.0, #334): `when` is an expression of the experience's expression language
 * (`../text/expressions.ts`, SCUTOONS' / ink's: `hasKey && tries > 2`, `mood == "happy" || !door`); the rule runs when its
 * value is true the ink way (0, "", false and a variable that is not set are false). A variable is read like a plugin's
 * `api.vars.get(name)` – the innermost scope that has it. The object form of 2.19–4.x (`{ hasKey: true }`, `{ any: … }`,
 * `{ not: … }`) is gone; content of those schemas is migrated (`scripts/src/convert-content-5.ts`, `Migration5.php`).
 * **Actions**: `play: <clip>` (empty = the node's own clip – its `animation` component's, else the first), `pause` and
 * `stop` a clip, `show` and `hide` a node, `set: { name: value }` with an optional `scope` (the variable's declared scope,
 * else scene; `null` deletes) – **a value is an expression** (schema 5.0, B2: `tries + 1`, `hasKey`, `"calm"` – a text
 * value is written as the language's text literal, in YAML `'"calm"'`); a number or true / false may stand bare, being its
 * own expression – variables are typed (`./variables.ts`), `emit: <event>` (a scene event – never a core event), `goto: scene:<id>` (schema 5.0: the reader opens that
 * scene, as a link to `/scene/<id>`). In a node's rules the node of `play` (its `node`), of `pause`, `stop`, `show` and
 * `hide` (their value) is the behaviour's own node when empty, else the node of that `name` in its subtree; a shorthand
 * string runs a verb on the own node: `play`, `pause`, `stop`, `show`, `hide`. A scene's and the experience's rules have
 * no own node: those verbs take an address, and `play` needs its `node`.
 *
 * Every definition lives here – the content build, the guard, the engine (`engine/src/scene/behaviour.ts`), the editor's
 * rule editor and the MCP server read one grammar. `behaviourRules` / `readRules` read the content into the rules the code
 * works with; `behaviourData` / `rulesData` write rules back in the content's shortest form.
 */

import { CAMEL_CASE_NAME, isObject } from "../guards/primitives";
import { exprMessage, expressionVariables, parseExpression, showValue } from "../text/expressions";
import { addressProblem, GOTO_SCHEMES, NODE_SCHEMES } from "./address";

/** The component's type */
export const BEHAVIOUR_TYPE = "behaviour";

/** The node's own events: the target found, lost, tapped, unlocked (its first find) – a node's rules only */
export const BEHAVIOUR_NODE_EVENTS = ["found", "lost", "tap", "unlocked"] as const;
export type BehaviourNodeEvent = (typeof BEHAVIOUR_NODE_EVENTS)[number];

/** What a rule can do (`goto` since schema 5.0) */
export const BEHAVIOUR_ACTIONS = ["play", "pause", "stop", "show", "hide", "set", "emit", "goto"] as const;
export type BehaviourVerb = (typeof BEHAVIOUR_ACTIONS)[number];

/** The verbs that act on a node (a node's rules: the own node when its name is empty) – also the shorthands, a verb alone as a string */
export const BEHAVIOUR_NODE_ACTIONS = ["play", "pause", "stop", "show", "hide"] as const;

/** The variable scopes – the engine's runtime uses these as its own (`engine/src/scene/plugin-runtime.ts` `VAR_SCOPES`) */
export const BEHAVIOUR_SCOPES = ["scene", "stage", "session", "experience"] as const;
export type BehaviourScope = (typeof BEHAVIOUR_SCOPES)[number];

/** The engine's core events: listened to by a rule, never emitted (the runtime's `CORE_EVENTS` are these) */
export const BEHAVIOUR_CORE_EVENTS = ["targetFound", "targetLost", "targetUnlocked", "targetTapped", "sceneChanged", "arState", "varChanged"] as const;

/** A variable's or an event's name: camelCase letters and digits (the runtime's `EVENT_NAME_PATTERN` is this one) */
export const BEHAVIOUR_NAME_PATTERN = CAMEL_CASE_NAME;

/** Where a list of rules lives: a node's behaviour component, a scene's logic, the experience's logic */
export const RULE_HOMES = ["entity", "scene", "experience"] as const;
export type RuleHome = (typeof RULE_HOMES)[number];

/** A variable's value: a JSON scalar; `null` in a `set` = delete */
export type BehaviourValue = string | number | boolean;

export type BehaviourAction =
  /** `node`: a node's rules – a name in the subtree ("" = the own node); scene / experience rules – an address */
  | { action: "play"; clip: string; node: string }
  | { action: "pause" | "stop" | "show" | "hide"; node: string }
  /**
   * `value`: the expression's source (schema 5.0 – a bare number or true / false of the content reads as its text, `3`,
   * `true`); null deletes. `scope`: undefined = the variable's declared scope, else `scene`
   */
  | { action: "set"; name: string; value: string | null; scope?: BehaviourScope }
  | { action: "emit"; event: string }
  /** Schema 5.0: `scene:<id>` (`./address.ts`) – the host opens it (`ArSceneOptions.navigate`) */
  | { action: "goto"; address: string };

export interface BehaviourRule {
  on: string;
  /** An expression (`../text/expressions.ts`); null = always */
  when: string | null;
  do: BehaviourAction[];
}

export const isBehaviourNodeEvent = (event: string): event is BehaviourNodeEvent => (BEHAVIOUR_NODE_EVENTS as readonly string[]).includes(event);

export const isBehaviourCoreEvent = (event: string): boolean => (BEHAVIOUR_CORE_EVENTS as readonly string[]).includes(event);

const isScalar = (value: unknown): value is BehaviourValue =>
  typeof value === "string" || typeof value === "boolean" || (typeof value === "number" && Number.isFinite(value));

const show = (value: unknown) => JSON.stringify(value);

// ── Checking the content ──────────────────────────────────────────────────────────────────────

/** Problems of a behaviour component's fields (all but `type`) – the content build and the guard (`componentProblems`) */
export const behaviourProblems = (params: Record<string, unknown>, at: string): string[] => {
  const problems: string[] = [];
  Object.keys(params).filter(name => name !== "rules").forEach(name => problems.push(`${at}.${name}: unknown field of behaviour (rules)`));
  if (params.rules === undefined) return problems;
  return [...problems, ...rulesProblems(params.rules, `${at}.rules`, "entity")];
};

/** Problems of a list of rules of a home (a node's, a scene's, the experience's) */
export const rulesProblems = (rules: unknown, at: string, home: RuleHome): string[] => {
  if (!Array.isArray(rules)) return [`${at}: expected a list of rules { on, when?, do }`];
  return rules.flatMap((rule, i) => ruleProblems(rule, `${at}[${i}]`, home));
};

/** What a rule of this home listens to, in words */
const listenedTo = (home: RuleHome) =>
  home === "entity"
    ? `${BEHAVIOUR_NODE_EVENTS.join(", ")} or an event's name (camelCase)`
    : `an event of the bus – a scene event's name (camelCase) or a core event (${BEHAVIOUR_CORE_EVENTS.join(", ")})`;

const ruleProblems = (rule: unknown, at: string, home: RuleHome): string[] => {
  if (!isObject(rule)) return [`${at}: expected a rule { on, when?, do }`];
  const problems: string[] = [];
  Object.keys(rule).filter(key => !["on", "when", "do"].includes(key)).forEach(key => problems.push(`${at}.${key}: unknown field of a rule (on, when, do)`));
  if (typeof rule.on !== "string" || !BEHAVIOUR_NAME_PATTERN.test(rule.on)) {
    problems.push(`${at}.on: expected ${listenedTo(home)}, got ${show(rule.on)}`);
  } else if (home !== "entity" && isBehaviourNodeEvent(rule.on)) {
    problems.push(`${at}.on: ${rule.on} is a node's own event – a ${home}'s rule listens to ${listenedTo(home)}`);
  }
  if (rule.when !== undefined) problems.push(...conditionProblems(rule.when, `${at}.when`));
  if (rule.do === undefined) problems.push(`${at}.do: a rule needs an action or a list of actions`);
  else if (Array.isArray(rule.do)) rule.do.forEach((action, i) => problems.push(...actionProblems(action, `${at}.do[${i}]`, home)));
  else problems.push(...actionProblems(rule.do, `${at}.do`, home));
  return problems;
};

/** A condition: an expression (schema 5.0) – the object form of 2.19–4.x is named as such */
export const conditionProblems = (when: unknown, at: string): string[] => {
  if (isObject(when)) return [`${at}: a condition is an expression since content schema 5.0 (hasKey && tries > 2) – the object form ${show(when)} is gone`];
  if (typeof when !== "string") return [`${at}: expected an expression (hasKey && tries > 2), got ${show(when)}`];
  return parseExpression(when).problems.filter(problem => problem.severity === "error")
    .map(problem => `${at}: ${exprMessage(problem)} – ${show(when)}${when.length > 1 ? ` at ${problem.start + 1}` : ""}`);
};

const actionProblems = (action: unknown, at: string, home: RuleHome): string[] => {
  const ownNode = home === "entity";
  if (typeof action === "string") {
    if (!(BEHAVIOUR_NODE_ACTIONS as readonly string[]).includes(action)) {
      return [`${at}: expected an action – ${ownNode ? `${BEHAVIOUR_NODE_ACTIONS.join(", ")} or ` : ""}{ ${BEHAVIOUR_ACTIONS.join(" | ")}: … }, got ${show(action)}`];
    }
    return ownNode ? [] : [`${at}: ${action} alone acts on the rule's own node – a ${home}'s rule has none: { ${action}: target:<targetId>/<node name> }`];
  }
  if (!isObject(action)) return [`${at}: expected an action { ${BEHAVIOUR_ACTIONS.join(" | ")}: … }`];
  const verbs = Object.keys(action).filter(key => (BEHAVIOUR_ACTIONS as readonly string[]).includes(key));
  if (verbs.length !== 1) return [`${at}: an action has one of ${BEHAVIOUR_ACTIONS.join(", ")}, got ${verbs.length ? verbs.join(" and ") : "none"}`];
  const verb = verbs[0] as BehaviourVerb;
  const value = action[verb];
  const options = verb === "play" ? ["node"] : verb === "set" ? ["scope"] : [];
  const problems = Object.keys(action).filter(key => key !== verb && !options.includes(key))
    .map(key => `${at}.${key}: unknown field of ${verb}${options.length ? ` (${options.join(", ")})` : ""}`);
  const nodeName = (name: unknown, where: string) => {
    if (ownNode) {
      if (name !== undefined && typeof name !== "string") problems.push(`${where}: expected a node's name (empty = the behaviour's own node), got ${show(name)}`);
      return;
    }
    const problem = addressProblem(name, NODE_SCHEMES);
    if (problem) problems.push(`${where}: a ${home}'s rule names a node by address – ${problem}`);
  };
  switch (verb) {
    case "play":
      if (typeof value !== "string") problems.push(`${at}.play: expected a clip's name (empty = the node's own clip), got ${show(value)}`);
      nodeName(action.node, `${at}.node`);
      break;
    case "pause": case "stop": case "show": case "hide":
      nodeName(value === true ? undefined : value, `${at}.${verb}`);
      break;
    case "set":
      if (!isObject(value) || !Object.keys(value).length) {
        problems.push(`${at}.set: expected { variable: value, … }`);
      } else {
        Object.entries(value).forEach(([name, v]) => {
          if (!BEHAVIOUR_NAME_PATTERN.test(name)) problems.push(`${at}.set.${name}: not a variable name (camelCase letters and digits)`);
          if (typeof v === "string") {
            parseExpression(v).problems.filter(problem => problem.severity === "error")
              .forEach(problem => problems.push(`${at}.set.${name}: ${exprMessage(problem)} – ${show(v)} (a value is an expression; a text is written "…")`));
          } else if (v !== null && !isScalar(v)) problems.push(`${at}.set.${name}: expected an expression, a number, true / false or null (deletes), got ${show(v)}`);
        });
      }
      if (action.scope !== undefined && !(BEHAVIOUR_SCOPES as readonly string[]).includes(action.scope as string)) {
        problems.push(`${at}.scope: expected one of ${BEHAVIOUR_SCOPES.join(", ")}, got ${show(action.scope)}`);
      }
      break;
    case "emit":
      if (typeof value !== "string" || !BEHAVIOUR_NAME_PATTERN.test(value)) problems.push(`${at}.emit: expected an event's name (camelCase letters and digits), got ${show(value)}`);
      else if (isBehaviourCoreEvent(value) || isBehaviourNodeEvent(value)) problems.push(`${at}.emit: ${value} is the engine's own event – a rule listens to it, never emits it`);
      break;
    case "goto": {
      const problem = addressProblem(value, GOTO_SCHEMES);
      if (problem) problems.push(`${at}.goto: ${problem}`);
      break;
    }
  }
  return problems;
};

// ── Reading and writing ─────────────────────────────────────────────────────────────────────────

/** The rules of a behaviour component as the code works with them (a piece that does not check out is left out) */
export const behaviourRules = (component: Readonly<Record<string, unknown>> | undefined): BehaviourRule[] => readRules(component?.rules);

/** A list of rules in the content's form (a behaviour's, a scene's or the experience's `rules`) as the code works with them */
export const readRules = (rules: unknown): BehaviourRule[] => {
  if (!Array.isArray(rules)) return [];
  return rules.flatMap(rule => {
    if (!isObject(rule) || typeof rule.on !== "string") return [];
    const actions = (Array.isArray(rule.do) ? rule.do : rule.do === undefined ? [] : [rule.do]).flatMap(readAction);
    const when = typeof rule.when === "string" && rule.when.trim() ? rule.when.trim() : null;
    return [{ on: rule.on, when, do: actions }];
  });
};

const readAction = (raw: unknown): BehaviourAction[] => {
  if (typeof raw === "string") {
    if (raw === "play") return [{ action: "play", clip: "", node: "" }];
    if (raw === "pause" || raw === "stop" || raw === "show" || raw === "hide") return [{ action: raw, node: "" }];
    return [];
  }
  if (!isObject(raw)) return [];
  const text = (value: unknown) => (typeof value === "string" ? value : "");
  if ("play" in raw) return [{ action: "play", clip: text(raw.play), node: text(raw.node) }];
  for (const verb of ["pause", "stop", "show", "hide"] as const) {
    if (verb in raw) return [{ action: verb, node: text(raw[verb]) }];
  }
  if ("set" in raw && isObject(raw.set)) {
    const scope = (BEHAVIOUR_SCOPES as readonly string[]).includes(raw.scope as string) ? (raw.scope as BehaviourScope) : undefined;
    return Object.entries(raw.set).flatMap(([name, value]): BehaviourAction[] => {
      const source = value === null ? null : typeof value === "string" ? value.trim() : isScalar(value) ? showValue(value) : undefined;
      return source === undefined ? [] : [{ action: "set", name, value: source, ...(scope ? { scope } : {}) }];
    });
  }
  if ("emit" in raw && typeof raw.emit === "string") return [{ action: "emit", event: raw.emit }];
  if ("goto" in raw && typeof raw.goto === "string") return [{ action: "goto", address: raw.goto.trim() }];
  return [];
};

/** Rules in the content's shortest form – a behaviour's, a scene's or the experience's `rules` */
export const rulesData = (rules: readonly BehaviourRule[]): Record<string, unknown>[] =>
  rules.map(rule => {
    const actions = rule.do.map(writeAction);
    const when = rule.when?.trim();
    return {
      on: rule.on,
      ...(when ? { when } : {}),
      do: actions.length === 1 ? actions[0] : actions,
    };
  });

/** A behaviour component in the content's shortest form – what the editor saves */
export const behaviourData = (rules: readonly BehaviourRule[]): { type: typeof BEHAVIOUR_TYPE; rules: Record<string, unknown>[] } => ({
  type: BEHAVIOUR_TYPE,
  rules: rulesData(rules),
});

const writeAction = (action: BehaviourAction): unknown => {
  switch (action.action) {
    case "play":
      if (!action.clip && !action.node) return "play";
      return { play: action.clip, ...(action.node ? { node: action.node } : {}) };
    case "pause": case "stop": case "show": case "hide":
      return action.node ? { [action.action]: action.node } : action.action;
    case "set":
      return { set: { [action.name]: setValueData(action.value) }, ...(action.scope ? { scope: action.scope } : {}) };
    case "emit":
      return { emit: action.event };
    case "goto":
      return { goto: action.address };
  }
};

/** A set's expression in the content's shortest form: a plain number or true / false bare, anything else its source */
export const setValueData = (source: string | null): BehaviourValue | null => {
  if (source === null) return null;
  if (source === "true" || source === "false") return source === "true";
  if (/^-?(0|[1-9]\d*)(\.\d+)?$/.test(source)) return Number(source);
  return source;
};

/** The variables a set of rules reads (its conditions and set values) and writes, sorted (the editor offers them) */
export const behaviourVariables = (rules: readonly BehaviourRule[]): string[] =>
  [...new Set(rules.flatMap(rule => [
    ...(rule.when ? expressionVariables(rule.when) : []),
    ...rule.do.flatMap(action => (action.action === "set" ? [action.name, ...(action.value ? expressionVariables(action.value) : [])] : [])),
  ]))].sort();

/** The bus events a set of rules listens to and emits, sorted (the node's own events and an empty `on` left out) */
export const behaviourEvents = (rules: readonly BehaviourRule[]): string[] =>
  [...new Set(rules.flatMap(rule => [
    ...(!rule.on || isBehaviourNodeEvent(rule.on) ? [] : [rule.on]),
    ...rule.do.flatMap(action => (action.action === "emit" ? [action.event] : [])),
  ]))].sort();

/** The addresses a set of rules goes to (`goto`), each once in order */
export const behaviourGotos = (rules: readonly BehaviourRule[]): string[] =>
  [...new Set(rules.flatMap(rule => rule.do.flatMap(action => (action.action === "goto" ? [action.address] : []))))];

/** A rule's actions in words for a log line: `play rattle · set hasKey = true (experience) · emit keyRattled · goto scene:cover` */
export const describeActions = (actions: readonly BehaviourAction[]): string =>
  actions.map(action => {
    switch (action.action) {
      case "play": return `play ${action.clip || "its clip"}${action.node ? ` on ${action.node}` : ""}`;
      case "pause": case "stop": case "show": case "hide": return `${action.action}${action.node ? ` ${action.node}` : ""}`;
      case "set": return `set ${action.name} = ${action.value === null ? "unset" : action.value}${action.scope ? ` (${action.scope})` : ""}`;
      case "emit": return `emit ${action.event}`;
      case "goto": return `goto ${action.address}`;
    }
  }).join(" · ");
