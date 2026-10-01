<script lang="ts">
	import '@scutoons/tokens';
	import type { Snippet } from 'svelte';
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { Icon, ToastStack } from '@scutoons/ui';
	import { AuthGate, takeMagicToken } from '@scutoons/auth';
	import { setEmbassySession } from '$lib/session.svelte.ts';
	import { setToaster } from '$lib/toasts.ts';
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

	// role gate (AuthGate): liaison / ambassador / admin – the server checks it again on every /embassy call
	const isAmbassador = $derived(session.hasRole('ambassador') || session.hasRole('admin'));
	const otherLocale = $derived(locales.find((l) => l !== getLocale()) ?? 'en');

	const nav = $derived([
		{ href: '/', label: m['nav.today'](), icon: 'clock' },
		{ href: '/leads', label: m['nav.leads'](), icon: 'list' },
		{ href: '/orgs/new', label: m['nav.new'](), icon: 'plus' },
		...(isAmbassador ? [{ href: '/import', label: m['nav.import'](), icon: 'upload' }] : [])
	]);
	const isCurrent = (href: string) =>
		href === '/' ? page.url.pathname === '/' : page.url.pathname === href || (href === '/leads' && page.url.pathname.startsWith('/orgs/') && page.url.pathname !== '/orgs/new');
</script>

<svelte:head>
	<title>{m['shell.title']()}</title>
</svelte:head>

<div class="embassy sc-tool">
	<AuthGate {session} app="embassy" appName="Embassy" locale={getLocale()} readerHref={VIEWER_URL}>
		<header class="topbar">
			<a class="brand" href="/">
				<img src="/SCUTOONS-logo.svg" alt="SCUTOONS" width="120" height="20" />
				<span>{m['shell.embassy']()}</span>
			</a>
			<nav class="nav" aria-label={m['shell.navigation']()}>
				{#each nav as item (item.href)}
					<a href={item.href} aria-current={isCurrent(item.href) ? 'page' : undefined}>
						<span class="icon"><Icon name={item.icon} /></span><span>{item.label}</span>
					</a>
				{/each}
			</nav>
			<div class="account">
				<span class="email" title={session.user?.email}>
					{session.user?.email} · {isAmbassador ? m['shell.ambassador']() : m['shell.liaison']()}
				</span>
				<button type="button" class="plain" onclick={() => setLocale(otherLocale)} title={m['shell.language']()}>
					{otherLocale.toUpperCase()}
				</button>
				<button type="button" class="plain" data-testid="sign-out" onclick={() => session.logout()}>{m['shell.sign_out']()}</button>
			</div>
		</header>
		<main>
			{@render children()}
		</main>
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
		position: sticky;
		top: 0;
		z-index: 10;
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
		white-space: nowrap;
	}
	.brand img {
		display: block;
		height: 20px;
		width: auto;
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
	.account {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		margin-left: auto;
		min-width: 0;
	}
	.email {
		color: var(--text-muted);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--type-caption-size);
	}
	.plain {
		padding: var(--space-025) var(--space-05);
		border: var(--border-width-hairline) solid transparent;
		background: none;
		color: var(--ink);
		font: inherit;
		cursor: pointer;
		white-space: nowrap;
	}
	.plain:hover {
		border-color: var(--ink);
	}
	main {
		max-width: 64rem;
		margin: 0 auto;
		padding: var(--space-15) var(--space-15) var(--space-3);
		box-sizing: border-box;
	}

	@media (max-width: 767px) {
		.email {
			display: none;
		}
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
	:global(.embassy .form-grid) {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(min(14rem, 100%), 1fr));
		gap: var(--space-075) var(--space-1);
	}
	:global(.embassy .form-grid .wide) {
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
