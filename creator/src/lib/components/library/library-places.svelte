<script lang="ts">
	import type { LibraryScope } from '@scutoons/media';
	import { parseEntityKey } from '@scutoons/content';
	import { library } from '$lib/editor/library.svelte.ts';
	import { fileLabel } from '$lib/labels.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The browser's places (OSCT's sidebar): the scope, recently added (7 days), one place per item with its count (the
	 * chapters, characters … that use or hold files), one per season cast (its characters' files). The counts are the core's.
	 */
	const view = $derived(library.view);
	const scopes = $derived<{ id: LibraryScope; label: string; count: number }[]>([
		...(library.address && !library.address.startsWith('toon:') ? [{ id: 'context' as const, label: m['library.place_this']({ what: library.address }), count: view?.scopes.context ?? 0 }] : []),
		...(library.toon ? [{ id: 'toon' as const, label: m['library.place_toon']({ toon: library.toon }), count: view?.scopes.toon ?? 0 }] : []),
		{ id: 'all', label: m['library.place_all'](), count: view?.scopes.all ?? 0 },
		{ id: 'unused', label: m['library.scope_unused'](), count: view?.scopes.unused ?? 0 }
	]);
	const special = (s: string) => (library.special = library.special === s ? null : s);
	/** `chapter:catpope:3` → `chapter catpope 3` */
	const words = (address: string) => address.replace(/[:_-]+/g, ' ');
	/** `season:catpope:1` → `catpope · Season 1` */
	const castLabel = (address: string) => [parseEntityKey(address)?.toon, fileLabel(address)].filter(Boolean).join(' · ');
</script>

<nav class="places" aria-label={m['library.places']()} data-testid="library-places">
	<h3>{m['library.scope']()}</h3>
	<ul>
		{#each scopes as s (s.id)}
			<li><button type="button" class="place" data-control-exempt="tree row" class:on={library.scope === s.id && !library.special} onclick={() => library.setScope(s.id)}>{s.label}<span class="n">{s.count}</span></button></li>
		{/each}
	</ul>
	<h3>{m['library.places']()}</h3>
	<ul>
		<li><button type="button" class="place" data-control-exempt="tree row" class:on={library.special === 'recent'} onclick={() => special('recent')} data-testid="place-recent">{m['library.recent']()}<span class="n">{view?.specials.recent ?? 0}</span></button></li>
	</ul>
	{#if view?.specials.items.length}
		<h3>{m['library.by_item']()}</h3>
		<ul data-testid="place-items">
			{#each view.specials.items as [item, n] (item)}
				<li><button type="button" class="place" data-control-exempt="tree row" class:on={library.special === `item:${item}`} title={item} onclick={() => special(`item:${item}`)}><span class="label">{words(item)}</span><span class="n">{n}</span></button></li>
			{/each}
		</ul>
	{/if}
	{#if view?.specials.casts.length}
		<h3>{m['library.by_cast']()}</h3>
		<ul data-testid="place-casts">
			{#each view.specials.casts as [cast, n] (cast)}
				<li><button type="button" class="place" data-control-exempt="tree row" class:on={library.special === `cast:${cast}`} title={cast} onclick={() => special(`cast:${cast}`)}><span class="label">{castLabel(cast)}</span><span class="n">{n}</span></button></li>
			{/each}
		</ul>
	{/if}
</nav>

<style>
	.places {
		overflow: auto;
		padding: 0.3rem 0.5rem var(--editor-pad) var(--editor-pad);
		border-right: var(--editor-edge) solid var(--tool-line);
		font-size: var(--editor-small);
	}
	h3 {
		margin: 0.8rem 0 0.2rem;
		color: var(--tool-muted);
		font-size: var(--editor-label);
		font-weight: 400;
		letter-spacing: var(--editor-label-spacing, 0.08em);
		text-transform: uppercase;
	}
	ul {
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.place {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		width: 100%;
		min-height: var(--editor-tree-row);
		padding: 0 0.3rem;
		border: 0;
		border-radius: var(--editor-radius);
		background: none;
		color: var(--tool-text);
		font: inherit;
		text-align: start;
		cursor: pointer;
	}
	.place:hover {
		background: var(--tool-hover);
	}
	.place.on {
		background: var(--tool-pressed);
		color: var(--tool-pressed-text);
	}
	.label {
		flex: 1 1 auto;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.n {
		margin-inline-start: auto;
		color: var(--tool-muted);
		font-family: var(--editor-mono, ui-monospace, monospace);
		font-size: var(--editor-label);
	}
</style>
