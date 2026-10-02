<script lang="ts">
	import { onMount } from 'svelte';
	import { ApiError } from '$lib/api.ts';
	import { addClaimFile, fetchCode, fileClaim, type AccessResult, type VerifyResult } from '$lib/api.ts';
	import { getInventoryContext, localized } from '$lib/contexts/inventory.svelte.ts';
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { getMembershipContext } from '$lib/contexts/membership.svelte.ts';
	import type { CodeInfo, CodeOption, ItemVariant } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { LoginForm } from '@scutoons/auth';
	import ItemIcon from './item-icon.svelte';

	/**
	 * A scanned SCUTOONS code (in-app scanner or `?code=`): grants device viewing access via `/access/` right away
	 * (no login) and offers — never forces — "Add to my account" (login → `/inventory/claim/`).
	 * Promo codes (not claimable, `signup`): no item to collect — offers "Create account" instead (email → one-time
	 * code → opt-in checkbox); the code is only the signup code, nobody owns it.
	 * `auto` = the reader explicitly came to add it (`/account?code=`): go straight to sign-in / claim / signup.
	 * Not registered yet → "Add to my account" is the main action, with "Register it now – otherwise someone else can".
	 * Registered to someone else → reading still works, plus "This is mine": an ownership claim (statement + optional
	 * proof files; a reader without an account signs up for it with `dispute`) – agents/ITEMS.md "Ownership, claims,
	 * gifting". `dispute` = open straight in that form (the QR landing page's "File a claim" link: `?code=…&claim=1`).
	 */
	let {
		code,
		auto = false,
		dispute = false,
		onclose,
		onopeninventory
	}: { code: string; auto?: boolean; dispute?: boolean; onclose: () => void; onopeninventory?: () => void } = $props();

	const inventory = getInventoryContext();
	const session = getSessionContext();
	const membership = getMembershipContext();

	type Phase = 'loading' | 'ready' | 'login' | 'signup' | 'welcome' | 'done' | 'error' | 'dispute_login' | 'dispute' | 'disputed';
	let phase = $state<Phase>('loading');
	let access = $state<AccessResult | null>(null);
	let info = $state<CodeInfo | null>(null);
	let error = $state<string | null>(null);
	let claimError = $state<string | null>(null);
	let options = $state<CodeOption[]>([]);
	let chosenItem = $state<string | null>(null);
	let chosenVariant = $state<string | null>(null);
	let claimedItem = $state<string | null>(null);
	let working = $state(false);
	let accountCreated = $state(false);
	// "This is mine" (ownership claim)
	let statement = $state('');
	let proofFiles = $state<FileList | null>(null);
	let disputeError = $state<string | null>(null);
	let fileErrors = $state<string[]>([]);

	const unlocksHere = $derived(!!access?.permissions?.includes('catalog') && membership.deviceValid);
	const claimable = $derived(access ? access.claimable : (info?.claimable ?? true));
	/** promo: unlocks and allows a signup, but there is nothing to own */
	const signupOnly = $derived(!claimable && !!(access?.signup ?? info?.signup));
	const ownedByOther = $derived(
		!!(info?.claimed ?? access?.claimed) && !(info?.claimed_by_me || access?.claimed_by_me)
	);
	const needsChoice = $derived(options.length > 0 && (info?.items?.length ?? 0) === 0);
	const chosenOption = $derived(options.find((o) => o.item_id === chosenItem));
	const variantChoices = $derived<ItemVariant[]>(
		(chosenOption?.variants ?? []).map((v) =>
			typeof v === 'string'
				? (inventory.item(chosenOption!.item_id)?.variants?.find((cv) => cv.id === v) ?? {
						id: v,
						name: { de: v, en: v }
					})
				: v
		)
	);
	const canClaim = $derived(
		!needsChoice || (!!chosenItem && (variantChoices.length === 0 || !!chosenVariant))
	);
	const previewItems = $derived(
		info?.items?.length
			? info.items
			: options.map((o) => ({ item_id: o.item_id, variant: null as string | null, qty: 1 }))
	);

	function describe(e: unknown) {
		if (e instanceof ApiError && e.status === 404) return m['claim.not_found']();
		if (e instanceof ApiError && e.status === 410)
			return e.code === 'expired' ? m['claim.expired']() : m['claim.no_longer_valid']();
		if (e instanceof ApiError && e.status === 0) return m['account.network_error']();
		return m['claim.failed']();
	}

	async function load() {
		phase = 'loading';
		const [accessResult, codeResult] = await Promise.allSettled([
			membership.grantFromCode(code),
			fetchCode(code)
		]);
		if (accessResult.status === 'rejected' && codeResult.status === 'rejected') {
			error = describe(accessResult.reason);
			phase = 'error';
			return;
		}
		access = accessResult.status === 'fulfilled' ? accessResult.value : null;
		info = codeResult.status === 'fulfilled' ? codeResult.value : null;

		// codes that neither unlock nor can be owned (page / event / xr per the server's permissions map): continue
		if (access && !access.claimable && !access.permissions?.includes('catalog')) {
			onclose();
			return;
		}

		options = info?.options ?? [];
		if (options.length === 1) {
			chosenItem = options[0].item_id;
			const only = options[0].variants?.length === 1 ? options[0].variants[0] : null;
			if (only) chosenVariant = typeof only === 'string' ? only : only.id;
		}
		if (info?.claimed_by_me || access?.claimed_by_me) {
			phase = 'done';
			return;
		}
		phase = 'ready';
		if (dispute && ownedByOther) {
			startDispute();
			return;
		}
		if (auto && signupOnly && !session.isLoggedIn) phase = 'signup';
		else if (auto && claimable && !ownedByOther && canClaim) await claim();
	}

	async function claim() {
		if (!session.isLoggedIn) {
			phase = 'login';
			return;
		}
		working = true;
		claimError = null;
		try {
			await inventory.claim(code, chosenItem, chosenVariant);
			claimedItem = chosenItem ?? info?.items?.[0]?.item_id ?? null;
			phase = 'done';
		} catch (e) {
			phase = 'ready';
			if (e instanceof ApiError && e.code === 'choose_item') {
				options = e.body?.options ?? options;
				claimError = m['claim.choose']();
			} else if (e instanceof ApiError && e.code === 'email_required') {
				claimError = m['claim.email_required']();
			} else if (e instanceof ApiError && (e.status === 409 || e.code === 'already_claimed')) {
				claimError = m['claim.in_someones_account']();
			} else if (e instanceof ApiError && e.code === 'not_claimable') {
				claimError = m['claim.not_claimable']();
			} else {
				claimError = describe(e);
			}
		} finally {
			working = false;
		}
	}

	async function afterLogin(result: VerifyResult) {
		if (result.claimed?.includes(code)) {
			claimedItem = chosenItem ?? info?.items?.[0]?.item_id ?? null;
			await session.loadMe();
			phase = 'done';
			return;
		}
		phase = 'ready';
		await claim();
	}

	function startDispute() {
		disputeError = null;
		phase = session.isLoggedIn ? 'dispute' : 'dispute_login';
	}

	async function sendDispute(event: SubmitEvent) {
		event.preventDefault();
		if (statement.trim().length < 10) {
			disputeError = m['claim.dispute_short']();
			return;
		}
		working = true;
		disputeError = null;
		fileErrors = [];
		try {
			const { claim: filed } = await fileClaim(code, statement.trim());
			for (const file of Array.from(proofFiles ?? []).slice(0, 5)) {
				try {
					await addClaimFile(filed.id, file, file.name);
				} catch {
					fileErrors = [...fileErrors, m['claim.dispute_file_failed']({ name: file.name })];
				}
			}
			phase = 'disputed';
		} catch (e) {
			if (e instanceof ApiError && e.code === 'claim_open') disputeError = m['claim.dispute_open']();
			else if (e instanceof ApiError && e.code === 'statement_required') disputeError = m['claim.dispute_short']();
			else if (e instanceof ApiError && e.code === 'already_yours') phase = 'done';
			else if (e instanceof ApiError && e.code === 'not_registered') {
				phase = 'ready';
				claimError = null;
			} else disputeError = m['claim.dispute_failed']();
		} finally {
			working = false;
		}
	}

	async function afterSignup(result: VerifyResult) {
		accountCreated = !!result.created;
		phase = 'welcome';
	}

	onMount(load);
</script>

<div class="claim-backdrop" role="presentation" onclick={onclose}></div>
<div class="claim" role="dialog" aria-modal="true" aria-label={signupOnly ? m['claim.promo_title']() : m['claim.title']()}>
	<header>
		<h2>{signupOnly ? m['claim.promo_title']() : m['claim.title']()}</h2>
		<button class="close" onclick={onclose} aria-label={m['items.close']()}>×</button>
	</header>

	{#if phase === 'loading'}
		<p>{m['claim.loading']()}</p>
	{:else if phase === 'error'}
		<p class="error">{error}</p>
		<button class="btn-secondary" onclick={onclose}>{m['items.close']()}</button>
	{:else}
		{#if unlocksHere}
			<p class="unlocked">{m['claim.device_unlocked']()}</p>
		{/if}

		{#if info?.edition}
			<p class="edition">
				{info.edition.name}{info.edition.number
					? ` · ${
							info.edition.count
								? m['items.edition']({ number: info.edition.number, total: info.edition.count })
								: m['items.edition_number']({ number: info.edition.number })
						}`
					: ''}
			</p>
		{/if}

		{#if signupOnly}
			{#if phase === 'welcome'}
				<p>{accountCreated ? m['claim.account_created']() : m['claim.signed_in']()}</p>
				<button onclick={onclose}>{m['claim.continue_reading']()}</button>
			{:else if session.isLoggedIn}
				<p class="note">{m['claim.promo_signed_in']()}</p>
				<button onclick={onclose}>{m['claim.continue_reading']()}</button>
			{:else}
				<p class="note">{m['claim.promo_note']()}</p>
				{#if phase === 'signup'}
					<LoginForm {session} locale={getLocale()} {code} signup onlogin={afterSignup} />
					<button class="btn-secondary" onclick={onclose}>{m['claim.continue_reading']()}</button>
				{:else}
					<div class="actions">
						<button class="btn-secondary" onclick={onclose}>{m['claim.continue_reading']()}</button>
						<button onclick={() => (phase = 'signup')}>{m['claim.create_account']()}</button>
					</div>
				{/if}
			{/if}
		{:else if phase === 'done'}
			{@const id = claimedItem ?? previewItems[0]?.item_id}
			<div class="done">
				{#if id}
					<div class="hero">
						<ItemIcon
							srcs={[inventory.item(id)?.icon ?? '']}
							name={inventory.nameOf({ item_id: id, variant: chosenVariant })}
						/>
					</div>
					<strong>{inventory.nameOf({ item_id: id, variant: chosenVariant })}</strong>
				{/if}
				<p>{claimedItem ? m['claim.done']() : m['claim.claimed_by_me']()}</p>
				<div class="actions">
					{#if onopeninventory}
						<button onclick={onopeninventory}>{m['account.open_inventory']()}</button>
					{/if}
					<button class="btn-secondary" onclick={onclose}>{m['items.close']()}</button>
				</div>
			</div>
		{:else}
			{#if needsChoice && options.length > 1 && !ownedByOther}
				<h3>{m['claim.choose']()}</h3>
			{/if}
			<ul class="items">
				{#each previewItems as it}
					{@const def = inventory.item(it.item_id)}
					{@const shown = needsChoice && it.item_id === chosenItem ? { ...it, variant: chosenVariant } : it}
					<li>
						<button
							class="option"
							data-control-exempt="item choice card (picture + text)"
							class:selected={needsChoice && chosenItem === it.item_id}
							disabled={!needsChoice || ownedByOther}
							onclick={() => ((chosenItem = it.item_id), (chosenVariant = null))}
						>
							<div class="thumb">
								<ItemIcon srcs={[inventory.iconOf(shown), def?.icon ?? '']} name={inventory.nameOf(shown)} />
							</div>
							<span>
								<strong>{inventory.nameOf(shown)}</strong>
								{#if def && !needsChoice}<small>{localized(def.description)}</small>{/if}
							</span>
						</button>
					</li>
				{/each}
			</ul>

			{#if needsChoice && variantChoices.length > 1 && !ownedByOther}
				<h3>{m['claim.choose_variant']()}</h3>
				<div class="variants">
					{#each variantChoices as variant}
						<button
							class="btn-secondary"
							class:selected={chosenVariant === variant.id}
							onclick={() => (chosenVariant = variant.id)}
						>
							{localized(variant.name)}
						</button>
					{/each}
				</div>
			{/if}

			{#if ownedByOther && phase === 'dispute_login'}
				<p class="note">{m['claim.dispute_intro']()}</p>
				<p class="note">{m['claim.dispute_login']()}</p>
				<LoginForm {session} locale={getLocale()} {code} signup dispute onlogin={() => (phase = 'dispute')} />
				<button class="btn-secondary" onclick={() => (phase = 'ready')}>{m['items.back']()}</button>
			{:else if ownedByOther && phase === 'dispute'}
				<form class="dispute" onsubmit={sendDispute} data-testid="claim-dispute">
					<h3>{m['claim.dispute_title']()}</h3>
					<p class="note">{m['claim.dispute_intro']()}</p>
					<label class="field-label" for="claim-statement">{m['claim.dispute_statement']()}</label>
					<textarea id="claim-statement" rows="4" maxlength="4000" bind:value={statement} aria-describedby="claim-statement-hint"></textarea>
					<small id="claim-statement-hint">{m['claim.dispute_statement_hint']()}</small>
					<label class="field-label" for="claim-files">{m['claim.dispute_files']()}</label>
					<input id="claim-files" type="file" multiple accept="image/jpeg,image/png,image/webp,application/pdf" bind:files={proofFiles} aria-describedby="claim-files-hint" />
					<small id="claim-files-hint">{m['claim.dispute_files_hint']()}</small>
					{#if disputeError}<p class="error" role="alert">{disputeError}</p>{/if}
					<div class="actions">
						<button type="button" class="btn-secondary" onclick={() => (phase = 'ready')}>{m['items.back']()}</button>
						<button type="submit" class:disabled={working} disabled={working} data-testid="claim-dispute-send">{m['claim.dispute_send']()}</button>
					</div>
				</form>
			{:else if ownedByOther && phase === 'disputed'}
				<p class="unlocked" role="status" data-testid="claim-dispute-sent">{m['claim.dispute_sent']()}</p>
				{#each fileErrors as fileError}<p class="error">{fileError}</p>{/each}
				<button class="btn-secondary" onclick={onclose}>{m['items.close']()}</button>
			{:else if ownedByOther}
				<p class="note">{m['claim.in_someones_account']()}</p>
				<div class="actions">
					<button class="btn-secondary" onclick={onclose}>{unlocksHere ? m['claim.continue_reading']() : m['items.close']()}</button>
					<button class="btn-secondary" onclick={startDispute} data-testid="claim-this-is-mine">{m['claim.this_is_mine']()}</button>
				</div>
			{:else if claimable}
				<p class="warn" data-testid="claim-register-now">{m['claim.register_now']()}</p>
				{#if claimError}
					<p class="error">{claimError}</p>
				{/if}
				{#if phase === 'login'}
					<LoginForm
						{session}
						locale={getLocale()}
						{code}
						itemId={chosenItem}
						variant={chosenVariant}
						signup={access?.signup ?? info?.signup ?? true}
						onlogin={afterLogin}
					/>
					<button class="btn-secondary" onclick={onclose}>{m['claim.later']()}</button>
				{:else}
					<div class="actions">
						<button class="btn-secondary" onclick={onclose}>
							{unlocksHere ? m['claim.continue_reading']() : m['claim.later']()}
						</button>
						<button class:disabled={!canClaim || working} disabled={!canClaim || working} onclick={claim}>
							{m['claim.add_to_account']()}
						</button>
					</div>
				{/if}
			{:else}
				<button class="btn-secondary" onclick={onclose}>{m['items.close']()}</button>
			{/if}
		{/if}
	{/if}
</div>

<style>
	.claim-backdrop {
		position: fixed;
		inset: 0;
		z-index: 2100;
		background: rgba(0, 0, 0, 0.5);
	}
	.claim {
		position: fixed;
		z-index: 2101;
		left: 0;
		right: 0;
		bottom: 0;
		max-width: 500px;
		margin: 0 auto;
		max-height: 90dvh;
		overflow-y: auto;
		box-sizing: border-box;
		background: white;
		border: 2px solid black;
		border-bottom: none;
		padding: 1rem;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}
	header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}
	h2 {
		margin: 0;
		font-size: 1rem;
		font-weight: 700;
	}
	h3 {
		margin: 0;
		font-size: 1rem;
	}
	p {
		margin: 0;
	}
	.close {
		box-sizing: border-box;
		width: var(--control-height);
		height: var(--control-height);
		padding: 0;
		border-radius: 50%;
		background: white;
		color: black;
		border: 2px solid black;
		font-size: 1.25rem;
		line-height: 1;
	}
	.close:hover {
		background: var(--primary-color);
	}
	.edition {
		font-size: 0.75rem;
		color: #6b6b6b;
	}
	.items {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.option {
		width: 100%;
		display: grid;
		grid-template-columns: 4rem 1fr;
		gap: 0.75rem;
		align-items: center;
		text-align: left;
		padding: 0.25rem;
		background: white;
		color: black;
		border: 2px solid #ccc;
		cursor: pointer;
	}
	.option:disabled {
		cursor: default;
		border-color: black;
	}
	.option.selected {
		border-color: black;
		filter: drop-shadow(0 0 4px var(--primary-color));
	}
	.option span {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}
	small {
		font-size: 0.75rem;
		line-height: 1.3;
	}
	.variants {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}
	.variants button {
		padding: 0 1rem;
	}
	.variants button.selected {
		border-color: black;
		background: black;
		color: white;
	}
	.actions {
		display: flex;
		gap: 0.5rem;
	}
	.actions button {
		flex: 1;
		padding: 0 1rem;
	}
	.unlocked {
		background: black;
		color: white;
		padding: 0.5rem;
		filter: drop-shadow(0 0 6px var(--primary-color));
	}
	.note {
		font-size: 0.75rem;
		line-height: 1.3;
		color: #6b6b6b;
	}
	.error {
		border: 2px solid black;
		padding: 0.5rem;
	}
	/* "Register it now – otherwise someone else can" (registration strategy, ITEMS.md "Ownership, claims, gifting") */
	.warn {
		border-left: 0.375rem solid black;
		padding: 0.25rem 0.5rem;
		font-weight: 700;
	}
	.dispute {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.field-label {
		font-size: 0.75rem;
		font-weight: 700;
	}
	.dispute textarea,
	.dispute input[type='file'] {
		width: 100%;
		box-sizing: border-box;
		font: inherit;
		border: 2px solid black;
		padding: 0.5rem;
		background: white;
		color: black;
	}
	.done {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.5rem;
		text-align: center;
	}
	.done .actions {
		width: 100%;
	}
	.hero {
		width: 10rem;
		filter: drop-shadow(0 0 12px var(--primary-color));
	}
</style>
