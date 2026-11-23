import { getPlugins } from "@engine/content/game-config";
import { ENGINE_OWNER, passRegistration, registeredPassTypes } from "@engine/scene/post/registry";
import { PASS_TYPES, PASSES, type PassSpec, type PluginData } from "@engine/types";

/**
 * The passes the post-processing section offers and the specs its rows come from (#216): the built-in passes of the
 * contract and the passes of the experience's plugins – the ones in the engine's pass registry (a declared plugin loaded on the
 * stage, a plugin run in the Code tab: its pass is offered at once, no reload) and the ones the declared plugins recorded
 * before they loaded. Pure functions over those two sources for the tests; `livePassGroups` and
 * `passSpecOf` read them from the engine (the caller depends on `plugins.version` to follow a Run).
 */

/** A pass in the registry: its type, who registered it (`engine` or a plugin's id) and its spec */
export interface RegisteredPass {
  type: string;
  owner: string;
  spec: PassSpec;
}

/** A pass *add pass* offers */
export interface PassChoice {
  type: string;
  /** The plugin that registers it, null for a built-in pass */
  plugin: string | null;
  spec: PassSpec;
}

/** The choices of one origin: the built-in passes, or one plugin's (headed by its id) */
export interface PassGroup {
  plugin: string | null;
  label: string;
  choices: PassChoice[];
}

const builtIn = (): PassChoice[] => PASS_TYPES.map(type => ({ type, plugin: null, spec: PASSES[type] as PassSpec }));

/**
 * The plugins' passes: the registry's (what runs on the stage) first, then what a declared plugin recorded and has not
 * registered (yet) – a type registered by one plugin is not offered again from another's record
 */
export const pluginPasses = (registered: readonly RegisteredPass[], declared: readonly PluginData[]): PassChoice[] => {
  const choices = new Map<string, PassChoice>();
  registered.filter(pass => pass.owner !== ENGINE_OWNER).forEach(pass => choices.set(pass.type, { type: pass.type, plugin: pass.owner, spec: pass.spec }));
  declared.forEach(plugin => Object.entries(plugin.registered?.passes ?? {}).forEach(([type, spec]) => {
    if (!choices.has(type) && !(PASS_TYPES as readonly string[]).includes(type)) choices.set(type, { type, plugin: plugin.id, spec });
  }));
  return [...choices.values()];
};

/** *add pass*'s groups: the built-in passes, then each plugin's passes under its id (plugins in alphabetical order) */
export const passGroups = (registered: readonly RegisteredPass[], declared: readonly PluginData[]): PassGroup[] => {
  const byPlugin = new Map<string, PassChoice[]>();
  pluginPasses(registered, declared).forEach(choice => byPlugin.set(choice.plugin!, [...(byPlugin.get(choice.plugin!) ?? []), choice]));
  return [
    { plugin: null, label: "built-in", choices: builtIn() },
    ...[...byPlugin.entries()].sort(([a], [b]) => a.localeCompare(b)).map(([plugin, choices]) => ({ plugin, label: plugin, choices })),
  ];
};

/** A pass type's spec: built in, else the plugins' (registered, then recorded); undefined for a type nobody knows */
export const specOfPass = (type: string, registered: readonly RegisteredPass[], declared: readonly PluginData[]): PassSpec | undefined =>
  (PASSES as Record<string, PassSpec>)[type] ?? pluginPasses(registered, declared).find(choice => choice.type === type)?.spec;

/** The plugin a pass type belongs to (null: built in or unknown) */
export const pluginOfPass = (type: string, registered: readonly RegisteredPass[], declared: readonly PluginData[]): string | null =>
  pluginPasses(registered, declared).find(choice => choice.type === type)?.plugin ?? null;

/** The engine's pass registry as it stands */
export const registeredPasses = (): RegisteredPass[] =>
  registeredPassTypes().flatMap(type => {
    const registration = passRegistration(type);
    return registration ? [{ type, owner: registration.owner, spec: registration.spec }] : [];
  });

export const livePassGroups = (): PassGroup[] => passGroups(registeredPasses(), getPlugins());
export const passSpecOf = (type: string): PassSpec | undefined => specOfPass(type, registeredPasses(), getPlugins());
export const passPluginOf = (type: string): string | null => pluginOfPass(type, registeredPasses(), getPlugins());
