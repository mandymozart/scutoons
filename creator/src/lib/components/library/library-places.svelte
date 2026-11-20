<script lang="ts">
	import type { LibraryScope } from '@scutoons/media';
	import { Section } from '@scutoons/ui/editor';
	import { library } from '$lib/editor/library.svelte.ts';
	import { NEEDS } from '$lib/editor/library-actions.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The browser's places (OSCT's sidebar): the scope, recently added (7 days), missing credits, one place per item with
	 * its count (the chapters, characters … that use or hold files); the missing versions on demand. The counts are the
	 * core's.
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
		<li>
			<button type="button" class="place" data-control-exempt="tree row" class:on={library.special === 'credits'} disabled={view?.specials.credits == null} title={view?.specials.credits == null ? NEEDS.credits() : undefined} onclick={() => special('credits')}>{m['library.missing_credits']()}<span class="n">{view?.specials.credits ?? '–'}</span></button>
		</li>
	</ul>
	{#if view?.specials.items.length}
		<h3>{m['library.by_item']()}</h3>
		<ul data-testid="place-items">
			{#each view.specials.items as [item, n] (item)}
				<li><button type="button" class="place" data-control-exempt="tree row" class:on={library.special === `item:${item}`} title={item} onclick={() => special(`item:${item}`)}><span class="label">{words(item)}</span><span class="n">{n}</span></button></li>
			{/each}
		</ul>
	{/if}
	<Section id="library-places-missing" label={m['library.todo_missing']()} open={false} summary={String(view?.specials.missing ?? 0)}>
		<button type="button" class="place" data-control-exempt="tree row" class:on={library.special === 'missing'} onclick={() => special('missing')}>{m['library.show_missing']()}<span class="n">{view?.specials.missing ?? 0}</span></button>
	</Section>
</nav>

<style>
	.places {
		overflow: auto;
		padding: 0.3rem 0.5rem var(--editor-pad) var(--editor-pad);
		border-right: var(--editor-edge) solid var(--editor-line);
		font-size: var(--editor-small);
	}
	h3 {
		margin: 0.8rem 0 0.2rem;
		color: var(--editor-muted);
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
		color: var(--editor-text);
		font: inherit;
		text-align: start;
		cursor: pointer;
	}
	.place:hover:not(:disabled) {
		background: var(--editor-hover);
	}
	.place:disabled {
		color: var(--editor-muted);
		cursor: default;
	}
	.place.on {
		background: var(--editor-pressed);
		color: var(--editor-pressed-text);
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
		color: var(--editor-muted);
		font-family: var(--editor-mono, ui-monospace, monospace);
		font-size: var(--editor-label);
	}
</style>
