<script lang="ts">
	import FoldButton from './fold-button.svelte';
	import { shellDock } from './dock.svelte.ts';

	/**
	 * The fold button of the AppShell's right dock (owner 2026-10-06): placed in a docked panel's header, next to its
	 * close cross – folds the dock to its rail (the rail unfolds it; closing stays the ✕). Nothing outside a foldable dock.
	 */
	let { label = 'Fold the panel', testid = 'dock-fold' }: { label?: string; testid?: string } = $props();
	const size = $derived(shellDock.current?.size);
</script>

{#if size?.foldable}
	<FoldButton side="right" {label} controls="sc-shell-dock" onclick={() => size.fold(true)} {testid} class="sc-dock-fold" />
{/if}

<style>
	/* in a panel's header: right, next to its close cross */
	:global(.sc-dock-fold) {
		margin-left: auto;
	}
</style>
