<script lang="ts">
	import '@scutoons/tokens';
	import type { Snippet } from 'svelte';
	import { onMount } from 'svelte';
	import { afterNavigate, goto } from '$app/navigation';
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
		type Command
	} from '@scutoons/ui';
	import { AuthGate, takeMagicToken } from '@scutoons/auth';
	import { setAdminSession } from '$lib/session.svelte.ts';
	import { setToaster } from '$lib/toasts.ts';
	import { activeItem, navigation, navItems } from '$lib/navigation.ts';
	import { VIEWER_URL } from '$lib/config.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { getLocale, locales, setLocale } from '$lib/paraglide/runtime.js';

	let { children }: { children: Snippet } = $props();

	const session = setAdminSession();
	const toaster = setToaster();

	let menuOpen = $state(false);
	let paletteOpen = $state(false);

	onMount(() => {
		// magic link from the login mail: `<admin origin>/?login=<token>`; else the shared session (single sign-on)
		const magic = takeMagicToken(page.url);
		if (magic) goto(magic.cleaned, { replaceState: true });
		void session.init(magic?.token);
	});

	afterNavigate(() => (menuOpen = false));

	// role gate (AuthGate): `admin` – the server checks it again on every /admin call
	const current = $derived(activeItem(page.url.pathname));

	const otherLocale = $derived(nextLocale(locales, getLocale()));

	const commands = $derived<Command[]>([
		...linkCommands(
			navItems.map((item) => ({ id: `nav-${item.id}`, label: item.title(), hint: item.description(), keywords: item.keywords, href: item.href })),
			goto,
			m['palette.go_to']()
		),
		...linkCommands(
			[
				{ id: 'new-batch', label: m['palette.new_batch'](), keywords: 'batch edition', href: '/batches?new=1' },
				{ id: 'generate', label: m['palette.generate'](), keywords: 'code qr', href: '/codes/new' },
				{ id: 'revoked', label: m['palette.revoked_codes'](), href: '/codes?status=revoked' },
				{ id: 'admins', label: m['palette.admins'](), href: '/users?role=admin' }
			],
			goto,
			m['palette.actions']()
		),
		languageCommand({ locales, current: getLocale(), setLocale, label: (locale) => m['palette.language']({ locale }), group: m['palette.actions']() }),
		signOutCommand({ label: m['shell.sign_out'](), run: () => session.logout(), group: m['palette.actions']() })
	]);

	function dynamicCommands(query: string): Command[] {
		if (!query) return [];
		const group = m['palette.search']();
		const list: Command[] = [];
		if (/^[A-Za-z0-9_-]{1,32}$/.test(query)) {
			list.push({ id: 'open-code', label: m['palette.open_code']({ code: query }), group, run: () => goto(`/codes/${encodeURIComponent(query)}`) });
		}
		list.push(
			{ id: 'search-codes', label: m['palette.search_codes']({ q: query }), group, run: () => goto(`/codes?q=${encodeURIComponent(query)}`) },
			{ id: 'search-users', label: m['palette.search_users']({ q: query }), group, run: () => goto(`/users?q=${encodeURIComponent(query)}`) },
			{ id: 'search-events', label: m['palette.search_events']({ q: query }), group, run: () => goto(`/events?q=${encodeURIComponent(query)}`) }
		);
		return list;
	}
</script>

<svelte:head>
	<title>{current ? `${current.title()} · ` : ''}{m['shell.title']()}</title>
</svelte:head>

<div class="admin sc-tool">
	<AuthGate {session} app="admin" appName="Admin" locale={getLocale()} readerHref={VIEWER_URL}>
		<AppShell
			sidebarWidth="var(--admin-sidebar-width)"
			sidebarId="admin-nav"
			sidebarLabel={m['shell.navigation']()}
			collapsible
			sidebarOpen={menuOpen}
			onclose={() => (menuOpen = false)}
			closeLabel={m['shell.close_menu']()}
		>
			{#snippet topbar()}
				<header class="topbar">
					<button
						type="button"
						class="burger"
						aria-expanded={menuOpen}
						aria-controls="admin-nav"
						onclick={() => (menuOpen = !menuOpen)}
					>
						<Icon name="menu" /><span class="sr-only">{m['shell.menu']()}</span>
					</button>
					<a class="brand" href="/">
						<img src="/SCUTOONS-logo.svg" alt="SCUTOONS" width="120" height="20" />
						<span class="brand-name">{m['shell.admin']()}</span>
					</a>
					<PaletteTrigger label={m['shell.search']()} onclick={() => (paletteOpen = true)} />
					<div class="account">
						<span class="email" title={session.user?.email}>{session.user?.email}</span>
						<button type="button" class="plain" onclick={() => setLocale(otherLocale)} title={m['shell.language']()}>
							{otherLocale.toUpperCase()}
						</button>
						<button type="button" class="plain" data-testid="sign-out" onclick={() => session.logout()} aria-label={m['shell.sign_out']()} title={m['shell.sign_out']()}><Icon name="log-out" /><span class="wide-only">{m['shell.sign_out']()}</span></button>
					</div>
				</header>
			{/snippet}
			{#snippet sidebar()}
				<div class="sidebar">
					{#each navigation as group (group.id)}
						<p class="sc-nav-head">{group.title()}</p>
						<ul>
							{#each group.items as item (item.id)}
								<li>
									<a href={item.href} aria-current={current?.id === item.id ? 'page' : undefined}>
										<span class="icon"><Icon name={item.icon} /></span>
										<span class="title">{item.title()}</span>
										<span class="description">{item.description()}</span>
									</a>
								</li>
							{/each}
						</ul>
					{/each}
				</div>
			{/snippet}
			<main>
				{@render children()}
			</main>
		</AppShell>
		<CommandPalette
			bind:open={paletteOpen}
			{commands}
			dynamic={dynamicCommands}
			label={m['palette.label']()}
			placeholder={m['palette.placeholder']()}
			emptyLabel={m['palette.empty']()}
			hint={m['palette.keys']()}
		/>
	</AuthGate>
	<ToastStack {toaster} dismissLabel={m['shell.dismiss']()} />
</div>

<style>
	/* Admin chrome: DESIGN.md "Brutalist tools" – the reader's fluid root and type styles (tools.css via `sc-tool`:
	   1rem text, caption the smallest), 2px ink frames, square, ≥ 44px controls, selected = inverted, cyan only "live".
	   No px font sizes and no root override. */
	.admin {
		--topbar-height: var(--tool-bar-height);
		--action-bar-height: var(--topbar-height);
		--split-view-top: calc(var(--topbar-height) + var(--space-1));
		--split-view-bottom: calc(var(--action-bar-height) + var(--space-1));
		--admin-sidebar-width: 14rem;
		--info-panel-width: var(--tool-panel-width);
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
	}
	.brand img {
		display: block;
		height: 1.25rem;
		width: auto;
	}
	/* the palette trigger (@scutoons/ui PaletteTrigger) keeps a little more room after the brand */
	.topbar :global(.sc-palette-trigger) {
		margin-left: var(--space-1);
	}
	.account {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		margin-left: auto;
		min-width: 0;
	}
	.email {
		color: var(--text-muted);
		font-size: var(--type-caption-size);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.plain,
	.burger {
		min-height: var(--control-height);
		min-width: var(--control-height);
		padding: 0 var(--space-05);
		border: var(--border-width-regular) solid transparent;
		background: none;
		color: var(--ink);
		font: inherit;
		cursor: pointer;
		white-space: nowrap;
	}
	.plain:hover,
	.burger:hover {
		border-color: var(--ink);
	}
	.burger {
		display: none;
	}

	/* the frame (top bar, sticky sidebar reaching the ActionBar, off-canvas < 768px) is @scutoons/ui AppShell;
	   group heads = tools.css sc-nav-head */
	.sidebar {
		padding: 0 0 var(--space-1);
	}
	.sidebar ul {
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.sidebar a {
		display: grid;
		grid-template-columns: 1.25rem 1fr;
		column-gap: var(--space-05);
		align-content: center;
		box-sizing: border-box;
		min-height: var(--control-height);
		padding: var(--space-05) var(--space-1);
		color: var(--ink);
		text-decoration: none;
	}
	.sidebar a:hover {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-regular));
	}
	.sidebar a:focus-visible {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-focus));
	}
	/* current = inverted */
	.sidebar a[aria-current='page'] {
		background: var(--ink);
		color: var(--paper);
	}
	.sidebar a[aria-current='page'] .description {
		color: inherit;
	}
	.icon {
		grid-row: span 2;
		align-self: start;
		text-align: center;
		line-height: var(--type-body-line);
	}
	.plain {
		display: inline-flex;
		align-items: center;
		gap: var(--space-05);
	}
	@media (max-width: 767px) {
		.wide-only {
			display: none;
		}
	}
	@media (max-width: 479px) {
		.brand-name {
			display: none;
		}
	}
	.burger {
		align-items: center;
		justify-content: center;
	}
	.title {
		font-weight: var(--font-weight-regular);
	}
	.description {
		color: var(--text-muted);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	main {
		min-width: 0;
		padding: var(--space-15) var(--space-15) var(--space-3);
	}
	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip-path: inset(50%);
		white-space: nowrap;
	}

	@media (max-width: 767px) {
		.burger {
			display: block;
		}
		.topbar :global(.sc-palette-trigger) {
			margin-left: 0;
		}
		.email {
			display: none;
		}
		main {
			padding: var(--space-1) var(--space-1) var(--space-3);
		}
	}
	@media (min-width: 768px) and (max-width: 1023px) {
		.admin {
			--admin-sidebar-width: 12rem;
		}
		.description {
			display: none;
		}
	}
	@media (max-width: 1279px) {
		.email {
			display: none;
		}
	}

	/* Admin-wide content styles (fields, focus, headings, tables: tools.css; @scutoons/ui components style themselves) */
	:global(:where(.admin) input:not([type='checkbox'], [type='radio'])),
	:global(:where(.admin) select),
	:global(:where(.admin) textarea) {
		max-width: 100%;
	}
	:global(.admin .muted) {
		color: var(--text-muted);
	}
	:global(.admin .mono-code) {
		font-weight: var(--font-weight-bold);
		letter-spacing: 0.02em;
	}
	:global(.admin .toolbar) {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-end;
		gap: var(--space-075) var(--space-1);
		margin-bottom: var(--space-1);
	}
	:global(.admin .toolbar .filter) {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
	}
	/* filter labels: the tool label (caption, bold, uppercase, ink) */
	:global(.admin .toolbar .filter > span),
	:global(.admin .bar-filter > span) {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--ink);
	}
	/* compact filters in the bottom ActionBar: label left of the control, one line */
	:global(.admin .bar-filter) {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		white-space: nowrap;
	}
	:global(.admin .grid-2) {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(min(22rem, 100%), 1fr));
		gap: 0 var(--space-15);
		align-items: start;
	}
	:global(.admin dl.facts) {
		display: grid;
		grid-template-columns: max-content minmax(0, 1fr);
		gap: var(--space-05) var(--space-1);
		align-items: baseline;
		margin: 0;
	}
	/* fact names = the tool label */
	:global(.admin dl.facts dt) {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	:global(.admin dl.facts dd) {
		margin: 0;
		overflow-wrap: anywhere;
	}
	:global(.admin .error-box) {
		padding: var(--space-075);
		border: var(--border-width-regular) solid var(--state-error);
		border-left-width: 0.5rem;
	}
	@media (max-width: 479px) {
		:global(.admin dl.facts) {
			grid-template-columns: minmax(0, 1fr);
			gap: 0;
		}
		:global(.admin dl.facts dd) {
			margin-bottom: var(--space-05);
		}
	}
</style>
