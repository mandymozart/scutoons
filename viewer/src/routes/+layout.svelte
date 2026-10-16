<script lang="ts">
	import '@scutoons/tokens/index.css';
	import '$lib/chrome.css';
	import { getLocale, localizeHref, locales } from '$lib/paraglide/runtime.js';
	import { page, navigating } from '$app/state';
	import { goto, afterNavigate } from '$app/navigation';
	import type { LayoutData } from './$types.js';
	import { onMount, type Snippet } from 'svelte';
	import Header from '$lib/components/header.svelte';
	import { draftPreview } from '$lib/draft-preview.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { DialogHost, Icon } from '@scutoons/ui';
	import Loader from '$lib/components/loader.svelte';
	import ItemsHost from '$lib/components/items/items-host.svelte';
	import HomeHost from '$lib/components/home/home-host.svelte';
	import { setHomesContext } from '$lib/contexts/homes.svelte.ts';
	import ReaderEffect from '$lib/components/items/reader-effect.svelte';
	import { setMembershipContext } from '$lib/contexts/membership.svelte.ts';
	import { setSessionContext } from '$lib/contexts/session.svelte.ts';
	import { setInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import { setItemsUiContext } from '$lib/contexts/items-ui.svelte.ts';
	import { setWorldContext } from '$lib/contexts/world.svelte.ts';
	import { initAnimatedFavicon } from '$lib/utils/favicon.ts';
	import { listenToCreator } from '$lib/creator-link.svelte.ts';
	import { fxSettings } from '$lib/fx-settings.svelte.ts';
	import UpdatePrompt from '$lib/components/update-prompt.svelte';
	import { pwa } from '$lib/pwa.svelte.ts';
	import InstallPrompt from '$lib/components/install-prompt.svelte';
	import { install } from '$lib/install.svelte.ts';
	import { savedChapters } from '$lib/offline/saved-chapters.svelte.ts';
	import { checkSavedVersions } from '$lib/offline/refresh-saved.svelte.ts';
	import { API_URL } from '$lib/config.ts';
	import { accessUntilOf } from '$lib/sw/saved-rules.ts';

	let { data, children }: { data: LayoutData; children: Snippet } = $props();

	let toons = $derived(data.toons);
	const session = setSessionContext();
	// "Show hints" (account page, off by default): the outlines of what can be tapped (vn/triggers, audio sample areas)
	$effect(() => {
		document.documentElement.classList.toggle('sc-show-hints', session.me?.user?.show_hints === true);
	});
	const membership = setMembershipContext(session);
	const inventory = setInventoryContext(session);
	const itemsUi = setItemsUiContext();
	// the avatar's home (agents/research/CAVE.md): the reader's homes, the home view
	const homes = setHomesContext(inventory);
	// the map discoveries of the whole universe (agents/MAP.md §2.4), synced like progress for members
	setWorldContext(session);

	// the static build serves one fallback index.html (`<html lang="de">`): the document language follows the locale of
	// the URL (`/en/…` prefix, else cookie / browser language), like the dev server's `%paraglide.lang%`
	$effect(() => {
		void page.url.pathname;
		document.documentElement.lang = getLocale();
	});

	// the install bar's moment comes with a page change (after a finished chapter), never over the chapter's end bar
	afterNavigate(({ from }) => {
		if (from) install.check();
	});

	// framed by the creator's dock Reader (BACKLOG 10.49): "show group / slice" – only from the creator, only in the draft preview
	onMount(() => listenToCreator());

	onMount(async () => {
		initAnimatedFavicon();
		// the service worker: app shell + the "new version" prompt (agents/research/PWA.md)
		void pwa.init();
		// "Install SCUTOONS as an app" (Android / Chromium): counts the visit, offers it from the 2nd one
		install.init();
		// chapters saved to this device (PWA phase 3)
		savedChapters.init();
		// the effect sounds' routing + this device's volume / vibration (agents/FX.md)
		fxSettings.init();

		// device access from a scanned code (+ silent legacy migration, periodic re-check)
		const membershipChecked = membership.init();
		inventory.init();

		const params = page.url.searchParams;
		// `/account?code=` = "Add to my account" from the QR landing page: sign in, then claim
		const addToAccount = /\/account\/?$/.test(page.url.pathname);
		// magic link from the login mail: `<origin>/?login=<token>[&c=<code>]`
		const magicToken = params.get('login') || params.get('token') || null;
		const code = params.get('code') || (magicToken ? params.get('c') : null) || null;
		// `?code=…&claim=1` = "This is mine? File a claim" from the QR landing page (ITEMS.md "Ownership, claims, gifting")
		const dispute = params.get('claim') === '1';
		// `?visit=<token>` = a friend's visit link (research/CAVE.md D6): their home, read-only
		const visit = params.get('visit');

		// `?preview=draft` from the creator: already read by lib/draft-preview.svelte.ts
		if (code || magicToken || visit || params.has('preview')) {
			const url = new URL(window.location.href);
			url.searchParams.delete('preview');
			url.searchParams.delete('code');
			url.searchParams.delete('claim');
			url.searchParams.delete('visit');
			if (magicToken) url.searchParams.delete('c');
			// already consumed by the toon inventory (child onMount runs first)
			url.searchParams.delete('inventory');
			url.searchParams.delete('login');
			url.searchParams.delete('token');
			goto(url.pathname + url.search, { replaceState: true });
		}

		// magic link from the login mail, else the single sign-on session (any SCUTOONS app) or a stored token
		await session.init(magicToken);

		if (code) {
			// the sheet grants device access via /access/ right away and offers "Add to my account"
			itemsUi.openClaim(code, addToAccount, dispute);
		}
		if (visit && /^[A-Za-z0-9_-]{16,64}$/.test(visit)) homes.openVisit(visit);

		// saved chapters whose access ended go (PWA phase 3): the access counts as known only when the API answered and
		// the session loaded without an error – offline, each keeps the access end it was saved with
		await membershipChecked;
		await savedChapters.refresh();
		if (savedChapters.list.some((saved) => saved.gated)) {
			const reachable = await fetch(`${API_URL}/health`, { cache: 'no-store', credentials: 'omit' }).then((r) => r.ok, () => false);
			const known = reachable && session.ready && !session.error && (!session.isLoggedIn || !!session.me);
			const grants = [
				...(membership.deviceValid ? [membership.device?.expires_at ?? null] : []),
				...(session.hasAccess ? [session.me?.access?.expires_at ?? null] : [])
			];
			await savedChapters.checkAccess({ known, valid: membership.isValid, until: accessUntilOf(grants) });
		}
		// saved chapters with a new version take it in (only the changed files – lib/offline/refresh-saved.svelte.ts)
		void checkSavedVersions();
	});
</script>

<Loader visible={!!navigating.to} oncancel={() => goto(page.url.href, { noScroll: true })} />

{#if draftPreview.on}
	<!-- a creator's draft preview is on (account page): drafts and coming-soon chapters are readable here. Only a mark
	     (owner 2026-10-06): the switch is on the account page -->
	<a class="draft-preview" href={localizeHref('/account')} title={m['account.draft_preview_mark']()} aria-label={m['account.draft_preview_mark']()} data-testid="draft-preview-mark">
		<Icon name="eye" size="0.875rem" />
	</a>
{/if}
<Header {toons} />

<div class="content">
	{@render children()}
</div>

<ItemsHost />
<HomeHost />
<!-- in-page confirmations (confirmDialog / askDialog from @scutoons/ui) -->
<DialogHost />
<UpdatePrompt />
<InstallPrompt />
<!-- a running reader effect goes wherever the reader goes (owner 2026-10-08) – not only the toon pages -->
<ReaderEffect />

<div style="display:none">
	{#each locales as locale}
		<a href={localizeHref(page.url.pathname, { locale })}>{locale}</a>
	{/each}
</div>

<style>
	/* a tiny round mark top right, under the round toggles (main-menu.svelte: top 0.5rem, --control-height high) –
	   clear of the logo / pause mark (top left), the VN box and the next bar (bottom); below the slide-in menus */
	.draft-preview {
		position: fixed;
		top: calc(0.5rem + var(--control-height) + 0.5rem);
		right: calc(0.5rem + (var(--control-height) - 1.5rem) / 2);
		z-index: 900;
		box-sizing: border-box;
		width: 1.5rem;
		height: 1.5rem;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: var(--radius-full);
		background: var(--ink);
		color: var(--paper);
		text-decoration: none;
	}
</style>
