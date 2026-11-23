import { getPlugins } from "@engine/content/game-config";
import { appearanceRegistration, registeredAppearanceTypes } from "@engine/scene/appearances";
import { APPEARANCE_TYPES, APPEARANCES, type AppearanceSpec, type PluginData } from "@engine/types";

/**
 * The looks the appearance panel offers and the specs its rows come from (#265): the built-in looks of the contract
 * (`shared/types/appearance.ts`) and the looks of the experience's plugins – the ones in the engine's look registry (a declared
 * plugin loaded on the stage, a plugin run in the Code tab: mirrored into the page, so its look is offered at once) and
 * the ones the declared plugins recorded before they loaded. Pure functions over those two sources for the tests;
 * `liveLookChoices`, `lookSpecOf` and `lookPluginOf` read them from the engine (the caller depends on `plugins.version`
 * to follow a Run). The model is `plugin-passes.ts`.
 */

/** A look in the registry: its type, the plugin that registered it and its spec */
export interface RegisteredLook {
  type: string;
  owner: string;
  spec: AppearanceSpec;
}

/** A look the picker offers */
export interface LookChoice {
  type: string;
  /** The plugin that registers it, null for a built-in look */
  plugin: string | null;
  spec: AppearanceSpec;
}

const builtIn = (): LookChoice[] => APPEARANCE_TYPES.map(type => ({ type, plugin: null, spec: APPEARANCES[type] }));

const isBuiltIn = (type: string) => (APPEARANCE_TYPES as readonly string[]).includes(type);

/**
 * The plugins' looks: the registry's (what runs on the stage) first, then what a declared plugin recorded and has not
 * registered (yet) – a type registered by one plugin is not offered again from another's record
 */
export const pluginLooks = (registered: readonly RegisteredLook[], declared: readonly PluginData[]): LookChoice[] => {
  const choices = new Map<string, LookChoice>();
  registered.filter(look => !isBuiltIn(look.type)).forEach(look => choices.set(look.type, { type: look.type, plugin: look.owner, spec: look.spec }));
  declared.forEach(plugin => Object.entries(plugin.registered?.appearances ?? {}).forEach(([type, spec]) => {
    if (!choices.has(type) && !isBuiltIn(type)) choices.set(type, { type, plugin: plugin.id, spec });
  }));
  return [...choices.values()];
};

/** What the picker offers: the built-in looks, then the plugins' looks by plugin (alphabetical), each plugin's in its order */
export const lookChoices = (registered: readonly RegisteredLook[], declared: readonly PluginData[]): LookChoice[] => [
  ...builtIn(),
  ...pluginLooks(registered, declared).map((choice, i) => ({ choice, i })).sort((a, b) => a.choice.plugin!.localeCompare(b.choice.plugin!) || a.i - b.i).map(({ choice }) => choice),
];

/** A look's spec: built in, else the plugins' (registered, then recorded); undefined for a type nobody knows */
export const specOfLook = (type: string, registered: readonly RegisteredLook[], declared: readonly PluginData[]): AppearanceSpec | undefined =>
  isBuiltIn(type) ? APPEARANCES[type as keyof typeof APPEARANCES] : pluginLooks(registered, declared).find(choice => choice.type === type)?.spec;

/** The plugin a look belongs to (null: built in or unknown) */
export const pluginOfLook = (type: string, registered: readonly RegisteredLook[], declared: readonly PluginData[]): string | null =>
  pluginLooks(registered, declared).find(choice => choice.type === type)?.plugin ?? null;

/** The engine's look registry as it stands */
export const registeredLooks = (): RegisteredLook[] =>
  registeredAppearanceTypes().flatMap(type => {
    const registration = appearanceRegistration(type);
    return registration ? [{ type, owner: registration.owner, spec: registration.spec }] : [];
  });

export const liveLookChoices = (): LookChoice[] => lookChoices(registeredLooks(), getPlugins());
export const lookSpecOf = (type: string): AppearanceSpec | undefined => specOfLook(type, registeredLooks(), getPlugins());
export const lookPluginOf = (type: string): string | null => pluginOfLook(type, registeredLooks(), getPlugins());
