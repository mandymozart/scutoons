<script lang="ts">
	import FoldButton from './fold-button.svelte';
	import MaximizeButton from './maximize-button.svelte';
	import { shellDock } from './dock.svelte.ts';

	/**
	 * The buttons of the AppShell's right dock (owner 2026-10-06): placed in a docked panel's header, next to its close
	 * cross – fold to the rail (the rail unfolds it; closing stays the ✕) and maximize (the panel fills everything between
	 * the sidebar's rail and Ask until restored; agents/DOCKING.md "Maximize"). While maximized there is nothing to fold.
	 * Nothing outside a foldable dock.
	 */
	let {
		label = 'Fold the panel',
		maximizeLabel = 'Maximize',
		restoreLabel = 'Restore',
		testid = 'dock-fold'
	}: { label?: string; maximizeLabel?: string; restoreLabel?: string; testid?: string } = $props();
	const size = $derived(shellDock.current?.size);
</script>

{#if size?.foldable}
	<span class="sc-dock-buttons">
		{#if shellDock.maximized !== 'dock'}
			<FoldButton side="right" {label} controls="sc-shell-dock" onclick={() => size.fold(true)} {testid} />
		{/if}
		<MaximizeButton target="dock" label={maximizeLabel} {restoreLabel} testid="dock-maximize" />
	</span>
{/if}

<style>
	/* in a panel's header: right, next to its close cross */
	.sc-dock-buttons {
		display: inline-flex;
		align-items: center;
		gap: var(--space-025);
		margin-left: auto;
	}
</style>
