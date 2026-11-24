<script lang="ts">
	import { untrack } from 'svelte';
	import { page } from '$app/state';
	import { Icon } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { getCreatorNav } from '$lib/nav.svelte.ts';
	import { places } from '$lib/shell/places.svelte.ts';
	import { toonTree } from '$lib/shell/toon-tree.svelte.ts';
	import ChapterBadges from '$lib/components/status/chapter-badges.svelte';

	/**
	 * The Toon tree (lap2 C1): one toon, from its board down – its seasons, each with its chapters and its Cast – then its
	 * script, credits and settings. Every toon is found in the Library, not here.
	 */
	let { toon }: { toon: string } = $props();

	const path = $derived(page.url.pathname);
	const pages = $derived([
		{ href: `/${toon}/script`, label: m['creator.script'](), testid: 'nav-script' },
		{ href: `/${toon}/credits`, label: m['creator.credits'](), testid: 'nav-credits' },
		{ href: `/${toon}/toon`, label: m['creator.toon_settings'](), testid: 'nav-toon-settings' }
	]);
	const nav = getCreatorNav();
	$effect(() => places.load(nav.toons.filter((t) => t.id === toon)));
	const seasons = $derived(places.parts[toon]?.seasons ?? []);
	// read again on every page change: a title or a status edited there shows here
	$effect(() => {
		const [t, numbers] = [toon, seasons];
		void path;
		untrack(() => void toonTree.read(t, numbers));
	});
	const shown = $derived(toonTree.toon === toon ? toonTree.seasons : []);

	let folded = $state<Record<number, boolean>>({});
	const seasonHref = (n: number) => `/${toon}/season/${n}`;
	const castHref = (n: number) => `${seasonHref(n)}/cast`;
	const current = (on: boolean) => (on ? 'page' : undefined);
</script>

<nav class="toon-panel" aria-label={m['creator.toon_tabs']()} data-testid="toon-tree">
	<a class="row" href="/{toon}" aria-current={current(path === `/${toon}`)} data-testid="nav-stories">{m['creator.toon_stories']()}</a>
	{#each shown as s (s.number)}
		{@const open = !folded[s.number]}
		<div class="row season" aria-current={current(path.startsWith(seasonHref(s.number)) && !path.startsWith(castHref(s.number)))}>
			<button type="button" class="fold" aria-expanded={open} title={open ? m['creator.tree_fold']() : m['creator.tree_unfold']()} aria-label={open ? m['creator.tree_fold']() : m['creator.tree_unfold']()} onclick={() => (folded[s.number] = open)}>
				<Icon name={open ? 'chevron-down' : 'chevron-right'} size="0.9rem" />
			</button>
			<a href={seasonHref(s.number)} data-testid="nav-season-{s.number}">{toonTree.seasonLabel(s)}</a>
		</div>
		{#if open}
			{#each s.chapters as n (n)}
				<a class="row child" href="/{toon}/chapter/{n}" aria-current={current(path === `/${toon}/chapter/${n}` || path.startsWith(`/${toon}/chapter/${n}/`))} data-testid="nav-chapter-{n}">
					<span class="num">{n}</span>
					<span class="title">{toonTree.chapterLabel(n)}</span>
					<ChapterBadges season={s.doc} number={n} doc={toonTree.chapters[n]} />
				</a>
			{/each}
			<a class="row child" href={castHref(s.number)} aria-current={current(path === castHref(s.number))} title={m['creator.season_cast_about']()} data-testid="nav-cast-{s.number}">
				<Icon name="users" size="1rem" />
				<span class="title">{m['creator.cast']()}</span>
			</a>
		{/if}
	{/each}
	{#each pages as p (p.href)}
		<a class="row" href={p.href} aria-current={current(path === p.href)} data-testid={p.testid}>{p.label}</a>
	{/each}
</nav>

<style>
	.toon-panel {
		display: flex;
		flex-direction: column;
		gap: 1px;
		/* rows inset in the panel, the hover ground inside it with the radius (the deck's tree row) */
		padding: 0 var(--editor-pad) var(--editor-pad);
	}
	.row {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		min-height: var(--editor-tree-row);
		padding: 0 0.5rem;
		border-radius: var(--editor-radius);
		color: var(--tool-text);
		text-decoration: none;
		text-shadow: var(--tool-halo);
	}
	.row:hover {
		background: var(--tool-hover);
	}
	.row[aria-current='page'] {
		background: var(--tool-selected);
	}
	.season {
		gap: 0.25rem;
		padding-left: 0;
	}
	.season a {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		color: inherit;
		text-decoration: none;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.fold {
		all: unset;
		display: inline-flex;
		flex: none;
		align-items: center;
		justify-content: center;
		width: 1.25rem;
		height: var(--editor-tree-row);
		color: var(--tool-muted);
		cursor: pointer;
	}
	.fold:hover,
	.fold:focus-visible {
		color: var(--tool-text);
	}
	.child {
		padding-left: 1.75rem;
	}
	.num {
		flex: none;
		min-width: 1.2rem;
		color: var(--tool-muted);
		font-variant-numeric: tabular-nums;
	}
	.title {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
