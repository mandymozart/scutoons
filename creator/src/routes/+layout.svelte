<script lang="ts">
	import '@scutoons/tokens';
	import type { Snippet } from 'svelte';
	import { onMount } from 'svelte';
	import { afterNavigate, goto } from '$app/navigation';
	import { page } from '$app/state';
	import { MediaQuery } from 'svelte/reactivity';
	import {
		AppShell,
		Breadcrumbs,
		CommandPalette,
		Icon,
		PaletteTrigger,
		languageCommand,
		linkCommands,
		signOutCommand,
		type Command,
		type PaletteLink,
		installTooltips,
		DialogHost,
		ToastStack,
		provideUiLabels
	} from '@scutoons/ui';
	import { toaster } from '$lib/toasts.ts';
	import { InstallBar, UpdateBar, pwaLabels } from '@scutoons/pwa';
	import { APP_NAME, initPwa, install, pwa } from '$lib/pwa.svelte.ts';
	import { AuthGate, canUseApp, takeMagicToken } from '@scutoons/auth';
	import { entities, parseEntityKey, toonLanguages } from '@scutoons/content';
	import { setCreatorSession } from '$lib/session.svelte.ts';
	import { ADMIN_URL, VIEWER_URL } from '$lib/config.ts';
	import { setupCatalog } from '@scutoons/catalog';
	import { CATALOG_LABELS, CATALOG_PAGE_KINDS, isCatalogPageKind } from '$lib/editor/catalog-docs.ts';
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
	import { PLATFORM_SOURCE_LANGUAGE, textLanguage } from '$lib/editor/text-language.svelte.ts';

	let { children }: { children: Snippet } = $props();

	const session = setCreatorSession();
	// the shared media tooling (@scutoons/media: asset picker, media slots, uploads) uses this app's API, CDN and locale
	setupMedia({ assets: assetsApi, cdn: asset, locale: getLocale });
	// the catalog editors (@scutoons/catalog, agents/CATALOG-IN-CREATOR.md): texts in this app's locale, links to the admin
	setupCatalog({ locale: getLocale, locales, adminUrl: ADMIN_URL });
	// the shared fields' words (reset, the colour popover, "Mixed") for every inspector (owner 2026-10-18)
	provideUiLabels({
		get reset() { return m['creator.field_reset'](); },
		get mixed() { return m['creator.field_mixed'](); },
		get pickColour() { return m['creator.colour_pick'](); },
		get palette() { return m['creator.colour_palette'](); },
		get recent() { return m['creator.colour_recent'](); },
		get shade() { return m['creator.colour_shade'](); },
		get hue() { return m['creator.colour_hue'](); },
		get hex() { return m['creator.colour_hex'](); }
	});

	onMount(() => {
		// magic link from the login mail: `<creator origin>/?login=<token>`; else the shared session (single sign-on)
		const magic = takeMagicToken(page.url);
		if (magic) goto(magic.cleaned, { replaceState: true });
		void session.init(magic?.token);
	});

	// every `title` in the tool shows in the one SCUTOONS tooltip (tools.css .sc-tooltip)
	onMount(() => installTooltips());
	// installable web app (agents/research/PWA-APPS.md): the service worker + the install moment, checked on every page
	onMount(initPwa);
	afterNavigate(({ from }) => from && install.check());

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
	// every shown content name follows the switch as it stands here (textLanguage.names, BACKLOG 10.51)
	$effect(() => {
		textLanguage.page = textLanguages;
	});
	/**
	 * the crumbs of the route: SCUTOONS (the logo, the root crumb) / toon / season / chapter (the chapter editor adds
	 * group and slice) – UI-CONCEPTS E.2
	 */
	const base = $derived.by<Crumb[]>(() => {
		const out: Crumb[] = [{ href: '/', label: 'SCUTOONS' }];
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
		// the catalog (agents/CATALOG-IN-CREATOR.md): Creator / Katalog / <kind> (a score's collage stage: Items, its entry and
		// Collage come from the stage – research/COLLAGE.md §4.4)
		if (route === '/catalog/[kind]' || route === '/catalog/items/[id]/collage') {
			out.push({ href: '/catalog/items', label: m['creator.catalog']() });
			const kind = page.params.kind ?? 'items';
			if (isCatalogPageKind(kind)) out.push({ href: `/catalog/${kind}`, label: CATALOG_LABELS[kind]() });
		}
		// the SCUTOONS-wide Flow (owner 2026-10-06): Creator / Ablauf
		if (route === '/flow') out.push({ href: '/flow', label: m['creator.flow_universe']() });
		if (!toonId) return out;
		out.push({ href: `/${toonId}`, label: toonName(toonId) });
		// a chapter's season: the season whose sequence lists it (BACKLOG 10.45; none lists it / still loading: no season crumb)
		const chapterSeason = page.params.chapterNumber ? nav.chapterSeason(toonId, Number(page.params.chapterNumber)) : null;
		const season = page.params.seasonNumber ?? (chapterSeason != null ? String(chapterSeason) : null);
		if (season) out.push({ href: `/${toonId}/season/${season}`, label: fileLabel(entities.season(toonId, Number(season))) });
		if (route === '/[toonId]/season/[seasonNumber]/cover') out.push({ href: `/${toonId}/season/${season}/cover`, label: m['creator.cover_editor']() });
		if (page.params.chapterNumber) {
			const n = page.params.chapterNumber;
			out.push({ href: `/${toonId}/chapter/${n}`, label: fileLabel(entities.chapter(toonId, Number(n))) });
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
	// the toon's seasons (where a chapter sits): read again on every chapter opened (a season's sequence may have changed)
	$effect(() => {
		const toon = toonId;
		const chapter = page.params.chapterNumber;
		if (isCreator && toon && chapter) void nav.loadSeasons(toon);
	});
	/**
	 * narrow screens (< 768px – where the sidebar is the ☰ drawer; DESIGN.md "Header on phones"): the crumbs get their own
	 * row under the header, the text language + the page's Flow move to the top of the drawer
	 */
	const phone = new MediaQuery('max-width: 767px', false);
	// the drawer (< 768px) closes on every navigation
	$effect(() => {
		void page.url.href;
		menuOpen = false;
	});
	/** a pick inside the drawer closes it on phones – a group that only folds (aria-expanded false afterwards) keeps it open */
	function closeOnPick(event: MouseEvent) {
		if (!phone.current || !menuOpen) return;
		const el = (event.target as HTMLElement | null)?.closest<HTMLElement>('[data-drawer-pick]');
		if (!el) return;
		requestAnimationFrame(() => {
			if (el.getAttribute('aria-expanded') !== 'false') menuOpen = false;
		});
	}
	const universe = $derived([
		{ href: '/characters', label: m['creator.characters'](), tab: null as string | null },
		{ href: '/characters?tab=moods', label: m['creator.characters_moods'](), tab: 'moods' },
		{ href: '/characters?tab=wanted', label: m['creator.characters_wanted'](), tab: 'wanted' },
		{ href: '/map', label: m['creator.map'](), tab: 'map' },
		{ href: '/catalog/items', label: m['creator.catalog'](), tab: 'catalog', title: m['creator.catalog_hint']() },
		{ href: '/flow', label: m['creator.flow_universe'](), tab: 'flow', title: m['creator.flow_universe_hint']() }
	]);
	const universeCurrent = (tab: string | null) =>
		tab === 'catalog'
			? page.route.id?.startsWith('/catalog/')
				? 'page'
				: undefined
			: tab === 'map' || tab === 'flow'
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
			// every catalog page (owner 2026-10-14: the search should know the new pages – home scenes, airlift carriers …)
			...CATALOG_PAGE_KINDS.map((kind) => ({ id: `catalog-${kind}`, label: `${m['creator.catalog']()} · ${CATALOG_LABELS[kind]()}`, href: `/catalog/${kind}`, keywords: `catalog katalog ${kind.replace('-', ' ')}` })),
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

{#snippet logo()}<img src="/SCUTOONS-logo.svg" alt="SCUTOONS" width="120" height="20" />{/snippet}
{#snippet logoCrumb()}<span class="brand" data-testid="creator-brand">{@render logo()}</span>{/snippet}

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
			minContent={24}
			dockLabel={m['creator.dock_label']()}
			dockResizeLabel={m['creator.resize_panel']()}
			dockUnfoldLabel={m['creator.unfold_panel']()}
			pageLabel={nav.crumbs.at(-1)?.label}
			pageFoldLabel={m['creator.fold_page']()}
			pageUnfoldLabel={m['creator.unfold_page']()}
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
					{#if phone.current}
						<!-- phones: the logo alone here, the trail in its own row below (without the root) -->
						<a class="brand" href="/" data-testid="creator-brand">{@render logo()}</a>
						<span class="spacer"></span>
					{:else}
						<!-- the logo is the trail's root crumb (the "CREATOR" word went – the app name is in the tab title) -->
						<Breadcrumbs items={nav.crumbs} label={m['creator.breadcrumb']()} moreLabel={m['creator.breadcrumb_more']()} root={logoCrumb} />
						<!-- the page's own thing at the end of the trail (the map's toon pill – owner 2026-10-11, desktop like phones) -->
						{#if nav.crumbEnd}{@render nav.crumbEnd()}{/if}
						{#if textLanguages.languages.length > 1}
							<TextLanguageSwitch languages={textLanguages.languages} source={textLanguages.source} />
						{/if}
						{#if nav.actions}{@render nav.actions()}{/if}
					{/if}
					<PaletteTrigger label={m['palette.search']()} compact onclick={() => (paletteOpen = true)} />
					<!-- the signed-in account → "Benachrichtigungen" (push, the kinds, the app – research/PWA-APPS.md §6) -->
					<a class="who" href="/notifications" title={pwaLabels(getLocale()).notifications}>{session.user?.email ?? ''}</a>
					<a class="who-icon sc-round" href="/notifications" aria-label={pwaLabels(getLocale()).notifications} title={pwaLabels(getLocale()).notifications}><Icon name="bell" /></a>
				</header>
				{#if phone.current && nav.crumbs.length > 1}
					<div class="crumb-row" data-testid="crumb-row">
						<Breadcrumbs items={nav.crumbs.slice(1)} label={m['creator.breadcrumb']()} moreLabel={m['creator.breadcrumb_more']()} testid="phone-crumb" />
						{#if nav.crumbEnd}<span class="crumb-end">{@render nav.crumbEnd()}</span>{/if}
					</div>
				{/if}
			{/snippet}
			{#snippet sidebar()}
				{#if phone.current && (textLanguages.languages.length > 1 || nav.actions)}
					<!-- phones: the header's text language + the page's Flow sit at the top of the drawer (BACKLOG 10.50 draft) -->
					<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
					<div class="drawer-tools" data-testid="drawer-tools" onclick={(e) => (e.target as HTMLElement).closest('button') && (menuOpen = false)}>
						{#if textLanguages.languages.length > 1}
							<TextLanguageSwitch languages={textLanguages.languages} source={textLanguages.source} />
						{/if}
						{#if nav.actions}{@render nav.actions()}{/if}
					</div>
				{/if}
				<!-- phones: a pick in the drawer (a group, a slice, Meta / Audio – `data-drawer-pick`) closes it, like a link (10.50 b) -->
				<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
				<div class="nav-children" onclick={closeOnPick}>
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
								<li><a class="sc-nav-item" href={item.href} title={'title' in item ? item.title : undefined} aria-current={universeCurrent(item.tab)} data-testid="nav-universe-{item.tab ?? 'characters'}">{item.label}</a></li>
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
		<DialogHost cancelLabel={m['creator.cancel']()} />
	</AuthGate>
	<!-- transient info + errors (UI-CONCEPTS E.1): bottom-right above the ActionBar, never in the page flow -->
	<ToastStack {toaster} dismissLabel={m['creator.close']()} />
	<UpdateBar {pwa} app={APP_NAME} labels={pwaLabels(getLocale())} />
	<InstallBar {install} app={APP_NAME} labels={pwaLabels(getLocale())} hidden={pwa.updateReady} />
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
		/* the one top bar */
		--top: var(--tool-bar-height);
		min-height: 100dvh;
	}

	/* ── the one top bar (same on every page, only the depth of the breadcrumb changes) ── */
	.creator-bar {
		display: flex;
		align-items: center;
		gap: var(--space-075);
		box-sizing: border-box;
		min-height: var(--top);
		padding: 0 var(--space-1);
		border-bottom: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
	}
	/* the logo: the root crumb (wide screens) or alone (phones) */
	.brand {
		display: flex;
		flex: none;
		align-items: center;
		color: var(--ink);
		text-decoration: none;
	}
	:global(.creator .brand img) {
		display: block;
		height: 1.25rem;
		width: auto;
	}
	.burger {
		display: none;
	}
	/* touch screens: the logo link is a full-height target (BACKLOG 10.50 d) */
	@media (pointer: coarse) {
		a.brand {
			min-height: var(--control-height);
		}
	}
	.spacer {
		flex: 1;
	}
	/* phones: the trail in its own row under the header (the root is the header's logo) */
	.crumb-end {
		margin-left: auto;
		flex: none;
		padding-left: var(--space-05);
	}
	.crumb-row {
		display: flex;
		align-items: center;
		box-sizing: border-box;
		min-height: var(--control-height);
		padding: var(--space-025) var(--space-1);
		border-bottom: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
	}
	.drawer-tools {
		display: flex;
		align-items: center;
		gap: var(--space-075);
		padding: var(--space-05) var(--space-1);
		border-bottom: var(--border-width-regular) solid var(--ink);
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
	.who {
		text-decoration: none;
	}
	.who:hover {
		color: var(--ink);
	}
	.who-icon {
		display: none;
		flex: none;
	}
	/* the account only where the full trail still fits beside it (the crumbs come first, E.2) */
	@media (max-width: 1599px) {
		.who {
			display: none;
		}
		.who-icon {
			display: inline-flex;
			align-items: center;
			justify-content: center;
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
