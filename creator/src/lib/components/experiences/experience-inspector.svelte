<script lang="ts">
	import { Section } from '@scutoons/ui/editor';
	import { inspection, type NodeRef } from '$lib/editor/xr-nodes.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The Inspector tab: what is selected on the stage or in the Tree, as the experience stores it – a node's placement,
	 * files, light and components with their parameters; a target's place and picture with its entry. Read only so far.
	 */
	let { configuration, selected }: { configuration: unknown; selected: NodeRef | null } = $props();

	const shown = $derived(inspection(configuration, selected));
</script>

<div class="inspector" data-testid="experience-inspector">
	{#if !shown}
		<p class="sc-empty">{m['creator.experiences_inspector_none']()}</p>
	{:else}
		<p class="head"><strong>{shown.title}</strong> <span class="muted">{shown.kind}</span></p>
		{#each shown.groups as g, i (i)}
			<Section id="xr-inspector-{g.label}" label={g.label} open>
				<dl>
					{#each g.fields as f, j (j)}
						<dt>{f.label}</dt>
						<dd>{f.value}</dd>
					{/each}
				</dl>
			</Section>
		{/each}
		<p class="muted note">{m['creator.experiences_inspector_read_only']()}</p>
	{/if}
</div>

<style>
	.inspector {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		min-width: 0;
	}
	.head {
		display: flex;
		align-items: baseline;
		gap: 0.5rem;
		margin: 0 0 0.25rem;
	}
	.muted {
		color: var(--editor-muted, var(--text-muted));
	}
	dl {
		display: grid;
		grid-template-columns: minmax(5rem, max-content) 1fr;
		gap: 0.25rem 0.75rem;
		margin: 0;
	}
	dt {
		color: var(--editor-muted, var(--text-muted));
	}
	dd {
		margin: 0;
		min-width: 0;
		overflow-wrap: anywhere;
		font-variant-numeric: tabular-nums;
	}
	.note {
		margin: 0.5rem 0 0;
		font-size: var(--editor-label, 0.85em);
	}
</style>
