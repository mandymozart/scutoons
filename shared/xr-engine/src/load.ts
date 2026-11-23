import { getPlugins, loadGameConfiguration } from '../osct/engine/src/content/game-config';
import { loadPlugins } from '../osct/engine/src/scene/plugins';
import type { XrPluginCode } from './reader-api';

type VarValue = string | number | boolean;

/** every call names its plugins anew: the engine keeps a loaded module by its name */
let opened = 0;

/**
 * Reads `configuration` with `plugins` as its plugins and loads their code; the problems (one line each) when the
 * configuration is not one this engine can read.
 */
export async function loadExperience(configuration: unknown, plugins: readonly XrPluginCode[], assetsBase: string): Promise<string[] | null> {
	// the contract wants a plugin's `src` to be a .js file; the code is in the document, so each gets a name here
	const run = ++opened;
	const code = new Map(plugins.map((plugin) => [`xr-plugin/${run}/${plugin.id}.js`, plugin.code]));
	const declared = plugins.map(({ id, api: version }) => ({ id, api: version, src: `xr-plugin/${run}/${id}.js` }));
	const failed = await loadGameConfiguration({ bundled: async () => ({ ...(configuration as object), plugins: declared }), assetsBase });
	if (failed) return failed.details ?? [failed.msg];
	// loaded here, from a blob each, before the scene asks for them by the same names
	await loadPlugins(getPlugins(), async (src) => {
		const url = URL.createObjectURL(new Blob([code.get(src) ?? ''], { type: 'text/javascript' }));
		try {
			return await import(/* @vite-ignore */ url);
		} finally {
			URL.revokeObjectURL(url);
		}
	});
	return null;
}

/** What was found and set, kept as long as the page: the first find of a target plays its reveal */
export function pageProgress() {
	const unlocked = new Set<string>();
	const variables = new Map<string, VarValue>();
	return {
		/** true the first time a target is found */
		unlock: (target: string) => !unlocked.has(target) && !!unlocked.add(target),
		reader: {
			unlocked: (target: string) => unlocked.has(target),
			variables: {
				get: (name: string) => variables.get(name),
				set: (name: string, value: VarValue | null) => void (value === null ? variables.delete(name) : variables.set(name, value)),
				all: () => Object.fromEntries(variables)
			}
		}
	};
}
