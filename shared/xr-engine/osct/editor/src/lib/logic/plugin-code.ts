/**
 * What a plugin's code listens to, emits, reads and sets (#239, moved into the logic model with #334): the content build does
 * not record it, so it is read from the plugin's source as written (`api.events.on("x")`, `api.vars.set("x", …)` …). It
 * stays the fallback once plugins declare their bus use in `registered` (L-T6, research §5.2). Pure.
 */

/** What a plugin's code listens to, emits, reads and sets – by name, as written in it */
export interface PluginUses {
  listens: string[];
  emits: string[];
  reads: string[];
  sets: string[];
}

const NAME = "([A-Za-z][A-Za-z0-9]*)";
const QUOTE = "[\"'`]";
const CALL = (object: string, verbs: string) => new RegExp(`\\b${object}\\s*\\.\\s*(${verbs})\\s*\\(\\s*${QUOTE}${NAME}${QUOTE}`, "g");

/** What a plugin's source uses of the bus and the variables: `events.on/once/off/emit("x")`, `vars.get/on/set/delete("x")` */
export const pluginCodeUses = (source: string): PluginUses => {
  const uses = { listens: new Set<string>(), emits: new Set<string>(), reads: new Set<string>(), sets: new Set<string>() };
  for (const [, verb, name] of source.matchAll(CALL("events", "on|once|emit"))) (verb === "emit" ? uses.emits : uses.listens).add(name);
  for (const [, verb, name] of source.matchAll(CALL("vars", "get|on|set|delete"))) (verb === "set" || verb === "delete" ? uses.sets : uses.reads).add(name);
  return { listens: [...uses.listens].sort(), emits: [...uses.emits].sort(), reads: [...uses.reads].sort(), sets: [...uses.sets].sort() };
};
