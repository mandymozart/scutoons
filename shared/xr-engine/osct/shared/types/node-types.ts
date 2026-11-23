/**
 * Node types a plugin registers (content schema 2.13, plugin API 1.2 – `api.registerNodeType` in
 * `engine/src/scene/plugins.ts`): the plugin builds the node's three.js object itself, the engine places it like
 * every node (placement, children, components, layers). What the editor and the content build must know is the
 * **spec**: the parameters (the kinds of `./components.ts`, so the Entity panel shows rows) and which file the node's
 * `src` takes. The values live in the node's `params`, next to the placement:
 *
 *   type: sparkle
 *   params:
 *     position: [0, 0.2, 0]
 *     count: 120
 *     color: "#f3cc94"
 *
 * `ENTITY_TYPES` stays the built-in list. A registered type joins one registry the build, the guard and the editor
 * read (`nodeTypeSpec`) – like a plugin's component (`registerComponentSpec`). A type nobody registered is refused by
 * the build; the guard lets it pass (a later type, or the plugin registers it when it loads).
 */

import { ENTITY_TYPES, type AssetType, type EntityType } from "./game-config";
import { paramProblem, paramSpecProblems, specLogicProblems, type ComponentParamSpec, type SpecLogic } from "./components";
import { LIGHT_PARAMS } from "./lights";
import { PLACEMENT_KEYS } from "./placement";
import { camelCaseName, isObject } from "../guards/primitives";

/** A plugin's node type: `camelCase`, letters and digits, at most 32 (the editor API's `entities.type` column) */
export const NODE_TYPE_PATTERN = camelCaseName(32);

/** Which file a node type's `src` takes: a model (glb / gltf), a video, an image, or none */
export const NODE_SRC_KINDS = ["none", "glb", "video", "image"] as const;
export type NodeSrcKind = (typeof NODE_SRC_KINDS)[number];

/** The asset types of each src kind */
export const NODE_SRC_ASSET_TYPES: Record<NodeSrcKind, readonly AssetType[]> = {
  none: [],
  glb: ["glb", "gltf"],
  video: ["video"],
  image: ["image"],
};

/** The file the built-in node types take (none: group, text, light) */
const BUILT_IN_SRC: Partial<Record<EntityType, NodeSrcKind>> = { model: "glb", video: "video", image: "image" };

/** A node type's spec; schema 5.1 adds `actions`, `events` and `uses` (`SpecLogic`, the Logic view's ports) */
export interface NodeTypeSpec extends SpecLogic {
  description: string;
  /** The file its `src` takes (default `none`: the node has no file) */
  src?: NodeSrcKind;
  /** Its parameters, set in the node's `params` (never `position`, `rotation` or `scale` – the placement's) */
  params: Record<string, ComponentParamSpec>;
}

/**
 * The built-in node types as specs (schema 5.1, #334): what the editor's Logic view draws as a node's ports – its file
 * (`src`: a media inlet), the values the node holds as parameters (a text's line and colour, a light's `LIGHT_PARAMS`) and
 * the rule verbs the engine answers on it (`actions`: `show` / `hide` on every node, `play` / `pause` / `stop` on a model's
 * clips – `engine/src/scene/behaviour.ts`). No events: a node's own `found`, `lost`, `tap` and `unlocked` are its rules'.
 * A text's `text` and `color` sit on the node itself (not in `params`); the Entity panel keeps its own rows for them.
 */
export const BUILT_IN_NODE_SPECS = {
  group: { description: "Holds child nodes; nothing of its own is seen.", params: {}, actions: ["show", "hide"] },
  model: { description: "A glTF model; a rule plays, pauses or stops its clip.", src: "glb", params: {}, actions: ["play", "pause", "stop", "show", "hide"] },
  video: { description: "A video plane: it plays when the target is found, a tap pauses or plays it.", src: "video", params: {}, actions: ["show", "hide"] },
  image: { description: "An image plane.", src: "image", params: {}, actions: ["show", "hide"] },
  text: {
    description: "A line of text.",
    params: {
      text: { kind: "string", default: "", label: "text", description: "The line shown; {{entry.title}} and {{experience.title}} are replaced." },
      color: { kind: "color", default: "#ffffff", label: "color", description: "The text's colour." },
    },
    actions: ["show", "hide"],
  },
  light: { description: "A light.", params: LIGHT_PARAMS, actions: ["show", "hide"] },
} as const satisfies Record<EntityType, NodeTypeSpec>;

/** The spec of any node type: a built-in one's (`BUILT_IN_NODE_SPECS`) or a plugin's from the registry */
export const anyNodeTypeSpec = (type: string): NodeTypeSpec | undefined =>
  (BUILT_IN_NODE_SPECS as Record<string, NodeTypeSpec>)[type] ?? registered.get(type);

/** Node types plugins registered (type → spec) */
const registered = new Map<string, NodeTypeSpec>();

/** A plugin's node type joins the registry the panel, the build and the guard read */
export const registerNodeTypeSpec = (type: string, spec: NodeTypeSpec): void => {
  registered.set(type, spec);
};

export const unregisterNodeTypeSpec = (type: string): void => {
  registered.delete(type);
};

/** The spec of a plugin's node type, or undefined (a built-in type has none) */
export const nodeTypeSpec = (type: string): NodeTypeSpec | undefined => registered.get(type);

/** The node types plugins registered right now */
export const pluginNodeTypes = (): string[] => [...registered.keys()];

/** Every node type known right now: the built-in ones, then the registered ones */
export const nodeTypes = (): string[] => [...ENTITY_TYPES, ...pluginNodeTypes()];

/** A built-in type or a registered one */
export const isKnownNodeType = (value: unknown): value is string =>
  typeof value === "string" && ((ENTITY_TYPES as readonly string[]).includes(value) || registered.has(value));

/** The asset types a node of this type takes (empty: no file); undefined for a type nobody registered */
export const nodeAssetTypes = (type: string): readonly AssetType[] | undefined => {
  if ((ENTITY_TYPES as readonly string[]).includes(type)) return NODE_SRC_ASSET_TYPES[BUILT_IN_SRC[type as EntityType] ?? "none"];
  const spec = registered.get(type);
  return spec ? NODE_SRC_ASSET_TYPES[spec.src ?? "none"] : undefined;
};

/** Problems of a node type spec a plugin registers (the engine and the content build check it before the registry takes it) */
export const nodeTypeSpecProblems = (type: string, spec: unknown): string[] => {
  const problems: string[] = [];
  if (!NODE_TYPE_PATTERN.test(type)) problems.push(`"${type}" is not a node type (camelCase letters and digits, e.g. sparkle)`);
  if ((ENTITY_TYPES as readonly string[]).includes(type)) problems.push(`"${type}" is a built-in node type`);
  if (typeof spec !== "object" || spec === null) return [...problems, `${type}: the spec must be an object with description and params`];
  const s = spec as Record<string, unknown>;
  if (typeof s.description !== "string") problems.push(`${type}: description must be a string`);
  if (s.src !== undefined && !(NODE_SRC_KINDS as readonly unknown[]).includes(s.src)) problems.push(`${type}: src must be one of ${NODE_SRC_KINDS.join(", ")}`);
  problems.push(...specLogicProblems(type, s));
  if (!isObject(s.params)) return [...problems, `${type}: params must be an object`];
  const params = s.params;
  (PLACEMENT_KEYS as readonly string[]).filter(key => key in params).forEach(key => problems.push(`${type}.${key}: the placement's name – a node type's parameter needs another`));
  return [...problems, ...paramSpecProblems(type, params)];
};

/** Problems of a plugin node's `params` against its spec: unknown names, wrong kinds (the placement keys are checked apart) */
export const nodeParamProblems = (spec: NodeTypeSpec, params: unknown, path: string, type: string): string[] => {
  if (!isObject(params)) return [];
  const problems: string[] = [];
  for (const [name, value] of Object.entries(params)) {
    if ((PLACEMENT_KEYS as readonly string[]).includes(name)) continue;
    const param = spec.params[name];
    if (!param) {
      problems.push(`${path}.${name}: unknown parameter of ${type} (${[...PLACEMENT_KEYS, ...Object.keys(spec.params)].join(", ")})`);
      continue;
    }
    const problem = paramProblem(param, value);
    if (problem) problems.push(`${path}.${name}: ${problem}`);
  }
  return problems;
};

/** A plugin node's parameters with every one resolved to its value in `params` or the spec's default */
export const resolveNodeParams = (spec: NodeTypeSpec, params: Record<string, unknown> | undefined): Record<string, unknown> => {
  const resolved: Record<string, unknown> = {};
  for (const [name, param] of Object.entries(spec.params)) {
    resolved[name] = params?.[name] !== undefined ? params[name] : param.default;
  }
  return resolved;
};
