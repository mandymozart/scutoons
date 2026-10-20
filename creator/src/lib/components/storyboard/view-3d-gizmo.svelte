<script lang="ts">
	// The picked figure's gizmo in the 3D view (#124; the studio's Viewport): three's TransformControls on the figure's
	// pivot – Move = the ground's two axes, Turn = about the vertical. While it drags, the camera rests; on release the
	// figure's place is written once (x / y / h / turn, one undo step). Drawn over the look (the stage's helper scene),
	// never through it.
	import { untrack } from 'svelte';
	import { TransformControls } from 'three/addons/controls/TransformControls.js';
	import type { Shot } from '@scutoons/content/storyboard';
	import type { Stage3d } from '$lib/storyboard/stage.svelte.ts';

	let { stage, tool, figure, commit }: { stage: Stage3d; tool: 'move' | 'turn'; figure: number | null; commit: (change: (shot: Shot) => void) => void } = $props();

	let gizmo = $state.raw<TransformControls | null>(null);

	$effect(() => {
		const built = stage.built;
		if (!built) return;
		const g = untrack(() => {
			const t = new TransformControls(built.camera, stage.renderer.domElement);
			t.size = 0.8;
			t.addEventListener('dragging-changed', (e) => {
				if (stage.controls) stage.controls.enabled = !e.value;
			});
			t.addEventListener('objectChange', () => stage.draw());
			t.addEventListener('mouseUp', () => {
				const change = figure != null ? stage.figureEdit(figure) : null;
				if (change) commit(change);
			});
			stage.helperScene.add(t.getHelper());
			stage.gizmo = t;
			return t;
		});
		gizmo = g;
		return () => {
			g.detach();
			stage.helperScene.remove(g.getHelper());
			g.dispose();
			if (stage.gizmo === g) stage.gizmo = null;
			gizmo = null;
		};
	});

	$effect(() => {
		if (!gizmo) return;
		gizmo.setMode(tool === 'turn' ? 'rotate' : 'translate');
		gizmo.showX = tool !== 'turn';
		gizmo.showZ = tool !== 'turn';
		gizmo.showY = tool === 'turn';
		const pivot = figure != null ? stage.pivots[figure] : undefined;
		if (pivot) gizmo.attach(pivot);
		else gizmo.detach();
		stage.draw();
	});
</script>
