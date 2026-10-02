<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { Badge, Button } from '@scutoons/ui';
	import { ORG_TYPES, STAGES, TIERS, TRACKS, type OrgSummary } from '@scutoons/api';
	import { embassy } from '$lib/api.ts';
	import { label, relativeDay } from '$lib/labels.ts';
	import { m } from '$lib/paraglide/messages.js';

	/** All organisations with their leads; the filters live in the URL (shareable, back button) */
	const FILTERS = ['q', 'track', 'stage', 'type', 'country', 'tier', 'owner', 'due'] as const;
	const PAGE = 100;

	const params = $derived(Object.fromEntries(FILTERS.map((k) => [k, page.url.searchParams.get(k) ?? ''])) as Record<(typeof FILTERS)[number], string>);

	let items = $state<OrgSummary[]>([]);
	let total = $state(0);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let q = $state('');

	$effect(() => {
		q = params.q;
	});

	async function load(offset = 0) {
		loading = true;
		error = null;
		try {
			const result = await embassy.orgs({ ...params, limit: PAGE, offset });
			items = offset ? [...items, ...result.items] : result.items;
			total = result.total;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		void params;
		load();
	});

	function set(key: string, value: string) {
		const url = new URL(page.url);
		if (value) url.searchParams.set(key, value);
		else url.searchParams.delete(key);
		goto(url.pathname + url.search, { replaceState: true, keepFocus: true, noScroll: true });
	}

	let searchTimer: ReturnType<typeof setTimeout> | undefined;
	function search(value: string) {
		q = value;
		clearTimeout(searchTimer);
		searchTimer = setTimeout(() => set('q', value.trim()), 300);
	}

	const shownLead = (org: OrgSummary) => (params.track ? org.leads.filter((l) => l.track === params.track) : org.leads);
</script>

<h1>{m['leads.title']()}</h1>

<div class="filters">
	<label class="field search">
		<span>{m['leads.search']()}</span>
		<input type="search" value={q} oninput={(e) => search(e.currentTarget.value)} />
	</label>
	<label class="field">
		<span>{m['leads.track']()}</span>
		<select value={params.track} onchange={(e) => set('track', e.currentTarget.value)}>
			<option value="">{m['leads.all']()}</option>
			{#each TRACKS as t (t)}<option value={t}>{label('track', t)}</option>{/each}
		</select>
	</label>
	<label class="field">
		<span>{m['leads.stage']()}</span>
		<select value={params.stage} onchange={(e) => set('stage', e.currentTarget.value)}>
			<option value="">{m['leads.all']()}</option>
			{#each STAGES as s (s)}<option value={s}>{label('stage', s)}</option>{/each}
		</select>
	</label>
	<label class="field">
		<span>{m['leads.type']()}</span>
		<select value={params.type} onchange={(e) => set('type', e.currentTarget.value)}>
			<option value="">{m['leads.all']()}</option>
			{#each ORG_TYPES as t (t)}<option value={t}>{label('type', t)}</option>{/each}
		</select>
	</label>
	<label class="field narrow">
		<span>{m['leads.country']()}</span>
		<input value={params.country} maxlength="2" placeholder="AT" onchange={(e) => set('country', e.currentTarget.value.trim().toUpperCase())} />
	</label>
	<label class="field narrow">
		<span>{m['leads.tier']()}</span>
		<select value={params.tier} onchange={(e) => set('tier', e.currentTarget.value)}>
			<option value="">{m['leads.all']()}</option>
			{#each TIERS as t (t)}<option value={t}>{t}</option>{/each}
		</select>
	</label>
	<label class="field">
		<span>{m['leads.owner']()}</span>
		<select value={params.owner} onchange={(e) => set('owner', e.currentTarget.value)}>
			<option value="">{m['leads.all']()}</option>
			<option value="me">{m['today.mine']()}</option>
			<option value="none">{m['today.unowned']()}</option>
		</select>
	</label>
	<label class="check">
		<input type="checkbox" checked={params.due === '1'} onchange={(e) => set('due', e.currentTarget.checked ? '1' : '')} />
		{m['leads.due_only']()}
	</label>
</div>

{#if error}
	<p class="error-box">{error}</p>
{/if}
<p class="muted caption">{m['leads.count']({ shown: items.length, total })}</p>

{#if !loading && !items.length && !error}
	<p class="card">{m['leads.empty']()}</p>
{/if}

<ul class="list" data-testid="org-list">
	{#each items as org (org.id)}
		<li>
			<a href={`/orgs/${org.id}`}>
				<span class="title">
					<span>{org.name}</span>
					<span class="muted caption">{label('type', org.type)}{org.city || org.country ? ` · ${[org.city, org.country].filter(Boolean).join(', ')}` : ''}</span>
					{#if org.tier}<Badge tone="accent">{org.tier}</Badge>{/if}
					{#if org.do_not_contact}<Badge tone="error">{m['org.blocked']()}</Badge>{/if}
				</span>
				<span class="leads">
					{#each shownLead(org) as lead (lead.id)}
						<span class="lead caption">
							<span>{label('track', lead.track)}</span> {label('stage', lead.stage)}{lead.next_at ? ` · ${m['leads.next']({ date: relativeDay(lead.next_at) })}` : ''}{lead.owner_email ? ` · ${lead.owner_email.split('@')[0]}` : ''}
						</span>
					{:else}
						<span class="caption muted">{m['leads.no_leads']()}</span>
					{/each}
				</span>
			</a>
		</li>
	{/each}
</ul>

{#if items.length < total}
	<Button variant="secondary" disabled={loading} onclick={() => load(items.length)}>{m['leads.more']()}</Button>
{/if}

<style>
	.filters {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(9rem, 1fr));
		align-items: end;
		gap: var(--space-05) var(--space-075);
		margin-bottom: var(--space-1);
	}
	.filters .search {
		grid-column: 1 / -1;
	}
	.list {
		margin: 0 0 var(--space-1);
		padding: 0;
		list-style: none;
		border-top: var(--border-width-hairline) solid var(--border-subtle);
	}
	.list a {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		padding: var(--space-075) var(--space-05);
		text-decoration: none;
		border-bottom: var(--border-width-hairline) solid var(--border-subtle);
	}
	.list a:hover {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-regular));
	}
	.title {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: var(--space-05);
	}
	.leads {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-025) var(--space-1);
	}
</style>
