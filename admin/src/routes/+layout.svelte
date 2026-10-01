<script lang="ts">
	import '@scutoons/tokens';
	import type { Snippet } from 'svelte';
	import { onMount } from 'svelte';
	import { afterNavigate, goto } from '$app/navigation';
	import { page } from '$app/state';
	import { AppShell, CommandPalette, ToastStack, type Command } from '@scutoons/ui';
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

	const otherLocale = $derived(locales.find((l) => l !== getLocale()) ?? 'en');

	const commands = $derived<Command[]>([
		...navItems.map((item) => ({
			id: `nav-${item.id}`,
			label: item.title(),
			hint: item.description(),
			group: m['palette.go_to'](),
			keywords: item.keywords,
			run: () => goto(item.href)
		})),
		{ id: 'new-batch', label: m['palette.new_batch'](), group: m['palette.actions'](), keywords: 'batch edition', run: () => goto('/batches?new=1') },
		{ id: 'generate', label: m['palette.generate'](), group: m['palette.actions'](), keywords: 'code qr', run: () => goto('/codes/new') },
		{ id: 'revoked', label: m['palette.revoked_codes'](), group: m['palette.actions'](), run: () => goto('/codes?status=revoked') },
		{ id: 'admins', label: m['palette.admins'](), group: m['palette.actions'](), run: () => goto('/users?role=admin') },
		{ id: 'locale', label: m['palette.language']({ locale: otherLocale }), group: m['palette.actions'](), keywords: 'deutsch english language sprache', run: () => setLocale(otherLocale) },
		{ id: 'logout', label: m['shell.sign_out'](), group: m['palette.actions'](), run: () => session.logout() }
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

<div class="admin">
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
						<span aria-hidden="true">☰</span><span class="sr-only">{m['shell.menu']()}</span>
					</button>
					<a class="brand" href="/">
						<img src="/SCUTOONS-logo.svg" alt="SCUTOONS" width="120" height="20" />
						<span>{m['shell.admin']()}</span>
					</a>
					<button type="button" class="palette-trigger" onclick={() => (paletteOpen = true)}>
						<span>{m['shell.search']()}</span><kbd>Ctrl K</kbd>
					</button>
					<div class="account">
						<span class="email" title={session.user?.email}>{session.user?.email}</span>
						<button type="button" class="plain" onclick={() => setLocale(otherLocale)} title={m['shell.language']()}>
							{otherLocale.toUpperCase()}
						</button>
						<button type="button" class="plain" onclick={() => session.logout()}>{m['shell.sign_out']()}</button>
					</div>
				</header>
			{/snippet}
			{#snippet sidebar()}
				<div class="sidebar">
					{#each navigation as group (group.id)}
						<p class="group">{group.title()}</p>
						<ul>
							{#each group.items as item (item.id)}
								<li>
									<a href={item.href} aria-current={current?.id === item.id ? 'page' : undefined}>
										<span class="icon" aria-hidden="true">{item.icon}</span>
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
	/* Dense desktop tool: the rem-based tokens (spacing, bar heights) stay at the 16px baseline instead of the reader's
	   fluid 24px root on wide screens (the text itself is 14px, see .admin) */
	:global(html) {
		font-size: 16px;
	}
	/* Admin chrome: DESIGN.md Part 1 – IBM Plex Mono, ink/paper, square, 2px structural rules, cyan only as fill.
	   Dense desktop tool: a fixed 14px base instead of the reader's fluid root size (like the creator). */
	.admin {
		--topbar-height: 3.25rem;
		--action-bar-height: var(--topbar-height);
		--split-view-top: calc(var(--topbar-height) + var(--space-1));
		--split-view-bottom: calc(var(--action-bar-height) + var(--space-1));
		--admin-sidebar-width: 15rem;
		font-family: var(--font-mono);
		font-size: 14px;
		line-height: 1.4;
		color: var(--ink);
		background: var(--paper);
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
		font-weight: var(--font-weight-bold);
	}
	.brand img {
		display: block;
		height: 20px;
		width: auto;
	}
	.palette-trigger {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-1);
		min-width: 14rem;
		margin-left: var(--space-1);
		padding: var(--space-025) var(--space-05);
		border: var(--border-width-hairline) solid var(--border-muted);
		background: var(--paper);
		color: var(--text-muted);
		font: inherit;
		cursor: pointer;
	}
	.palette-trigger:hover {
		border-color: var(--ink);
	}
	kbd {
		font: inherit;
		font-size: var(--type-caption-size);
		padding: 0 0.3em;
		border: var(--border-width-hairline) solid var(--border-subtle);
	}
	.account {
		display: flex;
		align-items: center;
		gap: var(--space-075);
		margin-left: auto;
		min-width: 0;
	}
	.email {
		color: var(--text-muted);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.plain,
	.burger {
		padding: var(--space-025) var(--space-05);
		border: var(--border-width-hairline) solid transparent;
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
		font-size: 1.25em;
	}

	/* the frame (top bar, sticky sidebar reaching the ActionBar, off-canvas < 768px) is @scutoons/ui AppShell */
	.sidebar {
		padding: var(--space-1) 0;
	}
	.sidebar ul {
		margin: 0 0 var(--space-1);
		padding: 0;
		list-style: none;
	}
	.group {
		margin: 0;
		padding: 0 var(--space-1) var(--space-025);
		color: var(--text-muted);
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}
	.sidebar a {
		display: grid;
		grid-template-columns: 1.25rem 1fr;
		column-gap: var(--space-05);
		padding: var(--space-05) var(--space-1);
		color: var(--ink);
		text-decoration: none;
		border-left: 4px solid transparent;
	}
	.sidebar a:hover {
		background: var(--surface-hover);
	}
	.sidebar a[aria-current='page'] {
		border-left-color: var(--ink);
		background: var(--surface-tint-cyan);
	}
	.sidebar a[aria-current='page'] .icon {
		background: var(--primary-cyan);
	}
	.icon {
		grid-row: span 2;
		align-self: start;
		text-align: center;
	}
	.title {
		font-weight: var(--font-weight-bold);
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
		.palette-trigger {
			min-width: 0;
			margin-left: 0;
		}
		.palette-trigger kbd,
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

	/* Admin-wide content styles (@scutoons/ui components style themselves) */
	:global(:where(.admin) a) {
		color: var(--ink);
	}
	:global(:where(.admin) h2) {
		font-size: 1.1em;
	}
	:global(:where(.admin) :is(input:not([type='checkbox']):not([type='radio']), select, textarea)) {
		box-sizing: border-box;
		margin: 0;
		padding: var(--space-05) var(--space-075);
		border: var(--border-width-hairline) solid var(--border-subtle);
		border-radius: 0;
		background: var(--paper);
		color: var(--ink);
		font: inherit;
	}
	:global(:where(.admin) :is(input, select, textarea):focus-visible) {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: -1px;
	}
	:global(:where(.admin) input[type='checkbox']) {
		accent-color: var(--ink);
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
	:global(.admin .toolbar .filter > span) {
		font-size: var(--type-caption-size);
		color: var(--text-muted);
	}
	/* compact filters in the bottom ActionBar: label left of the control, one line */
	:global(.admin .bar-filter) {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		white-space: nowrap;
	}
	:global(.admin .bar-filter > span) {
		color: var(--text-muted);
		font-size: var(--type-caption-size);
	}
	:global(.admin .sc-action-bar :is(select, input:not([type='checkbox']))) {
		padding: var(--space-025) var(--space-05);
	}
	:global(.admin .grid-2) {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(min(22rem, 100%), 1fr));
		gap: var(--space-15);
		align-items: start;
	}
	:global(.admin dl.facts) {
		display: grid;
		grid-template-columns: max-content minmax(0, 1fr);
		gap: var(--space-025) var(--space-1);
		margin: 0;
	}
	:global(.admin dl.facts dt) {
		color: var(--text-muted);
	}
	:global(.admin dl.facts dd) {
		margin: 0;
		overflow-wrap: anywhere;
	}
	:global(.admin .error-box) {
		padding: var(--space-075);
		border: var(--border-width-regular) solid var(--state-error);
	}
</style>
