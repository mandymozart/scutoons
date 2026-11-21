import { ArScene } from '../osct/engine/src/scene/index';
import { ImageTracker } from '../osct/engine/src/scene/xr8-tracker';
import { ArState } from '../osct/engine/src/types/scene';
import { loadExperience, pageProgress } from './load';
import type * as api from './reader-api';

export const openXr: typeof api.openXr = async (container, configuration, plugins, scene, { xr8Base, assetsBase, language, handlers }) => {
	const problems = await loadExperience(configuration, plugins, assetsBase);
	if (problems) return { problems };

	const seen = new Set<string>();
	const progress = pageProgress();
	const view = new ArScene(container, {
		createTracker: ({ container: host, options }) => new ImageTracker(host, options, xr8Base),
		subtitleLanguage: language,
		progress: progress.reader
	});
	const off = [
		view.on('status', (status, error) => handlers.status(status, error?.code)),
		view.on('subtitle', (text) => handlers.subtitle(text)),
		view.on('targetFound', (target) => {
			seen.add(target);
			handlers.found(seen.size);
			// the first find plays the reveal, every later one the engine's own found look
			if (progress.unlock(target)) view.celebrate(target);
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
