import { PLUGIN_API_VERSION, type PluginRegistrations } from "@engine/types";
import type { PluginRegistrySnapshot } from "@engine/scene/plugins";
import type { CompileResult } from "#lib/plugins/compile.js";

/**
 * What a publish carries of the Code tab (#78): every saved source of the experience as the browser's transpile makes it – the
 * one **Run** uses, no type check (#266) – with the plugin API it was written against and, where the source ran on the
 * stage in this session, the specs that run registered. The API puts each into the published version and declares it;
 * readers load it from the CDN like a compiled plugin.
 */

/** One carried plugin (`POST /editor/experiences/{id}/publish` → `plugins[]`) */
export interface PublishedPlugin {
  id: string;
  /** The module: the transpiled source, `export const register` */
  code: string;
  /** `PLUGIN_API_VERSION` */
  api: string;
  /** The specs a stage run registered, as the content declares them; absent when the source has not run here */
  registered?: PluginRegistrations;
}

/** A saved source: its id and text */
export interface SavedSource {
  id: string;
  source: string;
}

export type CarriedPlugins = { plugins: PublishedPlugin[] } | { error: string };

type SnapshotEntry = PluginRegistrySnapshot["plugins"][string];

/**
 * A stage run's registrations as the content declares them: its material layers among its components, its looks only
 * when it has some (as the content build writes them)
 */
export const declaredRegistrations = (entry: SnapshotEntry): PluginRegistrations => ({
  components: { ...entry.components, ...entry.layers },
  nodeTypes: { ...entry.nodeTypes },
  passes: { ...entry.passes },
  materials: { ...entry.materials },
  textures: { ...entry.textures },
  ...(Object.keys(entry.appearances ?? {}).length ? { appearances: { ...entry.appearances } } : {}),
});

/**
 * The plugins a publish carries: every saved source transpiled, in the order given. The first source that does not
 * transpile stops it – its id, line and message, so the publish step can say what to fix in the Code tab.
 */
export const carriedPlugins = async (
  sources: readonly SavedSource[],
  transpile: (source: string, name: string) => Promise<CompileResult>,
  registry: PluginRegistrySnapshot | null,
): Promise<CarriedPlugins> => {
  const plugins: PublishedPlugin[] = [];
  for (const { id, source } of sources) {
    const result = await transpile(source, id);
    if ("error" in result) {
      return { error: `The plugin ${id} does not compile${result.error.line ? ` (line ${result.error.line})` : ""}: ${result.error.message} – fix it in the Code tab.` };
    }
    const ran = registry?.plugins[id];
    plugins.push({ id, code: result.code, api: PLUGIN_API_VERSION, ...(ran ? { registered: declaredRegistrations(ran) } : {}) });
  }
  return { plugins };
};
