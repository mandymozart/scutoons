import { MockStage } from '#lib/stage/mock-stage.js';
import { MockTracker } from '../osct/editor/src/lib/stage/mock-tracker';
import { getPreview, setPreviewActive, subscribePreview, updatePreview } from '../osct/editor/src/lib/stage/preview';
import { deviceStorage } from '../osct/editor/src/lib/storage';
import { ArScene } from '../osct/engine/src/scene/index';
import { ArState } from '../osct/engine/src/types/scene';
import { setDeviceStorage } from '../osct/engine/src/utils/device-storage';
import { loadExperience, pageProgress } from './load';
import { stagePicking } from './stage-pick';
import type * as api from './stage-api';

// OSCT's stage as its editor wires it (editor/src/lib/stage/stage-core.svelte.ts), the viewing half only.
export const openStage: typeof api.openStage = async (container, configuration, plugins, scene, { assetsBase, stageBase, language, handlers }) => {
	const problems = await loadExperience(configuration, plugins, assetsBase);
	if (problems) return { problems };

	setPreviewActive(true);
	setDeviceStorage(deviceStorage);
	const found = new Set<string>();
	let tracker: MockTracker | null = null;
	const progress = pageProgress();
	const graphics = () => ({ onionSky: false, surroundings: false, textureQuality: getPreview().textureQuality });
	const view = new ArScene(container, {
		graphics: { getSettings: graphics, subscribe: (listener) => subscribePreview(() => listener(graphics())) },
		createTracker: ({ container: host, options, scene: world }) => {
			tracker = new MockTracker(host, options);
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

	// a click (not the end of an orbit drag: the tracker swallows those) picks what lies under the pointer
	const picking = stagePicking(() => view.inspect(), scene);
	const planes = () => tracker?.targetPlanes() ?? [];
	const onClick = (event: MouseEvent) => handlers.picked(picking.pick(container, event.clientX, event.clientY, planes()));
	container.addEventListener('click', onClick);

	return {
		stage: {
			view: (preset) => updatePreview({ view: preset }),
			select: (ref) => picking.select(ref, planes()),
			dispose: async () => {
				container.removeEventListener('click', onClick);
				picking.dispose();
				off.forEach((stop) => stop());
				await view.dispose();
				setPreviewActive(false);
			}
		}
	};
};
