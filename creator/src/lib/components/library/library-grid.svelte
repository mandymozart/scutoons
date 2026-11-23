<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import type { AssetFamily, LibraryGroup } from '@scutoons/media';
	import { library } from '$lib/editor/library.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import AssetTile from '$lib/components/library/asset-tile.svelte';

	/**
	 * The core's groups as the grid shows them, the panel's and the browser's: in All the open item's files pinned and
	 * framed on top, then the unused, one group per other item, the toon's – each but the first folded until a search; a
	 * frame (a video and its subtitles, a model and its parts) draws its families together.
	 */
	let { tile, list = false, onmenu, onuse }: { tile: number; list?: boolean; onmenu: (x: number, y: number) => void; onuse: (family: AssetFamily) => void } = $props();

	const groups = $derived(library.view?.groups ?? []);
	const single = $derived(groups.length === 1 && !groups[0].pinned);

	/** an item's address in words: `chapter:catpope:3` → Chapter 3 */
	function itemLabel(address: string | null): string {
		if (!address) return '';
		const [kind, a = '', b = ''] = address.split(':');
		switch (kind) {
			case 'chapter':
			case 'storyboard':
				return m['library.item_chapter']({ number: b });
			case 'season':
				return m['library.item_season']({ number: b });
			case 'character':
				return a.replace(/[-_]+/g, ' ');
			case 'catalog':
				return m['library.scope_items']();
			case 'map':
				return m['library.scope_map']();
			case 'toon':
				return a;
		}
		return address;
	}

	function label(g: LibraryGroup): string {
		switch (g.what) {
			case 'open':
				return library.indexed ? m['library.group_open']({ what: itemLabel(g.item) }) : m['library.group_home']({ what: itemLabel(g.item) });
			case 'unused':
				return m['library.scope_unused']();
			case 'item':
				return itemLabel(g.item);
			case 'toon':
				return m['library.group_toon']({ toon: itemLabel(g.item) });
			case 'other':
				return m['library.group_other']();
			case 'kind':
				return m[`library.kinds_${g.item as AssetFamily['kind']}`]();
			default:
				return m['library.group_all']();
		}
	}
</script>

{#each groups as g (g.key)}
	{@const folded = library.query.trim() ? false : library.folded(g)}
	<section class="group" class:pinned={g.pinned} data-testid="library-group" data-group={g.key}>
		{#if !single}
			<button type="button" class="head" data-control-exempt="fold header" aria-expanded={!folded} onclick={() => library.toggleFold(g)}>
				<Icon name={folded ? 'chevron-right' : 'chevron-down'} size="0.8rem" />
				<span class="label">{label(g)}</span>
				<span class="n">{g.count}</span>
			</button>
		{/if}
		{#if !folded}
			<div class="tiles" class:list role="listbox" aria-multiselectable="true" aria-label={label(g)} style:--tile="{tile}rem">
				{#each g.cells as cell (cell.keys[0])}
					{#if cell.frame}
						<div class="frame" title={m['library.frame']({ kind: m[`library.kind_${cell.frame}`]() })} data-testid="library-frame">
							{#each cell.keys as key (key)}
								{@const f = library.byKey.get(key)}
								{#if f}
									<AssetTile family={f} row={list} stacked={!library.single} selected={library.chosen.includes(key)} fresh={library.fresh === key} onpress={(k) => library.click(key, k)} onuse={() => onuse(f)} {onmenu} />
								{/if}
							{/each}
						</div>
					{:else}
						{@const f = library.byKey.get(cell.keys[0])}
						{#if f}
							<AssetTile family={f} row={list} stacked={!library.single} selected={library.chosen.includes(f.key)} fresh={library.fresh === f.key} onpress={(k) => library.click(f.key, k)} onuse={() => onuse(f)} {onmenu} />
						{/if}
					{/if}
				{/each}
			</div>
		{/if}
	</section>
{/each}

<style>
	.group {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
	}
	.group.pinned {
		padding: 0.4rem;
		border: var(--editor-edge) solid var(--tool-pressed);
		border-radius: var(--editor-radius);
	}
	.head {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		min-height: var(--editor-tree-row);
		padding: 0;
		border: 0;
		background: none;
		color: var(--tool-muted);
		font-size: var(--editor-label);
		letter-spacing: var(--editor-label-spacing, 0.08em);
		text-transform: uppercase;
		cursor: pointer;
	}
	.pinned .head {
		color: var(--tool-bright);
	}
	.label {
		flex: 1 1 auto;
		min-width: 0;
		overflow: hidden;
		text-align: start;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.n {
		font-family: var(--editor-mono, ui-monospace, monospace);
	}
	.tiles {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(var(--tile), 1fr));
		gap: 0.45rem;
	}
	.tiles.list {
		grid-template-columns: 1fr;
		gap: 0;
	}
	.frame {
		grid-column: span 2;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(calc(var(--tile) - 1rem), 1fr));
		gap: 0.3rem;
		padding: 0.3rem;
		border: var(--editor-edge) dashed var(--tool-line);
		border-radius: var(--editor-radius);
	}
	.list .frame {
		grid-column: auto;
		grid-template-columns: 1fr;
	}
</style>
