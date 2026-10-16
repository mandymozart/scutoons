<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { MediaQuery } from 'svelte/reactivity';
	import { Icon } from '@scutoons/ui';
	import { LoginForm, authLabels, canUseApp } from '@scutoons/auth';
	import { toonName, type ToonIndex } from '@scutoons/content';
	import { getLocale, localizeHref, locales } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { getMembershipContext } from '$lib/contexts/membership.svelte.ts';
	import { savedChapters } from '$lib/offline/saved-chapters.svelte.ts';
	import { install } from '$lib/install.svelte.ts';
	import { push } from '$lib/push.svelte.ts';
	import { onMount } from 'svelte';
	import { cameFrom, trackCameFrom } from '$lib/account/came-from.svelte.ts';
	import { ACCOUNT_GROUPS, GROUP_ICON, groupScope, groupVisible, type AccountGroup, type AccountReader } from '$lib/account/groups.ts';
	import accountIcon from '$lib/icons/account.svg';
	import BackLink from './back-link.svelte';
	import AccountIndex from './account-index.svelte';
	import AccountCard from './account-card.svelte';
	import GroupContent from './group-content.svelte';
	import { groupTitle } from './group-title.ts';

	/**
	 * The account pages (owner 2026-10-16, concept B of agents/research/ACCOUNT-PAGE.md: "Stil Entwurf B mit den
	 * Menüs"): `/account` = the index – who reads (sign-in for guests) and one row per group; `/account/<group>` = that
	 * group's page with "‹ Mein Konto" on top. Wide screens (≥ 1024px – below, the two columns are too narrow at the 24px root) show both side by side: the index left, the group
	 * right (`/account` shows "Stammdaten" there). The back links look the same on both levels (BackLink).
	 */
	let { group = null }: { group?: AccountGroup | null } = $props();

	trackCameFrom();
	const session = getSessionContext();
	const membership = getMembershipContext();
	const wide = new MediaQuery('min-width: 1024px');

	const reader = $derived<AccountReader>({
		signedIn: session.isLoggedIn,
		roles: session.roles,
		creator: canUseApp('creator', session.roles),
		offline: savedChapters.supported || install.available || install.iosHowTo,
		push: push.state !== 'hidden'
	});
	// whether this device can get push decides whether "Benachrichtigungen" shows for a guest
	onMount(() => void push.init());
	const groups = $derived(ACCOUNT_GROUPS.filter((g) => groupVisible(g, reader)));
	/** the group shown: the page's, else on a wide screen the first */
	const shown = $derived(group ?? (wide.current ? groups[0] : null));

	// a group that isn't for this reader (signed out on its page, a link to the creator group): the index
	$effect(() => {
		// only once this device's facts are known (push / the worker / install answer after the first render – a deep link
		// to "Offline-Inhalte" or a guest's "Benachrichtigungen" must not bounce)
		if (group && !session.pending && push.checked && !groups.includes(group)) goto(localizeHref('/account'), { replaceState: true });
	});

	// `?return=<path>` (the follow-up bar's "Anmelden" before a members-only chapter): back there after signing in
	// (same-origin paths only); `/account?code=…`: the code goes with the sign-up (owner 2026-10-03)
	const signupCode = $derived(page.url.searchParams.get('code') || null);
	const returnTo = $derived.by(() => {
		const value = page.url.searchParams.get('return');
		return value && value.startsWith('/') && !value.startsWith('//') && !value.includes('\\') ? value : null;
	});

	// back to where the reader came from (owner 2026-10-06): the toon's name when it was a toon's page
	const backToon = $derived.by(() => {
		if (!cameFrom.url) return null;
		const parts = cameFrom.url.pathname.split('/').filter(Boolean);
		const id = (locales as readonly string[]).includes(parts[0]) ? parts[1] : parts[0];
		return (page.data.toons as ToonIndex[] | undefined)?.find((t) => t.id === id) ?? null;
	});
	const backLabel = $derived(backToon ? m['account.back_to']({ name: toonName(backToon, getLocale()) }) : m['account.back']());
</script>

<div class="view view-account" class:wide={wide.current} class:in-group={!!group}>
	{#if wide.current || !group}
		<div class="index">
			{#if cameFrom.url}
				<BackLink href={cameFrom.url.pathname + cameFrom.url.search} label={backLabel} back testid="account-back" />
			{/if}
			<h1><img src={accountIcon} alt="" /> {m['account.my_account']()}</h1>

			{#if session.pending}
				<p>{m['common.loading']()}</p>
			{:else if session.isLoggedIn && session.user}
				<AccountCard />
			{:else}
				<section class="login" aria-labelledby="login-title">
					<h2 id="login-title">{m['account.login_title']()}</h2>
					<p>{m['account.login_hint']()}</p>
					<LoginForm
						{session}
						locale={getLocale()}
						code={signupCode}
						sentHint={authLabels(getLocale()).codeSentNoAccount}
						onlogin={() => returnTo && goto(returnTo)}
					/>
					<p class="hint">{m['account.signup_rule']()}</p>
				</section>
			{/if}
			{#if membership.deviceValid && membership.device}
				<p class="device-access">
					<strong>{m['account.device_access']()}</strong>
					<span>{m['account.device_access_info']({ code: membership.device.code, until: membership.device.expires_at || m['account.lifetime']() })}</span>
				</p>
			{/if}

			<AccountIndex {groups} {reader} current={wide.current ? shown : null} />
		</div>
	{/if}

	{#if shown}
		{@const scope = groupScope(shown, reader)}
		<main class="group" aria-labelledby="group-title" data-testid="account-group" style:padding-top={wide.current && cameFrom.url ? 'calc(var(--control-height) + 1rem)' : undefined}>
			{#if !wide.current}
				<BackLink href={localizeHref('/account')} label={m['account.my_account']()} back={cameFrom.indexBehind} testid="account-group-back" />
			{/if}
			<svelte:element this={wide.current ? 'h2' : 'h1'} id="group-title" class="group-title">
				<Icon name={GROUP_ICON[shown]} size="1em" />
				{groupTitle(shown)}
				{#if scope}<span class="sc-chip">{scope === 'device' ? m['account.scope_device']() : m['account.scope_account']()}</span>{/if}
			</svelte:element>
			<div class="body">
				<GroupContent group={shown} />
			</div>
		</main>
	{/if}
</div>

<style>
	.view.view-account {
		max-width: 600px;
		margin: 5rem auto;
		padding: 0 1rem;
	}
	.view.view-account.wide {
		max-width: 60rem;
		display: grid;
		grid-template-columns: minmax(0, 22rem) minmax(0, 1fr);
		gap: 0 3rem;
		align-items: start;
	}
	.index,
	.group {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		min-width: 0;
	}
	h1 {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin: 0;
	}
	h1 img {
		width: 4rem;
	}
	.group-title {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin: 0;
	}
	.wide .group-title {
		font-size: 1.25rem;
		/* level with the index's heading line */
		min-height: 4rem;
	}
	.body {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
	}
	.login {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}
	.login h2 {
		font-size: 1rem;
		margin: 0;
	}
	.login p {
		margin: 0;
	}
	.hint {
		font-size: 0.75rem;
		line-height: 1.3;
		color: var(--text-muted);
	}
	.device-access {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		margin: 0;
		padding: 0.5rem 0.75rem;
		border-left: 0.5rem solid var(--primary-cyan);
	}
</style>
