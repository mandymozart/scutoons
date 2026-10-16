<script lang="ts">
	import { MediaSlot, ModelPreview } from '@scutoons/media';
	import type { Data } from '../kinds.ts';
	import { m } from '../messages.ts';
	import CatalogField from './catalog-field.svelte';
	import CheckField from './check-field.svelte';

	/**
	 * A record's 3D look (ITEMS.md "3D item models" – owner 2026-10-16: "Shared 3D-Modelle"): the record and sleeve are
	 * one model built in code; an item gives its pictures (WebP slots, uploads go to <toon>/items/) and colour – `vinyl`.
	 * The live preview builds it. It wins over a `.glb` model.
	 */
	let { data = $bindable(), place }: { data: Data; place: string } = $props();

	let bust = $state(0);
	const PICTURES = [
		['sleeveFront', m['catalog.vinyl_front']()],
		['sleeveBack', m['catalog.vinyl_back']()],
		['sleeveSpine', m['catalog.vinyl_spine']()],
		['labelA', m['catalog.vinyl_label_a']()],
		['labelB', m['catalog.vinyl_label_b']()]
	] as const;
	function set(key: string, value: string | null | undefined) {
		const next = { ...data.vinyl };
		if (value) next[key] = value;
		else delete next[key];
		data.vinyl = next;
	}
</script>

<div class="vinyl">
	<CheckField label={m['catalog.vinyl']()} checked={!!data.vinyl} onchange={(on) => (on ? (data.vinyl = { format: 'lp12' }) : delete data.vinyl)} testid="item-vinyl" />
	{#if data.vinyl}
		<p class="sc-caption muted">{m['catalog.vinyl_hint']()}</p>
		<div class="slots">
			{#each PICTURES as [key, label] (key)}
				<MediaSlot toon="" value={data.vinyl[key] ?? undefined} {label} {place} {bust} onchange={(path) => (set(key, path), (bust = Date.now()))} testid={`item-vinyl-${key}`} />
			{/each}
		</div>
		<CatalogField label={m['catalog.vinyl_color']()} value={data.vinyl.color ?? null} placeholder="#000000" onchange={(x) => set('color', x ? String(x).trim() : null)} testid="item-vinyl-color" />
		<ModelPreview vinyl={data.vinyl} testid="item-vinyl-preview" />
	{/if}
</div>

<style>
	.slots {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-1);
	}

	.vinyl {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		margin-bottom: var(--space-15);
	}
</style>
