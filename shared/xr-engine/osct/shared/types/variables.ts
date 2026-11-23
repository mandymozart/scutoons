/**
 * Typed variables (content schema 5.0, #334 decision B7, agents/research/2026-11-20-logic-basic-nodes.md §5): every
 * variable of the rules has a type – `number`, `integer`, `boolean` or `string`. A variable is **declared** in the
 * experience (`variables`, a map name → `{ type, scope?, default? }`) or **inferred** from its first `set` in the
 * content (the experience's rules, then the scenes' in order, then the shared entities', then the targets' in entry order).
 *
 *   variables:
 *     tries: { type: integer, scope: stage, default: 0 }
 *     mood: { type: string, default: calm }
 *
 * A `set` of another type, a comparison of values of different types and a `when` that is no boolean are problems (the
 * guard, the content build, the editor). Expressions are typed with the parser's tree (`expressionType`): a comparison
 * gives a boolean; `&&`, `||` and `!` need booleans; `-`, `*`, `/`, `%`, `<` … need numbers; `+` takes numbers or texts,
 * never both. An integer goes where a number is wanted (lossless); nothing else is converted. The engine refuses a value of
 * the wrong type at runtime – also one a plugin sets through `api.vars.set` – and keeps the old one (a console warning).
 * A declared variable that is not set reads as its `default`. Variables no rule sets and nobody declares (a plugin's own)
 * stay untyped.
 */

import { isObject } from "../guards/primitives";
import { type Expr, parseExpression } from "../text/expressions";
import { BEHAVIOUR_NAME_PATTERN, BEHAVIOUR_SCOPES, BEHAVIOUR_TYPE, type BehaviourRule, type BehaviourScope, readRules } from "./behaviour";

export const VARIABLE_TYPES = ["number", "integer", "boolean", "string"] as const;
export type VariableType = (typeof VARIABLE_TYPES)[number];

/** A declared variable (`ExperienceData.variables`); `scope`: where a `set` without its own scope writes it */
export interface VariableDeclaration {
  type: VariableType;
  scope?: BehaviourScope;
  default?: string | number | boolean;
}

/** What the checks and the engine know of a variable: its type, and its scope and default when declared */
export interface VariableInfo extends VariableDeclaration {
  /** Where the type comes from: `experience.variables` or the first set (its path) */
  from: string;
}

const show = (value: unknown) => JSON.stringify(value);

/** The type of a JSON scalar (null for anything else) */
export const valueType = (value: unknown): VariableType | null =>
  typeof value === "boolean" ? "boolean"
    : typeof value === "string" ? "string"
      : typeof value === "number" && Number.isFinite(value) ? (Number.isInteger(value) ? "integer" : "number")
        : null;

/** Whether a value of type `actual` may go where `wanted` is asked: the same type, or an integer as a number */
export const typeFits = (wanted: VariableType, actual: VariableType): boolean => wanted === actual || (wanted === "number" && actual === "integer");

/** Whether a value fits a type */
export const valueFits = (type: VariableType, value: unknown): boolean => {
  const actual = valueType(value);
  return actual !== null && typeFits(type, actual);
};

/** Problems of `experience.variables` */
export const variablesProblems = (variables: unknown, at: string): string[] => {
  if (!isObject(variables)) return [`${at}: expected { name: { type, scope?, default? }, … }`];
  return Object.entries(variables).flatMap(([name, declaration]) => {
    const path = `${at}.${name}`;
    if (!BEHAVIOUR_NAME_PATTERN.test(name)) return [`${path}: not a variable name (camelCase letters and digits)`];
    if (!isObject(declaration)) return [`${path}: expected { type, scope?, default? }`];
    const problems = Object.keys(declaration).filter(key => !["type", "scope", "default"].includes(key)).map(key => `${path}.${key}: unknown field of a variable (type, scope, default)`);
    if (!(VARIABLE_TYPES as readonly unknown[]).includes(declaration.type)) problems.push(`${path}.type: expected ${VARIABLE_TYPES.join(", ")}, got ${show(declaration.type)}`);
    else if (declaration.default !== undefined && !valueFits(declaration.type as VariableType, declaration.default)) {
      problems.push(`${path}.default: ${show(declaration.default)} is no ${declaration.type}`);
    }
    if (declaration.scope !== undefined && !(BEHAVIOUR_SCOPES as readonly unknown[]).includes(declaration.scope)) {
      problems.push(`${path}.scope: expected one of ${BEHAVIOUR_SCOPES.join(", ")}, got ${show(declaration.scope)}`);
    }
    return problems;
  });
};

// ── Typing expressions ──────────────────────────────────────────────────────────────────────────

/** An expression's type; `unknown` = a variable nobody types (a plugin's), so nothing is claimed about it */
export type ExprType = VariableType | "unknown";

const numeric = (type: ExprType) => type === "number" || type === "integer" || type === "unknown";

/**
 * The type of a parsed expression with the variables' types (`typeOf(name)`), and why it is ill-typed. Unknown parts are
 * let through (a plugin's variable); a function call is unknown (the rules have none – the parser's problem).
 */
export const expressionType = (expr: Expr, typeOf: (name: string) => VariableType | undefined): { type: ExprType; problems: string[] } => {
  const problems: string[] = [];
  const word = (type: ExprType) => (type === "integer" ? "a whole number" : type === "number" ? "a number" : type === "boolean" ? "true / false" : "a text");
  const walk = (e: Expr): ExprType => {
    switch (e.type) {
      case "num": return Number.isInteger(e.value) ? "integer" : "number";
      case "str": return "string";
      case "bool": return "boolean";
      case "name": return (e.path.length === 1 && typeOf(e.path[0])) || "unknown";
      case "call": e.args.forEach(walk); return "unknown";
      case "unary": {
        const arg = walk(e.arg);
        if (e.op === "!") {
          if (arg !== "boolean" && arg !== "unknown") problems.push(`! needs true / false, not ${word(arg)}`);
          return "boolean";
        }
        if (!numeric(arg)) problems.push(`- needs a number, not ${word(arg)}`);
        return arg === "integer" ? "integer" : arg === "unknown" ? "unknown" : "number";
      }
      case "binary": {
        const a = walk(e.left);
        const b = walk(e.right);
        const op = e.op;
        if (op === "&&" || op === "||") {
          for (const side of [a, b]) if (side !== "boolean" && side !== "unknown") problems.push(`${op} needs true / false on both sides, not ${word(side)}`);
          return "boolean";
        }
        if (op === "==" || op === "!=") {
          const family = (t: ExprType) => (t === "integer" ? "number" : t);
          if (a !== "unknown" && b !== "unknown" && family(a) !== family(b)) problems.push(`${op} compares ${word(a)} with ${word(b)}`);
          return "boolean";
        }
        if (op === "<" || op === "<=" || op === ">" || op === ">=") {
          for (const side of [a, b]) if (!numeric(side)) problems.push(`${op} compares numbers, not ${word(side)}`);
          return "boolean";
        }
        if (op === "+" && (a === "string" || b === "string")) {
          if (a !== b && a !== "unknown" && b !== "unknown") problems.push(`+ joins two texts or adds two numbers – not ${word(a)} and ${word(b)}`);
          return "string";
        }
        for (const side of [a, b]) if (!numeric(side)) problems.push(`${op} needs numbers, not ${word(side)}`);
        if (a === "unknown" || b === "unknown") return "unknown";
        return op !== "/" && a === "integer" && b === "integer" ? "integer" : "number";
      }
    }
  };
  const type = walk(expr);
  return { type, problems };
};

/** The type of an expression's source (`unknown` when it does not parse – the parse problem is named elsewhere) */
export const sourceType = (source: string, typeOf: (name: string) => VariableType | undefined): { type: ExprType; problems: string[] } => {
  const { expr } = parseExpression(source);
  return expr ? expressionType(expr, typeOf) : { type: "unknown", problems: [] };
};

// ── Declared and inferred, and the rules checked against them ───────────────────────────────────

/** A list of rules with where it lives (its path – `experience.logic.rules`, `scenes[0].logic.rules` …) */
export interface RuleList {
  rules: BehaviourRule[];
  path: string;
}

/**
 * Every variable's type: the declarations, then each undeclared variable's first `set` (its value's expression typed
 * with what is known so far – a set whose type is unknown infers nothing)
 */
export const variableTypes = (declarations: unknown, lists: readonly RuleList[]): Map<string, VariableInfo> => {
  const known = new Map<string, VariableInfo>();
  if (isObject(declarations)) {
    for (const [name, declaration] of Object.entries(declarations)) {
      if (!isObject(declaration) || !(VARIABLE_TYPES as readonly unknown[]).includes(declaration.type)) continue;
      known.set(name, {
        type: declaration.type as VariableType,
        ...((BEHAVIOUR_SCOPES as readonly unknown[]).includes(declaration.scope) ? { scope: declaration.scope as BehaviourScope } : {}),
        ...(valueType(declaration.default) ? { default: declaration.default as string | number | boolean } : {}),
        from: `experience.variables.${name}`,
      });
    }
  }
  const typeOf = (name: string) => known.get(name)?.type;
  lists.forEach(({ rules, path }) => rules.forEach((rule, i) => rule.do.forEach((action, j) => {
    if (action.action !== "set" || action.value === null || known.has(action.name)) return;
    const { type } = sourceType(action.value, typeOf);
    if (type !== "unknown") known.set(action.name, { type, from: `${path}[${i}].do[${j}]` });
  })));
  return known;
};

/** The type problems of rules: a `when` that is no boolean, an ill-typed expression, a set of another type or scope */
export const ruleTypeProblems = (lists: readonly RuleList[], types: ReadonlyMap<string, VariableInfo>): string[] => {
  const typeOf = (name: string) => types.get(name)?.type;
  const problems: string[] = [];
  lists.forEach(({ rules, path }) => rules.forEach((rule, i) => {
    const at = `${path}[${i}]`;
    if (rule.when !== null) {
      const typed = sourceType(rule.when, typeOf);
      typed.problems.forEach(problem => problems.push(`${at}.when: ${problem} – ${show(rule.when)}`));
      if (typed.type !== "boolean" && typed.type !== "unknown") problems.push(`${at}.when: a condition is true / false – ${show(rule.when)} is ${typed.type === "string" ? "a text" : "a number"}`);
    }
    rule.do.forEach((action, j) => {
      if (action.action !== "set") return;
      const where = `${at}.do[${j}].set.${action.name}`;
      const info = types.get(action.name);
      if (info?.scope && action.scope && action.scope !== info.scope) problems.push(`${where}: ${action.name} lives in scope ${info.scope} (${info.from}), not ${action.scope}`);
      if (action.value === null) return;
      const typed = sourceType(action.value, typeOf);
      typed.problems.forEach(problem => problems.push(`${where}: ${problem} – ${show(action.value)}`));
      if (info && typed.type !== "unknown" && !typeFits(info.type, typed.type)) {
        problems.push(`${where}: ${action.name} is ${info.type === "integer" ? "an integer" : `a ${info.type}`} (${info.from}) – ${show(action.value)} is ${typed.type === "integer" ? "an integer" : `a ${typed.type}`}`);
      }
    });
  }));
  return problems;
};

/**
 * Every list of rules of a configuration in content order – the experience's, the scenes', the shared entities', the
 * targets' inline entities' (each node's behaviour, depth first). The engine infers the variables' types from it as the
 * guard does.
 */
export const contentRuleLists = (config: unknown): RuleList[] => {
  const lists: RuleList[] = [];
  if (!isObject(config)) return lists;
  const logic = (holder: unknown, path: string) => {
    if (isObject(holder) && isObject(holder.logic)) lists.push({ rules: readRules(holder.logic.rules), path: `${path}.logic.rules` });
  };
  const node = (value: unknown, path: string) => {
    if (!isObject(value)) return;
    if (Array.isArray(value.components)) value.components.forEach((component, i) => {
      if (isObject(component) && component.type === BEHAVIOUR_TYPE) lists.push({ rules: readRules(component.rules), path: `${path}.components[${i}].rules` });
    });
    if (Array.isArray(value.children)) value.children.forEach((child, i) => node(child, `${path}.children[${i}]`));
  };
  logic(config.experience, "experience");
  if (Array.isArray(config.scenes)) config.scenes.forEach((scene, i) => logic(scene, `scenes[${i}]`));
  if (isObject(config.entities)) Object.entries(config.entities).forEach(([id, entity]) => node(entity, `entities.${id}`));
  if (Array.isArray(config.entries)) config.entries.forEach((entry, i) => {
    const entity = isObject(entry) && isObject(entry.target) ? entry.target.entity : undefined;
    if (isObject(entity) && typeof entity.ref !== "string") node(entity, `entries[${i}].target.entity`);
  });
  return lists;
};
