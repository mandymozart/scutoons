import { getPlugins, loadGameConfiguration } from '../osct/engine/src/content/game-config';
import { ArScene } from '../osct/engine/src/scene/index';
import { loadPlugins } from '../osct/engine/src/scene/plugins';
import { ImageTracker } from '../osct/engine/src/scene/xr8-tracker';
import { ArState } from '../osct/engine/src/types/scene';
import type * as api from './reader-api';

type VarValue = string | number | boolean;

/** every call names its plugins anew: the engine keeps a loaded module by its name */
let opened = 0;

export const openXr: typeof api.openXr = async (container, configuration, plugins, scene, { xr8Base, assetsBase, language, handlers }) => {
	// the contract wants a plugin's `src` to be a .js file; the code is in the document, so each gets a name here
	const run = ++opened;
	const code = new Map(plugins.map((plugin) => [`xr-plugin/${run}/${plugin.id}.js`, plugin.code]));
	const declared = plugins.map(({ id, api: version }) => ({ id, api: version, src: `xr-plugin/${run}/${id}.js` }));
	const failed = await loadGameConfiguration({ bundled: async () => ({ ...(configuration as object), plugins: declared }), assetsBase });
	if (failed) return { problems: failed.details ?? [failed.msg] };
	// loaded here, from a blob each, before the scene asks for them by the same names
	await loadPlugins(getPlugins(), async (src) => {
		const url = URL.createObjectURL(new Blob([code.get(src) ?? ''], { type: 'text/javascript' }));
		try {
			return await import(/* @vite-ignore */ url);
		} finally {
			URL.revokeObjectURL(url);
		}
	});

	const seen = new Set<string>();
	const unlocked = new Set<string>();
	const variables = new Map<string, VarValue>();
	const view = new ArScene(container, {
		createTracker: ({ container: host, options }) => new ImageTracker(host, options, xr8Base),
		subtitleLanguage: language,
		progress: {
			unlocked: (target) => unlocked.has(target),
			variables: {
				get: (name) => variables.get(name),
				set: (name, value) => void (value === null ? variables.delete(name) : variables.set(name, value)),
				all: () => Object.fromEntries(variables)
			}
		}
	});
	const off = [
		view.on('status', (status, error) => handlers.status(status, error?.code)),
		view.on('subtitle', (text) => handlers.subtitle(text)),
		view.on('targetFound', (target) => {
			seen.add(target);
			handlers.found(seen.size);
			if (unlocked.has(target)) return;
			unlocked.add(target);
			// the first find plays the reveal, every later one the engine's own found look
			view.celebrate(target);
		}),
		view.on('targetLost', (target) => {
			seen.delete(target);
			handlers.found(seen.size);
		})
	];
	await view.load(scene);

	return {
		run: {
			start: () => view.setState(ArState.RUNNING),
			stop: () => view.setState(ArState.STOPPED),
			dispose: async () => {
				off.forEach((stop) => stop());
				await view.dispose();
			}
		}
	};
};
