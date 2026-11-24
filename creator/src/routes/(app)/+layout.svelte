<script lang="ts">
	import '@scutoons/tokens';
	import type { Snippet } from 'svelte';
	import { onMount, untrack } from 'svelte';
	import { afterNavigate, goto } from '$app/navigation';
	import { page } from '$app/state';
	import { installTooltips, DialogHost, ToastStack, provideColourSampler, provideUiLabels } from '@scutoons/ui';
	import { useOverlayLevel } from '@scutoons/ui/editor';
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
	import { libraryHost } from '$lib/editor/library-drop.ts';
	import { assetsApi, library } from '$lib/api.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { getLocale, locales } from '$lib/paraglide/runtime.js';
	import { routeId, setCreatorNav } from '$lib/nav.svelte.ts';
	import { recent } from '$lib/recent.svelte.ts';
	import { reviews } from '$lib/editor/reviews.svelte.ts';
	import { PLATFORM_SOURCE_LANGUAGE, textLanguage } from '$lib/editor/text-language.svelte.ts';
	import { addressOf, hrefOf, pickedInArea, startsAtTop, workspaceOf } from '$lib/shell/address.ts';
	import { pageLabel } from '$lib/shell/tabs.ts';
	import { routeCrumbs, tabLabel, tabPath } from '$lib/shell/crumbs.ts';
	import { isSearchKey, isSwitcherKey } from '$lib/shell/launcher.ts';
	import { ITEM_VIEW } from '$lib/shell/stage.ts';
	import { saveWorkspace } from '$lib/shell/workspace-actions.ts';
	import { shell } from '$lib/shell/shell.svelte.ts';
	import ShellBar from '$lib/components/shell/shell-bar.svelte';
	import Workspace from '$lib/components/shell/workspace.svelte';
	import StatusBar from '$lib/components/shell/status-bar.svelte';
	import Dashboard from '$lib/components/shell/dashboard.svelte';
	import Launcher from '$lib/components/shell/launcher.svelte';
	import CraftsQuestion from '$lib/components/shell/crafts-question.svelte';
	import ResumeOffer from '$lib/components/shell/resume-offer.svelte';
	import ReloadNotice from '$lib/components/shell/reload-notice.svelte';
	import { devices } from '$lib/shell/devices.svelte.ts';
	import { snapshots } from '$lib/shell/snapshots.svelte.ts';
	import { crafts } from '$lib/shell/crafts.svelte.ts';
	import { keepScroll, resume, resumeScroll } from '$lib/shell/resume.svelte.ts';
	import type { DashboardState } from '$lib/shell/resume.ts';
	import { openings } from '$lib/shell/openings.svelte.ts';
	import { startDesktop } from '$lib/shell/desktop.svelte.ts';
	import { log } from '$lib/log/log.svelte.ts';

	let { children }: { children: Snippet } = $props();

	const session = setCreatorSession();
	// the creator's dialogs and sheets open on the overlay level (RULES H15)
	useOverlayLevel();
	setupMedia({ assets: assetsApi, cdn: asset, locale: getLocale, host: libraryHost });
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
	// what the member does: the workspaces they start with, and the first login's question
	$effect(() => {
		if (isCreator) void crafts.load();
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
	// reopening resumes like a hibernated computer (lib/shell/resume.ts): the kept tabs with the URL opened, the dashboard
	// as it was, each workspace's full screen, the scroll positions; a link opened is the front tab
	const start = resume.start({ address: addressOf(page.url.pathname, page.url.search), href: page.url.pathname + page.url.search });
	/** the dashboard as it was, once the start's page is there */
	let startDashboard: DashboardState | null = start.dashboard;
	let lastPath = '';
	function sync(navigated: boolean) {
		const { pathname, search } = location;
		const address = addressOf(pathname, search);
		const href = pathname + search;
		if ((!navigated && pathname === lastPath) || pickedInArea(shell.state.front, address)) shell.retarget(address, href);
		else shell.visit(address, href);
		lastPath = pathname;
	}
	afterNavigate(() => sync(true));
	let stageEl: HTMLDivElement | undefined = $state();
	afterNavigate(({ from, to, type }) => {
		if (startsAtTop(from?.url, to?.url, type)) stageEl?.scrollTo({ top: 0 });
	});
	// nothing open: the dashboard lies over the empty stage; going to an item puts it away
	afterNavigate(() => {
		if (startDashboard !== null && (!start.go || location.pathname === new URL(start.go, location.origin).pathname)) {
			shell.dashboard = startDashboard;
			startDashboard = null;
			resumeScroll();
			return;
		}
		shell.dashboard = location.pathname === '/' ? shell.dashboard || 'cards' : false;
	});
	$effect(() => resume.dashboard(shell.dashboard));
	onMount(keepScroll);
	// another member signed in on this browser: their own tabs come back
	$effect(() => {
		const id = session.user?.id;
		if (id == null) return;
		const go = untrack(() => resume.member(String(id)));
		if (go && go !== location.pathname + location.search) void goto(go, { replaceState: true });
	});
	// the state follows the member across devices (lib/shell/devices.ts): this device's own first, another's newer one offered
	$effect(() => {
		const id = session.user?.id;
		if (id == null || !isCreator) return;
		untrack(() => void devices.start(String(id)));
	});
	onMount(() => () => devices.stop());
	// each workspace's picture as it was left (lib/shell/snapshots.svelte.ts): taken once it holds still, sent on leaving
	$effect(() => {
		if (session.user?.id == null || !isCreator) return;
		untrack(() => snapshots.start());
		return () => snapshots.stop();
	});
	$effect.pre(() => {
		const id = shell.workspace;
		const away = !!shell.dashboard;
		untrack(() => snapshots.front(id, away));
	});
	// the resumed tabs checked as the lists arrive: an item gone or no longer the member's leaves quietly, with a Log line
	$effect(() => {
		if (!isCreator || !shell.state.tabs.length) return;
		untrack(() => void openings.load());
	});
	$effect(() => {
		const lists = Object.fromEntries(Object.entries(openings.rows).map(([kind, rows]) => [kind, rows.map((r) => r.address)]));
		const toons = nav.toons.length ? nav.toons.flatMap((t) => [t.id, t.uri].filter((v): v is string => !!v)) : null;
		untrack(() => {
			const front = shell.state.front;
			const out = resume.check(lists, toons);
			if (!out) return;
			for (const name of out.gone) log.post({ level: 'info', text: m['shell.resume_dropped']({ name }), scope: '' });
			if (front && out.tabs.front !== front && addressOf(location.pathname, location.search) === front) {
				void goto(out.tabs.tabs.find((t) => t.address === out.tabs.front)?.href ?? '/', { replaceState: true });
			}
		});
	});
	// each workspace's full screen kept, and the kept one back once its view has content
	$effect(() => {
		const floats = shell.floats;
		const full = shell.full;
		const available = shell.available;
		if (!floats) return;
		untrack(() => {
			const back = resume.full(floats.workspace, full, available);
			if (back) floats.setState(back, 'full');
		});
	});
	onMount(() => {
		if (start.go) void goto(start.go, { replaceState: true });
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
	// after the resumed start: a link handed to the installed creator goes last, so it wins
	onMount(startDesktop);
	// the front tab's name once the names are known (a reload shows the stored one until then)
	const path = $derived(tabPath(nav.crumbs));
	$effect(() => {
		const front = shell.state.front;
		// only the page that is the front tab names it (during a navigation the old crumbs still stand)
		void page.url;
		if (!front || addressOf(location.pathname, location.search) !== front) return;
		const item = hrefOf(front);
		// an item picked inside an area's page (a character, a catalog entry) has no step of its own: its id names it
		const label = item?.includes('?') ? front.slice(front.indexOf(':') + 1) : pageLabel(nav.extra, item) || tabLabel(tabPath(nav.base), item);
		if (label) shell.rename(front, label);
	});
	const workspace = $derived(shell.state.front ? (workspaceOf(shell.state.front) ?? 'home') : 'home');
	/** where the open item is read: the toon or its chapter in the reader (printed URLs, RULES I1) */
	const reader = $derived(toonId ? `${VIEWER_URL}/${toonId}${page.params.chapterNumber ? `/chapter/${page.params.chapterNumber}` : ''}` : null);

	// where this member was (the dashboard's Last opened): every place with a tab, once the toons' names are known
	$effect(() => {
		const crumbs = tabPath(nav.crumbs);
		if (!isCreator || !nav.toons.length || !shell.state.front || !crumbs.length) return;
		recent.visit({ href: page.url.pathname + page.url.search, trail: crumbs.filter((c, i, all) => i === all.length - 1 || !c.href.includes('/season/')).map((c) => c.label), toon: toonId });
	});

	// this machine's settings on the page: the theme (night first), the colour-blind set, the UI size, motion
	$effect(() => {
		const root = document.documentElement;
		root.dataset.theme = shell.settings.theme;
		if (shell.settings.colorblind) root.dataset.colorblind = '';
		else delete root.dataset.colorblind;
		root.style.setProperty('--ui-scale', String(shell.settings.scale));
		root.dataset.motion = shell.settings.reduceMotion ? 'reduced' : 'full';
		root.dataset.panelTransitions = shell.settings.panelTransitions && !shell.settings.reduceMotion ? 'on' : 'off';
	});

	/** Alt Space / Cmd Space / Ctrl O / `/`: Search; Ctrl K / Cmd K / Alt Tab (where the page gets it): the switcher; Alt+ArrowUp: one step up the path; Ctrl+,: Settings; Alt+V: the ☰ menu's Views; Ctrl+S: Save workspace where the page has no documents */
	function onKeydown(event: KeyboardEvent) {
		const mod = event.ctrlKey || event.metaKey;
		const target = event.target instanceof Element ? event.target : null;
		const typing = !!target?.closest('input, textarea, select, [contenteditable=""], [contenteditable="true"]');
		if (isSwitcherKey(event)) {
			event.preventDefault();
			shell.toggleSwitcher();
			return;
		}
		if (isSearchKey(event, typing)) {
			event.preventDefault();
			shell.toggleLauncher();
			return;
		}
		if (event.altKey && !mod && !event.shiftKey && event.code === 'KeyV') {
			event.preventDefault();
			shell.openViews();
			return;
		}
		// a page with documents of its own saves them on Ctrl S (lib/editor/editor-keys.ts)
		if (mod && !event.altKey && !event.shiftKey && event.key.toLowerCase() === 's' && !shell.doc?.docs?.length) {
			event.preventDefault();
			void saveWorkspace();
			return;
		}
		if (mod && !event.altKey && !event.shiftKey && event.key === ',') {
			event.preventDefault();
			shell.open('settings');
			return;
		}
		if (!event.altKey || mod || event.key !== 'ArrowUp') return;
		if (target && (target.tagName === 'SELECT' || target.tagName === 'TEXTAREA')) return;
		if (!nav.parent) return;
		event.preventDefault();
		goto(nav.parent.href);
	}

	const title = $derived(shell.front?.label);
</script>

<svelte:head>
	<title>{title ? `${title} · ` : ''}{m['creator.title']()}</title>
</svelte:head>

<svelte:window onkeydown={onKeydown} />

<div class="creator sc-tool sc-desktop-only" class:item-full={shell.full === ITEM_VIEW && !shell.dashboard && !shell.launcher} data-testid="creator">
	<AuthGate {session} app="creator" appName="Creator" locale={getLocale()} readerHref={VIEWER_URL}>
		<!-- the stage is the whole window, under the bar too: the open item's page is its ground; the bar's pills, the
		     panels and the status bar float on it, and the stage never gives way to them -->
		<div class="stage" bind:this={stageEl} class:fill={nav.fill} class:clear={shell.clear} class:sheet={!nav.fill && workspace !== 'home'} data-testid="stage">
			<!-- another toon = the same route with a new param: the pages load once on mount, so they start anew -->
			<div class="stage-page">{#key toonId}{@render children()}{/key}</div>
		</div>
		<ShellBar {reader} />
		<Workspace kind={workspace} toon={toonId} />
		<StatusBar />
		<ReloadNotice />
		{#if shell.dashboard}<Dashboard />{/if}
		{#if shell.launcher}<Launcher />{/if}
		{#if crafts.ask}<CraftsQuestion />{:else if devices.offer}<ResumeOffer />{/if}
		<DialogHost cancelLabel={m['creator.cancel']()} />
	</AuthGate>
	<ToastStack {toaster} dismissLabel={m['creator.close']()} />
	<UpdateBar {pwa} app={APP_NAME} labels={pwaLabels(getLocale())} />
	<InstallBar {install} app={APP_NAME} labels={pwaLabels(getLocale())} hidden={pwa.updateReady} />
</div>

<style>
	/* the window and every page on it stand on the stage (--tool-ground: black at night, white by day; RULES H11) */
	.creator {
		background: var(--tool-ground);
		/* the bar's row: the kit's bar pills and a margin above and below (deck-kit bar.rs; the panels start under it) */
		--shell-top: calc(var(--kit-bar) + 2 * var(--kit-margin));
		--top: var(--shell-top);
		/* the status bar along the bottom and the room it takes (the panels end above it) */
		--status-height: var(--kit-status);
		--status-room: calc(var(--status-height) + 2 * var(--kit-margin));
		min-height: 100dvh;
	}
	.stage {
		position: fixed;
		inset: 0;
		box-sizing: border-box;
		padding-bottom: var(--status-room);
		overflow: auto;
		background: var(--tool-ground);
		color: var(--tool-text);
	}
	/* the item's own view taken full (RULES H15): the stage is the box over the overlay's veil, close to the window's edges */
	.creator.item-full .stage {
		z-index: 91;
		inset: var(--kit-margin);
		padding-top: 0;
		padding-bottom: 0;
		border: var(--editor-edge) solid var(--kit-overlay-box-border);
		border-radius: var(--kit-radius-box);
	}
	/* a view clear of the bar keeps its content under the bar's row (the stage itself still runs under it) */
	.stage.clear {
		padding-top: var(--shell-top);
	}
	.stage.fill {
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}
	.stage-page {
		display: flex;
		flex-direction: column;
		min-height: 100%;
	}
	/* a page that scrolls (a form, a board) is a column in the stage's middle, on the stage itself – no sheet, no frame
	   (RULES H11); the panels float beside it */
	.stage.sheet .stage-page {
		box-sizing: border-box;
		width: min(60rem, 100% - 2rem);
		min-height: 0;
		margin: 0 auto 1rem;
	}
	.stage.fill .stage-page {
		flex: 1;
		min-height: 0;
		height: 100%;
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
	/* buttons on the deck editor's dogma (deck/kit/src/editor/controls.rs, #750): quiet controls on the control ground with
	   the radius; an option that is on is pressed (`pills`); a lone icon shows its ground only under the pointer
	   (`icon_button`); the view's one primary action is the pressed fill (`primary`) */
	:global(:where(.creator button)),
	:global(:where(.creator .btn)) {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-025);
		box-sizing: border-box;
		min-height: var(--control-height);
		padding: 0 var(--editor-pad);
		line-height: var(--type-button-line);
		background: var(--tool-control);
		color: var(--tool-text);
		border: var(--editor-edge) solid transparent;
		border-radius: var(--editor-radius);
		cursor: pointer;
		text-decoration: none;
	}
	:global(:where(.creator button:hover:not(:disabled))),
	:global(:where(.creator .btn:hover)) {
		background: var(--tool-hover);
	}
	:global(:where(.creator button.secondary)),
	:global(:where(.creator .btn.secondary)) {
		background: var(--tool-control);
		color: var(--tool-text);
	}
	:global(:where(.creator button.ghost)) {
		background: transparent;
		color: var(--tool-text);
		padding: 0 var(--space-05);
	}
	:global(:where(.creator button.ghost:hover:not(:disabled))) {
		background: var(--tool-hover);
	}
	:global(:where(.creator button.active)),
	:global(:where(.creator button[aria-pressed='true'])),
	:global(:where(.creator button.active:hover:not(:disabled))) {
		background: var(--tool-pressed);
		color: var(--tool-pressed-text);
	}
	:global(:where(.creator button.primary)),
	:global(:where(.creator .btn.primary)) {
		background: var(--tool-pressed);
		color: var(--tool-pressed-text);
		border-color: var(--tool-pressed);
	}
	:global(:where(.creator button.primary:hover:not(:disabled))) {
		background: var(--tool-pressed);
	}
	:global(:where(.creator button:disabled)) {
		opacity: 0.5;
		cursor: not-allowed;
	}
	:global(:where(.creator label)) {
		display: block;
	}
	:global(:where(.creator .hint)) {
		color: var(--tool-muted);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
</style>
