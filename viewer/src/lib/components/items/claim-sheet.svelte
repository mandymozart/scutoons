<script lang="ts">
	import { onMount } from 'svelte';
	import { ApiError } from '$lib/api/client.ts';
	import { fetchCode, type VerifyResult } from '$lib/api/qr.service.ts';
	import { getInventoryContext, localized } from '$lib/contexts/inventory.svelte.ts';
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import type { CodeInfo, CodeOption, ItemVariant } from '$lib/models/items.ts';
	import { m } from '$lib/paraglide/messages.js';
	import LoginForm from '$lib/components/account/login-form.svelte';
	import ItemIcon from './item-icon.svelte';

	/** Preview + claim of a SCUTOONS QR code (from the in-app scanner or `?code=`). */
	let {
		code,
		auto = false,
		onclose,
		onopeninventory
	}: { code: string; auto?: boolean; onclose: () => void; onopeninventory?: () => void } = $props();

	const inventory = getInventoryContext();
	const session = getSessionContext();

	type Phase = 'loading' | 'preview' | 'login' | 'done' | 'error';
	let phase = $state<Phase>('loading');
	let info = $state<CodeInfo | null>(null);
	let error = $state<string | null>(null);
	let options = $state<CodeOption[]>([]);
	let chosenItem = $state<string | null>(null);
	let chosenVariant = $state<string | null>(null);
	let claimedItem = $state<string | null>(null);
	let working = $state(false);

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
		(info?.items?.length ? info.items : options.map((o) => ({ item_id: o.item_id, variant: null, qty: 1 }))) ??
			[]
	);

	async function load() {
		phase = 'loading';
		try {
			info = await fetchCode(code);
			options = info.options ?? [];
			if (options.length === 1) {
				chosenItem = options[0].item_id;
				const only = options[0].variants?.length === 1 ? options[0].variants[0] : null;
				if (only) chosenVariant = typeof only === 'string' ? only : only.id;
			}
			phase = info.claimed_by_me ? 'done' : 'preview';
			if (info.claimed_by_me) claimedItem = info.items?.[0]?.item_id ?? null;
			// `?code=` while logged in: claim right away when there is nothing to choose
			if (auto && phase === 'preview' && !info.claimed && session.isLoggedIn && canClaim) await claim();
		} catch (e) {
			error =
				e instanceof ApiError && e.status === 404
					? m['claim.not_found']()
					: e instanceof ApiError && e.status === 0
						? m['account.network_error']()
						: m['claim.failed']();
			phase = 'error';
		}
	}

	async function claim() {
		if (!session.isLoggedIn) {
			phase = 'login';
			return;
		}
		working = true;
		error = null;
		try {
			await inventory.claim(code, chosenItem ?? undefined, chosenVariant);
			claimedItem = chosenItem ?? info?.items?.[0]?.item_id ?? null;
			phase = 'done';
		} catch (e) {
			if (e instanceof ApiError && e.code === 'choose_item') {
				options = e.body?.options ?? options;
				error = m['claim.choose']();
				phase = 'preview';
			} else if (e instanceof ApiError && (e.status === 409 || e.code === 'already_claimed')) {
				error = m['claim.already_claimed']();
			} else {
				error = m['claim.failed']();
			}
		} finally {
			working = false;
		}
	}

	async function afterLogin(result: VerifyResult) {
		if (result.claimed?.includes(code)) {
			claimedItem = info?.items?.[0]?.item_id ?? null;
			await session.loadMe();
			phase = 'done';
			return;
		}
		phase = 'preview';
		await claim();
	}

	onMount(load);
</script>

<div class="claim-backdrop" role="presentation" onclick={onclose}></div>
<div class="claim" role="dialog" aria-modal="true" aria-label={m['claim.title']()}>
	<header>
		<h2>{m['claim.title']()}</h2>
		<button class="close" onclick={onclose} aria-label={m['items.close']()}>×</button>
	</header>

	{#if phase === 'loading'}
		<p>{m['claim.loading']()}</p>
	{:else if phase === 'error'}
		<p class="error">{error}</p>
		<button class="btn-secondary" onclick={onclose}>{m['items.close']()}</button>
	{:else}
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

		{#if phase === 'done'}
			{@const id = claimedItem ?? previewItems[0]?.item_id}
			<div class="done">
				{#if id}
					<div class="hero"><ItemIcon srcs={[inventory.item(id)?.icon ?? '']} name={inventory.nameOf({ item_id: id, variant: chosenVariant })} /></div>
					<strong>{inventory.nameOf({ item_id: id, variant: chosenVariant })}</strong>
				{/if}
				<p>{info?.claimed_by_me && !claimedItem ? m['claim.claimed_by_me']() : m['claim.done']()}</p>
				<div class="actions">
					{#if onopeninventory}
						<button onclick={onopeninventory}>{m['account.open_inventory']()}</button>
					{/if}
					<button class="btn-secondary" onclick={onclose}>{m['items.close']()}</button>
				</div>
			</div>
		{:else if info?.claimed}
			<p class="error">{m['claim.already_claimed']()}</p>
			<button class="btn-secondary" onclick={onclose}>{m['items.close']()}</button>
		{:else}
			{#if needsChoice && options.length > 1}
				<h3>{m['claim.choose']()}</h3>
			{:else if !session.isLoggedIn}
				<h3>{m['claim.add_prompt']()}</h3>
			{/if}
			<ul class="items">
				{#each previewItems as it}
					{@const def = inventory.item(it.item_id)}
					{@const shown = needsChoice && it.item_id === chosenItem ? { ...it, variant: chosenVariant } : it}
					<li>
						<button
							class="option"
							class:selected={needsChoice && chosenItem === it.item_id}
							disabled={!needsChoice}
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

			{#if needsChoice && variantChoices.length > 1}
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

			{#if error}
				<p class="error">{error}</p>
			{/if}

			{#if phase === 'login'}
				<p>{m['claim.login_first']()}</p>
				<LoginForm {code} onlogin={afterLogin} />
			{:else}
				<div class="actions">
					<button class="btn-secondary" onclick={onclose}>{m['claim.later']()}</button>
					<button class:disabled={!canClaim || working} disabled={!canClaim || working} onclick={claim}>
						{m['claim.add']()}
					</button>
				</div>
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
