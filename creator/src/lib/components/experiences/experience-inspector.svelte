<script lang="ts">
	import { Segmented } from '@scutoons/ui';
	import { ScrubNumber, Section } from '@scutoons/ui/editor';
	import { xrDefaultAddPlace, xrNodeAt, xrNodeFileTypes, xrNodeTypes, type XrAddPlace, type XrNode } from '@scutoons/xr-engine/tree';
	import { entitySlot, fileName, inspection, type Field, type NodeRef, type XrFile } from '$lib/editor/xr-nodes.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The Inspector tab: what is selected on the stage or in the Tree, as the experience stores it – a node's name,
	 * placement, light and each component's parameters as fields (a change shows on the stage at once and is saved a
	 * moment later); its files, and a target with its entry, read only so far. Below, "Add a part" adds a node of a type
	 * (a model, image or video with one of the experience's files) next to the selected part or inside it; on a target
	 * that shows nothing yet the part becomes its content (OSCT's editor: panels/EntityPanel.svelte, + node).
	 */
	let {
		configuration,
		selected,
		onedit,
		onadd,
		files
	}: {
		configuration: unknown;
		selected: NodeRef | null;
		onedit: (at: (string | number)[], value: string | number | boolean) => void;
		onadd: (type: string, file: XrFile | null, place: XrAddPlace) => void;
		files: XrFile[];
	} = $props();

	const shown = $derived(inspection(configuration, selected));
	// ── add a part ──
	const slot = $derived(selected ? entitySlot(configuration, selected.target) : null);
	const tree = $derived((slot?.tree ?? null) as XrNode | null);
	const at = $derived(tree ? xrNodeAt(tree, selected?.path ?? []) : undefined);
	let type = $state('');
	let file = $state('');
	let chosenPlace = $state<XrAddPlace | null>(null);
	const place = $derived<XrAddPlace>(chosenPlace ?? xrDefaultAddPlace(at));
	const fileTypes = $derived(type ? xrNodeFileTypes(type) : undefined);
	const choices = $derived(fileTypes ? files.filter((f) => fileTypes.includes(f.assetType)) : []);
	const canAdd = $derived(!!type && (!fileTypes || choices.some((f) => f.src === file)));
	const PLACES = $derived([
		{ value: 'next' as const, label: m['creator.experiences_add_next']() },
		{ value: 'inside' as const, label: m['creator.experiences_add_inside']() }
	]);
	$effect(() => {
		void selected?.target;
		void selected?.path?.join('.');
		chosenPlace = null;
	});
	function add() {
		if (!canAdd) return;
		onadd(type, choices.find((f) => f.src === file) ?? null, place);
		type = '';
		file = '';
	}

	const step = (f: Field) => (f.at[0] === 'place' ? 1 : f.at.includes('scale') ? 0.01 : f.at.includes('rotation') ? 1 : f.at.includes('position') ? 0.001 : undefined);
</script>

<div class="inspector" data-testid="experience-inspector">
	{#if !shown}
		<p class="sc-empty">{m['creator.experiences_inspector_none']()}</p>
	{:else}
		<p class="head"><strong>{shown.title}</strong> <span class="muted">{shown.kind}</span></p>
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
							<input {id} type="checkbox" checked={f.value as boolean} onchange={(e) => onedit(f.at, e.currentTarget.checked)} data-testid="xr-field-{f.at.join('.')}" />
						{:else}
							<span {id} class="value">{f.value}</span>
						{/if}
					{/each}
				</div>
			</Section>
		{/each}
	{/if}
	{#if slot}
		<Section id="xr-inspector-add" label={m['creator.experiences_add_part']()} open>
			{#if !tree}<p class="muted note">{m['creator.experiences_add_root']()}</p>{/if}
			<div class="rows" data-testid="xr-add">
				<label class="label" for="xr-add-type">{m['creator.experiences_add_type']()}</label>
				<select id="xr-add-type" bind:value={type} onchange={() => (file = '')} data-testid="xr-add-type">
					<option value="">…</option>
					{#each xrNodeTypes() as t (t)}<option value={t}>{t}</option>{/each}
				</select>
				{#if fileTypes}
					<label class="label" for="xr-add-file">{m['creator.experiences_add_file']()}</label>
					{#if choices.length}
						<select id="xr-add-file" bind:value={file} data-testid="xr-add-file">
							<option value="">…</option>
							{#each choices as f (f.src)}<option value={f.src} title={f.src}>{fileName(f.src)}</option>{/each}
						</select>
					{:else}
						<span id="xr-add-file" class="muted">{m['creator.experiences_add_file_none']()}</span>
					{/if}
				{/if}
				{#if tree}
					<span class="label">{m['creator.experiences_add_place']()}</span>
					<Segmented options={PLACES} value={place} onchange={(v) => (chosenPlace = v)} label={m['creator.experiences_add_place']()} />
				{/if}
				<span></span>
				<button type="button" class="add" disabled={!canAdd} onclick={add} data-testid="xr-add-button">{m['creator.experiences_add']()}</button>
			</div>
			{#if slot.shared}<p class="muted note">{m['creator.experiences_shared_note']({ entity: slot.shared })}</p>{/if}
		</Section>
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
	.rows {
		display: grid;
		grid-template-columns: minmax(5rem, max-content) minmax(0, 1fr);
		align-items: center;
		gap: 0.25rem 0.75rem;
	}
	.label {
		color: var(--editor-muted, var(--text-muted));
		overflow-wrap: anywhere;
	}
	.value {
		min-width: 0;
		overflow-wrap: anywhere;
	}
	.note {
		margin: 0 0 0.25rem;
	}
	select {
		min-width: 0;
		margin: 0;
	}
	.add {
		justify-self: start;
	}
	input[type='text'] {
		min-width: 0;
		margin: 0;
	}
	input[type='checkbox'] {
		justify-self: start;
		margin: 0;
	}
	input[type='color'] {
		justify-self: start;
		margin: 0;
		inline-size: var(--control-height);
		block-size: var(--control-height);
		padding: 0;
	}
</style>
