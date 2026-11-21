<script lang="ts">
	import { page } from '$app/state';
	import { entities } from '@scutoons/content';
	import { fileLabel } from '$lib/labels.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { places } from '$lib/shell/places.svelte.ts';

	/** The toon workspace's own pages: its board (seasons › chapters), script, credits, settings, and its seasons */
	let { toon }: { toon: string } = $props();

	const path = $derived(page.url.pathname);
	const pages = $derived([
		{ href: `/${toon}`, label: m['creator.toon_stories'](), testid: 'nav-stories' },
		{ href: `/${toon}/script`, label: m['creator.script'](), testid: 'nav-script' },
		{ href: `/${toon}/credits`, label: m['creator.credits'](), testid: 'nav-credits' },
		{ href: `/${toon}/toon`, label: m['creator.toon_settings'](), testid: 'nav-toon-settings' }
	]);
	const seasons = $derived(places.parts[toon]?.seasons ?? []);
</script>

<nav class="toon-panel" aria-label={m['creator.toon_tabs']()}>
	{#each pages as p (p.href)}
		<a class="row" href={p.href} aria-current={path === p.href ? 'page' : undefined} data-testid={p.testid}>{p.label}</a>
	{/each}
	{#if seasons.length}
		<span class="head">{m['creator.seasons']()}</span>
		{#each seasons as n (n)}
			<a class="row" href="/{toon}/season/{n}" aria-current={path.startsWith(`/${toon}/season/${n}`) ? 'page' : undefined}>{fileLabel(entities.season(toon, n))}</a>
		{/each}
	{/if}
</nav>

<style>
	.toon-panel {
		display: flex;
		flex-direction: column;
		padding: 0 0 var(--editor-pad);
	}
	.row {
		display: flex;
		align-items: center;
		min-height: var(--editor-tree-row);
		padding: 0 var(--editor-pad);
		border-radius: var(--editor-radius);
		color: var(--editor-text);
		text-decoration: none;
		text-shadow: var(--editor-halo);
	}
	.row:hover {
		background: var(--editor-hover);
	}
	.row[aria-current='page'] {
		background: var(--editor-selected);
	}
	.head {
		padding: var(--editor-pad) var(--editor-pad) 0.2rem;
		color: var(--editor-muted);
		font-size: var(--editor-label);
		letter-spacing: var(--editor-label-spacing);
		text-transform: uppercase;
	}
</style>
