<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { Path } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';
	import { itemCatalog, itemName, loadCatalog } from '$lib/editor/catalog.svelte.ts';
	import PropField from './prop-field.svelte';
	import VnEvents from './vn-events.svelte';
	import type { CharacterChoices } from './characters/character-ref-field.svelte';

	/**
	 * A catalog item placed on a slice (`Slice.items`): which item, where (x / y / width, rotation – also with the
	 * handles on the canvas) and what a tap does (`action`, the creator's choice: nothing, pick up, or VN events run like
	 * a click trigger's).
	 */
	let {
		doc,
		path,
		editLocale = null,
		characters = [],
		choices,
		background = '',
		onremove
	}: {
		doc: ContentDoc;
		/** path of the placed item */
		path: Path;
		/** geometry scope: null = all locales, else only this locale */
		editLocale?: string | null;
		characters?: { name: string; label?: string }[];
		choices?: CharacterChoices;
		background?: string;
		onremove?: () => void;
	} = $props();

	$effect(() => {
		loadCatalog();
	});

	const placed = $derived(doc.get(doc.primary, path));
	const def = $derived(itemCatalog.catalog?.items.find((i) => i.id === placed?.item));
	const action = $derived(placed?.action ?? 'none');
	const itemOptions = $derived.by(() => {
		const list = (itemCatalog.catalog?.items ?? []).map((i) => ({ value: i.id, label: `${itemName(i, i.id)} · ${i.id}` }));
		if (placed?.item && !list.some((o) => o.value === placed.item)) list.unshift({ value: placed.item, label: placed.item });
		return list;
	});
	const p = (...keys: (string | number)[]) => [...path, ...keys];
</script>

{#if placed}
	<div class="inspector" data-testid="item-inspector">
		<div class="row head">
			<span class="icon">{#if def?.icon}<img src={asset(def.icon)} alt="" />{/if}</span>
			<h3>{itemName(def, placed.item ?? '–')}</h3>
			{#if onremove}<button class="secondary" onclick={onremove}>{m['creator.remove']()}</button>{/if}
		</div>

		<PropField {doc} path={p('item')} label="item" type="select" options={itemOptions} required />

		<div class="grid">
			<PropField {doc} path={p('rect', 'x')} label="x %" type="number" required {editLocale} />
			<PropField {doc} path={p('rect', 'y')} label="y %" type="number" required {editLocale} />
			<PropField {doc} path={p('rect', 'width')} label="width %" type="number" required {editLocale} />
			<PropField {doc} path={p('rotation')} label="rotation °" type="number" step={1} placeholder="0" />
		</div>
		{#if editLocale}<p class="hint">Geometry: only {editLocale} (override).</p>{/if}

		<PropField
			{doc}
			path={p('action')}
			label={m['creator.slice_item_action']()}
			type="select"
			options={[
				{ value: 'pickup', label: m['creator.slice_item_action_pickup']() },
				{ value: 'events', label: m['creator.slice_item_action_events']() }
			]}
			placeholder={m['creator.slice_item_action_none']()}
		/>

		{#if action === 'pickup'}
			<PropField {doc} path={p('qty')} label={m['creator.slice_item_qty']()} type="number" step={1} min={1} placeholder="1" />
			<PropField
				{doc}
				path={p('pickupMode')}
				label={m['creator.slice_item_pickup_mode']()}
				type="select"
				options={[{ value: 'respawn', label: m['creator.slice_item_pickup_respawn']() }]}
				placeholder={m['creator.slice_item_pickup_once']()}
			/>
			{#if placed.pickupMode === 'respawn'}<p class="hint">{m['creator.slice_item_hint_respawn']()}</p>{/if}
			<p class="hint">{m['creator.slice_item_hint_pickup']()}</p>
			{#if def && !def.grantable?.includes('story')}
				<p class="warn" data-testid="item-not-grantable">{m['creator.slice_item_not_grantable']()}</p>
			{/if}
		{:else if action === 'events'}
			<PropField {doc} path={p('once')} label={m['creator.slice_item_once']()} type="bool" placeholder="true" />
			<p class="hint">{m['creator.slice_item_hint_events']()}</p>
			<VnEvents {doc} eventsPath={p('events')} {characters} {choices} {background} />
		{/if}
	</div>
{/if}

<style>
	.inspector {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}
	.row {
		display: flex;
		align-items: center;
		gap: var(--space-05);
	}
	h3 {
		margin: 0;
		flex: 1;
		min-width: 0;
	}
	.icon {
		flex: none;
		width: var(--control-height);
		height: var(--control-height);
		display: inline-flex;
	}
	.icon img {
		width: 100%;
		height: 100%;
		object-fit: contain;
	}
	/* two fields per row: the reader's type size needs the width (DESIGN.md "Brutalist tools") */
	.grid {
		display: grid;
		gap: var(--space-075) var(--space-1);
		grid-template-columns: repeat(2, minmax(0, 1fr));
	}
	.hint {
		margin: 0;
		color: var(--text-muted);
		font-size: var(--type-caption-size);
	}
	.warn {
		margin: 0;
		padding-left: var(--space-05);
		border-left: 0.5rem solid var(--state-warning);
	}
</style>
