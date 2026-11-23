import {
  BEHAVIOUR_TYPE, BUILT_IN_NODE_SPECS, COMPONENTS, isMoment, paramLabel,
  type ComponentParamSpec, type EntityData, type NodeTypeSpec, type PluginData, type SpecLogic,
} from "@engine/types";
import type { PortSpec, PortType, Signal } from "./types";

/**
 * **Ports from the specs** (#334 L-T6, research §5.9 – reflection, as Unreal, Houdini, Blender and ComfyUI do it): a node's
 * box shows what its specs say, nothing written by hand per component. Its node type's spec (`BUILT_IN_NODE_SPECS` for the
 * built-in types, a plugin's recorded `registered.nodeTypes`), its components' (`COMPONENTS` – `animation` – and the
 * plugins' `registered.components`) and its material's (`registered.materials`):
 * - the node's file (`src`) → a **media** inlet `src`;
 * - every parameter → a **value** inlet (`p:<name>` for the node type's own, `p:<owner>.<name>` for a component's or the
 *   material's), typed by its kind (`PortType`); a texture or model parameter is a **media** inlet;
 * - every verb in `actions` → an action inlet `act:<verb>` (one per verb: a rule's verb reaches every part that answers it);
 * - every name in `events` → an **event** outlet `ev:<name>` (a scene event on the bus, by name).
 * The ports are `hidden` until `project.ts` finds them wired or exposed (the Blender way). Pure.
 */

/** A spec as the ports read it: parameters, maybe a file, and what it declares */
type PortedSpec = SpecLogic & { params: Record<string, ComponentParamSpec>; src?: NodeTypeSpec["src"] };

/** The type of a parameter's port */
export const paramPortType = (param: ComponentParamSpec): PortType => {
  switch (param.kind) {
    case "number": return param.integer ? { kind: "integer", min: param.min, max: param.max } : { kind: "number", min: param.min, max: param.max };
    case "choice": return { kind: "choice", options: param.options };
    default: return { kind: param.kind };
  }
};

/** A type in words: `a number 0–1`, `a whole number 1–8`, `true or false`, `one of loop, once`, `a picture`, `a video file` */
export const typeWords = (type: PortType): string => {
  switch (type.kind) {
    case "number": return `a number ${type.min}–${type.max}`;
    case "integer": return `a whole number ${type.min}–${type.max}`;
    case "boolean": return "true or false";
    case "string": return "a text";
    case "color": return "a colour";
    case "choice": return `one of ${type.options.join(", ")}`;
    case "texture": return "a picture";
    case "model": return "a model file";
    case "file": return `a ${type.accepts === "glb" ? "model" : type.accepts} file`;
  }
};

/** The signal a type travels as: a picture or a file is media, anything else a value */
const signalOf = (type: PortType): Signal => (type.kind === "texture" || type.kind === "model" || type.kind === "file" ? "media" : "value");

/** The ports one spec gives, its parameters' ids prefixed by `prefix` (`""` for the node type's own) */
export const specPorts = (spec: PortedSpec, owner: string, prefix = ""): PortSpec[] => {
  const ports: PortSpec[] = [];
  if (spec.src && spec.src !== "none") ports.push({ id: "src", label: "file", signal: "media", dir: "in", kind: "src", type: { kind: "file", accepts: spec.src }, owner });
  for (const [name, param] of Object.entries(spec.params)) {
    const type = paramPortType(param);
    ports.push({ id: `p:${prefix}${name}`, label: paramLabel(name, param), signal: signalOf(type), dir: "in", kind: "param", type, owner });
  }
  for (const verb of spec.actions ?? []) ports.push({ id: `act:${verb}`, label: verb, signal: "node", dir: "in", kind: "action", owner });
  for (const event of spec.events ?? []) ports.push({ id: `ev:${event}`, label: event, signal: "event", dir: "out", many: true, kind: "event", owner });
  return ports;
};

/** The spec a plugin recorded for a type of a kind (null: none of the declared plugins recorded it) */
const recorded = (plugins: readonly PluginData[], kind: "components" | "nodeTypes" | "materials", type: string): PortedSpec | null => {
  for (const plugin of plugins) {
    const spec = plugin.registered?.[kind]?.[type] as PortedSpec | undefined;
    if (spec && typeof spec === "object" && spec.params && typeof spec.params === "object") return spec;
  }
  return null;
};

/** A node's specs with whose they are: its node type's, its components' (not the behaviour's, not a moment's look), its material's */
export const nodeSpecs = (node: EntityData, plugins: readonly PluginData[]): Array<{ owner: string; prefix: string; spec: PortedSpec }> => {
  const specs: Array<{ owner: string; prefix: string; spec: PortedSpec }> = [];
  const own = (BUILT_IN_NODE_SPECS as Record<string, PortedSpec>)[node.type] ?? recorded(plugins, "nodeTypes", node.type);
  if (own) specs.push({ owner: node.type, prefix: "", spec: own });
  for (const component of node.components ?? []) {
    if (component.type === BEHAVIOUR_TYPE || isMoment(component.type)) continue;
    const spec = (COMPONENTS as Record<string, PortedSpec>)[component.type] ?? recorded(plugins, "components", component.type);
    if (spec) specs.push({ owner: component.type, prefix: `${component.type}.`, spec });
  }
  const material = node.material ? recorded(plugins, "materials", node.material.type) : null;
  if (material && node.material) specs.push({ owner: node.material.type, prefix: `${node.material.type}.`, spec: material });
  return specs;
};

/** Every port a node's specs give, each id once (the first spec that names it wins) */
export const nodePorts = (node: EntityData, plugins: readonly PluginData[]): PortSpec[] => {
  const ports: PortSpec[] = [];
  for (const { owner, prefix, spec } of nodeSpecs(node, plugins)) {
    for (const port of specPorts(spec, owner, prefix)) if (!ports.some(p => p.id === port.id && p.dir === port.dir)) ports.push(port);
  }
  return ports;
};

/** Does any of a node's specs answer a rule's verb (`act:<verb>` among its ports)? */
export const answersVerb = (ports: readonly PortSpec[], verb: string): boolean => ports.some(p => p.kind === "action" && p.id === `act:${verb}`);

/** Can a value of type `from` go where `to` is expected? Only the same type – and an integer where a number goes (B8) */
export const typeTakes = (to: PortType, from: PortType): boolean => {
  if (to.kind === "number") return from.kind === "number" || from.kind === "integer";
  if (to.kind === "choice") return from.kind === "choice" && from.options.every(o => to.options.includes(o));
  if (to.kind === "file") return from.kind === "file" && from.accepts === to.accepts;
  return to.kind === from.kind;
};
