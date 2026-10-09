<script lang="ts">
	import FoldButton from './fold-button.svelte';
	import MaximizeButton from './maximize-button.svelte';
	import { getDockSlot, shellDock } from './dock.svelte.ts';

	/**
	 * The buttons of a panel in the AppShell's right dock (owner 2026-10-06): placed in the docked panel's header, next to
	 * its close cross – fold to the rail (the rail unfolds it; closing stays the ✕) and maximize (the panel fills
	 * everything right of the sidebar's rail until restored; agents/DOCKING.md "Maximize"). Several panels can be
	 * open (10.57): the buttons act on the panel they sit in (the shell's `DockSlot`). While maximized there is nothing
	 * to fold. Nothing outside a foldable dock.
	 */
	let {
		label = 'Fold the panel',
		maximizeLabel = 'Maximize',
		restoreLabel = 'Restore',
		testid = 'dock-fold'
	}: { label?: string; maximizeLabel?: string; restoreLabel?: string; testid?: string } = $props();
	const slot = getDockSlot();
	const entry = $derived(slot ? shellDock.entry(slot()) : shellDock.latest);
	const size = $derived(entry?.panel.size);
	const target = $derived(entry ? (`dock:${entry.id}` as const) : 'dock');
	/** the leftmost open panel can fold the page next to it (owner 2026-10-07, 10.57: the main folds only by hand) */
	const leftmost = $derived(!!entry && shellDock.shown[0] === entry.id && !shellDock.mainFolded && !shellDock.maximized);
</script>

{#if size?.foldable && entry}
	<span class="sc-dock-buttons">
		{#if leftmost}
			<FoldButton side="left" label={shellDock.pageFoldLabel} title="{shellDock.pageFoldLabel} (Ctrl + Shift + \)" controls="sc-shell-content" onclick={() => shellDock.foldMain(true)} testid="page-fold" class="sc-page-fold" />
		{/if}
		{#if shellDock.maximized !== target}
			<FoldButton side="right" {label} controls="sc-shell-dock-{entry.id}" onclick={() => size.fold(true)} {testid} />
		{/if}
		<MaximizeButton {target} label={maximizeLabel} {restoreLabel} controls="sc-shell-dock-{entry.id}" testid="dock-maximize" />
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
	@media (max-width: 767px) {
		.sc-dock-buttons :global(.sc-page-fold) {
			display: none;
		}
	}
</style>
