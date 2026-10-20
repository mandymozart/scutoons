<script lang="ts">
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The Characters area's pages as tabs (research/CHARACTER-PAGE.md §2: "the area's own tabs", the same row as a toon's
	 * – toon-tabs.svelte): Characters · Moods · Wanted images, each with its count. Links (`?tab=`), so back / forward and
	 * the menu's Characters entry work; the page follows the URL.
	 */
	let { tab, counts }: { tab: string; counts: { characters: number; moods: number; wanted: number } } = $props();

	const tabs = $derived([
		{ id: 'characters', href: '/characters', label: m['creator.characters'](), n: counts.characters },
		{ id: 'moods', href: '/characters?tab=moods', label: m['creator.characters_moods'](), n: counts.moods },
		{ id: 'wanted', href: '/characters?tab=wanted', label: m['creator.characters_wanted'](), n: counts.wanted }
	]);
</script>

<nav class="area-tabs" aria-label={m['creator.characters']()}>
	{#each tabs as t (t.id)}
		<a href={t.href} aria-current={t.id === tab ? 'page' : undefined} data-testid="area-tab-{t.id}">{t.label}{#if t.n}<span class="n">{t.n}</span>{/if}</a>
	{/each}
</nav>

<style>
	/* the toon tabs' look (toon-tabs.svelte): words, the current one underlined by a thick ink line */
	.area-tabs {
		display: flex;
		overflow-x: auto;
		scrollbar-width: none;
		padding: 0 var(--space-05);
		border-bottom: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
		flex: none;
	}
	a {
		display: inline-flex;
		flex: none;
		align-items: center;
		gap: var(--space-05);
		box-sizing: border-box;
		min-height: var(--tool-bar-height);
		padding: 0 var(--space-1);
		border-bottom: calc(2 * var(--border-width-regular)) solid transparent;
		color: var(--ink);
		text-decoration: none;
		white-space: nowrap;
	}
	a:hover {
		border-bottom-color: var(--border-muted);
	}
	a[aria-current='page'] {
		border-bottom-color: var(--ink);
	}
	a:focus-visible {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-focus));
	}
	.n {
		font-size: var(--type-caption-size);
		color: var(--text-muted);
	}
</style>
