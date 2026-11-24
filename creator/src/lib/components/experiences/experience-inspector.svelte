<script lang="ts">
	import { ScrubNumber, Section, Toggle } from '@scutoons/ui/editor';
	import { inspection, type Field, type NodeRef } from '$lib/editor/xr-nodes.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The Inspector tab: what is selected on the stage or in the Tree, as the experience stores it – a node's name,
	 * placement, light and each component's parameters as fields (a change shows on the stage at once and is saved a
	 * moment later); its files, and a target with its entry, read only so far (parts are added with the Tree's footer).
	 */
	let { configuration, selected, onedit }: { configuration: unknown; selected: NodeRef | null; onedit: (at: (string | number)[], value: string | number | boolean) => void } = $props();

	const shown = $derived(inspection(configuration, selected));
	const step = (f: Field) => (f.at[0] === 'place' ? 1 : f.at.includes('scale') ? 0.01 : f.at.includes('rotation') ? 1 : f.at.includes('position') ? 0.001 : undefined);
</script>

<div class="inspector" data-testid="experience-inspector">
	{#if !shown}
		<p class="sc-empty">{m['creator.experiences_inspector_none']()}</p>
	{:else}
		<p class="head"><span class="title">{shown.title}</span> <span class="muted">{shown.kind}</span></p>
		{#each shown.groups as g, i (`${i}-${g.label}`)}
			<Section id="xr-inspector-{g.label}" label={g.label} open>
				<div class="rows">
					{#each g.fields as f (f.at.join('.'))}
						{@const id = `xr-${f.at.join('-')}`}
						<label class="label" for={id}>{f.label}</label>
						{#if f.kind === 'number'}
							<ScrubNumber {id} value={f.value as number} label={f.label} step={step(f)} onchange={(v) => v !== undefined && onedit(f.at, v)} testid="xr-field-{f.at.join('.')}" />
						{:else if f.kind === 'text'}
							<input {id} type="text" value={f.value} onchange={(e) => onedit(f.at, e.currentTarget.value)} data-testid="xr-field-{f.at.join('.')}" />
						{:else if f.kind === 'color'}
							<input {id} type="color" value={f.value} oninput={(e) => onedit(f.at, e.currentTarget.value)} data-testid="xr-field-{f.at.join('.')}" />
						{:else if f.kind === 'bool'}
							<span class="bool"><Toggle {id} on={f.value as boolean} label={f.label} onchange={(on) => onedit(f.at, on)} data-testid="xr-field-{f.at.join('.')}" /></span>
						{:else}
							<span {id} class="value">{f.value}</span>
						{/if}
					{/each}
				</div>
			</Section>
		{/each}
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
	.title {
		font-size: var(--editor-title);
	}
	.muted {
		color: var(--tool-muted);
	}
	.rows {
		display: grid;
		grid-template-columns: minmax(5rem, max-content) minmax(0, 1fr);
		align-items: center;
		gap: 0.25rem 0.75rem;
	}
	.label {
		color: var(--tool-muted);
		overflow-wrap: anywhere;
	}
	.value {
		min-width: 0;
		overflow-wrap: anywhere;
	}
	input[type='text'] {
		min-width: 0;
		margin: 0;
	}
	.bool {
		display: flex;
	}
	input[type='color'] {
		justify-self: start;
		margin: 0;
		inline-size: var(--control-height);
		block-size: var(--control-height);
		padding: 0;
	}
</style>
