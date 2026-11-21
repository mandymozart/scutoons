import { MockStage } from '#lib/stage/mock-stage.js';
import { MockTracker } from '../osct/editor/src/lib/stage/mock-tracker';
import { getPreview, setPreviewActive, subscribePreview, updatePreview } from '../osct/editor/src/lib/stage/preview';
import { deviceStorage } from '../osct/editor/src/lib/storage';
import { ArScene } from '../osct/engine/src/scene/index';
import { ArState } from '../osct/engine/src/types/scene';
import { setDeviceStorage } from '../osct/engine/src/utils/device-storage';
import { loadExperience, pageProgress } from './load';
import type * as api from './stage-api';

// OSCT's stage as its editor wires it (editor/src/lib/stage/stage-core.svelte.ts), the viewing half only.
export const openStage: typeof api.openStage = async (container, configuration, plugins, scene, { assetsBase, stageBase, language, handlers }) => {
	const problems = await loadExperience(configuration, plugins, assetsBase);
	if (problems) return { problems };

	setPreviewActive(true);
	setDeviceStorage(deviceStorage);
	const found = new Set<string>();
	const progress = pageProgress();
	const graphics = () => ({ onionSky: false, surroundings: false, textureQuality: getPreview().textureQuality });
	const view = new ArScene(container, {
		graphics: { getSettings: graphics, subscribe: (listener) => subscribePreview(() => listener(graphics())) },
		createTracker: ({ container: host, options, scene: world }) => {
			const tracker = new MockTracker(host, options);
			tracker.attachStage(new MockStage(world, (path) => stageBase + path));
			return tracker;
		},
		subtitleLanguage: language,
		progress: progress.reader,
		// a rule's goto: the stage stays on its scene
		navigate: () => {}
	});
	const off = [
		view.on('status', (status, error) => handlers.status(status, error?.message)),
		view.on('subtitle', (text) => handlers.subtitle(text)),
		view.on('targetFound', (target) => {
			found.add(target);
			handlers.found([...found]);
			if (progress.unlock(target)) view.celebrate(target);
		}),
		view.on('targetLost', (target) => {
			found.delete(target);
			handlers.found([...found]);
		})
	];
	await view.load(scene);
	await view.setState(ArState.RUNNING);

	return {
		stage: {
			view: (preset) => updatePreview({ view: preset }),
			dispose: async () => {
				off.forEach((stop) => stop());
				await view.dispose();
				setPreviewActive(false);
			}
		}
	};
};
