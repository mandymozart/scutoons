<script lang="ts">
	// The 3D view's texts (#124; the studio's Viewport): the shot's texts in the SCUTOONS presets (the renderer's
	// drawDialog – a speech bubble's tail towards the nearest figure's head) on a canvas over the frame, redrawn with
	// every frame of the view; switchable (the tool row's Texts). Sized like the frame, in rem.
	import { onMount } from 'svelte';
	import { rem } from '$lib/editor/storyboard-3d.ts';
	import { drawDialog, figureBoxes } from '$lib/storyboard/render/index.ts';
	import type { Stage3d } from '$lib/storyboard/stage.svelte.ts';

	let { stage, texts }: { stage: Stage3d; texts: boolean } = $props();

	let overlay = $state<HTMLCanvasElement>();

	function paint() {
		const g = overlay?.getContext('2d');
		const { built, spec } = stage;
		const { w, h, scale } = stage.size;
		if (!g || !overlay) return;
		if (overlay.width !== w || overlay.height !== h) {
			overlay.width = w;
			overlay.height = h;
			const root = parseFloat(getComputedStyle(document.documentElement).fontSize);
			overlay.style.width = rem(w, root);
			overlay.style.height = rem(h, root);
		}
		g.clearRect(0, 0, w, h);
		if (texts && built && spec) drawDialog(g, spec, figureBoxes(built.figs, built.camera, w, h, 1), w, h, scale);
	}
	onMount(() => {
		stage.painters.add(paint);
		return () => void stage.painters.delete(paint);
	});
	$effect(() => {
		void texts;
		stage.draw();
	});
</script>

<canvas class="texts" bind:this={overlay}></canvas>

<style>
	.texts {
		position: absolute;
		inset: 0;
		pointer-events: none;
	}
</style>
