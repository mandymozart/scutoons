<script lang="ts">
	import '@scutoons/tokens';
	import type { Snippet } from 'svelte';
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { AppShell, Button } from '@scutoons/ui';
	import { entities } from '@scutoons/content';
	import { setCreatorSession } from '$lib/session.svelte.ts';
	import { VIEWER_URL } from '$lib/config.ts';
	import { library } from '$lib/api.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { setCreatorNav, type Crumb } from '$lib/nav.svelte.ts';
	import { fileLabel } from '$lib/labels.ts';
	import LoginForm from '$lib/components/login-form.svelte';
	import NavSidebar from '$lib/components/nav-sidebar.svelte';

	let { children }: { children: Snippet } = $props();

	const session = setCreatorSession();

	onMount(async () => {
		// magic link from the login mail: `<creator origin>/?login=<token>`
		const magicToken = page.url.searchParams.get('login');
		if (magicToken) {
			const url = new URL(page.url);
			url.searchParams.delete('login');
			goto(url.pathname + url.search, { replaceState: true });
			if (!session.token) {
				try {
					await session.verifyToken(magicToken);
				} catch (e) {
					console.warn('Magic link login failed', e);
				}
			}
		}
		await session.init();
	});

	// ── Navigation (agents/DESIGN.md "Creator navigation"): one top bar, sidebar = children of the current level ──
	const nav = setCreatorNav();
	let menuOpen = $state(false);
	// the toons (names for crumbs, sidebar, titles): loaded once the signed-in creator is known – a request before the
	// session is restored (or before a login) would be refused and leave the ids in the UI
	let toonsLoaded = false;
	$effect(() => {
		if (!isCreator || toonsLoaded) return;
		toonsLoaded = true;
		library
			.fetchToons({ draft: true })
			.then((list) => (nav.toons = list))
			.catch(() => (toonsLoaded = false));
	});

	const toonId = $derived((page.params.toonId as string | undefined) ?? null);
	const toonName = (id: string) => nav.toonName(id);
	/** the crumbs of the route: Creator / toon / season / chapter (the chapter editor adds group and slice) */
	const base = $derived.by<Crumb[]>(() => {
		const out: Crumb[] = [{ href: '/', label: 'Creator' }];
		const route = page.route.id ?? '/';
		const tab = page.url.searchParams.get('tab');
		if (route === '/guide') out.push({ href: '/guide', label: m['creator.style_guide']() });
		if (route === '/characters') {
			out.push({ href: '/characters', label: m['creator.characters']() });
			if (tab === 'moods') out.push({ href: '/characters?tab=moods', label: m['creator.characters_moods']() });
			if (tab === 'wanted') out.push({ href: '/characters?tab=wanted', label: m['creator.characters_wanted']() });
		}
		if (!toonId) return out;
		out.push({ href: `/${toonId}`, label: toonName(toonId) });
		// every chapter belongs to a season; the editor works on season 1 for now
		const season = page.params.seasonNumber ?? (page.params.chapterNumber ? '1' : null);
		if (season) out.push({ href: `/${toonId}/season/${season}`, label: fileLabel(entities.season(toonId, Number(season))) });
		if (page.params.chapterNumber) {
			const n = page.params.chapterNumber;
			out.push({ href: `/${toonId}/chapter/${n}`, label: fileLabel(entities.chapter(toonId, Number(n))) });
			if (route.endsWith('/preview')) out.push({ href: page.url.pathname + page.url.search, label: m['creator.preview_title']() });
		}
		if (route === '/[toonId]/cast') out.push({ href: `/${toonId}/cast`, label: m['creator.cast']() });
		if (route === '/[toonId]/credits') out.push({ href: `/${toonId}/credits`, label: m['creator.credits']() });
		if (route === '/[toonId]/script') out.push({ href: `/${toonId}/script`, label: m['creator.script']() });
		if (route === '/[toonId]/toon') out.push({ href: `/${toonId}/toon`, label: fileLabel(entities.toonSettings(toonId)) });
		return out;
	});
	$effect(() => {
		nav.base = base;
	});
	// the drawer (< 768px) closes on every navigation
	$effect(() => {
		void page.url.href;
		menuOpen = false;
	});
	const universe = $derived([
		{ href: '/characters', label: m['creator.characters'](), tab: null as string | null },
		{ href: '/characters?tab=moods', label: m['creator.characters_moods'](), tab: 'moods' },
		{ href: '/characters?tab=wanted', label: m['creator.characters_wanted'](), tab: 'wanted' }
	]);
	const universeCurrent = (tab: string | null) =>
		page.route.id === '/characters' && (page.url.searchParams.get('tab') ?? null) === tab ? 'page' : undefined;

	/** Alt+ArrowUp: one level up (the parent crumb); Alt+ArrowDown: the first child in the sidebar */
	function onKeydown(event: KeyboardEvent) {
		if (!event.altKey || event.ctrlKey || event.metaKey || (event.key !== 'ArrowUp' && event.key !== 'ArrowDown')) return;
		const target = event.target as HTMLElement | null;
		if (target && (target.tagName === 'SELECT' || target.tagName === 'TEXTAREA')) return;
		if (event.key === 'ArrowUp') {
			if (!nav.parent) return;
			event.preventDefault();
			goto(nav.parent.href);
		} else {
			const child = document.querySelector<HTMLElement>('.sc-shell-sidebar [data-nav-child], .sc-shell-sidebar [data-child-slices] .thumb');
			if (!child) return;
			event.preventDefault();
			child.click();
		}
	}

	// role gate: `creator` (the server checks it again on every content write)
	const waiting = $derived(!session.ready || (session.loading && !session.me));
	const isCreator = $derived(session.hasRole('creator'));
</script>

<svelte:head>
	<title>{nav.crumbs.length > 1 ? `${nav.crumbs[nav.crumbs.length - 1].label} · ` : ''}{m['creator.title']()}</title>
</svelte:head>

<svelte:window onkeydown={onKeydown} />

<div class="creator">
	{#if waiting}
		<p class="creator-gate">{m['creator.loading']()}</p>
	{:else if !session.isLoggedIn}
		<section class="creator-gate" data-testid="creator-sign-in">
			<img class="logo" src="/SCUTOONS-logo.svg" alt="SCUTOONS" width="200" />
			<h1>{m['creator.sign_in_title']()}</h1>
			<LoginForm />
		</section>
	{:else if !isCreator}
		<section class="creator-gate" data-testid="creator-no-access">
			<h1>{m['creator.no_access_title']()}</h1>
			<p>{m['creator.no_access_text']({ email: session.user?.email ?? '' })}</p>
			<p class="gate-actions">
				<Button href={VIEWER_URL}>{m['creator.open_reader']()}</Button>
				<Button variant="secondary" onclick={() => session.logout()}>{m['creator.sign_out']()}</Button>
			</p>
		</section>
	{:else}
		<!-- One stable frame for every page: top bar (breadcrumb of the full path), sidebar (children of the current
		     level + Universe), content, and the page's ActionBar at the bottom (reserved by the shell) -->
		<AppShell
			mode={nav.fill ? 'fill' : 'page'}
			sidebarWidth="240px"
			sidebarLabel={m['creator.nav_label']()}
			sidebarId="creator-nav"
			collapsible
			sidebarOpen={menuOpen}
			onclose={() => (menuOpen = false)}
			closeLabel={m['creator.nav_close']()}
		>
			{#snippet topbar()}
				<header class="creator-bar" data-testid="creator-bar">
					<button
						type="button"
						class="burger"
						aria-label={m['creator.nav_menu']()}
						aria-expanded={menuOpen}
						aria-controls="creator-nav"
						onclick={() => (menuOpen = !menuOpen)}
					>
						<span aria-hidden="true">☰</span>
					</button>
					<nav class="crumbs" aria-label="Breadcrumb">
						<ol>
							{#each nav.crumbs as crumb, i (i)}
								<li>
									<a href={crumb.href} aria-current={i === nav.crumbs.length - 1 ? 'page' : undefined} data-testid="crumb-{i}">{crumb.label}</a>
								</li>
							{/each}
						</ol>
					</nav>
					<span class="who">{session.user?.email ?? ''}</span>
				</header>
			{/snippet}
			{#snippet sidebar()}
				<div class="nav-children">
					{#if nav.sidebar}
						{@render nav.sidebar()}
					{:else}
						<NavSidebar toons={nav.toons} toon={toonId} />
					{/if}
				</div>
				<div class="universe">
					<p class="head">{m['creator.universe']()}</p>
					<ul>
						{#each universe as item (item.href)}
							<li><a href={item.href} aria-current={universeCurrent(item.tab)} data-testid="nav-universe-{item.tab ?? 'characters'}">{item.label}</a></li>
						{/each}
					</ul>
				</div>
			{/snippet}
			{@render children()}
		</AppShell>
	{/if}
</div>

<style>
	/* Editor chrome: design system Part 1 – IBM Plex Mono, ink/paper, square, 2px structural borders.
	   Dense desktop tool: fixed px base instead of the reader's fluid root size. */
	.creator {
		/* editor aliases of @scutoons/tokens (ink, paper, border-*, surface-* come from the tokens directly) */
		--cyan: var(--primary-cyan);
		--muted: var(--text-muted);
		--error: var(--state-error);
		--warn: var(--state-warning);
		/* every creator page: the bottom ActionBar mirrors the 64px top bars (AppShell reserves this much) */
		--action-bar-height: 64px;
		/* the one top bar is 64px high: side panels and the Info panel start below it */
		--top: 64px;
		--info-panel-top: 64px;
		--info-panel-width: 360px;
		font-family: var(--font-mono);
		font-size: 13px;
		line-height: 1.4;
		color: var(--ink);
		background: var(--paper);
		min-height: 100dvh;
	}

	/* ── the one top bar (same on every page, only the depth of the breadcrumb changes) ── */
	.creator-bar {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		box-sizing: border-box;
		height: var(--top);
		padding: 0 1rem;
		border-bottom: 2px solid var(--ink);
		background: var(--paper);
	}
	.burger {
		display: none;
	}
	.crumbs {
		flex: 1;
		min-width: 0;
	}
	.crumbs ol {
		display: flex;
		align-items: center;
		margin: 0;
		padding: 0;
		list-style: none;
		overflow: hidden;
		white-space: nowrap;
	}
	.crumbs li {
		display: flex;
		align-items: center;
		min-width: 3em;
	}
	/* "Creator" and the current level stay whole; the levels between shrink first */
	.crumbs li:first-child,
	.crumbs li:last-child {
		flex: none;
	}
	.crumbs li + li::before {
		content: '/';
		padding: 0 0.45rem;
		color: var(--muted);
	}
	.crumbs a {
		overflow: hidden;
		text-overflow: ellipsis;
		color: var(--ink);
	}
	.crumbs a[aria-current='page'] {
		font-weight: 700;
		text-decoration: none;
	}
	.who {
		flex: none;
		max-width: 16rem;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: 11px;
		color: var(--muted);
	}
	@media (max-width: 1099px) {
		.who {
			display: none;
		}
	}
	@media (max-width: 767px) {
		.burger {
			display: inline-block;
		}
	}
	:global(.creator .sc-shell-sidebar) {
		display: flex;
		flex-direction: column;
	}
	.nav-children {
		flex: 1 0 auto;
	}
	.universe {
		position: sticky;
		bottom: 0;
		background: var(--paper);
		border-top: 2px solid var(--ink);
	}
	.universe .head {
		margin: 0;
		padding: 0.6rem 0.75rem 0.25rem;
		font-size: 10px;
		text-transform: uppercase;
		color: var(--muted);
	}
	.universe ul {
		list-style: none;
		margin: 0;
		padding: 0 0 0.4rem;
	}
	.universe a {
		display: block;
		padding: 0.3rem 0.75rem;
		border-left: 3px solid transparent;
		color: var(--ink);
		text-decoration: none;
	}
	.universe a:hover {
		background: var(--surface-hover);
	}
	.universe a[aria-current='page'] {
		border-left-color: var(--ink);
		background: var(--surface-hover);
		font-weight: 700;
	}

	.creator-gate {
		max-width: 32rem;
		margin: 0 auto;
		padding: 6rem 1rem 2rem;
		font-size: 16px;
	}
	.creator-gate h1 {
		font-size: 1.25rem;
	}
	.creator-gate .logo {
		display: block;
		margin-bottom: var(--space-1);
	}
	.gate-actions {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05);
	}

	/* Dense editor controls (@scutoons/ui components style themselves) */
	:global(:where(.creator) input:not([type='checkbox']):not([type='radio']):not([type='range']):not([type='color'])),
	:global(:where(.creator) select),
	:global(:where(.creator) textarea) {
		width: 100%;
		margin: 0;
		padding: 0.3em 0.45em;
		font: inherit;
		font-size: 12px;
		border: 1px solid var(--border-subtle);
		border-radius: 0;
		background: var(--paper);
		box-sizing: border-box;
	}
	:global(:where(.creator) input:focus),
	:global(:where(.creator) select:focus),
	:global(:where(.creator) textarea:focus) {
		outline: 2px solid var(--ink);
		outline-offset: -1px;
	}
	:global(:where(.creator) input[type='checkbox']),
	:global(:where(.creator) input[type='radio']) {
		width: auto;
		margin: 0;
	}
	:global(:where(.creator) button),
	:global(:where(.creator) .btn) {
		font: inherit;
		font-size: 12px;
		padding: 0.45em 1em;
		border-radius: 0;
		line-height: 1.2;
		background: var(--ink);
		color: var(--paper);
		border: 2px solid var(--ink);
		cursor: pointer;
		text-decoration: none;
		display: inline-block;
	}
	:global(:where(.creator) button.secondary),
	:global(:where(.creator) .btn.secondary) {
		background: var(--paper);
		color: var(--ink);
		border-color: var(--border-muted);
	}
	:global(:where(.creator) button.secondary:hover:not(:disabled)) {
		background: var(--paper);
		border-color: var(--ink);
	}
	:global(:where(.creator) button.ghost) {
		background: transparent;
		color: var(--ink);
		border-color: transparent;
		padding: 0.2em 0.4em;
	}
	:global(:where(.creator) button.ghost:hover:not(:disabled)) {
		background: var(--surface-hover);
	}
	:global(:where(.creator) button.active),
	:global(:where(.creator) button.active:hover:not(:disabled)) {
		background: var(--ink);
		color: var(--paper);
		border-color: var(--ink);
	}
	:global(:where(.creator) button:disabled) {
		opacity: 0.4;
		cursor: not-allowed;
	}
	:global(:where(.creator) h1),
	:global(:where(.creator) h2),
	:global(:where(.creator) h3) {
		font-weight: 700;
		margin: 0 0 0.5em;
	}
	:global(:where(.creator) label) {
		display: block;
	}
	:global(:where(.creator) .hint) {
		color: var(--muted);
		font-size: 11px;
		line-height: 1.35;
	}
</style>
