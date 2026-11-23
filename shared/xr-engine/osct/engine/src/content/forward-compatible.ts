/**
 * Forward compatibility (RULES #10, #220): content of the app's content schema MAJOR and a higher MINOR loads in this
 * app. What the app does not know of a kind it knows is dropped from a copy of the configuration before the guard
 * checks it, and listed (`SkippedFeature`), so the reader can say that an update shows everything:
 * - a component in a node's `components`, a filter in a video's `filters`, a pass in a `passes` list, a node's
 *   `material` – a type neither built in, nor registered, nor recorded by a declared plugin;
 * - a behaviour action, a field of a behaviour, a rule or an action – in a node's behaviour and in a scene's or the
 *   experience's `logic` (schema 5.0), whose fields beside `rules` and `canvas` are dropped too;
 * - a parameter a known spec does not have (a component's, a pass's, a filter's, a light's, a material's, a plugin node
 *   type's), and a parameter of a plugin's recorded spec whose kind this app does not know;
 * - a node type nobody registered stays in place (the engine renders nothing for it and keeps its children) and is
 *   listed, as is a plugin built against a newer plugin API MINOR (what it uses of that MINOR does not work).
 * A broken shape, a wrong type for a known field and a value outside a required field's enum stay the guard's errors; a
 * higher MAJOR is refused before (`checkContentCompatibility`). Content of the app's own or an older schema is passed
 * through untouched – there an unknown type is the author's mistake, and the guard reports it.
 *
 * `KnownFeatures` is what this app knows (`currentFeatures()`); a test hands an older list to read content as an older
 * app would.
 */
import { isObject } from "@shared/guards/game-config";
import {
  APPEARANCE_TYPES,
  APPEARANCES,
  BEHAVIOUR_ACTIONS,
  BEHAVIOUR_NODE_ACTIONS,
  BEHAVIOUR_TYPE,
  COMPONENT_TYPE_PATTERN,
  type ComponentParamSpec,
  componentSpec,
  componentTypes,
  CONTENT_SCHEMA_VERSION,
  ENTITY_TYPES,
  FILTER_TYPES,
  FILTERS,
  LIGHT_PARAMS,
  isMoment,
  MATERIAL_TYPE_PATTERN,
  materialSpec,
  MOMENTS,
  NODE_TYPE_PATTERN,
  nodeTypeSpec,
  parseContentSchema,
  PASS_FIELDS,
  PASS_TYPES,
  PASSES,
  PLACEMENT_KEYS,
  PLUGIN_API_VERSION,
  recordedRegistrations,
} from "../types/index";

/** The parameter kinds of a spec (`ComponentParamSpec` in shared/ – a test keeps them equal) */
export const PARAM_KINDS = ["color", "number", "choice", "boolean", "string", "texture", "model"] as const;

/** What an app knows of the content contract */
export interface KnownFeatures {
  /** The content schema it reads (`CONTENT_SCHEMA_VERSION`) */
  schema: string;
  /** Component types, built in and registered */
  components: readonly string[];
  /** Built-in node types */
  nodeTypes: readonly string[];
  /** Built-in pass types */
  passes: readonly string[];
  /** The fields of a pass that are no parameter: `type`, and `enabled` (#233) – an older app knows only `type` */
  passFields: readonly string[];
  /** Built-in looks an entity appears with (`experience.appearance`) */
  appearances: readonly string[];
  /** Video filter types */
  filters: readonly string[];
  /** The verbs of a behaviour rule's actions */
  behaviourActions: readonly string[];
  /** The kinds of a spec's parameters */
  paramKinds: readonly string[];
  /** The plugin API it hands plugins (`PLUGIN_API_VERSION`) */
  pluginApi: string;
  /** What a declared plugin's recorded specs may supply in the content (passes in `passes` lists, materials, …) */
  pluginKinds: readonly PluginKind[];
}

/** The kinds of a plugin's recorded registrations the content can use */
export type PluginKind = "components" | "nodeTypes" | "passes" | "materials" | "appearances";

/** What this app knows right now (the registered components included) */
export const currentFeatures = (): KnownFeatures => ({
  schema: CONTENT_SCHEMA_VERSION,
  components: componentTypes(),
  nodeTypes: ENTITY_TYPES,
  passes: PASS_TYPES,
  passFields: PASS_FIELDS,
  appearances: APPEARANCE_TYPES,
  filters: FILTER_TYPES,
  behaviourActions: BEHAVIOUR_ACTIONS,
  paramKinds: PARAM_KINDS,
  pluginApi: PLUGIN_API_VERSION,
  pluginKinds: ["components", "nodeTypes", "passes", "materials", "appearances"],
});

/** What a skipped feature is */
export type SkippedKind =
  | "component" | "filter" | "pass" | "look" | "material" | "node type" | "behaviour action" | "parameter" | "parameter kind" | "field" | "plugin API";

/** A feature of newer content this app skipped */
export interface SkippedFeature {
  /** Where in the configuration, e.g. `entities.fx.components[1]` */
  path: string;
  kind: SkippedKind;
  /** Its type, verb, name or version */
  name: string;
}

/** A skipped feature in words: `entities.fx.components[1]: component "sparkle"` */
export const describeSkipped = (skipped: SkippedFeature): string => `${skipped.path}: ${skipped.kind} "${skipped.name}"`;

/** `a` is the same MAJOR as `b` and a higher MINOR (both `MAJOR.MINOR`) */
export const isNewerMinor = (a: unknown, b: unknown): boolean => {
  const newer = parseContentSchema(a);
  const older = parseContentSchema(b);
  return !!newer && !!older && newer.major === older.major && newer.minor > older.minor;
};

/** The options of the guard the skipping follows (`GuardOptions`) */
export interface SkipOptions {
  unknownComponentTypes?: "declared" | "allow";
}

type Spec = { params: Record<string, ComponentParamSpec> };

/**
 * The configuration as this app can read it, and what was skipped. Content of a newer MINOR (same MAJOR) comes back as
 * a copy without the unknown parts; any other content comes back as it is, with nothing skipped.
 */
export const skipUnknownFeatures = (
  raw: unknown,
  known: KnownFeatures = currentFeatures(),
  options: SkipOptions = {},
): { config: unknown; skipped: SkippedFeature[] } => {
  if (!isObject(raw) || !isObject(raw.version) || !isNewerMinor(raw.version.schema, known.schema)) return { config: raw, skipped: [] };
  const config = structuredClone(raw);
  const skipped: SkippedFeature[] = [];
  const skip = (path: string, kind: SkippedKind, name: string) => skipped.push({ path, kind, name });

  // The declared plugins: a newer plugin API, a recorded parameter of a kind this app does not know
  const plugins = Array.isArray(config.plugins) ? config.plugins : [];
  plugins.forEach((plugin, i) => {
    if (!isObject(plugin)) return;
    if (isNewerMinor(plugin.api, known.pluginApi)) skip(`plugins[${i}]`, "plugin API", String(plugin.api));
    if (!isObject(plugin.registered)) return;
    for (const [kind, specs] of Object.entries(plugin.registered)) {
      if (!isObject(specs)) continue;
      for (const [type, spec] of Object.entries(specs)) {
        if (!isObject(spec) || !isObject(spec.params)) continue;
        const params = spec.params;
        for (const [name, param] of Object.entries(params)) {
          if (isObject(param) && typeof param.kind === "string" && !known.paramKinds.includes(param.kind)) {
            delete params[name];
            skip(`plugins[${i}].registered.${kind}.${type}.params.${name}`, "parameter kind", param.kind);
          }
        }
      }
    }
  });
  const all = recordedRegistrations(plugins);
  const none = new Map<string, never>();
  const recorded = {
    components: known.pluginKinds.includes("components") ? all.components : none,
    nodeTypes: known.pluginKinds.includes("nodeTypes") ? all.nodeTypes : none,
    passes: known.pluginKinds.includes("passes") ? all.passes : none,
    materials: known.pluginKinds.includes("materials") ? all.materials : none,
    appearances: known.pluginKinds.includes("appearances") ? all.appearances : none,
    complete: all.complete,
  };
  // As the guard: with a plugin that recorded nothing (or on the editor's stage) an unknown type may be registered at load
  const allowUnknown = options.unknownComponentTypes === "allow" || (plugins.length > 0 && !recorded.complete);

  /** Drops the parameters a spec does not have; `skipKeys` are not parameters (the placement in a node's params) */
  const dropUnknownParams = (params: Record<string, unknown>, spec: Spec, path: string, skipKeys: readonly string[] = []) => {
    for (const name of Object.keys(params)) {
      if (skipKeys.includes(name) || name in spec.params) continue;
      delete params[name];
      skip(`${path}.${name}`, "parameter", name);
    }
  };

  /**
   * An entity's own look (`reveal`, `found`, `lost` components – schema 2.30): its parameters are its look's. A look this app
   * does not know is skipped with its parameters – the component then names none and the experience's look plays; a known look
   * loses the parameters it does not have
   */
  const momentComponent = (item: Record<string, unknown>, path: string) => {
    const look = typeof item.look === "string" ? item.look : "";
    if (!look) return;
    const spec = (known.appearances.includes(look) ? (APPEARANCES as unknown as Record<string, Spec>)[look] : undefined) ?? recorded.appearances.get(look);
    if (spec) {
      dropUnknownParams(item, spec, path, ["type", "look"]);
    } else if (!allowUnknown) {
      Object.keys(item).filter(key => key !== "type").forEach(key => delete item[key]);
      skip(`${path}.look`, "look", look);
    }
  };

  /**
   * A typed list (`components`, `filters`, `passes`): unknown types dropped, known ones without unknown parameters;
   * `fields` are an item's keys that are no parameter
   */
  const typedList = (list: unknown, path: string, kind: SkippedKind, specOf: (type: string) => Spec | undefined, allow: boolean,
    fields: readonly string[] = ["type"]) => {
    if (!Array.isArray(list)) return;
    const kept = list.filter((item: unknown, i) => {
      if (!isObject(item) || typeof item.type !== "string" || !COMPONENT_TYPE_PATTERN.test(item.type)) return true;
      const spec = specOf(item.type);
      if (!spec) {
        if (!allow) skip(`${path}[${i}]`, kind, item.type);
        return allow;
      }
      if (item.type === BEHAVIOUR_TYPE && kind === "component") behaviour(item, `${path}[${i}]`);
      else if (kind === "component" && isMoment(item.type)) momentComponent(item, `${path}[${i}]`);
      else dropUnknownParams(item, spec, `${path}[${i}]`, fields);
      return true;
    });
    list.splice(0, list.length, ...kept);
  };

  const behaviourVerbs = known.behaviourActions;
  const shorthands = BEHAVIOUR_NODE_ACTIONS.filter(verb => behaviourVerbs.includes(verb)) as readonly string[];
  const ACTION_OPTIONS: Record<string, readonly string[]> = { play: ["node"], set: ["scope"] };

  /** One action of a rule: false = unknown (dropped by the caller); unknown fields of a known verb dropped */
  const action = (value: unknown, path: string): boolean => {
    if (typeof value === "string") {
      if (shorthands.includes(value) || !COMPONENT_TYPE_PATTERN.test(value)) return true;
      skip(path, "behaviour action", value);
      return false;
    }
    if (!isObject(value)) return true;
    const keys = Object.keys(value);
    const verbs = keys.filter(key => behaviourVerbs.includes(key));
    if (verbs.length === 0 && keys.length > 0 && COMPONENT_TYPE_PATTERN.test(keys[0])) {
      skip(path, "behaviour action", keys[0]);
      return false;
    }
    if (verbs.length === 1) {
      const options = ACTION_OPTIONS[verbs[0]] ?? [];
      keys.filter(key => key !== verbs[0] && !options.includes(key)).forEach(key => {
        delete value[key];
        skip(`${path}.${key}`, "field", key);
      });
    }
    return true;
  };

  /** A behaviour component: unknown fields and actions dropped; a rule left without an action goes too */
  const behaviour = (component: Record<string, unknown>, path: string) => {
    Object.keys(component).filter(key => key !== "type" && key !== "rules").forEach(key => {
      delete component[key];
      skip(`${path}.${key}`, "field", key);
    });
    ruleList(component.rules, `${path}.rules`);
  };

  /** A list of rules (a behaviour's, a scene's or the experience's): unknown fields and actions dropped; a rule left without an action goes too */
  const ruleList = (rules: unknown, path: string) => {
    if (!Array.isArray(rules)) return;
    const kept = rules.filter((rule: unknown, r) => {
      if (!isObject(rule)) return true;
      const at = `${path}[${r}]`;
      Object.keys(rule).filter(key => !["on", "when", "do"].includes(key)).forEach(key => {
        delete rule[key];
        skip(`${at}.${key}`, "field", key);
      });
      if (Array.isArray(rule.do)) {
        const actions = rule.do.filter((one: unknown, a) => action(one, `${at}.do[${a}]`));
        rule.do.splice(0, rule.do.length, ...actions);
        return actions.length > 0;
      }
      return rule.do === undefined || action(rule.do, `${at}.do`);
    });
    rules.splice(0, rules.length, ...kept);
  };

  const node = (value: unknown, path: string) => {
    if (!isObject(value)) return;
    const type = typeof value.type === "string" ? value.type : undefined;
    if (type && !known.nodeTypes.includes(type)) {
      const spec = nodeTypeSpec(type) ?? recorded.nodeTypes.get(type);
      if (spec && isObject(value.params)) dropUnknownParams(value.params, spec, `${path}.params`, PLACEMENT_KEYS);
      else if (!spec && NODE_TYPE_PATTERN.test(type) && !allowUnknown) skip(`${path}.type`, "node type", type);
    }
    if (isObject(value.light)) dropUnknownParams(value.light, { params: LIGHT_PARAMS as Record<string, ComponentParamSpec> }, `${path}.light`);
    typedList(value.filters, `${path}.filters`, "filter", t => (known.filters.includes(t) ? (FILTERS as unknown as Record<string, Spec>)[t] : undefined), false);
    typedList(value.components, `${path}.components`, "component",
      t => (known.components.includes(t) ? componentSpec(t) : undefined) ?? recorded.components.get(t), allowUnknown);
    const material = value.material;
    if (isObject(material) && typeof material.type === "string" && MATERIAL_TYPE_PATTERN.test(material.type)) {
      const spec = materialSpec(material.type) ?? recorded.materials.get(material.type);
      if (!spec) {
        if (!allowUnknown) {
          delete value.material;
          skip(`${path}.material`, "material", material.type);
        }
      } else {
        Object.keys(material).filter(key => key !== "type" && key !== "params").forEach(key => {
          delete material[key];
          skip(`${path}.material.${key}`, "field", key);
        });
        if (isObject(material.params)) dropUnknownParams(material.params, spec, `${path}.material.params`);
      }
    }
    if (Array.isArray(value.children)) value.children.forEach((child, i) => node(child, `${path}.children[${i}]`));
  };

  const passes = (list: unknown, path: string) =>
    typedList(list, path, "pass", t => (known.passes.includes(t) ? (PASSES as unknown as Record<string, Spec>)[t] : undefined) ?? recorded.passes.get(t), allowUnknown,
      known.passFields);

  /** The experience's looks by moment: an unknown moment or look is dropped (the moment then plays `found`'s or the default) */
  const appearance = (value: unknown, path: string) => {
    if (!isObject(value)) return;
    for (const [moment, look] of Object.entries(value)) {
      if (!(MOMENTS as readonly string[]).includes(moment)) {
        delete value[moment];
        skip(`${path}.${moment}`, "field", moment);
        continue;
      }
      if (!isObject(look) || typeof look.type !== "string") continue;
      const spec = (known.appearances.includes(look.type) ? (APPEARANCES as unknown as Record<string, Spec>)[look.type] : undefined)
        ?? recorded.appearances.get(look.type);
      if (spec) dropUnknownParams(look, spec, `${path}.${moment}`, ["type"]);
      else if (!allowUnknown) {
        delete value[moment];
        skip(`${path}.${moment}`, "look", look.type);
      }
    }
  };

  /** A scene's or the experience's logic (schema 5.0): its rules as a behaviour's; a field beside rules and canvas dropped */
  const logic = (value: unknown, path: string) => {
    if (!isObject(value)) return;
    Object.keys(value).filter(key => key !== "rules" && key !== "canvas").forEach(key => {
      delete value[key];
      skip(`${path}.${key}`, "field", key);
    });
    ruleList(value.rules, `${path}.rules`);
  };

  if (isObject(config.experience)) passes(config.experience.passes, "experience.passes");
  if (isObject(config.experience)) logic(config.experience.logic, "experience.logic");
  if (Array.isArray(config.scenes)) config.scenes.forEach((scene, i) => isObject(scene) && logic(scene.logic, `scenes[${i}].logic`));
  if (isObject(config.experience)) appearance(config.experience.appearance, "experience.appearance");
  if (Array.isArray(config.scenes)) config.scenes.forEach((scene, i) => isObject(scene) && passes(scene.passes, `scenes[${i}].passes`));
  if (isObject(config.entities)) Object.entries(config.entities).forEach(([id, entity]) => node(entity, `entities.${id}`));
  if (Array.isArray(config.entries)) {
    config.entries.forEach((entry, i) => {
      const entity = isObject(entry) && isObject(entry.target) ? entry.target.entity : undefined;
      if (isObject(entity) && typeof entity.ref !== "string") node(entity, `entries[${i}].target.entity`);
    });
  }
  return { config, skipped };
};
