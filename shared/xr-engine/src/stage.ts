import { MockStage } from '#lib/stage/mock-stage.js';
import { Gizmo } from '../osct/editor/src/lib/stage/gizmo';
import { MockTracker } from '../osct/editor/src/lib/stage/mock-tracker';
import { getPreview, setPreviewActive, subscribePreview, updatePreview } from '../osct/editor/src/lib/stage/preview';
import { deviceStorage } from '../osct/editor/src/lib/storage';
import { ArScene } from '../osct/engine/src/scene/index';
import type { EntityData } from '../osct/engine/src/types';
import { ArState } from '../osct/engine/src/types/scene';
import { setDeviceStorage } from '../osct/engine/src/utils/device-storage';
import { getTarget } from '../osct/engine/src/content/game-config';
import { loadExperience, pageProgress } from './load';
import { stagePicking } from './stage-pick';
import type * as api from './stage-api';
import type { XrNodeRef } from './stage-api';

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
	let selected: XrNodeRef | null = null;
	// the gizmo (OSCT's: three.js TransformControls in the node's own space); its drags report the placement
	let gizmo: Gizmo | null = null;
	let tool: api.XrStageTool = 'select';
	let always = false;
	const handles = () => {
		if (!gizmo) return;
		if (tool !== 'select') gizmo.mode = tool;
		gizmo.handles = tool !== 'select' || always;
	};
	const inspection = view.inspect();
	if (inspection) {
		gizmo = new Gizmo(inspection, (at, placement) => handlers.placed({ target: at.targetId, path: [...at.path] }, placement as api.XrPlacement));
		handles();
	}
	if (tracker) (tracker as MockTracker).ignorePointer = () => !!gizmo?.busy;
	const outlineSelection = () => {
		// a node: the gizmo holds and outlines it; a print: the picking's outline
		gizmo?.select(selected?.path ? { targetId: selected.target, path: selected.path } : null);
		picking.select(selected && !selected.path ? selected : null, planes());
	};
	const onClick = (event: MouseEvent) => {
		if (gizmo?.recentlyDragged()) return;
		handlers.picked(picking.pick(container, event.clientX, event.clientY, planes()));
	};
	container.addEventListener('click', onClick);

	return {
		stage: {
			view: (preset) => updatePreview({ view: preset }),
			select: (ref) => {
				selected = ref;
				outlineSelection();
			},
			tool: (next, on) => {
				tool = next;
				always = on;
				handles();
			},
			// the engine builds the target's entity anew: the selection's outline goes onto the new objects
			override: (target, entity) => {
				view.overrideEntity(target, (entity ?? null) as EntityData | null);
				gizmo?.sync();
				outlineSelection();
			},
			// the engine's content holds the target the tracker lays out: its place changes there, the desk is laid out anew
			// (OSCT's MockTracker.relayout – the content as it is now, nothing loaded again, the found targets stay found)
			place: (target, place) => {
				const data = getTarget(target);
				if (!data) return;
				data.place = { ...place };
				tracker?.relayout();
				gizmo?.sync();
				outlineSelection();
			},
			dispose: async () => {
				container.removeEventListener('click', onClick);
				gizmo?.dispose();
				picking.dispose();
				off.forEach((stop) => stop());
				await view.dispose();
				setPreviewActive(false);
			}
		}
	};
};
