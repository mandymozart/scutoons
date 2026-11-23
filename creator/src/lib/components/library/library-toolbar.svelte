<script lang="ts">
	import { Icon, Menu } from '@scutoons/ui';
	import { LIBRARY_GROUPINGS, LIBRARY_SORTS, type LibraryGrouping, type LibrarySort } from '@scutoons/media';
	import { TILE_SIZE, library } from '$lib/editor/library.svelte.ts';
	import { NEEDS } from '$lib/editor/library-actions.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The Library's view tools, the panel's and the browser's: grouping (by use, by kind, none), sort, the tile size (a
	 * continuous step, remembered), tiles / list, the levels (the browser: stacked / single) and ⋯ – where the files
	 * came from, Name by use.
	 */
	let { levels = false }: { levels?: boolean } = $props();
	let more = $state(false);
	const origins = $derived(library.view?.origins ?? []);
</script>

<div class="tools" data-testid="library-tools">
	<select value={library.grouping} onchange={(e) => library.setView({ grouping: e.currentTarget.value as LibraryGrouping })} aria-label={m['library.grouping']()} title={m['library.grouping']()} data-testid="library-grouping">
		{#each LIBRARY_GROUPINGS as g (g)}<option value={g}>{m[`library.group_by_${g}`]()}</option>{/each}
	</select>
	<select value={library.sort} onchange={(e) => library.setView({ sort: e.currentTarget.value as LibrarySort })} aria-label={m['library.sort']()} title={m['library.sort']()}>
		{#each LIBRARY_SORTS as s (s)}<option value={s}>{m[`library.sort_${s}`]()}</option>{/each}
	</select>
	{#if levels}
		<button type="button" class="square" aria-pressed={library.single} title={library.single ? m['library.levels_single']() : m['library.levels_stacked']()} aria-label={m['library.levels']()} onclick={() => library.setView({ single: !library.single })} data-testid="library-levels"><Icon name={library.single ? 'grid' : 'layers'} size="0.9rem" /></button>
	{/if}
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
	<button type="button" class="square" aria-pressed={library.list} title={m['library.as_list']()} aria-label={m['library.as_list']()} onclick={() => library.setView({ list: !library.list })}><Icon name="list" size="0.9rem" /></button>
	<Menu label={m['library.more']()} align="end" bind:open={more}>
		{#snippet trigger()}<Icon name="more-horizontal" size="0.9rem" />{/snippet}
		<span class="lbl">{m['library.from']()}</span>
		<button type="button" role="menuitemradio" aria-checked={!library.origin} onclick={() => (library.origin = null)}>{m['library.from_all']()}</button>
		{#each origins as [o, n] (o)}
			<button type="button" role="menuitemradio" aria-checked={library.origin === o} onclick={() => (library.origin = o)}>{m[`library.from_${o}`]()} <span class="n">{n}</span></button>
		{/each}
		<hr />
		<button type="button" disabled title={NEEDS.server()}>{m['library.name_by_use']()}</button>
	</Menu>
</div>

<style>
	.tools {
		display: flex;
		align-items: center;
		gap: 0.35rem;
	}
	select {
		max-width: 10rem;
	}
	.sp {
		flex: 1 1 auto;
	}
	.size {
		width: 6rem;
	}
	.square {
		flex: none;
		width: var(--control-height);
		border: 0;
		border-radius: var(--editor-radius);
		background: none;
		color: var(--tool-muted);
		display: inline-grid;
		place-items: center;
		cursor: pointer;
	}
	.square[aria-pressed='true'] {
		background: var(--tool-pressed);
		color: var(--tool-pressed-text);
	}
	.lbl {
		padding: 0.2rem 0.5rem;
		color: var(--tool-muted);
		font-size: var(--editor-label);
		text-transform: uppercase;
	}
	[aria-checked='true'] {
		color: var(--tool-text);
	}
	.n {
		opacity: 0.7;
	}
	hr {
		margin: 0.2rem 0;
		border: 0;
		border-top: var(--editor-edge) solid var(--tool-line);
	}
</style>
