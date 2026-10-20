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
		Rail,
		PaletteTrigger,
		languageCommand,
		linkCommands,
		signOutCommand,
		type Command,
		type PaletteLink,
		installTooltips,
		DialogHost,
		ToastStack,
		shellDock,
		provideColourSampler,
		provideUiLabels
	} from '@scutoons/ui';
	import { picturePick } from '$lib/editor/picture-pick.svelte.ts';
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
	import AreaMenu from '$lib/components/page/area-menu.svelte';
	import ToonTabs from '$lib/components/toon/toon-tabs.svelte';
	import PathMenu from '$lib/components/page/path-menu.svelte';
	import { recent } from '$lib/recent.svelte.ts';
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
		get unit() { return m['creator.field_unit'](); },
		get pickColour() { return m['creator.colour_pick'](); },
		get palette() { return m['creator.colour_palette'](); },
		get recent() { return m['creator.colour_recent'](); },
		get shade() { return m['creator.colour_shade'](); },
		get hue() { return m['creator.colour_hue'](); },
		get hex() { return m['creator.colour_hex'](); },
		get eyedropper() { return m['creator.colour_eyedropper'](); },
		get eyedropperPicture() { return m['creator.colour_eyedropper_slice'](); },
		get paint() { return m['creator.paint'](); },
		get paintSolid() { return m['creator.paint_solid'](); },
		get paintLinear() { return m['creator.paint_linear'](); },
		get paintRadial() { return m['creator.paint_radial'](); },
		get gradientStops() { return m['creator.paint_gradient_stops'](); },
		get gradientStop() { return m['creator.paint_gradient_stop'](); },
		get stopPosition() { return m['creator.paint_stop_position'](); },
		get removeStop() { return m['creator.paint_remove_stop'](); },
		get gradientAngle() { return m['creator.paint_gradient_angle'](); },
		get gradientCentreX() { return m['creator.paint_gradient_centre_x'](); },
		get gradientCentreY() { return m['creator.paint_gradient_centre_y'](); },
		get gradientRadius() { return m['creator.paint_gradient_radius'](); },
		get paintLinearShort() { return m['creator.paint_linear_short'](); },
		get paintRadialShort() { return m['creator.paint_radial_short'](); }
	});
	// the eyedropper's fallback (no EyeDropper in the browser): a point of the open slice canvas' picture
	provideColourSampler(() => (picturePick.canvases > 0 ? () => picturePick.start() : null));

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

	// ── Navigation (agents/DESIGN.md "Creator navigation", CREATOR-NAV.md §2a – draft D): one top bar with the path, the
	// menu by areas on the left (folded by hand to the areas' rail; a page's own list – the chapter tree – beside it);
	// phones: one row (logo = home · path · the page's end · ☰), the menu slides in from the right ──
	const nav = setCreatorNav();
	let menuOpen = $state(false);
	/** the desktop menu folded to its rail (by hand: its fold button, Ctrl + \; remembered) */
	const FOLD_KEY = 'sc-creator-menu-folded';
	let menuFolded = $state(false);
	onMount(() => {
		try {
			menuFolded = localStorage.getItem(FOLD_KEY) === '1';
		} catch {
			// no storage: unfolded
		}
	});
	/** the page's own list (the chapter tree …) folded to a slim rail by hand (its head's «; remembered) */
	const LIST_FOLD_KEY = 'sc-creator-list-folded';
	let listFolded = $state(false);
	onMount(() => {
		try {
			listFolded = localStorage.getItem(LIST_FOLD_KEY) === '1';
		} catch {
			// unfolded
		}
	});
	function foldList(folded: boolean) {
		listFolded = folded;
		try {
			localStorage.setItem(LIST_FOLD_KEY, folded ? '1' : '0');
		} catch {
			// the fold holds for this visit
		}
	}
	function foldMenu(folded: boolean) {
		menuFolded = folded;
		try {
			localStorage.setItem(FOLD_KEY, folded ? '1' : '0');
		} catch {
			// the fold holds for this visit
		}
	}
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
	 * the path of the route (UI-CONCEPTS E.2, CREATOR-NAV.md §2a): the area first (Dashboard, Toons, Characters …), then
	 * toon / season / chapter (the chapter editor adds group and slice); the logo is home, not a crumb
	 */
	const base = $derived.by<Crumb[]>(() => {
		const out: Crumb[] = [];
		const route = page.route.id ?? '/';
		const tab = page.url.searchParams.get('tab');
		if (route === '/') out.push({ href: '/', label: m['creator.dashboard']() });
		if (route === '/toons' || toonId) out.push({ href: '/toons', label: m['creator.toons']() });
		if (route === '/notifications') out.push({ href: '/notifications', label: pwaLabels(getLocale()).notifications });
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
		// a Kanon text (research/KANON.md §5): Creator / <its area> / Kanon · …
		if (route === '/kanon/[subject]') {
			const subject = page.params.subject ?? '';
			const area = subject.split('.')[0];
			if (area === 'character') out.push({ href: '/characters', label: m['creator.characters']() });
			if (area === 'place') out.push({ href: '/map', label: m['creator.map']() });
			if (area === 'toon') out.push({ href: '/toons', label: m['creator.toons']() }, { href: `/${subject.slice(5)}`, label: toonName(subject.slice(5)) });
			out.push({ href: `/kanon/${subject}`, label: fileLabel(entities.kanon(subject)) });
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
		if (route === '/[toonId]/toon') out.push({ href: `/${toonId}/toon`, label: m['creator.toon_settings']() });
		return out;
	});
	/** a toon's own pages show its tabs (Stories · Cast · Script · Credits · Settings); the editors have their own frames */
	const TOON_TAB_ROUTES = ['/[toonId]', '/[toonId]/cast', '/[toonId]/credits', '/[toonId]/script', '/[toonId]/toon', '/[toonId]/season/[seasonNumber]'];
	const toonTabs = $derived(!!toonId && TOON_TAB_ROUTES.includes(page.route.id ?? ''));
	// where this member was (the phone menu's Recent, the dashboard's Continue): every place below an area's top, once
	// the toons' names are known (no ids in the list)
	const NOT_RECENT = ['/', '/toons', '/notifications', '/guide'];
	$effect(() => {
		const crumbs = nav.crumbs;
		if (!isCreator || !nav.toons.length || NOT_RECENT.includes(page.route.id ?? '/')) return;
		// the season only where it is the place itself (a chapter reads "Cat Pope · Chapter 2 · …")
		const trail = (crumbs[0]?.href === '/toons' ? crumbs.slice(1) : crumbs).filter((c, i, all) => i === all.length - 1 || !c.href.includes('/season/')).map((c) => c.label);
		recent.visit({ href: page.url.pathname + page.url.search, trail, toon: toonId });
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
	 * narrow screens (< 768px – where the menu is the ☰ drawer from the right; DESIGN.md "Header on phones"): one row,
	 * the text language + the page's Flow + search + notifications move to the drawer's foot
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
	/** the desktop menu as its rail: folded by hand, or while a region is maximized (it gets everything right of it) */
	const railed = $derived(menuFolded || !!shellDock.maximized);
	/** the page's own list beside the menu (the chapter tree, a season's chapters, the map's lists) – not while maximized */
	const pageList = $derived(!!nav.sidebar && !shellDock.maximized);
	const menuWidth = $derived(railed ? 'var(--sc-shell-rail-width)' : 'var(--creator-menu-width)');
	/** phones: the name on the path button – a page's own first level (the chapter editor's group = the scene; the slices
	 * scroll inside it), else the place */
	const phoneTitle = $derived(nav.extra[0] ?? nav.crumbs.at(-1) ?? null);
	/** the list's name in its head and on its rail: the page's place (Chapter 2, Season 1, Map) */
	const listTitle = $derived(nav.base.at(-1)?.label ?? '');
	const listWidth = $derived(listFolded ? 'var(--sc-shell-rail-width)' : 'var(--tool-sidebar-width)');
	// Ctrl + \ folds / unfolds the menu (desktop; phones open it with ☰)
	function onFoldKey(event: KeyboardEvent) {
		if (!(event.ctrlKey || event.metaKey) || event.shiftKey || event.altKey || (event.key !== '\\' && event.code !== 'Backslash') || phone.current) return;
		event.preventDefault();
		foldMenu(!menuFolded);
	}
	/** what waits for this member: the submits to review (the dashboard's count in the menu) */
	const waitingCount = $derived(new Set((reviews.overview?.to_review ?? []).map((r) => `${r.author?.id ?? ''}|${r.created_at}|${r.toon}`)).size);

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
			const child = document.querySelector<HTMLElement>('.sc-shell-sidebar .page-list [data-nav-child], .sc-shell-sidebar [data-child-slices] .thumb');
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
			{ id: 'toons', label: m['creator.toons'](), href: '/toons', keywords: 'toons stories' },
			{ id: 'universe-characters', label: m['creator.characters'](), href: '/characters', keywords: 'characters figuren' },
			{ id: 'universe-moods', label: m['creator.characters_moods'](), hint: m['creator.characters'](), href: '/characters?tab=moods' },
			{ id: 'universe-wanted', label: m['creator.characters_wanted'](), hint: m['creator.characters'](), href: '/characters?tab=wanted' },
			{ id: 'universe-map', label: m['creator.map'](), href: '/map', keywords: 'map karte' },
			{ id: 'universe-flow', label: m['creator.flow_universe'](), href: '/flow', keywords: 'flow ablauf' },
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
				entry('settings', m['creator.toon_settings'](), `/${t.id}/toon`, 'settings einstellungen')
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

<svelte:window onkeydown={(e) => (onKeydown(e), onFoldKey(e))} />

{#snippet logo()}<img src="/SCUTOONS-logo-animated.svg" alt="SCUTOONS" width="40" height="64" />{/snippet}
{#snippet pageTools()}
	<!-- the page's own tools: the desktop's top bar; on phones the drawer's foot -->
	{#if textLanguages.languages.length > 1}
		<TextLanguageSwitch languages={textLanguages.languages} source={textLanguages.source} />
	{/if}
	{#if nav.actions}{@render nav.actions()}{/if}
{/snippet}
{#snippet findTools()}
	<PaletteTrigger label={m['palette.search']()} compact onclick={() => (paletteOpen = true)} />
	<!-- the signed-in account → "Benachrichtigungen" (push, the kinds, the app – research/PWA-APPS.md §6) -->
	<a class="who-icon sc-round" href="/notifications" aria-label={pwaLabels(getLocale()).notifications} title={pwaLabels(getLocale()).notifications}><Icon name="bell" /></a>
{/snippet}

<div class="creator sc-tool">
	<AuthGate {session} app="creator" appName="Creator" locale={getLocale()} readerHref={VIEWER_URL}>
		<!-- One stable frame for every page: the top bar (the reader's mark = home, the path, the page's tools), the menu by
		     areas on the left (+ the page's own list beside it), content, and the page's ActionBar at the bottom -->
		<AppShell
			mode={nav.fill ? 'fill' : 'page'}
			sidebarWidth={pageList ? `calc(${menuWidth} + ${listWidth})` : menuWidth}
			resizeKey={pageList && !listFolded ? (railed ? 'creator-rail' : 'creator') : undefined}
			resizeLabel={m['creator.resize_sidebar']()}
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
			drawerSide="right"
			sidebarOpen={menuOpen}
			onclose={() => (menuOpen = false)}
			closeLabel={m['creator.nav_close']()}
		>
			{#snippet topbar()}
				<header class="creator-bar" data-testid="creator-bar">
					<!-- the reader's square mark, a full-height cell: home (the dashboard) -->
					<a class="brand" href="/" title={m['creator.dashboard']()} aria-label={m['creator.dashboard']()} data-testid="creator-brand">{@render logo()}</a>
					{#if !phone.current}
						<Breadcrumbs items={nav.crumbs} label={m['creator.breadcrumb']()} moreLabel={m['creator.breadcrumb_more']()} plain />
					{:else if nav.crumbs.length > 1 && phoneTitle}
						<!-- phones: the place's name (a chapter: its scene); the whole path drops down from it -->
						<PathMenu items={nav.crumbs} current={phoneTitle} />
					{:else}
						<span class="phone-title">{nav.crumbs[0]?.label ?? ''}</span>
					{/if}
					<!-- the page's own thing at the end of the path (the map's toon pill; on phones the strip editor's ⚠ · Save · Publish · •••) -->
					{#if nav.crumbEnd}<span class="crumb-end">{@render nav.crumbEnd()}</span>{/if}
					{#if phone.current}
						{#if page.route.id === '/'}{@render findTools()}{/if}
						<button
							type="button"
							class="burger"
							aria-label={m['creator.nav_menu']()}
							title={m['creator.nav_menu']()}
							aria-expanded={menuOpen}
							aria-controls="creator-nav"
							onclick={() => (menuOpen = !menuOpen)}
							data-testid="menu-button"
						>
							<Icon name="menu" />
						</button>
					{:else}
						{@render pageTools()}
						<span class="vsep" aria-hidden="true"></span>
						{@render findTools()}
					{/if}
				</header>
			{/snippet}
			{#snippet sidebar()}
				{#if phone.current}
					<!-- phones: the areas, the page's own list, Recent; search + notifications + the page's tools at the foot -->
					<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
					<div class="drawer" onclick={closeOnPick}>
						<AreaMenu variant="drawer" toons={nav.toons} toon={toonId} account={session.user?.email ?? ''} badge={waitingCount}>
							{#snippet list()}{#if nav.sidebar}<div class="page-list">{@render nav.sidebar()}</div>{/if}{/snippet}
							{#snippet foot()}
								<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
								<span class="drawer-tools" data-testid="drawer-tools" onclick={(e) => (e.target as HTMLElement).closest('button') && (menuOpen = false)}>{@render findTools()}<span class="spacer"></span>{@render pageTools()}</span>
							{/snippet}
						</AreaMenu>
					</div>
				{:else}
					<div class="side" class:with-list={pageList && !!nav.sidebar}>
						<div class="areas" class:railed>
							<AreaMenu variant={railed ? 'rail' : 'menu'} toons={nav.toons} toon={toonId} account={session.user?.email ?? ''} badge={waitingCount} onfold={() => (shellDock.maximized ? shellDock.restore() : foldMenu(!menuFolded))} />
						</div>
						{#if pageList && nav.sidebar && listFolded}
							<div class="list-rail">
								<Rail side="left" label={listTitle} unfoldLabel={m['creator.unfold_list']()} onunfold={() => foldList(false)} testid="list-rail" />
							</div>
						{:else if pageList && nav.sidebar}
							<div class="page-list">
								<div class="list-head">
									<span class="sc-nav-head">{listTitle}</span>
									<button type="button" class="sc-round sc-quiet" onclick={() => foldList(true)} aria-label={m['creator.fold_list']()} title={m['creator.fold_list']()} aria-expanded="true" data-testid="list-fold"><Icon name="chevrons-left" /></button>
								</div>
								{@render nav.sidebar()}
							</div>
						{/if}
					</div>
				{/if}
			{/snippet}
			{#if toonTabs && toonId}<ToonTabs toon={toonId} />{/if}
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

	/* ── the one top bar (same on every page, only the path changes; CREATOR-NAV.md §2a) ── */
	.creator-bar {
		display: flex;
		align-items: center;
		gap: var(--space-025);
		box-sizing: border-box;
		min-height: var(--top);
		padding: 0 var(--space-05) 0 0;
		border-bottom: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
	}
	/* the reader's square mark, a full-height cell (as wide as the menu's rail): home */
	.brand {
		display: flex;
		flex: none;
		align-items: center;
		justify-content: center;
		box-sizing: border-box;
		width: var(--sc-shell-rail-width);
		align-self: stretch;
		color: var(--ink);
		text-decoration: none;
	}
	:global(.creator .brand img) {
		display: block;
		height: calc(var(--top) - var(--space-05));
		width: auto;
	}
	.creator-bar > :global(.sc-breadcrumbs) {
		padding-left: var(--space-05);
	}
	.phone-title {
		flex: 1 1 auto;
		min-width: 0;
		padding-left: var(--space-075);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.crumb-end {
		display: flex;
		flex: none;
		align-items: center;
		gap: var(--space-025);
	}
	.vsep {
		flex: none;
		align-self: stretch;
		width: var(--border-width-hairline);
		margin: var(--space-05) var(--space-025);
		background: var(--border-muted);
	}
	.spacer {
		flex: 1;
	}
	/* the bar's icon buttons are borderless squares (CREATOR-NAV.md §2: one shape in the row – hover = the nav outline,
	   on = inverted); the pages' own round buttons too while they sit in the bar */
	.creator-bar :global(.sc-round),
	.creator-bar :global(.sc-palette-trigger.compact),
	.drawer-tools :global(.sc-round),
	.drawer-tools :global(.sc-palette-trigger.compact),
	.burger {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		box-sizing: border-box;
		width: auto;
		min-width: var(--control-height);
		min-height: var(--control-height);
		padding: 0;
		border: 0;
		border-radius: var(--radius-none);
		background: transparent;
		color: var(--ink);
	}
	.creator-bar :global(.sc-round:hover:not(:disabled)),
	.creator-bar :global(.sc-palette-trigger.compact:hover),
	.drawer-tools :global(.sc-round:hover:not(:disabled)),
	.drawer-tools :global(.sc-palette-trigger.compact:hover),
	.burger:hover {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-regular));
	}
	.creator-bar :global(.sc-round[aria-pressed='true']),
	.drawer-tools :global(.sc-round[aria-pressed='true']),
	.burger[aria-expanded='true'] {
		background: var(--ink);
		color: var(--paper);
	}
	/* the text language in the bar: a word, no box (the frame comes on hover like the squares') */
	.creator-bar :global(.text-language.sc-control:not(:hover):not(:focus-within)),
	.drawer-tools :global(.text-language.sc-control:not(:hover):not(:focus-within)) {
		border-color: transparent;
	}
	/* phones: one row – the path gives way to the page's end and ☰ */
	@media (max-width: 767px) {
		.creator-bar {
			padding-right: 0;
		}
		.creator-bar > :global(.sc-breadcrumbs ol) {
			flex-wrap: nowrap;
			overflow: hidden;
		}
	}

	/* ── the left side: the menu by areas (or its rail) + the page's own list beside it, each scrolling on its own ── */
	.creator {
		--creator-menu-width: 12rem;
	}
	:global(.creator .sc-shell-sidebar) {
		display: flex;
		flex-direction: column;
	}
	/* the columns scroll on their own; the region itself never scrolls (sideways it cut the menu's words) */
	:global(.creator .sc-shell-sidebar:has(> .side)) {
		overflow: hidden;
	}
	.side {
		display: flex;
		flex: 1 1 auto;
		min-height: 0;
	}
	/* alone the menu is the region (its width is the sidebar's); with the page's list it keeps its own width */
	.areas {
		flex: 1 1 auto;
		min-width: 0;
		overflow: hidden auto;
		overscroll-behavior: contain;
	}
	.with-list .areas {
		flex: none;
		width: var(--creator-menu-width);
	}
	.with-list .areas.railed {
		width: calc(var(--sc-shell-rail-width) - var(--border-width-regular));
	}
	.side > .page-list {
		flex: 1 1 auto;
		min-width: 0;
		overflow-y: auto;
		overscroll-behavior: contain;
		border-left: var(--border-width-regular) solid var(--ink);
	}
	/* phones: the drawer fills the sidebar; its list scrolls, the foot stays */
	:global(.creator .sc-shell-sidebar:has(> .drawer)) {
		overflow: hidden;
	}
	.drawer {
		display: flex;
		flex: 1 1 auto;
		flex-direction: column;
		min-height: 0;
	}
	.list-rail {
		flex: none;
		width: calc(var(--sc-shell-rail-width) - var(--border-width-regular));
		border-left: var(--border-width-regular) solid var(--ink);
	}
	/* the list's head: its place, the quiet fold at the right (CREATOR-NAV d-slice) */
	.list-head {
		position: sticky;
		top: 0;
		z-index: 2;
		background: var(--paper);
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-025);
		padding-right: var(--space-025);
	}
	.list-head .sc-nav-head {
		min-width: 0;
		padding-bottom: var(--space-1);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.drawer .page-list {
		border-top: var(--border-width-regular) solid var(--ink);
		margin-top: var(--space-05);
	}
	.drawer-tools {
		display: flex;
		flex: 1;
		align-items: center;
		gap: var(--space-05);
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
