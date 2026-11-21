import { Raycaster, Vector2, type Object3D } from 'three';
import { modelOf, nodeOf, pickable, pickByFilter, type StageHit } from '../osct/editor/src/lib/stage/pick';
import { outline, SELECTION_COLOR, type Outline } from '../osct/editor/src/lib/stage/shading';
import { entityNodes, getTarget, getTargets } from '../osct/engine/src/content/game-config';
import { BACKDROP_LAYER } from '../osct/engine/src/scene/view';
import type { SceneInspection } from '../osct/engine/src/types/scene';
import type { XrNodeRef } from './stage-api';

/**
 * Picking on the stage and the selection's outline, as OSCT's editor does it (editor/src/lib/stage/stage-core.svelte.ts
 * `hits`, `pickedObject`): a click picks the nearest node under the pointer at its most specific (the node, else the
 * printed image); the selected node wears the selection's outline.
 */
export function stagePicking(inspect: () => SceneInspection | null, scene: string) {
	const raycaster = new Raycaster();
	// the printed images and the desk lie on the backdrop layer (OSCT's stage-core enables it too)
	raycaster.layers.enable(BACKDROP_LAYER);
	let selected: Outline | null = null;

	function hits(container: HTMLElement, clientX: number, clientY: number, planes: Object3D[]): StageHit[] {
		const inspection = inspect();
		if (!inspection) return [];
		const rect = container.getBoundingClientRect();
		const point = new Vector2(((clientX - rect.left) / rect.width) * 2 - 1, -((clientY - rect.top) / rect.height) * 2 + 1);
		raycaster.setFromCamera(point, inspection.camera);
		const byObject = new Map<Object3D, { targetId: string; path: number[]; model: number[] | null }>();
		const roots: Object3D[] = [];
		for (const target of getTargets(scene)) {
			const tree = getTarget(target.id)?.entity;
			if (!tree) continue;
			const nodes = entityNodes(tree);
			const models = new Set(nodes.filter(({ node }) => node.type === 'model').map(({ path }) => path.join('.')));
			for (const { path } of nodes) {
				const object = inspection.nodeObject(target.id, path);
				if (object) byObject.set(object, { targetId: target.id, path, model: modelOf(path, (at) => models.has(at.join('.'))) });
				if (object && !path.length) roots.push(object);
			}
		}
		const found: Array<{ distance: number; hit: StageHit }> = [];
		for (const hit of raycaster.intersectObjects(roots, true)) {
			if (!pickable(hit.object)) continue;
			const node = nodeOf(hit.object, byObject);
			if (node) found.push({ distance: hit.distance, hit: { kind: 'node', ...node } });
		}
		for (const hit of raycaster.intersectObjects(planes, false)) {
			const targetId = hit.object.userData.targetId as string | undefined;
			if (targetId && pickable(hit.object)) found.push({ distance: hit.distance, hit: { kind: 'print', targetId } });
		}
		return found.sort((a, b) => a.distance - b.distance).map(({ hit }) => hit);
	}

	return {
		/** what a click at these client px picks (null: nothing) */
		pick(container: HTMLElement, clientX: number, clientY: number, planes: Object3D[]): XrNodeRef | null {
			const pick = pickByFilter([], hits(container, clientX, clientY, planes));
			return pick ? { target: pick.targetId, path: pick.printed ? null : pick.path } : null;
		},
		/** outline the selected node (or the printed image of a target selected as a whole); null: none */
		select(ref: XrNodeRef | null, planes: Object3D[]) {
			selected?.remove();
			selected = null;
			if (!ref) return;
			const object = ref.path ? inspect()?.nodeObject(ref.target, ref.path) : planes.find((p) => p.userData.targetId === ref.target);
			if (object) selected = outline(object, SELECTION_COLOR);
		},
		dispose() {
			selected?.remove();
			selected = null;
		}
	};
}
