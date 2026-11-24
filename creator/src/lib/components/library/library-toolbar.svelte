<script lang="ts">
	import { IconButton, Select, type SelectOption } from '@scutoons/ui/editor';
	import { TILE_SIZE, library } from '$lib/editor/library.svelte.ts';
	import { useShelf } from '$lib/editor/library-shelf.svelte.ts';
	import { SHELF_GROUPINGS, SHELF_SORTS, TYPE_CHOICES, type ShelfGrouping, type ShelfSort } from '$lib/editor/library-list.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The Library's view tools (lap2 C12, F1): the type – one quiet select, "All types", then the things and the files in
	 * two groups –, Grouping and Sort as quiet selects, the tile size (a continuous step, remembered), tiles / list. A pick
	 * offers only the types its slot takes (`only`).
	 */
	let { only }: { only?: readonly string[] } = $props();
	const shelf = useShelf();
	const types = $derived<SelectOption[]>(
		TYPE_CHOICES.filter((c) => !only || only.includes(c.value)).map((c) => ({
			value: c.value,
			label: c.group === 'files' ? m[`library.kinds_${c.value as 'image'}`]() : c.group === 'things' ? m[`library.types_${c.value as 'toon'}`]() : m['library.all_types'](),
			group: c.group === 'things' ? m['library.things']() : c.group === 'files' ? m['library.files']() : undefined
		}))
	);
	const groupings = $derived<SelectOption[]>(SHELF_GROUPINGS.map((g) => ({ value: g, label: m[`library.group_by_${g}`]() })));
	const sorts = $derived<SelectOption[]>(SHELF_SORTS.map((s) => ({ value: s, label: m[`library.sort_${s}`]() })));
</script>

<div class="tools" data-testid="library-tools">
	<Select quiet options={types} value={shelf.choice} label={m['library.type']()} onchange={(v) => ((shelf.choice = v), shelf.leave())} data-testid="library-type" />
	<Select
		quiet
		options={groupings}
		value={shelf.grouping}
		label={m['library.grouping']()}
		onchange={(v) => {
			shelf.grouping = v as ShelfGrouping;
			shelf.remember();
		}}
		data-testid="library-grouping"
	/>
	<Select
		quiet
		options={sorts}
		value={shelf.sort}
		label={m['library.sort']()}
		onchange={(v) => {
			shelf.sort = v as ShelfSort;
			shelf.remember();
		}}
		data-testid="library-sort"
	/>
	<span class="sp"></span>
	{#if !library.list}
		<input
			class="size"
			type="range"
			min={TILE_SIZE.min}
			max={TILE_SIZE.max}
			step={TILE_SIZE.step}
			value={library.tile}
			oninput={(e) => library.setView({ tile: Number(e.currentTarget.value) })}
			aria-label={m['library.tile_size']()}
			title={m['library.tile_size']()}
			data-testid="library-tile-size"
		/>
	{/if}
	<IconButton icon="list" label={m['library.as_list']()} pressed={library.list} onclick={() => library.setView({ list: !library.list })} />
</div>

<style>
	.tools {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.25rem;
	}
	.sp {
		flex: 1 1 auto;
	}
	.size {
		width: 3rem;
	}
</style>
