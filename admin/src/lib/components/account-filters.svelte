<script lang="ts">
	import type { AdminSegment, BatchRow, Campaign } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { loadCatalog } from '$lib/catalog.ts';
	import { day } from '$lib/format.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The account filters (agents/research/GIFT-INVITES.md 1b; server AccountFilters) as bar filters: signed up with,
	 * edition, batch, campaign (+ invited / joined), segment. The Users list and a campaign's account list use them;
	 * `onchange` gets the changed parameters ('' = off). Role and opt-in stay the Users page's own.
	 */
	let {
		filters,
		onchange
	}: {
		filters: Record<string, string>;
		onchange: (patch: Record<string, string>) => void;
	} = $props();

	let editions = $state<{ id: string; name: string }[]>([]);
	let batches = $state<BatchRow[]>([]);
	let campaigns = $state<Campaign[]>([]);
	let segments = $state<AdminSegment[]>([]);

	$effect(() => {
		loadCatalog()
			.then((c) => (editions = c.editions.map((e) => ({ id: String(e.id), name: String(e.name ?? e.id) }))))
			.catch(() => {});
		admin
			.listBatches({ page_size: 500 })
			.then((r) => (batches = r.items))
			.catch(() => {});
		admin
			.campaigns()
			.then((r) => (campaigns = r.items))
			.catch(() => {});
		admin
			.segments()
			.then((r) => (segments = r.items))
			.catch(() => {});
	});

	const SIGNUPS = $derived([
		{ value: 'membership', label: m['accounts.signup_membership']() },
		{ value: 'promo', label: m['accounts.signup_promo']() },
		{ value: 'invite', label: m['accounts.signup_invite']() },
		{ value: 'none', label: m['accounts.signup_none']() }
	]);
</script>

<label class="bar-filter">
	<span>{m['accounts.signup']()}</span>
	<select value={filters.signup ?? ''} onchange={(e) => onchange({ signup: e.currentTarget.value })}>
		<option value="">{m['common.all']()}</option>
		{#each SIGNUPS as s (s.value)}<option value={s.value}>{s.label}</option>{/each}
	</select>
</label>
<label class="bar-filter">
	<span>{m['accounts.edition']()}</span>
	<select value={filters.edition ?? ''} onchange={(e) => onchange({ edition: e.currentTarget.value })}>
		<option value="">{m['common.all']()}</option>
		{#each editions as e (e.id)}<option value={e.id}>{e.name}</option>{/each}
	</select>
</label>
<label class="bar-filter">
	<span>{m['accounts.batch']()}</span>
	<select value={filters.batch ?? ''} onchange={(e) => onchange({ batch: e.currentTarget.value })}>
		<option value="">{m['common.all']()}</option>
		{#each batches as b (b.filename)}<option value={b.filename}>{b.edition} · {day(b.created_at)}</option>{/each}
	</select>
</label>
<label class="bar-filter">
	<span>{m['accounts.campaign']()}</span>
	<select value={filters.campaign ?? ''} onchange={(e) => onchange({ campaign: e.currentTarget.value, campaign_role: e.currentTarget.value ? (filters.campaign_role ?? '') : '' })}>
		<option value="">{m['common.all']()}</option>
		{#each campaigns as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
	</select>
</label>
{#if filters.campaign}
	<label class="bar-filter">
		<span>{m['accounts.campaign_role']()}</span>
		<select value={filters.campaign_role ?? ''} onchange={(e) => onchange({ campaign_role: e.currentTarget.value })}>
			<option value="">{m['accounts.campaign_both']()}</option>
			<option value="invited">{m['accounts.campaign_invited']()}</option>
			<option value="joined">{m['accounts.campaign_joined']()}</option>
		</select>
	</label>
{/if}
<label class="bar-filter">
	<span>{m['accounts.segment']()}</span>
	<select value={filters.segment ?? ''} onchange={(e) => onchange({ segment: e.currentTarget.value })}>
		<option value="">{m['common.all']()}</option>
		<option value="none">{m['accounts.segment_none']()}</option>
		{#each segments as s (s.id)}<option value={s.id}>{s.name} ({s.members})</option>{/each}
	</select>
</label>
