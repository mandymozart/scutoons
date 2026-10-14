<script lang="ts">
	import { MediaSlot } from '@scutoons/media';
	import type { Data } from '../kinds.ts';
	import { m } from '../messages.ts';
	import CatalogField from './catalog-field.svelte';
	import CheckField from './check-field.svelte';

	/**
	 * Furniture (agents/research/CAVE.md D4, workflows/home/furniture.md): its size in a home – the width in % of the
	 * room's front (the room = 4 m: metres ÷ 4 × 100) – its picture as it stands in a home (three-quarter view, its back
	 * to the left wall) and whether it is a workbench (the workshop recipes run at it). Uploads go to scutoons/furniture/.
	 */
	let { value, error, onchange }: { value: Data | null | undefined; error?: string | null; onchange: (next: Data) => void } = $props();

	let bust = $state(0);
	const f = $derived<Data>(value ?? { size: 15, image: '' });
	const set = (patch: Data) => onchange({ ...f, ...patch });
</script>

<fieldset class="furniture" data-testid="item-furniture">
	<legend>{m['catalog.furniture']()}</legend>
	<CatalogField
		label={m['catalog.furniture_size']()}
		type="number"
		value={f.size}
		hint={m['catalog.furniture_size_hint']()}
		onchange={(v) => set({ size: Math.min(60, Math.max(5, Number(v) || 15)) })}
		testid="item-furniture-size"
	/>
	<MediaSlot toon="" value={f.image || undefined} label={m['catalog.furniture_image']()} place="scutoons/furniture/" {bust} onchange={(path) => (set({ image: path ?? '' }), (bust = Date.now()))} testid="item-furniture-image" />
	<CheckField label={m['catalog.furniture_workshop']()} checked={f.workshop === true} onchange={(on) => set({ workshop: on || undefined })} testid="item-furniture-workshop" />
	{#if error}<p class="sc-error">{error}</p>{/if}
</fieldset>

<style>
	.furniture {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		margin-bottom: var(--space-15);
	}
</style>
