<script lang="ts" module>
	export type DesignerTabId = 'deck' | 'parameters' | 'nodes' | 'rust' | 'assignment';
</script>

<script lang="ts">
	// The tab row of /live/deck for creators: Deck · Parameters · Nodes · Rust · Assignment (RIG-EDITING.md §2). Deck and
	// Rust are live; the other three are labels with the title "coming next" (the structure the owner reviews). One
	// horizontally scrolling row on a phone. A grouped row of text buttons: square (DESIGN "Round vs square").
	import { m } from '../messages.ts';

	let { value = 'deck', onchange }: { value?: DesignerTabId; onchange: (id: DesignerTabId) => void } = $props();

	const tabs: { id: DesignerTabId; label: () => string; live: boolean }[] = [
		{ id: 'deck', label: m['designer.tab_deck'], live: true },
		{ id: 'parameters', label: m['designer.tab_parameters'], live: false },
		{ id: 'nodes', label: m['designer.tab_nodes'], live: false },
		{ id: 'rust', label: m['designer.tab_rust'], live: true },
		{ id: 'assignment', label: m['designer.tab_assignment'], live: false }
	];
</script>

<div class="tabs" role="tablist" aria-label={m['designer.tabs']()} data-testid="designer-tabs">
	{#each tabs as tab (tab.id)}
		<button
			type="button"
			role="tab"
			aria-selected={value === tab.id}
			class:on={value === tab.id}
			disabled={!tab.live}
			title={tab.live ? undefined : m['designer.coming_next']()}
			data-testid="designer-tab-{tab.id}"
			onclick={() => onchange(tab.id)}
		>
			{tab.label()}
		</button>
	{/each}
</div>

<style>
	.tabs {
		display: flex;
		overflow-x: auto;
		scrollbar-width: none;
		background: var(--ink);
		color: var(--paper);
		border-top: 1px solid color-mix(in srgb, var(--paper) 15%, transparent);
		border-bottom: var(--border-width-regular) solid var(--paper);
		font-family: var(--font-mono);
	}
	button {
		flex: none;
		height: var(--control-height);
		padding: 0 var(--space-1);
		border: 0;
		border-radius: 0;
		background: none;
		color: inherit;
		font: inherit;
		font-size: 0.75rem;
		text-transform: uppercase;
		letter-spacing: var(--tracking-label, 0.08em);
		cursor: pointer;
	}
	/* the open tab is inverted (DESIGN "Selection") */
	button.on {
		background: var(--paper);
		color: var(--ink);
	}
	button:disabled {
		opacity: 0.4;
		cursor: default;
	}
</style>
