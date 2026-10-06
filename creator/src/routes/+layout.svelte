<script lang="ts">
	import '@scutoons/tokens';
	import type { Snippet } from 'svelte';
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import {
		AppShell,
		CommandPalette,
		Icon,
		PaletteTrigger,
		languageCommand,
		linkCommands,
		signOutCommand,
		type Command,
		type PaletteLink,
		installTooltips,
		DialogHost
	} from '@scutoons/ui';
	import { AuthGate, canUseApp, takeMagicToken } from '@scutoons/auth';
	import { entities, parseEntityKey, toonLanguages } from '@scutoons/content';
	import { setCreatorSession } from '$lib/session.svelte.ts';
	import { VIEWER_URL } from '$lib/config.ts';
	import { setupMedia } from '@scutoons/media';
	import { assetsApi, contentIndex, library } from '$lib/api.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { getLocale, locales, setLocale } from '$lib/paraglide/runtime.js';
	import { setCreatorNav, type Crumb } from '$lib/nav.svelte.ts';
	import { fileLabel } from '$lib/labels.ts';
	import NavSidebar from '$lib/components/page/nav-sidebar.svelte';
	import { reviews } from '$lib/editor/reviews.svelte.ts';
	import TextLanguageSwitch from '$lib/components/text/text-language-switch.svelte';
	import { PLATFORM_SOURCE_LANGUAGE } from '$lib/editor/text-language.svelte.ts';
	import AskPanel from '$lib/components/ask/ask-panel.svelte';
	import { ask, pagePanel } from '$lib/ask.svelte.ts';

	let { children }: { children: Snippet } = $props();

	const session = setCreatorSession();
	// the shared media tooling (@scutoons/media: asset picker, media slots, uploads) uses this app's API, CDN and locale
	setupMedia({ assets: assetsApi, cdn: asset, locale: getLocale });

	onMount(() => {
		// magic link from the login mail: `<creator origin>/?login=<token>`; else the shared session (single sign-on)
		const magic = takeMagicToken(page.url);
		if (magic) goto(magic.cleaned, { replaceState: true });
		void session.init(magic?.token);
	});

	// every `title` in the tool shows in the one SCUTOONS tooltip (tools.css .sc-tooltip)
	onMount(() => installTooltips());

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
	// review before publish: this account's rules + open reviews (Publish vs "Submit for review", the dashboard list)
	$effect(() => {
		if (isCreator) void reviews.load();
	});

	const toonId = $derived((page.params.toonId as string | undefined) ?? null);
	const toonName = (id: string) => nav.toonName(id);
	/**
	 * the TEXT language switch (content, not the UI language): a toon page offers the toon's languages (source first);
	 * the universe pages every language of every toon, the platform's source `de` first and the default; hidden while
	 * there is only one
	 */
	const textLanguages = $derived.by<{ languages: string[]; source: string }>(() => {
		const toon = toonId ? nav.toons.find((t) => t.id === toonId || t.uri === toonId) : null;
		if (toon) {
			const l = toonLanguages(toon);
			return { languages: l.languages as string[], source: l.source };
		}
		if (toonId) return { languages: [], source: PLATFORM_SOURCE_LANGUAGE };
		return { languages: [...new Set([PLATFORM_SOURCE_LANGUAGE, ...nav.toons.flatMap((t) => toonLanguages(t).languages as string[])])], source: PLATFORM_SOURCE_LANGUAGE };
	});
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
		// the world map (agents/MAP.md §4.1): Creator / Karte / <selection – added by the builder>
		if (route === '/map') out.push({ href: '/map', label: m['creator.map']() });
		// the SCUTOONS-wide Flow (owner 2026-10-06): Creator / Ablauf
		if (route === '/flow') out.push({ href: '/flow', label: m['creator.flow_universe']() });
		if (!toonId) return out;
		out.push({ href: `/${toonId}`, label: toonName(toonId) });
		// every chapter belongs to a season; the editor works on season 1 for now
		const season = page.params.seasonNumber ?? (page.params.chapterNumber ? '1' : null);
		if (season) out.push({ href: `/${toonId}/season/${season}`, label: fileLabel(entities.season(toonId, Number(season))) });
		if (route === '/[toonId]/season/[seasonNumber]/cover') out.push({ href: `/${toonId}/season/${season}/cover`, label: m['creator.cover_editor']() });
		if (page.params.chapterNumber) {
			const n = page.params.chapterNumber;
			out.push({ href: `/${toonId}/chapter/${n}`, label: fileLabel(entities.chapter(toonId, Number(n))) });
		}
		if (route === '/[toonId]/cast') out.push({ href: `/${toonId}/cast`, label: m['creator.cast']() });
		if (route === '/[toonId]/credits') out.push({ href: `/${toonId}/credits`, label: m['creator.credits']() });
		if (route === '/[toonId]/script') out.push({ href: `/${toonId}/script`, label: m['creator.script']() });
		if (route === '/[toonId]/convert') out.push({ href: `/${toonId}/convert`, label: m['creator.convert']() });
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
		{ href: '/characters?tab=wanted', label: m['creator.characters_wanted'](), tab: 'wanted' },
		{ href: '/map', label: m['creator.map'](), tab: 'map' },
		{ href: '/flow', label: m['creator.flow_universe'](), tab: 'flow' }
	]);
	const universeCurrent = (tab: string | null) =>
		tab === 'map' || tab === 'flow'
			? page.route.id === `/${tab}`
				? 'page'
				: undefined
			: page.route.id === '/characters' && (page.url.searchParams.get('tab') ?? null) === tab
				? 'page'
				: undefined;

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

	// role gate (AuthGate): `creator` – the server checks it again on every content write
	const isCreator = $derived(session.isLoggedIn && canUseApp('creator', session.roles));

	// ── Command palette (Ctrl/⌘K, `/`; @scutoons/ui CommandPalette like the admin): the universe pages, every toon with
	// its pages, seasons and chapters ("Cat Pope · Chapter 3"), language, sign out ──
	let paletteOpen = $state(false);
	/** seasons + chapters per toon from the content index (like the sidebar's seasons), loaded on the first open */
	let toonParts = $state<Record<string, { seasons: number[]; chapters: number[] }>>({});
	const partsRequested = new Set<string>();
	$effect(() => {
		if (!paletteOpen) return;
		for (const t of nav.toons) {
			if (partsRequested.has(t.id)) continue;
			partsRequested.add(t.id);
			contentIndex(t.id)
				.then((index) => {
					const refs = index.entities.map((e) => parseEntityKey(e.key));
					const numbers = (kind: string) => [...new Set(refs.filter((r) => r?.kind === kind && r.number !== undefined).map((r) => r!.number!))].sort((a, b) => a - b);
					const seasons = numbers('season');
					toonParts[t.id] = { seasons: seasons.length ? seasons : [1], chapters: numbers('chapter') };
				})
				.catch(() => partsRequested.delete(t.id));
		}
	});

	const paletteCommands = $derived.by<Command[]>(() => {
		const goTo = m['palette.go_to']();
		const actions = m['palette.actions']();
		const pages: PaletteLink[] = [
			{ id: 'home', label: m['palette.home'](), href: '/', keywords: 'toons home start' },
			...universe.map((item) => ({ id: `universe-${item.tab ?? 'characters'}`, label: item.label, hint: m['creator.universe'](), href: item.href })),
			{ id: 'guide', label: m['creator.style_guide'](), href: '/guide', keywords: 'style guide design' }
		];
		// the toon being worked on comes first
		const toons = [...nav.toons].sort((a, b) => Number(b.id === toonId) - Number(a.id === toonId));
		const toonLinks = toons.flatMap((t): PaletteLink[] => {
			const name = nav.toonName(t.id);
			const parts = toonParts[t.id];
			const entry = (id: string, label: string, href: string, keywords?: string): PaletteLink => ({ id: `${t.id}-${id}`, label: `${name} · ${label}`, href, keywords: `${t.id} ${keywords ?? ''}` });
			return [
				{ id: `${t.id}-toon`, label: name, hint: m['palette.toon'](), href: `/${t.id}`, keywords: t.id },
				...(parts?.seasons ?? []).flatMap((n) => [
					entry(`season-${n}`, fileLabel(entities.season(t.id, n)), `/${t.id}/season/${n}`, 'season staffel'),
					entry(`season-${n}-cover`, `${fileLabel(entities.season(t.id, n))} · ${m['creator.cover_editor']()}`, `/${t.id}/season/${n}/cover`, 'cover lettering schriftzug titel season staffel')
				]),
				...(parts?.chapters ?? []).map((n) => entry(`chapter-${n}`, fileLabel(entities.chapter(t.id, n)), `/${t.id}/chapter/${n}`, 'chapter kapitel')),
				entry('script', m['creator.script'](), `/${t.id}/script`, 'script drehbuch screenplay'),
				entry('cast', m['creator.cast'](), `/${t.id}/cast`, 'cast besetzung'),
				entry('credits', m['creator.credits'](), `/${t.id}/credits`, 'credits'),
				entry('settings', fileLabel(entities.toonSettings(t.id)), `/${t.id}/toon`, 'settings einstellungen')
			];
		});
		return [
			...linkCommands(pages, goto, goTo),
			...linkCommands(toonLinks, goto, m['palette.toons']()),
			languageCommand({ locales, current: getLocale(), setLocale, label: (locale) => m['palette.language']({ locale: locale.toUpperCase() }), group: actions }),
			signOutCommand({ label: m['creator.sign_out'](), run: () => session.logout(), group: actions })
		];
	});
</script>

<svelte:head>
	<title>{nav.crumbs.length > 1 ? `${nav.crumbs[nav.crumbs.length - 1].label} · ` : ''}{m['creator.title']()}</title>
</svelte:head>

<svelte:window onkeydown={onKeydown} />

<div class="creator sc-tool">
	<AuthGate {session} app="creator" appName="Creator" locale={getLocale()} readerHref={VIEWER_URL}>
		<!-- One stable frame for every page: top bar (breadcrumb of the full path), sidebar (children of the current
		     level + Universe), content, and the page's ActionBar at the bottom (reserved by the shell) -->
		<AppShell
			mode={nav.fill ? 'fill' : 'page'}
			sidebarWidth="var(--tool-sidebar-width)"
			resizeKey="creator"
			resizeLabel={m['creator.resize_sidebar']()}
			foldable
			railLabel={nav.crumbs.slice(-2).map((c) => c.label).join(' · ')}
			foldLabel={m['creator.fold_sidebar']()}
			unfoldLabel={m['creator.unfold_sidebar']()}
			foldWhen={ask.open && pagePanel.open}
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
						class="burger sc-round"
						aria-label={m['creator.nav_menu']()}
						aria-expanded={menuOpen}
						aria-controls="creator-nav"
						onclick={() => (menuOpen = !menuOpen)}
					>
						<Icon name="menu" />
					</button>
					<a class="brand" href="/" data-testid="creator-brand">
						<img src="/SCUTOONS-logo.svg" alt="SCUTOONS" width="120" height="20" />
						<span class="brand-name">{m['creator.app_label']()}</span>
					</a>
					<nav class="crumbs" aria-label="Breadcrumb">
						<ol>
							<!-- the root crumb ("Creator") is the brand next to the logo -->
							{#each nav.crumbs.slice(1) as crumb, i (i)}
								<li>
									<a href={crumb.href} aria-current={i === nav.crumbs.length - 2 ? 'page' : undefined} data-testid="crumb-{i + 1}">{crumb.label}</a>
								</li>
							{/each}
						</ol>
					</nav>
					{#if textLanguages.languages.length > 1}
						<TextLanguageSwitch languages={textLanguages.languages} source={textLanguages.source} />
					{/if}
					{#if nav.actions}{@render nav.actions()}{/if}
					<button type="button" class="sc-round" class:is-active={ask.open} title={m['creator.ask_open']()} aria-label={m['creator.ask_open']()} aria-pressed={ask.open} data-testid="ask-toggle" onclick={() => (ask.open = !ask.open)}><Icon name="message-square" /></button>
					<PaletteTrigger label={m['palette.search']()} compact onclick={() => (paletteOpen = true)} />
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
				<!-- Universe only on the dashboard (owner 2026-10-03) -->
				{#if page.route.id === '/'}
					<div class="universe">
						<p class="sc-nav-head">{m['creator.universe']()}</p>
						<ul>
							{#each universe as item (item.href)}
								<li><a class="sc-nav-item" href={item.href} aria-current={universeCurrent(item.tab)} data-testid="nav-universe-{item.tab ?? 'characters'}">{item.label}</a></li>
							{/each}
						</ul>
					</div>
				{/if}
			{/snippet}
			{@render children()}
		</AppShell>
		<CommandPalette
			bind:open={paletteOpen}
			commands={paletteCommands}
			label={m['palette.label']()}
			placeholder={m['palette.placeholder']()}
			hint={m['palette.keys']()}
		/>
		{#if ask.open}<AskPanel context={nav.crumbs.slice(1).map((c) => c.label).join(' · ') || m['creator.app_label']()} onclose={() => (ask.open = false)} />{/if}
		<DialogHost cancelLabel={m['creator.cancel']()} />
	</AuthGate>
</div>

<style>
	/* Editor chrome: DESIGN.md "Brutalist tools" – the reader's fluid root and type styles (tools.css via `sc-tool`),
	   2px ink frames, square, ≥ 44px controls, selected = inverted. No px font sizes. */
	.creator {
		/* editor aliases of @scutoons/tokens (ink, paper, border-*, surface-* come from the tokens directly) */
		--cyan: var(--primary-cyan);
		--muted: var(--text-muted);
		--error: var(--state-error);
		--warn: var(--state-warning);
		/* every creator page: the bottom ActionBar mirrors the top bar (AppShell reserves this much) */
		--action-bar-height: var(--tool-bar-height);
		/* the one top bar: side panels and the Info panel start below it */
		--top: var(--tool-bar-height);
		--info-panel-top: var(--tool-bar-height);
		--info-panel-width: var(--tool-panel-width);
		min-height: 100dvh;
	}

	/* ── the one top bar (same on every page, only the depth of the breadcrumb changes) ── */
	.creator-bar {
		display: flex;
		align-items: center;
		gap: var(--space-075);
		box-sizing: border-box;
		height: var(--top);
		padding: 0 var(--space-1);
		border-bottom: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
	}
	.brand {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		color: var(--ink);
		text-decoration: none;
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		white-space: nowrap;
	}
	.brand img {
		display: block;
		height: 1.25rem;
		width: auto;
	}
	.burger {
		display: none;
	}
	@media (max-width: 479px) {
		.brand-name {
			display: none;
		}
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
	}
	.crumbs a {
		overflow: hidden;
		text-overflow: ellipsis;
		color: var(--ink);
	}
	/* phones: only the current page, truncated (the burger + the sidebar give the way up) */
	@media (max-width: 479px) {
		.crumbs li:not(:last-child) {
			display: none;
		}
		.crumbs li:last-child {
			flex: 0 1 auto;
			min-width: 0;
		}
		.crumbs li:last-child::before {
			content: none;
		}
	}
	.crumbs a[aria-current='page'] {
		font-weight: var(--font-weight-regular);
		text-decoration: none;
	}
	.who {
		flex: none;
		max-width: 16rem;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--type-caption-size);
		color: var(--muted);
	}
	@media (max-width: 1099px) {
		.who {
			display: none;
		}
	}
	@media (max-width: 767px) {
		.burger {
			display: inline-flex;
			align-items: center;
			justify-content: center;
			min-width: var(--control-height);
		}
	}
	:global(.creator .sc-shell-sidebar) {
		display: flex;
		flex-direction: column;
	}
	/* Universe follows right below the page's own entries (owner 2026-10-03: pushed to the bottom it looked absent) */
	.nav-children {
		flex: none;
	}
	.universe {
		position: sticky;
		bottom: 0;
		background: var(--paper);
		border-top: var(--border-width-regular) solid var(--ink);
	}
	.universe ul {
		list-style: none;
		margin: 0;
		padding: 0 0 var(--space-05);
	}

	/* ── Creator-wide controls (fields, focus, headings: tools.css; @scutoons/ui components style themselves) ──
	   The viewer's buttons (owner 2026-10-01): button = ink (primary) · .secondary = paper + 2px border-muted → ink on
	   hover · .ghost = no frame until hover · .active (a toggle) = inverted · disabled 0.5. Compact side padding
	   space-15 (rem), the bars need it; one control height (owner 2026-10-02): exactly --control-height, no vertical
	   padding. */
	:global(:where(.creator input:not([type='checkbox'], [type='radio'], [type='range'], [type='color']))),
	:global(:where(.creator select)),
	:global(:where(.creator textarea)) {
		width: 100%;
	}
	:global(:where(.creator button)),
	:global(:where(.creator .btn)) {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-025);
		box-sizing: border-box;
		min-height: var(--control-height);
		padding: 0 var(--space-15);
		line-height: var(--type-button-line);
		background: var(--ink);
		color: var(--paper);
		border: var(--border-width-regular) solid var(--ink);
		border-radius: 0;
		cursor: pointer;
		text-decoration: none;
	}
	:global(:where(.creator button.secondary)),
	:global(:where(.creator .btn.secondary)) {
		background: var(--paper);
		color: var(--ink);
		border-color: var(--border-muted);
	}
	:global(:where(.creator button.secondary:hover:not(:disabled))),
	:global(:where(.creator .btn.secondary:hover)) {
		border-color: var(--ink);
	}
	:global(:where(.creator button.ghost)) {
		background: transparent;
		color: var(--ink);
		border-color: transparent;
		padding: 0 var(--space-05);
	}
	:global(:where(.creator button.ghost:hover:not(:disabled))) {
		border-color: var(--border-muted);
	}
	:global(:where(.creator button.active)),
	:global(:where(.creator button.active:hover:not(:disabled))) {
		background: var(--ink);
		color: var(--paper);
		border-color: var(--ink);
	}
	:global(:where(.creator button:disabled)) {
		opacity: 0.5;
		cursor: not-allowed;
	}
	:global(:where(.creator label)) {
		display: block;
	}
	:global(:where(.creator .hint)) {
		color: var(--muted);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
</style>
