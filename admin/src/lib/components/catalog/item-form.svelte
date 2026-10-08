<script lang="ts">
	import { AVATAR_SLOTS, FX_SOUNDS, READER_EFFECTS } from '@scutoons/content';
	import { MediaSlot } from '@scutoons/media';
	import { problemFor, text, type CatalogFormProps, type Data } from '$lib/catalog/kinds.ts';
	import { m } from '$lib/paraglide/messages.js';
	import CatalogField from '$lib/components/catalog/catalog-field.svelte';
	import CatalogRows from '$lib/components/catalog/catalog-rows.svelte';
	import CheckField from '$lib/components/catalog/check-field.svelte';
	import LocaleText from '$lib/components/catalog/locale-text.svelte';
	import RespawnField from '$lib/components/catalog/respawn-field.svelte';

	/**
	 * An item (agents/ITEMS.md "Item definition"): texts per language, kind / category, the picture (a slot; uploads go
	 * to <toon>/items/), the flags, slot / edition / product / album, variants (each with its picture) and when a placed
	 * one reappears. Skills, effects, wear, grantable … are the editor's Advanced JSON.
	 */
	let { data = $bindable(), id, lists, problems }: CatalogFormProps = $props();

	const CATEGORIES = ['tool', 'consumable', 'wearable', 'collectible', 'media', 'substance', 'food'];
	const FORMATS = ['vinyl', 'tape', 'cd'];
	const options = (kind: 'toons' | 'editions' | 'products' | 'albums') =>
		(lists[kind] ?? []).map((e) => ({ value: e.id, label: `${text(e.data?.name ?? e.data?.title) || e.id}` }));
	const soundOptions = [{ value: 'none', label: m['catalog.pickup_sound_none']() }, ...FX_SOUNDS.map((s) => ({ value: s }))];
	const place = $derived(`${data.toon || 'scutoons'}/items/`);
	let bust = $state(0);

	const variants = $derived<Data[]>(Array.isArray(data.variants) ? data.variants : []);
	const newVariant = (): Data => ({ id: '', name: { de: '', en: '' }, icon: null, ...(data.product ? { product: data.product } : {}) });
	const err = (...words: string[]) => problemFor(problems, ...words);
	/** a limit field: a whole number, empty = none (the key goes) */
	function setLimit(key: 'max' | 'placements' | 'max_world', value: unknown) {
		if (value === null || value === undefined || value === '') delete data[key];
		else data[key] = typeof value === 'number' ? Math.round(value) : value;
	}
</script>

<CatalogField label={m['catalog.toon']()} type="select" value={data.toon} options={options('toons')} none={m['catalog.toon_none']()} error={err('toon')} onchange={(v) => (data.toon = v)} testid="item-toon" />
<LocaleText label={m['catalog.name']()} required value={data.name} error={err('name')} onchange={(v) => (data.name = v)} testid="item-name" />
<LocaleText label={m['catalog.description']()} multiline value={data.description} onchange={(v) => (data.description = v)} testid="item-description" />

<div class="sc-fields" data-cols="fit" style:--fields-fit-min="10rem">
	<CatalogField
		label={m['catalog.kind']()}
		type="select"
		value={data.kind}
		options={[
			{ value: 'virtual', label: m['catalog.kind_virtual']() },
			{ value: 'physical', label: m['catalog.kind_physical']() }
		]}
		error={err('kind')}
		onchange={(v) => (data.kind = v)}
	/>
	<CatalogField label={m['catalog.category']()} type="select" value={data.category} options={CATEGORIES.map((c) => ({ value: c }))} error={err('category')} onchange={(v) => (data.category = v)} testid="item-category" />
</div>

<div class="picture">
	<MediaSlot toon="" value={data.icon ?? undefined} label={m['catalog.picture']()} {place} {bust} onchange={(path) => ((data.icon = path ?? null), (bust = Date.now()))} testid="item-icon" />
</div>

<fieldset class="flags">
	<legend>{m['catalog.flags']()}</legend>
	<CheckField label={m['catalog.stackable']()} checked={!!data.stackable} onchange={(v) => (data.stackable = v)} />
	<CheckField label={m['catalog.starter']()} checked={!!data.starter} onchange={(v) => (data.starter = v)} />
	<!-- a story may take it (VN takeItem, owner 2026-10-07 – BACKLOG 10.38); a physical item never: only claims move those -->
	{#if data.kind !== 'physical'}
		<CheckField label={m['catalog.takeable']()} checked={data.takeable === true} onchange={(v) => (data.takeable = v || undefined)} testid="item-takeable" />
	{/if}
</fieldset>

<CatalogField label={m['catalog.slot']()} type="select" value={data.slot} options={AVATAR_SLOTS.map((s) => ({ value: s }))} none={m['catalog.slot_none']()} error={err('slot')} onchange={(v) => (data.slot = v)} />
<CatalogField label={m['common.edition']()} type="select" value={data.edition_id} options={options('editions')} none={m['catalog.none']()} error={err('edition', 'edition_id')} onchange={(v) => (data.edition_id = v)} />
<CatalogField label={m['catalog.product']()} type="select" value={data.product} options={options('products')} none={m['catalog.none']()} error={err('product')} onchange={(v) => (data.product = v)} />

<div class="sc-fields" data-cols="fit" style:--fields-fit-min="10rem">
	<CatalogField
		label={m['catalog.album']()}
		type="select"
		value={data.album}
		options={options('albums')}
		none={m['catalog.none']()}
		error={err('album')}
		onchange={(v) => {
			if (v === null) delete data.album;
			else data.album = v;
		}}
	/>
	<CatalogField
		label={m['catalog.format']()}
		type="select"
		value={data.format}
		options={FORMATS.map((f) => ({ value: f }))}
		none={m['catalog.none']()}
		error={err('format')}
		onchange={(v) => {
			if (v === null) delete data.format;
			else data.format = v;
		}}
	/>
</div>

<CatalogRows
	legend={m['catalog.variants']()}
	items={variants}
	onchange={(next) => (data.variants = next.length ? next : null)}
	create={newVariant}
	addLabel={m['catalog.add_variant']()}
	removeLabel={m['catalog.remove_variant']()}
	error={err('variant', 'variants')}
	testid="item-variants"
	rowTestid="item-variant"
	addTestid="item-add-variant"
>
	{#snippet head(v)}<CatalogField label="id" value={v.id} onchange={(x) => (v.id = x)} testid="item-variant-id" />{/snippet}
	{#snippet row(v)}
		<LocaleText label={m['catalog.name']()} value={v.name} onchange={(x) => (v.name = x)} />
		<CatalogField
			label={m['catalog.product']()}
			type="select"
			value={v.product}
			options={options('products')}
			none={m['catalog.none']()}
			onchange={(x) => {
				if (x === null) delete v.product;
				else v.product = x;
			}}
		/>
		<MediaSlot toon="" value={v.icon ?? undefined} label={m['catalog.picture']()} {place} {bust} onchange={(path) => ((v.icon = path ?? null), (bust = Date.now()))} testid="item-variant-icon" />
	{/snippet}
	{#snippet footer()}{#if id && variants.length}<p class="sc-caption muted">{m['catalog.variant_files']({ id })}</p>{/if}{/snippet}
</CatalogRows>

<!-- item limits across the system (BACKLOG 10.39, ITEMS.md "Item limits"): empty = unlimited -->
<div class="sc-fields" data-cols="fit" style:--fields-fit-min="12rem">
	<CatalogField label={m['catalog.max']()} type="number" value={data.max} hint={m['catalog.max_hint']()} error={err('max')} onchange={(v) => setLimit('max', v)} testid="item-max" />
	<CatalogField label={m['catalog.placements']()} type="number" value={data.placements} hint={m['catalog.placements_hint']()} error={err('placements')} onchange={(v) => setLimit('placements', v)} testid="item-placements" />
	<CatalogField label={m['catalog.max_world']()} type="number" value={data.max_world} hint={m['catalog.max_world_hint']()} error={err('max_world')} onchange={(v) => setLimit('max_world', v)} testid="item-max-world" />
</div>

<RespawnField
	value={data.respawn}
	onchange={(rule) => {
		if (rule === null) delete data.respawn;
		else data.respawn = rule;
	}}
/>

<!-- agents/FX.md: the sound when a picked-up placement lands in the bag (a placement may choose its own) -->
<CatalogField label={m['catalog.pickup_sound']()} type="select" value={data.sound} options={soundOptions} none={m['catalog.pickup_sound_default']()} hint={m['catalog.pickup_sound_hint']()} error={err('sound')} onchange={(v) => (v ? (data.sound = v) : delete data.sound)} testid="item-sound" />

<!-- the reader effect it starts when used (owner 2026-10-08: "configure these effects per item" – content READER_EFFECTS) -->
<CatalogField label={m['catalog.reader_effect']()} type="select" value={data.effect?.kind ?? null} options={READER_EFFECTS.map((k) => ({ value: k, label: m[`catalog.reader_effect_${k}`]() }))} none={m['catalog.none']()} hint={m['catalog.reader_effect_hint']()} error={err('effect')} onchange={(v) => (v ? (data.effect = { kind: v, duration_min: data.effect?.duration_min ?? 30 }) : delete data.effect)} testid="item-effect" />
{#if data.effect}
	<CatalogField label={m['catalog.reader_effect_minutes']()} type="number" value={data.effect.duration_min} onchange={(v) => (data.effect = { ...data.effect, duration_min: Number(v) > 0 ? Number(v) : 30 })} testid="item-effect-minutes" />
{/if}
<!-- sobering up (owner 2026-10-08): each piece used takes this share off every running effect – shown in %, stored as 0…1 -->
<CatalogField label={m['catalog.sober']()} type="number" value={data.sober != null ? Math.round(data.sober * 100) : null} hint={m['catalog.sober_hint']()} error={err('sober')} onchange={(v) => (Number(v) > 0 && Number(v) < 100 ? (data.sober = Number(v) / 100) : delete data.sober)} testid="item-sober" />
<!-- a wearable's tint while worn (owner 2026-10-08: sunglasses make everything orange) -->
<CatalogField label={m['catalog.worn_tint']()} value={data.worn_tint ?? null} placeholder="#ff9a2e" hint={m['catalog.worn_tint_hint']()} error={err('worn_tint')} onchange={(v) => (v ? (data.worn_tint = String(v).trim()) : delete data.worn_tint)} testid="item-worn-tint" />

<style>
	.picture {
		margin-bottom: var(--space-15);
	}
	.flags {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05) var(--space-15);
		margin-bottom: var(--space-15);
	}
</style>
