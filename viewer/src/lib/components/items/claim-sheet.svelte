<script lang="ts">
	import { onMount } from 'svelte';
	import { ApiError } from '$lib/api/client.ts';
	import { fetchCode, type AccessResult, type VerifyResult } from '$lib/api/qr.service.ts';
	import { getInventoryContext, localized } from '$lib/contexts/inventory.svelte.ts';
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { getMembershipContext } from '$lib/contexts/membership.svelte.ts';
	import type { CodeInfo, CodeOption, ItemVariant } from '$lib/models/items.ts';
	import { m } from '$lib/paraglide/messages.js';
	import LoginForm from '$lib/components/account/login-form.svelte';
	import ItemIcon from './item-icon.svelte';

	/**
	 * A scanned SCUTOONS code (in-app scanner or `?code=`): grants device viewing access via `/access/` right away
	 * (no login) and offers — never forces — "Add to my account" (login → `/inventory/claim/`).
	 * Promo codes (not claimable, `signup`): no item to collect — offers "Create account" instead (email → one-time
	 * code → opt-in checkbox); the code is only the signup code, nobody owns it.
	 * `auto` = the reader explicitly came to add it (`/account?code=`): go straight to sign-in / claim / signup.
	 */
	let {
		code,
		auto = false,
		onclose,
		onopeninventory
	}: { code: string; auto?: boolean; onclose: () => void; onopeninventory?: () => void } = $props();

	const inventory = getInventoryContext();
	const session = getSessionContext();
	const membership = getMembershipContext();

	type Phase = 'loading' | 'ready' | 'login' | 'signup' | 'welcome' | 'done' | 'error';
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
					<LoginForm {code} signup onlogin={afterSignup} />
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

			{#if ownedByOther}
				<p class="note">{m['claim.in_someones_account']()}</p>
				<button class="btn-secondary" onclick={onclose}>{m['items.close']()}</button>
			{:else if claimable}
				<p class="note">{m['claim.add_to_account_hint']()}</p>
				{#if claimError}
					<p class="error">{claimError}</p>
				{/if}
				{#if phase === 'login'}
					<LoginForm {code} signup={access?.signup ?? info?.signup ?? true} onlogin={afterLogin} />
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
		width: 2rem;
		height: 2rem;
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
		padding: 0.5rem 1rem;
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
		padding: 0.75rem 1rem;
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
