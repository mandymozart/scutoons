import {
  BEHAVIOUR_CORE_EVENTS, BEHAVIOUR_TYPE, type BehaviourAction, type BehaviourRule, type BehaviourVerb,
  behaviourData, behaviourEvents, behaviourRules, behaviourVariables, componentProblems, type ComponentData, type EntityData,
} from "@engine/types";
import { namedPaths } from "@engine/scene/behaviour";
import { type Expr, exprMessage, expressionLiteral, expressionVariables, parseExpression } from "@shared/text/expressions";
import { sourceType, typeFits, type VariableInfo, type VariableType } from "@shared/types/variables";

/**
 * The Inspector's rule editor (`shared/types/behaviour.ts`) – the logic without the UI: the rules of a node's
 * `behaviour` component, a copy with one piece changed, new rules and actions with sensible defaults, the checks of a
 * condition and of a set's value (expressions, content schema 5.0 – B2, B8: refused with the reason before they are
 * saved), a condition with a term added or a variable's terms taken out (the Logic panel's `when` wires), and what the
 * pickers offer (the subtree's node names, the experience's variables and events). The component is written back in the
 * content's shortest form (`behaviourData`); a scene's and the experience's rules go through `logic-edit.svelte.ts`.
 */

/** The rules of the node's component at `index` (none when it is not a behaviour) */
export const rulesOf = (node: EntityData, index: number): BehaviourRule[] => {
  const component = node.components?.[index];
  return component?.type === BEHAVIOUR_TYPE ? behaviourRules(component) : [];
};

/** The node with the component at `index` holding `rules` */
export const withRules = (node: EntityData, index: number, rules: readonly BehaviourRule[]): EntityData => ({
  ...node,
  components: (node.components ?? []).map((component, i) => (i === index ? (behaviourData(rules) as ComponentData) : component)),
});

/** A copy of a list with the item at `index` replaced, or removed (`undefined`) */
export const replaceAt = <T>(list: readonly T[], index: number, item: T | undefined): T[] =>
  item === undefined ? list.filter((_, i) => i !== index) : list.map((current, i) => (i === index ? item : current));

/** A new rule: a node's – when the page is found; a scene's or the experience's – when the scene opens. Nothing to do yet */
export const newRule = (home: "entity" | "scene" | "experience" = "entity"): BehaviourRule =>
  ({ on: home === "entity" ? "found" : "sceneChanged", when: null, do: [] });

/** A variable's type by its name (declared or inferred, `shared/types/variables.ts`); undefined = untyped */
export type TypeOf = (name: string) => VariableType | undefined;

/** A new action of a verb; `previous` hands over what both have (the node, a set's variable) when the verb changes */
export const newAction = (verb: BehaviourVerb, previous?: BehaviourAction, variables: readonly string[] = [], typeOf: TypeOf = () => undefined): BehaviourAction => {
  const node = previous && "node" in previous ? previous.node : "";
  switch (verb) {
    case "play": return { action: "play", clip: "", node };
    case "pause": case "stop": case "show": case "hide": return { action: verb, node };
    case "set": {
      const name = previous?.action === "set" ? previous.name : variables[0] ?? "";
      return { action: "set", name, value: placeholderOf(typeOf(name)) };
    }
    case "emit": return { action: "emit", event: "" };
    case "goto": return { action: "goto", address: previous?.action === "goto" ? previous.address : "" };
  }
};

/** A rule with an action added (index past the end), changed or removed */
export const withAction = (rule: BehaviourRule, index: number, action: BehaviourAction | undefined): BehaviourRule => ({
  ...rule,
  do: index >= rule.do.length && action ? [...rule.do, action] : replaceAt(rule.do, index, action),
});

// ── Expressions: a rule's condition and a set's value (schema 5.0, B2, B8) ──────────────────────────

/** A typed placeholder value, as an expression: 0 for numbers, true, an empty text (`""`) */
export const placeholderOf = (type: VariableType | undefined): string =>
  type === "number" || type === "integer" ? "0" : type === "string" ? '""' : "true";

/** The literal a variable is compared with when a wire adds it to a condition: its declared default, else a typed placeholder */
export const literalOf = (info: Pick<VariableInfo, "type" | "default"> | undefined): string =>
  info?.default !== undefined ? expressionLiteral(info.default) : placeholderOf(info?.type);

/** A type in words: "a whole number", "a number", "true / false", "a text" */
export const typeWord = (type: VariableType): string =>
  type === "integer" ? "a whole number" : type === "number" ? "a number" : type === "boolean" ? "true / false" : "a text";

/** The parse problems of an expression in words (empty when it parses) */
const parseProblems = (source: string): string[] =>
  parseExpression(source).problems.filter(p => p.severity === "error").map(p => exprMessage(p));

/**
 * What is wrong with a condition (B8, checked before it is saved): it does not parse, it is ill-typed, or it is no
 * true / false. Empty is no problem – the rule always runs
 */
export const whenProblems = (source: string, typeOf: TypeOf): string[] => {
  if (!source.trim()) return [];
  const parse = parseProblems(source);
  if (parse.length) return parse;
  const { type, problems } = sourceType(source, typeOf);
  if (problems.length) return problems;
  return type === "boolean" || type === "unknown" ? [] : [`a condition is true / false – this is ${typeWord(type)}`];
};

/**
 * What is wrong with a set's value (B8): it does not parse, it is ill-typed, or it does not fit the variable's type.
 * Empty is no problem – the set deletes the variable
 */
export const setValueProblems = (source: string, name: string, typeOf: TypeOf): string[] => {
  if (!source.trim()) return [];
  const parse = parseProblems(source);
  if (parse.length) return parse.map(p => `${p} (a text is written in quotes: "calm")`);
  const { type, problems } = sourceType(source, typeOf);
  if (problems.length) return problems;
  const wanted = name ? typeOf(name) : undefined;
  return wanted && type !== "unknown" && !typeFits(wanted, type) ? [`${name} is ${typeWord(wanted)} – this is ${typeWord(type)}`] : [];
};

/** A condition with a term added (joined with `&&`); none before: the term alone */
export const appendCondition = (when: string | null, term: string): string => {
  const current = when?.trim();
  if (!current) return term;
  const { expr } = parseExpression(current);
  // An `||` chain keeps its meaning: it goes in parentheses
  const wrapped = expr?.type === "binary" && expr.op === "||" ? `(${current})` : current;
  return `${wrapped} && ${term}`;
};

/**
 * A condition without the terms that read `name`, when it is a top-level `&&` or `||` chain (or a term of the variable
 * alone): the rest (null = always), or `refused` when the variable sits deeper – the Inspector edits it then
 */
export const removeConditionTerms = (when: string | null, name: string): { when: string | null } | { refused: true } => {
  const source = when?.trim() ?? "";
  if (!source) return { when: null };
  const { expr } = parseExpression(source);
  if (!expr) return { refused: true };
  const all = expressionVariables(source);
  if (!all.includes(name)) return { when: source };
  const op = expr.type === "binary" && (expr.op === "&&" || expr.op === "||") ? expr.op : null;
  if (!op) return all.length === 1 ? { when: null } : { refused: true };
  const operands: Expr[] = [];
  const flatten = (e: Expr) => {
    if (e.type === "binary" && e.op === op) {
      flatten(e.left);
      flatten(e.right);
    } else operands.push(e);
  };
  flatten(expr);
  const text = (e: Expr) => {
    const raw = source.slice(e.start, e.end);
    return e.type === "binary" && (e.op === "&&" || e.op === "||") && !raw.startsWith("(") ? `(${raw})` : raw;
  };
  const kept: string[] = [];
  for (const operand of operands) {
    const names = expressionVariables(text(operand));
    if (!names.includes(name)) kept.push(text(operand));
    else if (names.length > 1) return { refused: true };
  }
  return { when: kept.length ? kept.join(` ${op} `) : null };
};

// ── What the pickers offer ──────────────────────────────────────────────────────────────────────────

/**
 * The names of the nodes under a node (itself included), in tree order, each once – what show, hide and play may name
 * (the engine's `namedPaths`: an action reaches the first node of a name)
 */
export const subtreeNames = (node: EntityData): string[] => [...namedPaths(node).keys()];

/** The path (relative to `node`) of the node a name stands for, as the engine finds it: empty = the node itself; null when no node has it */
export const pathOfName = (node: EntityData, name: string): number[] | null => {
  if (!name) return [];
  const path = namedPaths(node).get(name);
  if (path === undefined) return null;
  return path ? path.split(".").map(Number) : [];
};

/** Every node of a set of trees, depth first */
const allNodes = (trees: readonly EntityData[]): EntityData[] => trees.flatMap(tree => [tree, ...allNodes(tree.children ?? [])]);

/** Every behaviour rule of a set of trees */
const allRules = (trees: readonly EntityData[]): BehaviourRule[] =>
  allNodes(trees).flatMap(node => (node.components ?? []).filter(c => c.type === BEHAVIOUR_TYPE).flatMap(c => behaviourRules(c)));

/**
 * The variables the experience uses: its behaviours' (read and written), the scenes' and the experience's rules
 * (`rules`), the declared ones and the ones live on the stage (a plugin's too), sorted
 */
export const experienceVariables = (trees: readonly EntityData[], live: readonly string[] = [], rules: readonly BehaviourRule[] = []): string[] =>
  [...new Set([...behaviourVariables([...allRules(trees), ...rules]), ...live])].sort();

/**
 * The events a rule may listen to besides the node's own: the scene events the experience's behaviours and its scene and
 * experience rules (`rules`) listen to and emit, the ones seen on the stage's timeline (a plugin's too), then the engine's core events
 */
export const experienceEvents = (trees: readonly EntityData[], seen: readonly string[] = [], rules: readonly BehaviourRule[] = []): string[] => {
  const core = BEHAVIOUR_CORE_EVENTS as readonly string[];
  const scene = [...new Set([...behaviourEvents([...allRules(trees), ...rules]), ...seen])]
    .filter(event => !core.includes(event) && event !== BEHAVIOUR_TYPE && event !== "goto").sort();
  return [...scene, ...core];
};

/** What is wrong with a node's behaviour components – the panel shows it, and the tree is not saved until it is fixed */
export const behaviourIssues = (node: EntityData): string[] =>
  (node.components ?? []).flatMap((component, i) =>
    component.type === BEHAVIOUR_TYPE
      ? componentProblems([component], "behaviour").map(problem =>
        problem.replace(/^behaviour\[0\]\.rules\[(\d+)\]\.?/, (_, n: string) => `rule ${Number(n) + 1} – `).replace(/^behaviour\[0\]\.?/, `component ${i + 1} – `))
      : []);

/** The issues of every node of a tree (a save needs the whole tree complete) */
export const treeBehaviourIssues = (tree: EntityData): string[] => allNodes([tree]).flatMap(behaviourIssues);
