<script lang="ts">
	// The Storyboard page's main view (#125; VARIANTS-CANVAS §3.1, owner 2026-10-20: "as recommended"): two editors behind
	// one switch – the Viewport (the current shot's 3D view, shot-canvas) and Variants (the board: every generation of the
	// picked shots, a band each – variants-canvas). The switch sits at the start of the shown editor's tool row; V toggles
	// it outside text fields; each step remembers its editor (Generate opens Variants). The Viewport stays mounted under the
	// board (inert while the board is open) – switching back doesn't rebuild the scene.
	import { onDestroy } from 'svelte';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';
	import ShotCanvas from '$lib/components/storyboard/shot-canvas.svelte';
	import VariantsCanvas from '$lib/components/storyboard/variants-canvas.svelte';
	import EditorSwitch from '$lib/components/storyboard/editor-switch.svelte';

	let { sb }: { sb: StoryboardPage } = $props();
	onDestroy(() => sb.board.dispose());

	const typing = (el: Element | null) => !!el?.closest('input, textarea, select, [contenteditable]:not([contenteditable="false"]), [role="spinbutton"]');
	function onkeydown(e: KeyboardEvent) {
		if (e.ctrlKey || e.metaKey || e.altKey || e.repeat || e.defaultPrevented || typing(document.activeElement) || document.querySelector('dialog[open]')) return;
		if (e.key !== 'v' && e.key !== 'V') return;
		sb.board.toggleEditor();
		e.preventDefault();
	}
</script>

<svelte:window {onkeydown} />

{#snippet lead()}<EditorSwitch {sb} />{/snippet}

<div class="main" data-testid="main-view">
	<div class="viewport" inert={sb.board.open}>
		<ShotCanvas {sb} {lead} />
	</div>
	{#if sb.board.open}
		<VariantsCanvas {sb} {lead} />
	{/if}
</div>

<style>
	.main {
		position: relative;
		height: 100%;
		min-height: 0;
	}
	.viewport {
		height: 100%;
		min-height: 0;
	}
</style>
