<script lang="ts">
	import { FILE_ORIGINS } from '@scutoons/api';
	import { ListRow } from '@scutoons/ui/editor';
	import { getCreatorNav } from '$lib/nav.svelte.ts';
	import { library } from '$lib/editor/library.svelte.ts';
	import { shelf } from '$lib/editor/library-shelf.svelte.ts';
	import { placeAvailable, seasonToon, type PlaceContext, type ShelfPlace } from '$lib/editor/library-list.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The Library's places as rows (lap2 C12b, the panel taken full): This item · This toon · Everything · Recently added ·
	 * Unused, then By cast – every toon's seasons, each listing its characters and their files – then From. The selected
	 * one on the selected ground (no side bar, RULES H10).
	 */
	let { ctx, place }: { ctx: PlaceContext; place: ShelfPlace } = $props();
	const nav = getCreatorNav();

	$effect(() => {
		for (const t of nav.toons) shelf.loadToon(t.id);
	});

	const rows = $derived(
		(['item', 'toon', 'all', 'recent', 'unused'] as const)
			.filter((p) => placeAvailable(p, ctx))
			.map((p) => ({ place: p, label: p === 'toon' && ctx.toon ? m['library.place_toon_named']({ toon: nav.toonName(ctx.toon) }) : m[`library.place_${p}`]() }))
	);
	const casts = $derived(nav.toons.flatMap((t) => (shelf.toons[t.id]?.seasons ?? []).map((s) => ({ season: s.address, label: `${nav.toonName(seasonToon(s.address))} · ${s.name}` }))));
</script>

<nav class="places" aria-label={m['library.places']()} data-testid="library-places">
	<h3>{m['library.places']()}</h3>
	{#each rows as r (r.place)}
		<ListRow dense label={r.label} selected={place === r.place} onclick={() => shelf.setPlace(r.place, library.address)} data-testid="place-{r.place}" />
	{/each}
	{#if casts.length}
		<h3>{m['library.by_cast']()}</h3>
		{#each casts as c (c.season)}
			<ListRow dense label={c.label} title={c.season} selected={place === 'cast' && ctx.season === c.season} onclick={() => shelf.setPlace('cast', library.address, c.season)} data-testid="place-cast" />
		{/each}
	{/if}
	<h3>{m['library.from']()}</h3>
	<ListRow dense label={m['library.from_all']()} selected={!shelf.from} onclick={() => (shelf.from = null)} />
	{#each FILE_ORIGINS as o (o)}
		<ListRow dense label={m[`library.from_${o}`]()} selected={shelf.from === o} onclick={() => (shelf.from = o)} />
	{/each}
</nav>

<style>
	.places {
		display: flex;
		flex-direction: column;
		overflow: auto;
		padding: 0 var(--editor-pad) var(--editor-pad) 0;
		border-right: var(--editor-edge) solid var(--tool-line);
	}
	h3 {
		margin: 0.8rem 0 0.3rem;
		color: var(--tool-muted);
		font-size: var(--editor-label);
		font-weight: 400;
		letter-spacing: var(--editor-label-spacing, 0.08em);
		text-transform: uppercase;
	}
	h3:first-child {
		margin-top: 0;
	}
</style>
