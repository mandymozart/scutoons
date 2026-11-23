/**
 * Materials and procedural textures a plugin registers (content schema 2.17, plugin API 1.4 – `api.registerMaterial`,
 * `api.registerTexture` in `engine/src/scene/plugins.ts`). A **material** replaces the material of a node's meshes (a
 * model's, an image's or video's plane, a plugin node's): the plugin builds the three.js material itself (a
 * `ShaderMaterial` with its own GLSL, a `MeshStandardMaterial` with a texture); a node's material layers still add on
 * top. The node names it with its values next to its other fields:
 *
 *   type: model
 *   src: tank.glb
 *   material:
 *     type: water
 *     params:
 *       depthColor: "#0b4f6c"
 *       caustics: caustics        # a texture parameter: a procedural texture of a plugin, or a file
 *
 * A **procedural texture** is drawn by code (a canvas or a data texture, animated or not) and used wherever a spec has
 * a `texture` parameter (`./components.ts` – a material's, a layer's, a component's, a node type's). What the editor,
 * the content build and the guard must know is the **spec**, as for components and node types: one registry each,
 * filled by the engine when a plugin registers, by the content build when it records a plugin, and by the editor from
 * the recorded specs (`PluginData.registered.materials` / `.textures`).
 */

import { paramProblem, paramSpecProblems, specLogicProblems, type ComponentParamSpec, type SpecLogic } from "./components";
import { CAMEL_CASE_NAME, isObject } from "../guards/primitives";

/** A material's or a procedural texture's type: `camelCase`, letters and digits */
export const MATERIAL_TYPE_PATTERN = CAMEL_CASE_NAME;

/** Schema 5.1: `actions`, `events`, `uses` (`SpecLogic`) as a component's */
export interface MaterialSpec extends SpecLogic {
  description: string;
  /** The node types it applies to (none = every node with meshes) – built-in types or a plugin's */
  nodeTypes?: readonly string[];
  /** Its parameters, set in the node's `material.params` */
  params: Record<string, ComponentParamSpec>;
}

export interface TextureSpec extends SpecLogic {
  description: string;
  /** Its parameters, set in a texture value `{ type, …parameters }` (a plain type takes the defaults) */
  params: Record<string, ComponentParamSpec>;
}

/** A node's material as the content holds it (schema 2.17): the type a plugin registered and the values set */
export interface MaterialData {
  type: string;
  params?: Record<string, unknown>;
}

const materials = new Map<string, MaterialSpec>();
const textures = new Map<string, TextureSpec>();

/** A plugin's material joins the registry the panel, the build and the guard read */
export const registerMaterialSpec = (type: string, spec: MaterialSpec): void => {
  materials.set(type, spec);
};
export const unregisterMaterialSpec = (type: string): void => {
  materials.delete(type);
};
export const materialSpec = (type: string): MaterialSpec | undefined => materials.get(type);
/** The material types plugins registered right now */
export const materialTypes = (): string[] => [...materials.keys()];

/** A plugin's procedural texture joins the registry */
export const registerTextureSpec = (type: string, spec: TextureSpec): void => {
  textures.set(type, spec);
};
export const unregisterTextureSpec = (type: string): void => {
  textures.delete(type);
};
export const textureSpec = (type: string): TextureSpec | undefined => textures.get(type);
/** The procedural texture types plugins registered right now */
export const textureTypes = (): string[] => [...textures.keys()];

/** The material types a node of `type` may carry */
export const materialTypesFor = (nodeType: string): string[] =>
  materialTypes().filter(type => {
    const nodeTypes = materials.get(type)?.nodeTypes;
    return !nodeTypes || nodeTypes.includes(nodeType);
  });

const specProblems = (what: string, type: string, spec: unknown, nodeTypes: boolean): string[] => {
  const problems: string[] = [];
  if (!MATERIAL_TYPE_PATTERN.test(type)) problems.push(`"${type}" is not a ${what} type (camelCase letters and digits, e.g. water)`);
  if (!isObject(spec)) return [...problems, `${type}: the spec must be an object with description and params`];
  if (typeof spec.description !== "string") problems.push(`${type}: description must be a string`);
  if (nodeTypes && spec.nodeTypes !== undefined && !(Array.isArray(spec.nodeTypes) && spec.nodeTypes.every(t => typeof t === "string"))) {
    problems.push(`${type}: nodeTypes must be a list of node types`);
  }
  problems.push(...specLogicProblems(type, spec));
  if (!isObject(spec.params)) return [...problems, `${type}: params must be an object`];
  return [...problems, ...paramSpecProblems(type, spec.params)];
};

/** Problems of a material spec a plugin registers (the engine and the content build check it before the registry takes it) */
export const materialSpecProblems = (type: string, spec: unknown): string[] => specProblems("material", type, spec, true);

/** Problems of a procedural texture's spec; a texture's own parameters cannot be textures */
export const textureSpecProblems = (type: string, spec: unknown): string[] => {
  const problems = specProblems("texture", type, spec, false);
  if (isObject(spec) && isObject(spec.params)) {
    Object.entries(spec.params).forEach(([name, param]) => {
      if (isObject(param) && param.kind === "texture") problems.push(`${type}.${name}: a procedural texture's parameter cannot be a texture`);
    });
  }
  return problems;
};

export interface MaterialCheckOptions {
  /** A type the registry does not know: refused (the default), or allowed with its values unchecked (a plugin registers it at load) */
  unknownTypes?: "refuse" | "allow";
  /** Specs beyond the registry's – the ones the declared plugins recorded (`PluginData.registered.materials`) */
  specs?: ReadonlyMap<string, MaterialSpec>;
}

/** Problems of a node's `material` (the content build, the guard); `nodeType` checks that it applies */
export const materialProblems = (raw: unknown, path: string, nodeType?: string, options: MaterialCheckOptions = {}): string[] => {
  if (!isObject(raw)) return [`${path}: expected { type, params? }`];
  const { type, params, ...other } = raw;
  const problems = Object.keys(other).map(key => `${path}.${key}: a material has a type and params only`);
  if (typeof type !== "string" || !MATERIAL_TYPE_PATTERN.test(type)) return [...problems, `${path}.type: expected a material type (camelCase), got ${JSON.stringify(type)}`];
  if (params !== undefined && !isObject(params)) problems.push(`${path}.params: expected an object`);
  const spec = materialSpec(type) ?? options.specs?.get(type);
  if (!spec) {
    if (options.unknownTypes === "allow") return problems;
    const known = [...new Set([...materialTypes(), ...(options.specs?.keys() ?? [])])];
    return [...problems, `${path}.type: "${type}" is not a material ${known.length ? `of the experience's plugins (${known.join(", ")})` : "– the experience's plugins register none"}`];
  }
  if (nodeType && spec.nodeTypes && !spec.nodeTypes.includes(nodeType)) problems.push(`${path}: ${type} applies to ${spec.nodeTypes.join(", ")} nodes, not to a ${nodeType}`);
  if (isObject(params)) {
    for (const [name, value] of Object.entries(params)) {
      const param = spec.params[name];
      if (!param) {
        problems.push(`${path}.params.${name}: unknown parameter of ${type} (${Object.keys(spec.params).join(", ")})`);
        continue;
      }
      const problem = paramProblem(param, value);
      if (problem) problems.push(`${path}.params.${name}: ${problem}`);
    }
  }
  return problems;
};

/** A spec's parameters resolved: every one its value in `params` or the spec's default */
export const resolveSpecParams = (specParams: Record<string, ComponentParamSpec>, params: Record<string, unknown> | undefined): Record<string, unknown> => {
  const resolved: Record<string, unknown> = {};
  for (const [name, param] of Object.entries(specParams)) resolved[name] = params?.[name] !== undefined ? params[name] : param.default;
  return resolved;
};
