<script lang="ts">
	import { draftPreview } from '$lib/draft-preview.svelte.ts';
	import { invalidateAll } from '$app/navigation';
	import { getMembershipContext } from '$lib/contexts/membership.svelte.ts';
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import { isMembershipExpired } from '@scutoons/content';
	import { getLocale, localizeHref } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { formatDate } from '$lib/utils/date.ts';
	import { CREATOR_URL } from '$lib/config.ts';
	import accountIcon from '$lib/icons/account.svg';
	import { LoginForm, SignedInAs, authLabels, canUseApp } from '@scutoons/auth';
	import Avatar from '$lib/components/avatar/avatar.svelte';
	import ReaderStats from '$lib/components/items/reader-stats.svelte';
	import MediaQualitySetting from '$lib/components/account/media-quality-setting.svelte';
	import FxSetting from '$lib/components/account/fx-setting.svelte';
	import InstallAppSetting from '$lib/components/account/install-app-setting.svelte';
	import ReaderNameSetting from '$lib/components/account/reader-name-setting.svelte';
	import MyClaims from '$lib/components/account/my-claims.svelte';
	import AiConnections from '$lib/components/account/ai-connections.svelte';
	import { page } from '$app/state';
	import { goto, afterNavigate } from '$app/navigation';
	import { locales } from '$lib/paraglide/runtime.js';
	import { Icon, confirmDialog } from '@scutoons/ui';
	import { getWorldContext } from '$lib/contexts/world.svelte.ts';
	import { toonName, type ToonIndex } from '@scutoons/content';

	const membershipCtx = getMembershipContext();
	const session = getSessionContext();
	const inventory = getInventoryContext();
	const world = getWorldContext();
	/** the map reset happened in this visit (the hint says so) */
	let mapReset = $state(false);
	async function resetMap() {
		if (!world) return;
		const ok = await confirmDialog({ title: m['account.reset_map_confirm_title'](), message: m['account.reset_map_confirm'](), confirmLabel: m['account.reset_map'](), cancelLabel: m['account.reset_map_cancel']() });
		if (!ok) return;
		world.resetMap();
		mapReset = true;
	}

	// `?return=<path>` (the follow-up bar's "Anmelden" before a members-only chapter): back there after signing in;
	// same-origin paths only
	// `/account?code=…`: the code goes with the sign-up (owner 2026-10-03: an old email registration – the code is in
	// the reader's name – could not sign up from here; the API links the code when the address matches)
	const signupCode = $derived(page.url.searchParams.get('code') || null);
	const returnTo = $derived.by(() => {
		const value = page.url.searchParams.get('return');
		return value && value.startsWith('/') && !value.startsWith('//') && !value.includes('\\') ? value : null;
	});

	// back to where the reader came from (owner 2026-10-06: from a chapter, the menu's account link left no way back
	// to it – the logo goes to the landing page, the menu to the toon's start). Browser history restores the page and
	// its scroll position; only for a page of this app, not on a direct visit or reload (then the logo / menu remain)
	let cameFrom = $state<URL | null>(null);
	afterNavigate(({ from }) => {
		if (from?.url && !/\/account(\/|$)/.test(from.url.pathname)) cameFrom = from.url;
	});
	const backToon = $derived.by(() => {
		if (!cameFrom) return null;
		const parts = cameFrom.pathname.split('/').filter(Boolean);
		const id = (locales as readonly string[]).includes(parts[0]) ? parts[1] : parts[0];
		return (page.data.toons as ToonIndex[] | undefined)?.find((t) => t.id === id) ?? null;
	});
	function goBack(event: MouseEvent) {
		if (event.metaKey || event.ctrlKey || event.shiftKey || event.button !== 0) return;
		event.preventDefault();
		history.back();
	}

	const itemCount =$derived(inventory.entries.reduce((sum, e) => sum + e.qty, 0));

	const typeLabel = (type: string) => (m as any)[`account.${type}`]?.() ?? type;
	/** access only through the promo code the account was created with (not owned) */
	const promoAccessCode = $derived.by(() => {
		const access = session.me?.access;
		return access?.signup_code && access.codes.length === 1 ? access.signup_code : null;
	});
	const roleLabel = (role: string) => (m as any)[`account.role_${role}`]?.() ?? role;

	// "Send me news about new releases" (PUT /me/)
	let optInSaving = $state(false);
	let optInError = $state<string | null>(null);
	async function toggleOptIn(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const value = input.checked;
		optInSaving = true;
		optInError = null;
		try {
			await session.setMarketingOptIn(value);
		} catch {
			input.checked = !value;
			optInError = m['account.marketing_opt_in_failed']();
		} finally {
			optInSaving = false;
		}
	}

	// "Show hints" (PUT /me/ { show_hints }, off by default; guests: off – no toggle there): the reader outlines what can be
	// tapped / hovered (BACKLOG M.1)
	let hintsSaving = $state(false);
	let hintsError = $state<string | null>(null);
	async function toggleShowHints(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const value = input.checked;
		hintsSaving = true;
		hintsError = null;
		try {
			await session.setShowHints(value);
		} catch {
			input.checked = !value;
			hintsError = m['account.marketing_opt_in_failed']();
		} finally {
			hintsSaving = false;
		}
	}

	// "Hide sensitive content" (PUT /me/ { hide_sensitive }, on by default; guests always hide – no toggle there)
	let hideSaving = $state(false);
	let hideError = $state<string | null>(null);
	async function toggleHideSensitive(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const value = input.checked;
		hideSaving = true;
		hideError = null;
		try {
			await session.setHideSensitive(value);
		} catch {
			input.checked = !value;
			hideError = m['account.marketing_opt_in_failed']();
		} finally {
			hideSaving = false;
		}
	}

</script>

<div class="view view-account">
	{#if cameFrom}
		<a class="back" href={cameFrom.pathname + cameFrom.search} onclick={goBack} data-testid="account-back">
			<Icon name="chevron-left" size="1rem" />
			{backToon ? m['account.back_to']({ name: toonName(backToon, getLocale()) }) : m['account.back']()}
		</a>
	{/if}
	<header class="account-header">
		<h1><img src={accountIcon} alt="Account" /> {m['account.my_account']()}</h1>
	</header>

	{#if session.pending}
		<section><p>{m['common.loading']()}</p></section>
	{:else if session.isLoggedIn && session.user}
		<section>
			<div class="signed-in">
				<SignedInAs session={session.auth} locale={getLocale()} />
			</div>
			<table>
				<tbody>
					{#if session.roles.length}
						<tr>
							<td>{m['account.roles']()}</td>
							<td>
								<span class="roles">
									{#each session.roles as role}
										<span class="role-tag">{roleLabel(role)}</span>
									{/each}
								</span>
							</td>
						</tr>
					{/if}
					<tr>
						<td>{m['account.access']()}</td>
						<td>
							{#if session.hasAccess && promoAccessCode}
								{m['account.access_promo']({ code: promoAccessCode })}
							{:else if session.hasAccess}
								{m['account.access_unlocked']({
									until: session.me?.access?.expires_at || m['account.lifetime']()
								})}
							{:else}
								{m['account.access_none']()}
							{/if}
						</td>
					</tr>
					{#if session.user.created_at}
						<tr
							><td>{m['account.created_at']()}</td><td
								>{formatDate(new Date(session.user.created_at.replace(' ', 'T')))}</td
							></tr
						>
					{/if}
				</tbody>
			</table>
			<label class="opt-in">
				<input
					type="checkbox"
					checked={!!session.user.marketing_opt_in}
					disabled={optInSaving}
					onchange={toggleOptIn}
				/>
				<span>
					{m['account.marketing_opt_in']()}
					<small>{m['account.marketing_opt_in_hint']()}</small>
				</span>
			</label>
			{#if optInError}
				<p class="error" role="alert">{optInError}</p>
			{/if}
			<label class="opt-in" data-testid="hide-sensitive">
				<input
					type="checkbox"
					checked={session.user.hide_sensitive !== false}
					disabled={hideSaving}
					onchange={toggleHideSensitive}
				/>
				<span>
					{m['account.hide_sensitive']()}
					<small>{m['account.hide_sensitive_hint']()}</small>
				</span>
			</label>
			{#if hideError}
				<p class="error" role="alert">{hideError}</p>
			{/if}
			<label class="opt-in" data-testid="show-hints">
				<input type="checkbox" checked={session.user.show_hints === true} disabled={hintsSaving} onchange={toggleShowHints} />
				<span>
					{m['account.show_hints']()}
					<small>{m['account.show_hints_hint']()}</small>
				</span>
			</label>
			{#if hintsError}
				<p class="error" role="alert">{hintsError}</p>
			{/if}
			{#if canUseApp('creator', session.roles)}
				<p class="creator-line">
					<a class="btn btn-secondary" href={CREATOR_URL}>{m['account.creator_link']()}</a>
					<small>{m['account.creator_hint']()}</small>
				</p>
				<!-- creators: read draft and coming-soon chapters here too, for checking (this device only) -->
				<label class="opt-in" data-testid="draft-preview">
					<input type="checkbox" checked={draftPreview.on} onchange={(e) => { draftPreview.set(e.currentTarget.checked); invalidateAll(); }} />
					<span>
						{m['account.draft_preview']()}
						<small>{m['account.draft_preview_hint']()}</small>
					</span>
				</label>
				{#if draftPreview.on && world}
					<!-- for checking: the map as if read anew (owner 2026-10-11) – only the map, the rest of the progress stays -->
					<p class="creator-line">
						<button type="button" class="btn btn-secondary" onclick={resetMap} data-testid="reset-map">{m['account.reset_map']()}</button>
						<small>{mapReset ? m['account.reset_map_done']() : m['account.reset_map_hint']()}</small>
					</p>
				{/if}
			{/if}
		</section>

		<section>
			<h2>{m['account.memberships']()}</h2>
			{#if session.memberships.length === 0}
				<p>{m['account.no_memberships']()}</p>
			{:else}
				<table>
					<tbody>
						{#each session.memberships as ms}
							<tr>
								<td>{typeLabel(ms.type)}</td>
								<td>
									{ms.code} · {ms.expires_at || m['account.lifetime']()}
									{#if isMembershipExpired(ms.expires_at)}(✕){/if}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			{/if}
		</section>

		<MyClaims />
		<!-- members only (the section hides itself for readers): tokens for their own AI – agents/MCP.md -->
		<AiConnections />
	{:else}
		<section>
			<h2>{m['account.login_title']()}</h2>
			<p>{m['account.login_hint']()}</p>
			<LoginForm
				{session}
				locale={getLocale()}
				code={signupCode}
				sentHint={authLabels(getLocale()).codeSentNoAccount}
				onlogin={() => returnTo && goto(returnTo)}
			/>
			<p class="signup-rule">{m['account.signup_rule']()}</p>
		</section>
	{/if}

	{#if membershipCtx.deviceValid && membershipCtx.device}
		<section>
			<h2>{m['account.device_access']()}</h2>
			<p>
				{m['account.device_access_info']({
					code: membershipCtx.device.code,
					until: membershipCtx.device.expires_at || m['account.lifetime']()
				})}
			</p>
		</section>
	{/if}

	<section>
		<ReaderNameSetting />
		<MediaQualitySetting />
		<FxSetting />
		<InstallAppSetting />
	</section>

	<section>
		<h2>{m['account.inventory']()}</h2>
		<p class="inventory-line">
			<span>{m['account.items_count']({ count: itemCount })}</span>
			<a class="btn btn-secondary" href={localizeHref('/catpope') + '?inventory=open'}>
				{m['account.open_inventory']()}
			</a>
		</p>
	</section>

	<section>
		<!-- the reader's stats (testo, appetite) small at the right, as in the inventory's tab row (owner 2026-10-03) -->
		<div class="avatar-head">
			<h2>{m['account.avatar']()}</h2>
			<ReaderStats compact />
		</div>
		<Avatar />
	</section>

</div>

<style>
	.view.view-account {
		max-width: 600px;
		margin: 5rem auto;
	}
	.back {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		height: var(--control-height);
		margin: 0 1rem;
		color: var(--ink);
		text-decoration: none;
	}
	.back:hover {
		text-decoration: underline;
	}
	.back:focus-visible {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: 2px;
	}
	header {
		padding: 0 1rem;
		h1 {
			display: flex;
			align-items: center;
			gap: 0.5rem;
		}
		img {
			width: 4rem;
		}
		display: grid;
		grid-template-columns: auto 1fr;
		align-items: center;
		gap: 1rem;
	}
	.avatar-head {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem 1rem;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 0.75rem;
	}
	.avatar-head h2 {
		margin-bottom: 0;
	}
	section {
		padding: 1rem;
	}
	.signed-in {
		margin-bottom: 1rem;
	}
	h2 {
		font-size: 1rem;
		font-weight: 700;
		margin: 0 0 0.75rem;
	}
	table {
		width: 100%;
		border-collapse: collapse;
		margin-bottom: 1rem;
	}
	td {
		padding: 0.25rem 0.5rem;
		vertical-align: top;
		overflow-wrap: anywhere;
	}
	td:first-child {
		background: #f0f0f0;
		width: 40%;
	}
	.roles {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem;
	}
	.role-tag {
		display: inline-block;
		border: 2px solid black;
		padding: 0 0.375rem;
		font-size: 0.75rem;
		line-height: 1.4;
	}
	.opt-in {
		display: flex;
		align-items: flex-start;
		gap: 0.5rem;
		margin: 0 0 1rem;
		cursor: pointer;
	}
	.opt-in input {
		margin: 0.125rem 0 0;
		width: 1.25rem;
		height: 1.25rem; /* control-ok: a checkbox */
		flex: none;
		accent-color: black;
	}
	.opt-in span {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}
	.opt-in small,
	.creator-line small,
	.signup-rule {
		font-size: 0.75rem;
		line-height: 1.3;
		color: #6b6b6b;
	}
	.signup-rule {
		margin: 1rem 0 0;
	}
	.creator-line {
		display: flex;
		align-items: center;
		gap: 1rem;
		flex-wrap: wrap;
		margin: 0 0 1rem;
	}
	.creator-line .btn {
		padding: 0 1rem;
		text-decoration: none;
	}
	.error {
		border: 2px solid black;
		padding: 0.5rem;
	}
	.inventory-line {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		flex-wrap: wrap;
		margin: 0;
	}
	.inventory-line .btn {
		padding: 0 1rem;
		text-decoration: none;
	}
</style>
