/**
 * The checks every part of the contract builds on. A leaf: it imports nothing, so every module of `shared/` (and the
 * packages that read raw JSON) can use it without an import cycle.
 */

/** A JSON object: not null, not an array */
export const isObject = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

/** A camelCase name of at most `maxLength` characters: a lower-case letter, then letters and digits */
export const camelCaseName = (maxLength: number): RegExp => new RegExp(`^[a-z][a-zA-Z0-9]{0,${maxLength - 1}}$`);

/**
 * The name a type, a parameter, a variable or an event takes: camelCase letters and digits, at most 40 – a component's,
 * a pass's, a material's and a procedural texture's type, a spec's parameter, a behaviour's variable and event. A
 * plugin's node type is shorter (`NODE_TYPE_PATTERN`). The PHP API repeats it (`Books::PLUGIN_PASS_TYPE`,
 * `Editor::MATERIAL_TYPE`).
 */
export const CAMEL_CASE_NAME = camelCaseName(40);
