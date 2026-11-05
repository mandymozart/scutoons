<script lang="ts" module>
	export type DesignerTabId = 'parameters' | 'nodes' | 'rust' | 'assignment';
</script>

<script lang="ts">
	// The designer's tab row in the creator's Sound design workspace: Parameters · Nodes · Rust · Assignment (RIG-EDITING.md
	// §2). Parameters, Nodes and Rust are live; Assignment is a label with the title "coming next". One horizontally
	// scrolling row on a phone. A grouped row of text buttons: square (DESIGN "Round vs square").
	import { m } from '../messages.ts';

	let { value = 'rust', onchange }: { value?: DesignerTabId; onchange: (id: DesignerTabId) => void } = $props();

	const tabs: { id: DesignerTabId; label: () => string; live: boolean }[] = [
		{ id: 'parameters', label: m['designer.tab_parameters'], live: true },
		{ id: 'nodes', label: m['designer.tab_nodes'], live: true },
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
