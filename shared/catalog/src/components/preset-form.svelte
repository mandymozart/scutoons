<script lang="ts">
	import { Button, CloseIcon } from '@scutoons/ui';
	import { listText, parseList, problemFor, text, type CatalogFormProps, type Data } from '../kinds.ts';
	import { m } from '../messages.ts';
	import { catalogLocale } from '../setup.ts';
	import { exposeLabel, inherited, instrumentControls, instrumentPorts, lookupOf, parentOptions, valueRows, withValue } from '../rigs/edit.ts';
	import CatalogField from './catalog-field.svelte';
	import LocaleText from './locale-text.svelte';
	import RightsField from './rights-field.svelte';

	/**
	 * A preset (research/INSTRUMENTS.md §2, deck-space rig/preset.rs): one instrument, a parent it derives from (its
	 * lineage folds root → leaf), tags, and only what it changes – values on the instrument's control inputs (the
	 * parent's folded value greyed as the placeholder; empty = inherited), inputs it unlinks, links it adds, controller
	 * mappings – and its rights. The problems are the server's and the validator's.
	 */
	let { data = $bindable(), lists, problems }: CatalogFormProps = $props();

	const instruments = $derived(lists.instruments ?? []);
	const presets = $derived(lists.presets ?? []);
	const I = $derived(lookupOf(instruments));
	const P = $derived(lookupOf(presets));
	const instrument = $derived(I(String(data.instrument ?? '')));
	const parent = $derived(inherited(data, P));
	const rows = $derived(valueRows(instrument, data, parent));
	const shown = $derived(rows.filter((r) => r.exposed));
	const others = $derived(rows.filter((r) => !r.exposed));
	const ports = $derived(instrumentPorts(instrument));
	const controls = $derived(instrumentControls(instrument));
	const links = $derived<Data[]>(Array.isArray(data.link) ? data.link : []);
	const mappings = $derived<Data[]>(Array.isArray(data.mapping) ? data.mapping : []);
	const unlink = $derived<string[]>(Array.isArray(data.unlink) ? data.unlink : []);
	const locale = catalogLocale();
	const err = (...words: string[]) => problemFor(problems, ...words);
	const number = (raw: string) => (raw.trim() === '' || Number.isNaN(Number(raw)) ? null : Number(raw));

	/** another instrument: the parent, values and routing changes belong to the old one – they go */
	function setInstrument(next: string | null) {
		const { parent: _p, values: _v, unlink: _u, link: _l, mapping: _m, ...rest } = data;
		data = next && next !== data.instrument ? { ...rest, instrument: next } : { ...data, instrument: next ?? '' };
	}
	function setList(key: 'unlink' | 'link' | 'mapping', list: unknown[]) {
		const { [key]: _old, ...rest } = data;
		data = list.length ? { ...rest, [key]: list } : rest;
	}
	const setLink = (i: number, patch: Data) => setList('link', links.map((l, j) => (j === i ? { ...l, ...patch } : l)));
	const setMapping = (i: number, patch: Data) => setList('mapping', mappings.map((x, j) => (j === i ? { ...x, ...patch } : x)));
	function setRange(i: number, which: 0 | 1, raw: string) {
		const v = number(raw);
		const range: (number | null)[] = Array.isArray(mappings[i].range) ? [...mappings[i].range] : [null, null];
		range[which] = v;
		const { range: _old, ...rest } = mappings[i];
		setList('mapping', mappings.map((x, j) => (j === i ? (range[0] === null && range[1] === null ? rest : { ...rest, range: [range[0] ?? 0, range[1] ?? 1] }) : x)));
	}
	/** the inputs the instrument links – what `unlink` may name */
	const linkedInInstrument = $derived((Array.isArray(instrument?.link) ? (instrument!.link as Data[]) : []).map((l) => String(l.to)).filter((to) => !unlink.includes(to)));
	const rest = $derived(problems.filter((p) => !problemFor([p], ...rows.map((r) => r.path), 'mapping', 'link', 'unlink', 'parent', 'tags', 'name', 'instrument', 'id')));
</script>

{#snippet valueRow(r: (typeof rows)[number])}
	<div class="value" class:linked={r.linked} data-testid="preset-value-{r.path}">
		<span class="path"><span>{exposeLabel(instrument, r.path, locale)}</span><span class="sc-caption muted">{r.path}</span></span>
		<input
			type="number"
			step="any"
			value={r.own ?? ''}
			placeholder={r.linked ? '' : String(r.inherits ?? r.base ?? '')}
			disabled={r.linked}
			aria-label={r.path}
			onchange={(e) => (data = withValue(data, r.path, number(e.currentTarget.value)))}
		/>
		<span class="sc-caption muted state">{r.linked ? m['catalog.preset_linked']() : r.own === undefined ? m['catalog.preset_inherits']() : ''}</span>
		{#if err(r.path)}<p class="sc-error wide">{err(r.path)}</p>{/if}
	</div>
{/snippet}

{#if rest.length}<ul class="sc-error problems" role="alert" data-testid="preset-problems">{#each rest as p (p)}<li>{p}</li>{/each}</ul>{/if}

<div class="sc-fields" data-cols="fit" style:--fields-fit-min="12rem">
	<CatalogField label={m['catalog.instrument']()} type="select" required value={data.instrument} options={instruments.map((i) => ({ value: i.id, label: `${text(i.data?.name) || i.id} · ${i.id}` }))} error={err('instrument')} onchange={setInstrument} testid="preset-instrument" />
	<CatalogField
		label={m['catalog.preset_parent']()}
		type="select"
		value={data.parent}
		options={parentOptions(presets, data).map((id) => ({ value: id }))}
		none={m['catalog.preset_parent_none']()}
		error={err('parent', 'lineage', 'derives')}
		onchange={(v) => {
			const { parent: _old, ...r } = data;
			data = v ? { ...r, parent: v } : r;
		}}
		testid="preset-parent"
	/>
</div>
<LocaleText label={m['catalog.name']()} value={data.name} error={err('name')} onchange={(v) => (data = { ...data, name: v })} testid="preset-name" />
<CatalogField label={m['catalog.preset_tags']()} value={listText(data.tags)} hint={m['catalog.list_hint']()} error={err('tags')} onchange={(v) => (data = { ...data, tags: parseList(String(v ?? '')) })} testid="preset-tags" />

{#if instrument}
	<fieldset class="group">
		<legend>{m['catalog.preset_values']()}</legend>
		<p class="sc-caption muted">{m['catalog.preset_values_hint']()}</p>
		{#each shown as r (r.path)}{@render valueRow(r)}{/each}
		{#if others.length}
			<details>
				<summary>{m['catalog.preset_other_values']()} · {others.length}</summary>
				{#each others as r (r.path)}{@render valueRow(r)}{/each}
			</details>
		{/if}
	</fieldset>

	<fieldset class="group">
		<legend>{m['catalog.preset_unlink']()}</legend>
		{#each unlink as u, i (u)}
			<div class="row"><span>{u}</span><button type="button" class="sc-round" title={m['catalog.remove']()} aria-label={m['catalog.remove']()} onclick={() => setList('unlink', unlink.filter((_, j) => j !== i))}><CloseIcon /></button></div>
		{/each}
		<select value="" aria-label={m['catalog.preset_unlink']()} onchange={(e) => e.currentTarget.value && setList('unlink', [...unlink, e.currentTarget.value])} data-testid="preset-unlink-add">
			<option value="">{m['catalog.choose']()}</option>
			{#each linkedInInstrument as to (to)}<option value={to}>{to}</option>{/each}
		</select>
		<p class="sc-caption muted">{m['catalog.preset_unlink_hint']()}</p>
		{#if err('unlink', 're-route')}<p class="sc-error">{err('unlink', 're-route')}</p>{/if}
	</fieldset>

	<fieldset class="group">
		<legend>{m['catalog.preset_links']()}</legend>
		{#each links as l, i (i)}
			<div class="pair">
				<input type="text" list="preset-outputs" value={l.from ?? ''} placeholder={m['catalog.rig_route_from']()} aria-label={m['catalog.rig_route_from']()} onchange={(e) => setLink(i, { from: e.currentTarget.value.trim() })} />
				<input type="text" list="preset-inputs" value={l.to ?? ''} placeholder={m['catalog.rig_route_to']()} aria-label={m['catalog.rig_route_to']()} onchange={(e) => setLink(i, { to: e.currentTarget.value.trim() })} />
				<button type="button" class="sc-round" title={m['catalog.remove']()} aria-label={m['catalog.remove']()} onclick={() => setList('link', links.filter((_, j) => j !== i))}><CloseIcon /></button>
			</div>
		{/each}
		<datalist id="preset-outputs">{#each ports.outputs as p (p)}<option value={p}></option>{/each}</datalist>
		<datalist id="preset-inputs">{#each ports.inputs as p (p)}<option value={p}></option>{/each}</datalist>
		<span><Button size="sm" variant="secondary" onclick={() => setList('link', [...links, { from: ports.outputs[0] ?? '', to: ports.inputs[0] ?? '' }])} data-testid="preset-add-link">{m['catalog.preset_add_link']()}</Button></span>
		{#if err('link')}<p class="sc-error">{err('link')}</p>{/if}
	</fieldset>

	<fieldset class="group">
		<legend>{m['catalog.preset_mappings']()}</legend>
		{#each mappings as x, i (i)}
			<div class="mapping" data-testid="preset-mapping">
				<input type="text" value={x.source ?? ''} placeholder={m['catalog.preset_source']()} aria-label={m['catalog.preset_source']()} onchange={(e) => setMapping(i, { source: e.currentTarget.value.trim() })} />
				<select value={x.target ?? ''} aria-label={m['catalog.preset_target']()} onchange={(e) => setMapping(i, { target: e.currentTarget.value })}>
					<option value="">{m['catalog.choose']()}</option>
					{#each controls as c (c)}<option value={c}>{c}</option>{/each}
					{#if x.target && !controls.includes(x.target)}<option value={x.target}>{x.target} (?)</option>{/if}
				</select>
				<input type="number" step="any" value={x.range?.[0] ?? ''} placeholder="min" aria-label="{m['catalog.preset_range']()} min" onchange={(e) => setRange(i, 0, e.currentTarget.value)} />
				<input type="number" step="any" value={x.range?.[1] ?? ''} placeholder="max" aria-label="{m['catalog.preset_range']()} max" onchange={(e) => setRange(i, 1, e.currentTarget.value)} />
				<button type="button" class="sc-round" title={m['catalog.remove']()} aria-label={m['catalog.remove']()} onclick={() => setList('mapping', mappings.filter((_, j) => j !== i))}><CloseIcon /></button>
			</div>
		{/each}
		<span><Button size="sm" variant="secondary" onclick={() => setList('mapping', [...mappings, { source: '', target: controls[0] ?? '' }])} data-testid="preset-add-mapping">{m['catalog.preset_add_mapping']()}</Button></span>
		<p class="sc-caption muted">{m['catalog.preset_source_hint']()}</p>
		{#if err('mapping')}<p class="sc-error">{err('mapping')}</p>{/if}
	</fieldset>
{/if}

<RightsField {data} hint={m['catalog.preset_rights_hint']()} onchange={(next) => (data = next)} testid="preset-rights" />

<style>
	.muted {
		color: var(--text-muted);
	}
	.group {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		margin: 0 0 var(--space-15);
	}
	.group p {
		margin: 0;
	}
	.problems {
		margin: 0 0 var(--space-15);
		padding-left: 1rem;
	}
	.value {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 7rem minmax(5rem, auto);
		gap: var(--space-05);
		align-items: center;
	}
	.value .wide {
		grid-column: 1 / -1;
	}
	.path {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	.linked .path {
		color: var(--text-muted);
	}
	.row,
	.pair {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto;
		gap: var(--space-05);
		align-items: center;
	}
	.pair {
		grid-template-columns: minmax(0, 1fr) minmax(0, 1fr) auto;
	}
	.mapping {
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(0, 1fr) 5rem 5rem auto;
		gap: var(--space-05);
		align-items: center;
	}
	details {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
	}
	@media (max-width: 767px) {
		.mapping {
			grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
		}
	}
</style>
