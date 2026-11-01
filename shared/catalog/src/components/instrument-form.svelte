<script lang="ts">
	import { Button, CloseIcon } from '@scutoons/ui';
	import { problemFor, type CatalogFormProps, type Data } from '../kinds.ts';
	import { m } from '../messages.ts';
	import { catalogLocales } from '../setup.ts';
	import { INSTRUMENT_KINDS } from '../rigs/validate.ts';
	import { instrumentControls, instrumentOutputs } from '../rigs/edit.ts';
	import { canonical } from '../rigs/graph.ts';
	import CatalogField from './catalog-field.svelte';
	import LocaleText from './locale-text.svelte';
	import RightsField from './rights-field.svelte';

	/**
	 * An instrument, v1 (research/INSTRUMENTS.md §1, deck-space rig/instrument.rs): its kind, name, the audio output
	 * the track takes, the parameters it shows (path, label, a narrower range) and its rights. The node graph (`node`,
	 * `link`) stays the editor's "More fields" JSON – the graph editor is #203.
	 */
	let { data = $bindable(), problems }: CatalogFormProps = $props();

	const locales = catalogLocales();
	const controls = $derived(instrumentControls(data));
	const outputs = $derived(instrumentOutputs(data));
	const expose = $derived<Data[]>(Array.isArray(data.expose) ? data.expose : []);
	const err = (...words: string[]) => problemFor(problems, ...words);
	const number = (raw: string) => (raw.trim() === '' || Number.isNaN(Number(raw)) ? null : Number(raw));

	function setExpose(list: Data[]) {
		const { expose: _old, ...rest } = data;
		data = list.length ? { ...rest, expose: list } : rest;
	}
	const patch = (i: number, p: Data) => setExpose(expose.map((e, j) => (j === i ? { ...e, ...p } : e)));
	function setLabel(i: number, locale: string, value: string) {
		const label: Record<string, string> = { ...(expose[i].label ?? {}) };
		if (value) label[locale] = value;
		else delete label[locale];
		const { label: _old, ...rest } = expose[i];
		setExpose(expose.map((e, j) => (j === i ? (Object.keys(label).length ? { ...rest, label } : rest) : e)));
	}
	function setRange(i: number, which: 0 | 1, raw: string) {
		const v = number(raw);
		const range: (number | null)[] = Array.isArray(expose[i].range) ? [...expose[i].range] : [null, null];
		range[which] = v;
		const { range: _old, ...rest } = expose[i];
		setExpose(expose.map((e, j) => (j === i ? (range[0] === null && range[1] === null ? rest : { ...rest, range: [range[0] ?? 0, range[1] ?? 1] }) : e)));
	}
	const rest = $derived(problems.filter((p) => !problemFor([p], 'kind', 'name', 'out', 'expose', 'rights', 'id', ...controls)));
</script>

{#if rest.length}<ul class="sc-error problems" role="alert" data-testid="instrument-problems">{#each rest as p (p)}<li>{p}</li>{/each}</ul>{/if}

<div class="sc-fields" data-cols="fit" style:--fields-fit-min="10rem">
	<CatalogField label={m['catalog.instrument_kind']()} type="select" value={data.kind} options={INSTRUMENT_KINDS.map((k) => ({ value: k }))} error={err('kind')} onchange={(v) => (data = { ...data, kind: v })} testid="instrument-kind" />
	<CatalogField label={m['catalog.instrument_out']()} type="select" required value={data.out ? canonical(String(data.out)) : null} options={outputs.map((o) => ({ value: o }))} error={err('out')} onchange={(v) => (data = { ...data, out: v ?? '' })} testid="instrument-out" />
</div>
<LocaleText label={m['catalog.name']()} value={data.name} error={err('name')} onchange={(v) => (data = { ...data, name: v })} testid="instrument-name" />

<fieldset class="group">
	<legend>{m['catalog.instrument_expose']()}</legend>
	{#each expose as e, i (i)}
		<div class="expose" data-testid="instrument-expose">
			<div class="head">
				<select value={e.path ?? ''} aria-label={m['catalog.instrument_path']()} onchange={(ev) => patch(i, { path: ev.currentTarget.value })}>
					<option value="">{m['catalog.choose']()}</option>
					{#each controls as c (c)}<option value={c}>{c}</option>{/each}
					{#if e.path && !controls.includes(e.path)}<option value={e.path}>{e.path} (?)</option>{/if}
				</select>
				<button type="button" class="sc-round" title={m['catalog.remove']()} aria-label={m['catalog.remove']()} onclick={() => setExpose(expose.filter((_, j) => j !== i))}><CloseIcon /></button>
			</div>
			<div class="meta">
				{#each locales as locale (locale)}
					<input type="text" value={e.label?.[locale] ?? ''} placeholder="{m['catalog.instrument_label']()} {locale.toUpperCase()}" aria-label="{m['catalog.instrument_label']()} {locale}" onchange={(ev) => setLabel(i, locale, ev.currentTarget.value.trim())} />
				{/each}
				<input type="number" step="any" value={e.range?.[0] ?? ''} placeholder="min" aria-label="{m['catalog.instrument_range']()} min" onchange={(ev) => setRange(i, 0, ev.currentTarget.value)} />
				<input type="number" step="any" value={e.range?.[1] ?? ''} placeholder="max" aria-label="{m['catalog.instrument_range']()} max" onchange={(ev) => setRange(i, 1, ev.currentTarget.value)} />
			</div>
			{#if e.path && err(e.path)}<p class="sc-error">{err(e.path)}</p>{/if}
		</div>
	{/each}
	<span><Button size="sm" variant="secondary" disabled={!controls.length} onclick={() => setExpose([...expose, { path: controls.find((c) => !expose.some((e) => e.path === c)) ?? controls[0] }])} data-testid="instrument-add-expose">{m['catalog.instrument_add_expose']()}</Button></span>
	{#if err('expose')}<p class="sc-error">{err('expose')}</p>{/if}
</fieldset>
<p class="sc-caption muted">{m['catalog.instrument_graph_hint']()}</p>

<RightsField {data} hint={m['catalog.rights_hint']()} onchange={(next) => (data = next)} testid="instrument-rights" />

<style>
	.muted {
		color: var(--text-muted);
		margin: 0 0 var(--space-15);
	}
	.problems {
		margin: 0 0 var(--space-15);
		padding-left: 1rem;
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
	.expose {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		padding: var(--space-05) 0 var(--space-05) var(--space-1);
		border-left: 2px solid var(--ink);
	}
	.head {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto;
		gap: var(--space-05);
		align-items: center;
	}
	.meta {
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(0, 1fr) 5rem 5rem;
		gap: var(--space-05);
	}
	@media (max-width: 767px) {
		.meta {
			grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
		}
	}
</style>
