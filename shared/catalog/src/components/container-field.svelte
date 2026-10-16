<script lang="ts">
	import { Button, CloseIcon } from '@scutoons/ui';
	import { MEDIA_FORMATS } from '@scutoons/content';
	import { text, type Data, type CatalogLists } from '../kinds.ts';
	import { m } from '../messages.ts';
	import CatalogField from './catalog-field.svelte';
	import CheckField from './check-field.svelte';

	/**
	 * "Holds" – makes an item a container (agents/research/SCORES.md §2b, owner 2026-10-14: "a simple rule we can edit"):
	 * the categories, the items and / or the sound-carrier formats that fit (a CD case takes `cd` – research/PACKING.md §3),
	 * how many, and an optional overfill (past the capacity after a warning; overfilled for its days it breaks into another
	 * item). Empty = not a container (`holds` goes).
	 */
	let { value, categories, lists, error = null, onchange }: {
		value: Data | null | undefined;
		categories: string[];
		lists: CatalogLists;
		error?: string | null;
		onchange: (holds: Data | null) => void;
	} = $props();

	const holds = $derived<Data>(value ?? {});
	const cats = $derived<string[]>(Array.isArray(holds.categories) ? holds.categories : []);
	const listed = $derived<string[]>(Array.isArray(holds.items) ? holds.items : []);
	const formats = $derived<string[]>(Array.isArray(holds.formats) ? holds.formats : []);
	const itemOptions = $derived((lists.items ?? []).map((e) => ({ value: e.id, label: `${text(e.data?.name) || e.id} · ${e.id}` })));
	const nameOf = (id: string) => itemOptions.find((o) => o.value === id)?.label ?? `${id} (?)`;
	const whole = (v: unknown) => (v === null || v === '' || v === undefined ? undefined : Math.max(1, Math.round(Number(v))));

	/** writes the rule back; nothing that fits = not a container */
	function set(next: Data) {
		const clean: Data = {};
		if (next.categories?.length) clean.categories = next.categories;
		if (next.items?.length) clean.items = next.items;
		if (next.formats?.length) clean.formats = next.formats;
		if (next.max != null) clean.max = next.max;
		if (next.overfill) clean.overfill = next.overfill;
		onchange(clean.categories || clean.items || clean.formats ? clean : null);
	}
	const toggle = (c: string, on: boolean) => set({ ...holds, categories: on ? [...cats, c] : cats.filter((x) => x !== c) });
	const toggleFormat = (f: string, on: boolean) => set({ ...holds, formats: on ? [...formats, f] : formats.filter((x) => x !== f) });
	const overfill = $derived<Data | null>(holds.overfill ?? null);
</script>

<fieldset class="holds" data-testid="item-holds">
	<legend>{m['catalog.holds']()}</legend>
	<p class="sc-caption muted">{m['catalog.holds_hint']()}</p>
	<div class="cats">
		{#each categories as c (c)}
			<CheckField label={c} checked={cats.includes(c)} onchange={(on) => toggle(c, on)} testid="item-holds-category-{c}" />
		{/each}
	</div>
	<div class="cats" role="group" aria-label={m['catalog.holds_formats']()}>
		<span class="sc-caption">{m['catalog.holds_formats']()}</span>
		{#each MEDIA_FORMATS as f (f)}
			<CheckField label={f} checked={formats.includes(f)} onchange={(on) => toggleFormat(f, on)} testid="item-holds-format-{f}" />
		{/each}
	</div>
	{#each listed as id, i (id)}
		<div class="row">
			<span>{nameOf(id)}</span>
			<button type="button" class="sc-round" title={m['catalog.remove']()} aria-label={m['catalog.remove']()} onclick={() => set({ ...holds, items: listed.filter((_, j) => j !== i) })}><CloseIcon /></button>
		</div>
	{/each}
	<CatalogField
		label={m['catalog.holds_items']()}
		type="select"
		value={null}
		options={itemOptions.filter((o) => !listed.includes(o.value))}
		none={m['catalog.choose']()}
		onchange={(v) => v && set({ ...holds, items: [...listed, v] })}
		testid="item-holds-add-item"
	/>
	{#if cats.length || listed.length || formats.length}
		<div class="sc-fields" data-cols="fit" style:--fields-fit-min="10rem">
			<CatalogField label={m['catalog.holds_max']()} type="number" value={holds.max} hint={m['catalog.holds_max_hint']()} onchange={(v) => set({ ...holds, max: whole(v) })} testid="item-holds-max" />
		</div>
		{#if holds.max != null}
			{#if overfill}
				<div class="sc-fields" data-cols="fit" style:--fields-fit-min="10rem">
					<CatalogField label={m['catalog.overfill_max']()} type="number" value={overfill.max} onchange={(v) => set({ ...holds, overfill: { ...overfill, max: whole(v) } })} testid="item-overfill-max" />
					<CatalogField label={m['catalog.overfill_days']()} type="number" value={overfill.days} onchange={(v) => set({ ...holds, overfill: { ...overfill, days: whole(v) } })} testid="item-overfill-days" />
					<CatalogField label={m['catalog.overfill_breaks_into']()} type="select" value={overfill.breaks_into} options={itemOptions} onchange={(v) => set({ ...holds, overfill: { ...overfill, breaks_into: v } })} testid="item-overfill-breaks" />
				</div>
				<span><Button size="sm" variant="secondary" onclick={() => set({ ...holds, overfill: undefined })}>{m['catalog.overfill_remove']()}</Button></span>
			{:else}
				<span><Button size="sm" variant="secondary" onclick={() => set({ ...holds, overfill: { days: 3, max: holds.max + 1, breaks_into: '' } })} data-testid="item-overfill-add">{m['catalog.overfill_add']()}</Button></span>
			{/if}
		{/if}
	{/if}
	{#if error}<p class="sc-error">{error}</p>{/if}
</fieldset>

<style>
	.holds {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		margin-bottom: var(--space-15);
	}
	.cats {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05) var(--space-15);
	}
	.row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-05);
	}
</style>
