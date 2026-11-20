<script lang="ts">
	import '@scutoons/tokens';
	import type { Snippet } from 'svelte';
	import { onMount } from 'svelte';
	import { afterNavigate, goto } from '$app/navigation';
	import { page } from '$app/state';
	import {
		AppShell,
		CommandPalette,
		languageCommand,
		linkCommands,
		signOutCommand,
		type Command,
		type PaletteLink,
		installTooltips,
		DialogHost,
		ToastStack,
		provideColourSampler,
		provideUiLabels
	} from '@scutoons/ui';
	import { picturePick } from '$lib/editor/picture-pick.svelte.ts';
	import { toaster } from '$lib/toasts.ts';
	import { InstallBar, UpdateBar, pwaLabels } from '@scutoons/pwa';
	import { APP_NAME, initPwa, install, pwa } from '$lib/pwa.svelte.ts';
	import { AuthGate, canUseApp, takeMagicToken } from '@scutoons/auth';
	import { toonLanguages } from '@scutoons/content';
	import { setCreatorSession } from '$lib/session.svelte.ts';
	import { ADMIN_URL, VIEWER_URL } from '$lib/config.ts';
	import { setupCatalog } from '@scutoons/catalog';
	import { setupMedia } from '@scutoons/media';
	import { assetsApi, library } from '$lib/api.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { getLocale, locales, setLocale } from '$lib/paraglide/runtime.js';
	import { routeId, setCreatorNav } from '$lib/nav.svelte.ts';
	import { recent } from '$lib/recent.svelte.ts';
	import { reviews } from '$lib/editor/reviews.svelte.ts';
	import { PLATFORM_SOURCE_LANGUAGE, textLanguage } from '$lib/editor/text-language.svelte.ts';
	import { addressOf, hrefOf, workspaceOf } from '$lib/shell/address.ts';
	import { routeCrumbs, tabLabel, tabPath } from '$lib/shell/crumbs.ts';
	import { places } from '$lib/shell/places.svelte.ts';
	import { shell } from '$lib/shell/shell.svelte.ts';
	import ShellBar from '$lib/components/shell/shell-bar.svelte';
	import Workspace from '$lib/components/shell/workspace.svelte';

	let { children }: { children: Snippet } = $props();

	const session = setCreatorSession();
	setupMedia({ assets: assetsApi, cdn: asset, locale: getLocale });
	setupCatalog({ locale: getLocale, locales, adminUrl: ADMIN_URL });
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
	onMount(() => installTooltips());
	onMount(initPwa);
	afterNavigate(({ from }) => from && install.check());

	const nav = setCreatorNav();
	const isCreator = $derived(session.isLoggedIn && canUseApp('creator', session.roles));
	let toonsLoaded = false;
	$effect(() => {
		if (!isCreator || toonsLoaded) return;
		toonsLoaded = true;
		library
			.fetchToons({ draft: true })
			.then((list) => (nav.toons = list))
			.catch(() => (toonsLoaded = false));
	});
	$effect(() => {
		if (isCreator) void reviews.load();
	});

	const toonId = $derived((page.params.toonId as string | undefined) ?? null);
	/** the TEXT language switch: a toon's page offers the toon's languages, the universe pages every toon's (`de` first) */
	const textLanguages = $derived.by<{ languages: string[]; source: string }>(() => {
		const toon = toonId ? nav.toons.find((t) => t.id === toonId || t.uri === toonId) : null;
		if (toon) {
			const l = toonLanguages(toon);
			return { languages: l.languages as string[], source: l.source };
		}
		if (toonId) return { languages: [], source: PLATFORM_SOURCE_LANGUAGE };
		return { languages: [...new Set([PLATFORM_SOURCE_LANGUAGE, ...nav.toons.flatMap((t) => toonLanguages(t).languages as string[])])], source: PLATFORM_SOURCE_LANGUAGE };
	});
	$effect(() => {
		textLanguage.page = textLanguages;
	});
	$effect(() => {
		nav.base = routeCrumbs(routeId(page.route.id), page.params, page.url, nav);
	});
	$effect(() => {
		const chapter = page.params.chapterNumber;
		if (isCreator && toonId && chapter) void nav.loadSeasons(toonId);
	});

	// ── tabs: the URL is the front tab (lib/shell); a page that writes its selection into the URL moves its tab along ──
	const startAt = page.url.pathname;
	const resume = shell.settings.reopenTabs ? shell.front : null;
	if (!shell.settings.reopenTabs) shell.closeAll();
	let lastPath = '';
	function sync(navigated: boolean) {
		const { pathname, search } = location;
		const address = addressOf(pathname, search);
		const href = pathname + search;
		if (!navigated && pathname === lastPath) shell.retarget(address, href);
		else shell.visit(address, href);
		lastPath = pathname;
	}
	afterNavigate(() => sync(true));
	onMount(() => {
		if (startAt === '/' && resume) void goto(resume.href, { replaceState: true });
		const { pushState, replaceState } = history;
		const watch = (original: typeof pushState) =>
			function (this: History, ...args: Parameters<typeof pushState>) {
				original.apply(this, args);
				queueMicrotask(() => sync(false));
			};
		history.pushState = watch(pushState);
		history.replaceState = watch(replaceState);
		return () => {
			history.pushState = pushState;
			history.replaceState = replaceState;
		};
	});
	// the front tab's name once the names are known (a reload shows the stored one until then)
	const path = $derived(tabPath(nav.crumbs));
	$effect(() => {
		const front = shell.state.front;
		// only the page that is the front tab names it (during a navigation the old crumbs still stand)
		void page.url;
		if (!front || addressOf(location.pathname, location.search) !== front) return;
		const item = hrefOf(front);
		// an item picked inside an area's page (a character, a catalog entry) has no step of its own: its id names it
		const label = item?.includes('?') ? front.slice(front.indexOf(':') + 1) : tabLabel(tabPath(nav.base), item);
		if (label) shell.rename(front, label);
	});
	const workspace = $derived(shell.state.front ? (workspaceOf(shell.state.front) ?? 'home') : 'home');
	/** where the open item is read: the toon or its chapter in the reader (printed URLs, RULES I1) */
	const reader = $derived(toonId ? `${VIEWER_URL}/${toonId}${page.params.chapterNumber ? `/chapter/${page.params.chapterNumber}` : ''}` : null);

	// where this member was (Home's Continue): every place with a tab, once the toons' names are known
	$effect(() => {
		const crumbs = tabPath(nav.crumbs);
		if (!isCreator || !nav.toons.length || !shell.state.front || !crumbs.length) return;
		recent.visit({ href: page.url.pathname + page.url.search, trail: crumbs.filter((c, i, all) => i === all.length - 1 || !c.href.includes('/season/')).map((c) => c.label), toon: toonId });
	});

	// this machine's settings on the page: the theme (night first), the UI size, motion
	$effect(() => {
		const root = document.documentElement;
		root.dataset.theme = shell.settings.theme;
		root.style.setProperty('--ui-scale', String(shell.settings.scale));
		root.dataset.motion = shell.settings.reduceMotion ? 'reduced' : 'full';
		root.dataset.panelTransitions = shell.settings.panelTransitions && !shell.settings.reduceMotion ? 'on' : 'off';
	});

	/** Alt+ArrowUp: one step up the path; Ctrl+O: open (the Library); Ctrl+,: Settings */
	function onKeydown(event: KeyboardEvent) {
		const mod = event.ctrlKey || event.metaKey;
		if (mod && !event.altKey && !event.shiftKey && (event.key === 'o' || event.key === ',')) {
			event.preventDefault();
			shell.open(event.key === 'o' ? 'library' : 'settings');
			return;
		}
		if (!event.altKey || mod || event.key !== 'ArrowUp') return;
		const target = event.target as HTMLElement | null;
		if (target && (target.tagName === 'SELECT' || target.tagName === 'TEXTAREA')) return;
		if (!nav.parent) return;
		event.preventDefault();
		goto(nav.parent.href);
	}

	// the command palette (Ctrl/⌘K, `/`): the areas, every toon with its seasons and chapters, language, sign out
	let paletteOpen = $state(false);
	$effect(() => {
		if (paletteOpen) places.load(nav.toons);
	});
	const paletteCommands = $derived.by<Command[]>(() => {
		const link = (p: { address: string; label: string; hint?: string; href: string; keywords?: string }): PaletteLink => ({ id: p.address, label: p.hint && p.hint !== m['palette.toon']() ? `${p.hint} · ${p.label}` : p.label, hint: p.hint === m['palette.toon']() ? p.hint : undefined, href: p.href, keywords: `${p.address} ${p.keywords ?? ''}` });
		return [
			...linkCommands([{ id: 'home', label: m['palette.home'](), href: '/', keywords: 'toons home start' }, ...places.areas().map(link)], goto, m['palette.go_to']()),
			...linkCommands(places.toons(nav.toons, (id) => nav.toonName(id), toonId).map(link), goto, m['palette.toons']()),
			languageCommand({ locales, current: getLocale(), setLocale, label: (locale) => m['palette.language']({ locale: locale.toUpperCase() }), group: m['palette.actions']() }),
			signOutCommand({ label: m['creator.sign_out'](), run: () => session.logout(), group: m['palette.actions']() })
		];
	});
	const title = $derived(shell.front?.label);
</script>

<svelte:head>
	<title>{title ? `${title} · ` : ''}{m['creator.title']()}</title>
</svelte:head>

<svelte:window onkeydown={onKeydown} />

<div
	class="creator sc-tool"
	style:--ws-left="{shell.insets.left}rem"
	style:--ws-right="{shell.insets.right}rem"
	data-testid="creator"
>
	<AuthGate {session} app="creator" appName="Creator" locale={getLocale()} readerHref={VIEWER_URL}>
		<!-- the page host: the area's page is the workspace's background until the areas move onto the stage; its dock
		     and bottom region stay the page's own -->
		<AppShell
			mode={nav.fill ? 'fill' : 'page'}
			minContent={24}
			dockLabel={m['creator.dock_label']()}
			dockResizeLabel={m['creator.resize_panel']()}
			dockUnfoldLabel={m['creator.unfold_panel']()}
			pageLabel={nav.crumbs.at(-1)?.label}
			pageFoldLabel={m['creator.fold_page']()}
			pageUnfoldLabel={m['creator.unfold_page']()}
			bottom={nav.bottom ? pageBottom : undefined}
			bottomSize={nav.bottom?.size}
			bottomLabel={nav.bottom?.label}
			bottomIcon={nav.bottom?.icon}
			bottomResizeLabel={nav.bottom?.resizeLabel}
			bottomUnfoldLabel={nav.bottom?.unfoldLabel}
		>
			{#snippet topbar()}
				<ShellBar {path} languages={textLanguages.languages} source={textLanguages.source} {reader} page={nav.actions ?? undefined} />
			{/snippet}
			<!-- another toon = the same route with a new param: the pages load once on mount, so they start anew -->
			{#key toonId}{@render children()}{/key}
		</AppShell>
		<Workspace kind={workspace} tree={nav.sidebar} toon={toonId} />
		<CommandPalette bind:open={paletteOpen} commands={paletteCommands} label={m['palette.label']()} placeholder={m['palette.placeholder']()} hint={m['palette.keys']()} />
		<DialogHost cancelLabel={m['creator.cancel']()} />
	</AuthGate>
	<ToastStack {toaster} dismissLabel={m['creator.close']()} />
	<UpdateBar {pwa} app={APP_NAME} labels={pwaLabels(getLocale())} />
	<InstallBar {install} app={APP_NAME} labels={pwaLabels(getLocale())} hidden={pwa.updateReady} />
</div>

{#snippet pageBottom()}{@render nav.bottom?.panel()}{/snippet}

<style>
	.creator {
		--cyan: var(--primary-cyan);
		--muted: var(--text-muted);
		--error: var(--state-error);
		--warn: var(--state-warning);
		--action-bar-height: var(--tool-bar-height);
		/* the bar's row: the pills and a gap above and below (the panels start a gap under it) */
		--shell-top: calc(var(--control-height) + 1.4rem);
		--top: var(--shell-top);
		min-height: 100dvh;
	}
	/* the page gives way to the panels at the walls (Workspace sets the room) */
	.creator :global(.sc-shell-frame) {
		padding-left: var(--ws-left, 0);
		padding-right: var(--ws-right, 0);
		transition: padding 0.18s ease;
	}
	.creator :global(.sc-shell-top) {
		border-bottom: 0;
		background: var(--editor-stage);
	}
	:global(html[data-motion='reduced'] *),
	:global(html[data-panel-transitions='off'] [data-float]) {
		transition: none !important;
		animation: none !important;
	}

	/* ── Creator-wide controls (fields, focus, headings: tools.css; @scutoons/ui components style themselves) ── */
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
