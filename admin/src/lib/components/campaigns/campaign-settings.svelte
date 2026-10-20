<script lang="ts">
	import { untrack } from 'svelte';
	import { Button, Field, Section, Segmented } from '@scutoons/ui';
	import type { Campaign, CampaignInput } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { loadCatalog } from '$lib/catalog.ts';
	import { campaignError, fromLocalInput, localInput } from '$lib/campaigns.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A campaign's settings (agents/research/GIFT-INVITES.md): name, window, the gift (the member's own item or a
	 * campaign item from the catalog – physical: who pays, the price, where it ships), the invitees' reading (keep after
	 * the campaign, or until a date), limits. Once invites are out the gift stays as it is (the server says so).
	 */
	let { campaign, onsaved }: { campaign: Campaign; onsaved: (next: Campaign) => void } = $props();

	const toaster = getToaster();
	type Item = { id: string; name: string; kind: string; variants: { id: string; name: string }[] };
	let items = $state<Item[]>([]);
	let busy = $state(false);

	const label = (v: unknown) => (v && typeof v === 'object' ? String((v as Record<string, unknown>).de ?? (v as Record<string, unknown>).en ?? '') : String(v ?? ''));
	$effect(() => {
		loadCatalog()
			.then((c) => {
				items = c.catalog.items
					.map((i) => ({
						id: String(i.id),
						name: label(i.name) || String(i.id),
						kind: String(i.kind ?? 'virtual'),
						variants: Array.isArray(i.variants) ? i.variants.map((v: Record<string, unknown>) => ({ id: String(v.id), name: label(v.name) || String(v.id) })) : []
					}))
					.sort((a, b) => a.name.localeCompare(b.name));
			})
			.catch(() => {});
	});

	function fromCampaign(c: Campaign) {
		return {
			name: c.name,
			starts_at: localInput(c.starts_at),
			ends_at: localInput(c.ends_at),
			gift: c.gift,
			item_id: c.item_id ?? '',
			variant: c.variant ?? '',
			stock: c.stock !== null ? String(c.stock) : '',
			per_member: String(c.per_member),
			per_day: String(c.per_day),
			wait_days: String(c.wait_days),
			payer: c.payer ?? 'scutoons',
			price: c.price_cents !== null ? (c.price_cents / 100).toFixed(2) : '',
			countries: c.countries.join(', '),
			access: c.access,
			access_until: localInput(c.access_until)
		};
	}
	// the form starts from the campaign; a save sets both again
	let draft = $state(untrack(() => fromCampaign(campaign)));
	let base = $state(untrack(() => JSON.stringify(fromCampaign(campaign))));
	const dirty = $derived(JSON.stringify(draft) !== base);
	const item = $derived(items.find((i) => i.id === draft.item_id) ?? null);
	// "Verschenken" (research/CAMPAIGN-GIFTS.md): always the campaign's item, SCUTOONS pays, no inviters or limits
	const giveaway = $derived(campaign.kind === 'giveaway');
	const physical = $derived((giveaway || draft.gift === 'campaign') && item?.kind === 'physical');
	const locked = $derived(campaign.counts.sent > 0 || campaign.counts.claimed > 0);

	async function save() {
		busy = true;
		try {
			const input: CampaignInput = {
				name: draft.name,
				starts_at: fromLocalInput(draft.starts_at) ?? undefined,
				ends_at: fromLocalInput(draft.ends_at) ?? undefined,
				gift: giveaway ? 'campaign' : draft.gift,
				item_id: giveaway || draft.gift === 'campaign' ? draft.item_id || null : null,
				variant: giveaway || draft.gift === 'campaign' ? draft.variant || null : null,
				stock: (giveaway || draft.gift === 'campaign') && draft.stock !== '' ? Number(draft.stock) : null,
				per_member: Number(draft.per_member),
				per_day: Number(draft.per_day),
				wait_days: Number(draft.wait_days),
				payer: physical ? (giveaway ? 'scutoons' : (draft.payer as 'scutoons' | 'member')) : null,
				price_cents: physical && draft.payer === 'member' && draft.price !== '' ? Math.round(Number(draft.price.replace(',', '.')) * 100) : null,
				countries: physical ? draft.countries : [],
				access: draft.access,
				access_until: draft.access === 'until' ? fromLocalInput(draft.access_until) : null
			};
			const next = (await admin.updateCampaign(campaign.id, input)).item;
			draft = fromCampaign(next);
			base = JSON.stringify(draft);
			onsaved(next);
			toaster.addToast({ message: m['campaigns.saved'](), type: 'success' });
		} catch (e) {
			toaster.addToast({ message: campaignError(e), type: 'error' });
		} finally {
			busy = false;
		}
	}
</script>

<div class="settings">
	<Field label={m['campaigns.name']()} hint={m['campaigns.name_hint']()}>
		{#snippet children({ id, describedby })}<input {id} aria-describedby={describedby} maxlength="200" bind:value={draft.name} />{/snippet}
	</Field>

	<div class="row">
		<Field label={m['campaigns.starts_at']()}>
			{#snippet children({ id })}<input {id} type="datetime-local" bind:value={draft.starts_at} />{/snippet}
		</Field>
		<Field label={m['campaigns.ends_at']()} hint={giveaway ? m['campaigns.ends_at_hint_giveaway']() : m['campaigns.ends_at_hint']()}>
			{#snippet children({ id, describedby })}<input {id} type="datetime-local" aria-describedby={describedby} bind:value={draft.ends_at} />{/snippet}
		</Field>
	</div>

	<Section title={m['campaigns.gift']()}>
		{#if locked}<p class="muted">{m['campaigns.gift_locked']()}</p>{/if}
		{#if giveaway}<p class="muted choice">{m['campaigns.giveaway_hint']()}</p>{:else}
		<Segmented
			label={m['campaigns.gift']()}
			radio
			vertical
			options={[
				{ value: 'own', label: m['campaigns.gift_own'](), title: m['campaigns.gift_own_hint']() },
				{ value: 'campaign', label: m['campaigns.gift_campaign'](), title: m['campaigns.gift_campaign_hint']() }
			]}
			value={draft.gift}
			onchange={(v) => {
				if (!locked) draft.gift = v;
			}}
		/>
		<p class="muted choice">{draft.gift === 'own' ? m['campaigns.gift_own_hint']() : m['campaigns.gift_campaign_hint']()}</p>
		{/if}
		{#if giveaway || draft.gift === 'campaign'}
			<div class="row">
				<Field label={m['campaigns.item']()} hint={m['campaigns.item_hint']()}>
					{#snippet children({ id, describedby })}
						<select {id} aria-describedby={describedby} disabled={locked} bind:value={draft.item_id} onchange={() => (draft.variant = '')}>
							<option value="" disabled>{m['campaigns.item_choose']()}</option>
							{#each items as i (i.id)}<option value={i.id}>{i.name}{i.kind === 'physical' ? ` · ${m['campaigns.physical']()}` : ''}</option>{/each}
						</select>
					{/snippet}
				</Field>
				{#if item && item.variants.length}
					<Field label={m['campaigns.variant']()}>
						{#snippet children({ id })}
							<select {id} disabled={locked} bind:value={draft.variant}>
								<option value="">—</option>
								{#each item.variants as v (v.id)}<option value={v.id}>{v.name}</option>{/each}
							</select>
						{/snippet}
					</Field>
				{/if}
				<Field label={m['campaigns.stock']()} hint={giveaway ? m['campaigns.stock_hint_giveaway']() : m['campaigns.stock_hint']()}>
					{#snippet children({ id, describedby })}<input {id} aria-describedby={describedby} type="number" min="1" bind:value={draft.stock} />{/snippet}
				</Field>
			</div>
			{#if physical && giveaway}
				<div class="physical">
					<p class="muted choice">{m['campaigns.giveaway_physical_hint']()}</p>
					<Field label={m['campaigns.countries']()} hint={m['campaigns.countries_hint']()}>
						{#snippet children({ id, describedby })}<input {id} aria-describedby={describedby} bind:value={draft.countries} />{/snippet}
					</Field>
				</div>
			{:else if physical}
				<div class="physical">
					<Segmented
						label={m['campaigns.payer']()}
						radio
						vertical
						options={[
							{ value: 'scutoons', label: m['campaigns.payer_scutoons'](), title: m['campaigns.payer_scutoons_hint']() },
							{ value: 'member', label: m['campaigns.payer_member'](), title: m['campaigns.payer_member_hint']() }
						]}
						value={draft.payer}
						onchange={(v) => (draft.payer = v)}
					/>
					<p class="muted choice">{draft.payer === 'member' ? m['campaigns.payer_member_hint']() : m['campaigns.payer_scutoons_hint']()}</p>
					<div class="row">
						{#if draft.payer === 'member'}
							<Field label={m['campaigns.price']()} hint={m['campaigns.price_hint']()}>
								{#snippet children({ id, describedby })}<input {id} aria-describedby={describedby} inputmode="decimal" bind:value={draft.price} />{/snippet}
							</Field>
						{/if}
						<Field label={m['campaigns.countries']()} hint={m['campaigns.countries_hint']()}>
							{#snippet children({ id, describedby })}<input {id} aria-describedby={describedby} bind:value={draft.countries} />{/snippet}
						</Field>
					</div>
				</div>
			{/if}
		{/if}
	</Section>

	<Section title={giveaway ? m['campaigns.access_giveaway']() : m['campaigns.access']()}>
		{#if giveaway}<p class="muted choice">{m['campaigns.access_giveaway_hint']()}</p>{/if}
		<Segmented
			label={m['campaigns.access']()}
			radio
			vertical
			options={[
				{ value: 'keep', label: m['campaigns.access_keep'](), title: giveaway ? m['campaigns.access_keep_hint_giveaway']() : m['campaigns.access_keep_hint']() },
				{ value: 'until', label: m['campaigns.access_until'](), title: m['campaigns.access_until_hint']() }
			]}
			value={draft.access}
			onchange={(v) => (draft.access = v)}
		/>
		<p class="muted choice">{draft.access === 'keep' ? (giveaway ? m['campaigns.access_keep_hint_giveaway']() : m['campaigns.access_keep_hint']()) : m['campaigns.access_until_hint']()}</p>
		{#if draft.access === 'until'}
			<Field label={m['campaigns.access_until_date']()}>
				{#snippet children({ id })}<input {id} type="datetime-local" bind:value={draft.access_until} />{/snippet}
			</Field>
		{/if}
	</Section>

	{#if !giveaway}
	<Section title={m['campaigns.limits']()}>
		<div class="row">
			<Field label={m['campaigns.wait_days']()} hint={m['campaigns.wait_days_hint']()}>
				{#snippet children({ id, describedby })}<input {id} aria-describedby={describedby} type="number" min="1" max="365" bind:value={draft.wait_days} />{/snippet}
			</Field>
			<Field label={m['campaigns.per_member']()} hint={m['campaigns.per_member_hint']()}>
				{#snippet children({ id, describedby })}<input {id} aria-describedby={describedby} type="number" min="1" bind:value={draft.per_member} />{/snippet}
			</Field>
			<Field label={m['campaigns.per_day']()} hint={m['campaigns.per_day_hint']()}>
				{#snippet children({ id, describedby })}<input {id} aria-describedby={describedby} type="number" min="1" bind:value={draft.per_day} />{/snippet}
			</Field>
		</div>
	</Section>
	{/if}

	<div class="save">
		<Button disabled={busy || !dirty} onclick={save}>{m['campaigns.save']()}</Button>
	</div>
</div>

<style>
	.settings {
		display: grid;
		gap: var(--space-2);
		max-width: 48rem;
	}
	.row {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(min(100%, 12rem), 1fr));
		gap: var(--space-1);
	}
	.physical {
		display: grid;
		gap: var(--space-1);
		margin-top: var(--space-1);
	}
	.choice {
		margin: var(--space-05) 0 0;
	}
	.save {
		display: flex;
		justify-content: flex-end;
	}
</style>
