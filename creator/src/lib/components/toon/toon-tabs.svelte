<script lang="ts">
	import { page } from '$app/state';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A toon's pages as tabs (CREATOR-NAV.md §2a, draft D – the menu lists the toons flat, never deeper): Stories (the
	 * toon page with its seasons and chapters; a season's page belongs to it), Cast, Script, Credits, Settings. The
	 * layout shows them above the toon's own pages; the chapter editor and the cover editor have their own frames.
	 */
	let { toon }: { toon: string } = $props();

	const path = $derived(page.url.pathname);
	const tabs = $derived([
		{ href: `/${toon}`, label: m['creator.toon_stories'](), on: path === `/${toon}` || path.startsWith(`/${toon}/season/`), testid: 'nav-stories' },
		{ href: `/${toon}/cast`, label: m['creator.cast'](), on: path === `/${toon}/cast`, testid: 'nav-cast' },
		{ href: `/${toon}/script`, label: m['creator.script'](), on: path === `/${toon}/script`, testid: 'nav-script' },
		{ href: `/${toon}/credits`, label: m['creator.credits'](), on: path === `/${toon}/credits`, testid: 'nav-credits' },
		{ href: `/${toon}/toon`, label: m['creator.toon_settings'](), on: path === `/${toon}/toon`, testid: 'nav-toon-settings' }
	]);
</script>

<nav class="toon-tabs" aria-label={m['creator.toon_tabs']()}>
	{#each tabs as t (t.href)}
		<a href={t.href} aria-current={t.on ? 'page' : undefined} data-testid={t.testid}>{t.label}</a>
	{/each}
</nav>

<style>
	/* a tab row (DESIGN "Round vs square": grouped, square): words, the current one underlined by a thick ink line */
	.toon-tabs {
		display: flex;
		overflow-x: auto;
		scrollbar-width: none;
		padding: 0 var(--space-05);
		border-bottom: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
	}
	a {
		display: inline-flex;
		flex: none;
		align-items: center;
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
</style>
