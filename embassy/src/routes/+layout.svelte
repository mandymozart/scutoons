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
		ToastStack,
		languageCommand,
		linkCommands,
		otherLocale as nextLocale,
		signOutCommand,
		type Command,
		installTooltips,
	} from '@scutoons/ui';
	import { TRACKS, type OrgSummary } from '@scutoons/api';
	import { AuthGate, takeMagicToken } from '@scutoons/auth';
	import { setEmbassySession } from '$lib/session.svelte.ts';
	import { setToaster } from '$lib/toasts.ts';
	import { embassy } from '$lib/api.ts';
	import { loadPace, pace } from '$lib/pace.svelte.ts';
	import Onboarding from '$lib/components/onboarding.svelte';
	import { label } from '$lib/labels.ts';
	import { VIEWER_URL } from '$lib/config.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { getLocale, locales, setLocale } from '$lib/paraglide/runtime.js';

	let { children }: { children: Snippet } = $props();

	const session = setEmbassySession();
	const toaster = setToaster();

	onMount(() => {
		// magic link from the login mail: `<embassy origin>/?login=<token>`; else the shared session (single sign-on)
		const magic = takeMagicToken(page.url);
		if (magic) goto(magic.cleaned, { replaceState: true });
		void session.init(magic?.token);
	});

	// the person's pace: not onboarded yet → the onboarding instead of the page (owner 2026-10-05)
	$effect(() => {
		if (session.user?.id) loadPace();
	});

	// every `title` in the tool shows in the one SCUTOONS tooltip (tools.css .sc-tooltip)
	onMount(() => installTooltips());

	// role gate (AuthGate): liaison / ambassador / admin – the server checks it again on every /embassy call
	const isAmbassador = $derived(session.hasRole('ambassador') || session.hasRole('admin'));
	const otherLocale = $derived(nextLocale(locales, getLocale()));

	const nav = $derived([
		{ href: '/', label: m['nav.today'](), icon: 'clock' },
		{ href: '/leads', label: m['nav.leads'](), icon: 'list' },
		{ href: '/orgs/new', label: m['nav.new'](), icon: 'plus' },
		...(isAmbassador ? [{ href: '/import', label: m['nav.import'](), icon: 'upload' }] : [])
	]);
	// ── Command palette (Ctrl/⌘K, `/`; @scutoons/ui CommandPalette like the admin): the sections, lead views, every
	// organisation (loaded on the first open, the API's 500 maximum), "leads matching …", language, sign out ──
	let paletteOpen = $state(false);
	let orgs = $state<OrgSummary[]>([]);
	let orgsRequested = false;
	$effect(() => {
		if (!paletteOpen || orgsRequested) return;
		orgsRequested = true;
		embassy
			.orgs({ sort: 'name', limit: 500 })
			.then((result) => (orgs = result.items))
			.catch(() => (orgsRequested = false));
	});
	const paletteCommands = $derived.by<Command[]>(() => {
		const actions = m['palette.actions']();
		return [
			...linkCommands(
				nav.map((item) => ({ id: `nav-${item.href}`, label: item.href === '/orgs/new' ? m['palette.new_org']() : item.label, href: item.href })),
				goto,
				m['palette.go_to']()
			),
			...linkCommands(
				[
					{ id: 'leads-mine', label: m['palette.mine'](), href: '/leads?owner=me' },
					{ id: 'leads-due', label: m['palette.due'](), href: '/leads?due=1', keywords: 'overdue fällig' },
					{ id: 'leads-quiet', label: m['palette.quiet'](), href: '/leads?quiet=1', keywords: 'dormant quiet reconnect ruhig' },
					...TRACKS.map((track) => ({ id: `leads-${track}`, label: m['palette.track']({ track: label('track', track) }), href: `/leads?track=${track}` }))
				],
				goto,
				m['palette.leads']()
			),
			...linkCommands(
				orgs.map((org) => ({
					id: `org-${org.id}`,
					label: org.name,
					hint: [label('type', org.type), org.city, org.country].filter(Boolean).join(' · '),
					keywords: org.tags.join(' '),
					href: `/orgs/${encodeURIComponent(org.id)}`
				})),
				goto,
				m['palette.orgs']()
			),
			languageCommand({ locales, current: getLocale(), setLocale, label: (locale) => m['palette.language']({ locale: locale.toUpperCase() }), group: actions }),
			signOutCommand({ label: m['shell.sign_out'](), run: () => session.logout(), group: actions })
		];
	});
	function dynamicCommands(query: string): Command[] {
		if (!query) return [];
		return [{ id: 'search-leads', label: m['palette.search_leads']({ q: query }), group: m['palette.find'](), run: () => goto(`/leads?q=${encodeURIComponent(query)}`) }];
	}

	const isCurrent = (href: string) =>
		href === '/' ? page.url.pathname === '/' : page.url.pathname === href || (href === '/leads' && page.url.pathname.startsWith('/orgs/') && page.url.pathname !== '/orgs/new');
</script>

<svelte:head>
	<title>{m['shell.title']()}</title>
</svelte:head>

<div class="embassy sc-tool">
	<AuthGate {session} app="embassy" appName="Embassy" locale={getLocale()} readerHref={VIEWER_URL}>
		<!-- the tools' frame (@scutoons/ui AppShell, agents/DOCKING.md): top bar and the page -->
		<AppShell>
			{#snippet topbar()}
				<header class="topbar">
					<a class="brand" href="/">
						<img src="/SCUTOONS-logo.svg" alt="SCUTOONS" width="120" height="20" />
						<span>{m['shell.embassy']()}</span>
					</a>
					<nav class="nav" aria-label={m['shell.navigation']()}>
						{#each nav as item (item.href)}
							<a href={item.href} aria-current={isCurrent(item.href) ? 'page' : undefined}>
								<span class="icon"><Icon name={item.icon} /></span><span class="label">{item.label}</span>
							</a>
						{/each}
					</nav>
					<PaletteTrigger label={m['palette.search']()} compact onclick={() => (paletteOpen = true)} />
				</header>
			{/snippet}
			<main>
				{#if pace.loaded && pace.available && !pace.value}
					<Onboarding />
				{:else}
					{@render children()}
				{/if}
			</main>
		</AppShell>
		<CommandPalette
			bind:open={paletteOpen}
			commands={paletteCommands}
			dynamic={dynamicCommands}
			label={m['palette.label']()}
			placeholder={m['palette.placeholder']()}
			hint={m['palette.keys']()}
		/>
	</AuthGate>
	<ToastStack {toaster} dismissLabel={m['shell.dismiss']()} />
</div>

<style>
	/* Embassy chrome: DESIGN.md Part 1 like the admin (IBM Plex Mono, ink/paper, square, 2px rules, cyan as fill),
	   but phone first: the Today list is worked on the go, the navigation sits at the bottom below 768px.
	   DESIGN.md "Brutalist tools": the reader's fluid root and type styles (tools.css via `sc-tool`), no px font sizes. */
	.embassy {
		--topbar-height: var(--tool-bar-height);
		--bottom-nav-height: max(3.5rem, 56px);
		min-height: 100dvh;
	}
	.topbar {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		height: var(--topbar-height);
		padding: 0 var(--space-1);
		box-sizing: border-box;
		background: var(--paper);
		border-bottom: var(--border-width-regular) solid var(--ink);
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
	/* touch screens: the logo link is a full-height target (BACKLOG 10.50 audit, E.9) */
	@media (pointer: coarse) {
		.brand {
			min-height: var(--control-height);
		}
	}
	.nav {
		display: flex;
		gap: var(--space-025);
		margin-left: var(--space-1);
	}
	.nav a {
		display: flex;
		align-items: center;
		gap: var(--space-025);
		padding: var(--space-025) var(--space-075);
		color: var(--ink);
		text-decoration: none;
		border: var(--border-width-hairline) solid transparent;
	}
	.nav a:hover {
		border-color: var(--ink);
	}
	/* current = inverted (DESIGN.md "Brutalist tools"; cyan is for live states only) */
	.nav a[aria-current='page'] {
		background: var(--ink);
		color: var(--paper);
		border-color: var(--ink);
	}
	/* tablets (BACKLOG 10.50 audit, E.9): the reader's 24px root made brand + four labelled entries wider than 768px –
	   the top nav shows its icons only up to 1023px (the label stays the accessible name) */
	@media (min-width: 768px) and (max-width: 1023px) {
		.nav .label {
			position: absolute;
			width: 1px;
			height: 1px;
			overflow: hidden;
			clip-path: inset(50%);
			white-space: nowrap;
		}
	}
	main {
		max-width: 64rem;
		margin: 0 auto;
		padding: var(--space-15) var(--space-15) var(--space-3);
		box-sizing: border-box;
	}
	@media (max-width: 767px) {
		.nav {
			position: fixed;
			left: 0;
			right: 0;
			bottom: 0;
			z-index: 10;
			margin: 0;
			gap: 0;
			height: var(--bottom-nav-height);
			padding-bottom: env(safe-area-inset-bottom);
			background: var(--paper);
			border-top: var(--border-width-regular) solid var(--ink);
		}
		.nav a {
			flex: 1;
			flex-direction: column;
			justify-content: center;
			gap: 0;
			padding: 0;
			font-size: var(--type-caption-size);
			border: none;
		}
		.nav a[aria-current='page'] {
			border: none;
		}
		.nav .icon {
			font-size: 1.25rem;
			line-height: 1.2;
		}
		main {
			padding: var(--space-1) var(--space-1) calc(var(--bottom-nav-height) + var(--space-3));
		}
	}

	/* Embassy-wide content styles (@scutoons/ui components style themselves) */
	:global(:where(.embassy) a) {
		color: var(--ink);
	}
	:global(:where(.embassy) h2) {
		margin: var(--space-15) 0 var(--space-05);
	}
	/* fields, focus, checkboxes: tools.css (2px ink frame, --control-height, 3px focus outline) */
	:global(:where(.embassy) :is(input:not([type='checkbox']):not([type='radio']):not([type='file']), select, textarea)) {
		width: 100%;
	}
	:global(.embassy .muted) {
		color: var(--text-muted);
	}
	:global(.embassy .caption) {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	:global(.embassy label.field) {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		min-width: 0;
	}
	:global(.embassy label.field > span) {
		font-size: var(--type-caption-size);
		color: var(--text-muted);
	}
	:global(.embassy label.check) {
		display: flex;
		align-items: center;
		gap: var(--space-05);
	}
	/* the forms: tools.css .sc-fields data-cols="fit", 14rem columns */
	:global(.embassy .sc-fields) {
		--fields-fit-min: 14rem;
	}
	:global(.embassy .sc-fields .wide) {
		grid-column: 1 / -1;
	}
	:global(.embassy .row) {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-05);
	}
	:global(.embassy .card) {
		padding: var(--space-1);
		border: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
	}
	:global(.embassy .error-box) {
		padding: var(--space-075);
		border: var(--border-width-regular) solid var(--state-error);
	}
</style>
