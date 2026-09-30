<script lang="ts">
	import { infoBar } from '$lib/info.ts';
	import PageHelp from '$lib/components/page-help.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ActionBar, Button, Chart, Panel, Skeleton, listHref } from '@scutoons/ui';
	import type { Dashboard } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { errorText, number, percent, shortDay } from '$lib/format.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PageHeader from '$lib/components/page-header.svelte';

	const DAY_CHOICES = [7, 14, 30, 60, 90, 180, 365];

	const days = $derived(Number(page.url.searchParams.get('days')) || 30);
	const editionId = $derived(page.url.searchParams.get('edition_id') ?? '');

	let data = $state<Dashboard | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(true);

	$effect(() => {
		const params = { days, edition_id: editionId || null };
		loading = true;
		error = null;
		admin
			.dashboard(params)
			.then((result) => (data = result))
			.catch((e) => (error = errorText(e)))
			.finally(() => (loading = false));
	});

	const labels = $derived(data?.daily.map((d) => d.day) ?? []);
	const editions = $derived(data?.editions ?? []);

	const kpis = $derived(
		data
			? [
					{ label: m['dashboard.kpi_codes'](), value: number(data.totals.codes), note: m['dashboard.kpi_signed']({ count: number(data.totals.codes_signed) }) },
					{ label: m['dashboard.kpi_claimed'](), value: number(data.totals.codes_claimed), note: percent(data.totals.codes_claimed, data.totals.codes) },
					{ label: m['dashboard.kpi_revoked'](), value: number(data.totals.codes_revoked), note: '' },
					{ label: m['dashboard.kpi_users'](), value: number(data.totals.users), note: m['dashboard.kpi_opted_in']({ count: number(data.totals.users_opted_in) }) },
					{ label: m['dashboard.kpi_scans'](), value: number(data.period.scan), note: m['dashboard.kpi_devices']({ count: number(data.period.unique_devices) }) },
					{ label: m['dashboard.kpi_unlocks'](), value: number(data.period.access), note: m['dashboard.kpi_refused']({ count: number(data.period.claim_refused) }) },
					{ label: m['dashboard.kpi_claims'](), value: number(data.period.claim), note: '' },
					{ label: m['dashboard.kpi_signups'](), value: number(data.period.signup), note: m['dashboard.kpi_new_users']({ count: number(data.totals.users_new) }) }
				]
			: []
	);

	const setParam = (patch: Record<string, string | number | null>) =>
		goto(listHref(page.url, patch), { keepFocus: true, noScroll: true });
</script>

<PageHeader
	type={m['nav.group_overview']()}
	title={m['dashboard.title']()}
	meta={data ? m['dashboard.period']({ from: data.from, to: data.to }) : undefined}
/>

{#snippet barStart()}
	<label class="bar-filter">
		<span>{m['dashboard.days']()}</span>
		<select value={String(days)} onchange={(e) => setParam({ days: e.currentTarget.value })}>
			{#each DAY_CHOICES as d (d)}
				<option value={String(d)}>{m['dashboard.last_days']({ days: d })}</option>
			{/each}
		</select>
	</label>
	<label class="bar-filter">
		<span>{m['common.edition']()}</span>
		<select value={editionId} onchange={(e) => setParam({ edition_id: e.currentTarget.value })}>
			<option value="">{m['common.all']()}</option>
			<option value="none">{m['common.no_edition']()}</option>
			{#each editions.filter((e) => e.id) as e (e.id)}
				<option value={e.id}>{e.name ?? e.id}</option>
			{/each}
		</select>
	</label>
{/snippet}

<ActionBar start={barStart} label={m['shell.page_actions']()} moreLabel={m['shell.more']()} {...infoBar()} infoTitle={m['dashboard.title']()}>
	{#snippet info()}<PageHelp page="dashboard" />{/snippet}
	{#snippet end()}
		<Button size="sm" variant="secondary" href="/events">{m['nav.events']()}</Button>
		<Button size="sm" href="/codes">{m['nav.codes']()}</Button>
	{/snippet}
</ActionBar>

{#if error}
	<p class="error-box" role="alert">{error}</p>
{:else if !data}
	<Skeleton variant="block" height="6rem" label={m['shell.loading']()} />
	<br />
	<Skeleton variant="block" height="14rem" label={m['shell.loading']()} />
{:else}
	<ul class="kpis" aria-busy={loading}>
		{#each kpis as kpi (kpi.label)}
			<li>
				<span class="kpi-label">{kpi.label}</span>
				<strong class="kpi-value">{kpi.value}</strong>
				{#if kpi.note}<span class="kpi-note">{kpi.note}</span>{/if}
			</li>
		{/each}
	</ul>

	<div class="grid-2 charts">
		<Panel title={m['dashboard.chart_scans']()}>
			<Chart
				kind="line"
				{labels}
				series={[
					{ label: m['events.scan'](), values: data.daily.map((d) => d.scan) },
					{ label: m['events.access'](), values: data.daily.map((d) => d.access) }
				]}
				ariaLabel={m['dashboard.chart_scans']()}
				formatLabel={shortDay}
			/>
		</Panel>
		<Panel title={m['dashboard.chart_claims']()}>
			<Chart
				kind="bar"
				{labels}
				series={[
					{ label: m['events.claim'](), values: data.daily.map((d) => d.claim) },
					{ label: m['events.signup'](), values: data.daily.map((d) => d.signup) }
				]}
				ariaLabel={m['dashboard.chart_claims']()}
				formatLabel={shortDay}
			/>
		</Panel>
	</div>

	<Panel title={m['dashboard.editions']()}>
		<div class="table-scroll">
			<table class="editions">
				<thead>
					<tr>
						<th scope="col">{m['common.edition']()}</th>
						<th scope="col" class="meter-col">{m['dashboard.claimed_total']()}</th>
						<th scope="col" class="end">{m['dashboard.kpi_revoked']()}</th>
						<th scope="col" class="end">{m['dashboard.signed']()}</th>
						<th scope="col" class="end">{m['events.scan']()}</th>
						<th scope="col" class="end">{m['events.claim']()}</th>
					</tr>
				</thead>
				<tbody>
					{#each editions as e (e.id ?? 'none')}
						<tr>
							<th scope="row">
								{#if e.id}
									<a href="/codes?edition_id={encodeURIComponent(e.id)}">{e.name ?? e.id}</a>
									<span class="muted">{e.id}</span>
								{:else}
									<a href="/codes?edition_id=none">{m['common.no_edition']()}</a>
								{/if}
							</th>
							<td class="meter-col">
								<span class="meter" role="img" aria-label="{e.claimed} / {e.codes}">
									<span class="fill" style:width="{e.codes ? (e.claimed / e.codes) * 100 : 0}%"></span>
								</span>
								<span class="nums">{number(e.claimed)} / {number(e.codes)}{#if e.edition_count}<span class="muted"> ({m['dashboard.of_edition']({ count: e.edition_count })})</span>{/if}</span>
							</td>
							<td class="end">{number(e.revoked)}</td>
							<td class="end">{number(e.signed)}</td>
							<td class="end">{number(e.scan)}</td>
							<td class="end">{number(e.claim)}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	</Panel>
{/if}

<style>
	.kpis {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(10rem, 1fr));
		gap: 0;
		margin: 0 0 var(--space-15);
		padding: 0;
		list-style: none;
		border: var(--border-width-regular) solid var(--ink);
	}
	.kpis li {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		padding: var(--space-075) var(--space-1);
		border-right: var(--border-width-hairline) solid var(--border-subtle);
		border-bottom: var(--border-width-hairline) solid var(--border-subtle);
	}
	.kpis[aria-busy='true'] {
		opacity: 0.6;
	}
	.kpi-label,
	.kpi-note {
		color: var(--text-muted);
		font-size: var(--type-caption-size);
	}
	.kpi-value {
		font-size: 1.6em;
		font-variant-numeric: tabular-nums;
	}
	.charts {
		margin-bottom: var(--space-15);
	}
	.table-scroll {
		overflow-x: auto;
	}
	.editions {
		width: 100%;
		border-collapse: collapse;
		font-variant-numeric: tabular-nums;
	}
	.editions th,
	.editions td {
		padding: var(--space-05) var(--space-075);
		text-align: left;
		border-bottom: var(--border-width-hairline) solid var(--border-subtle);
		vertical-align: middle;
	}
	.editions thead th {
		border-bottom: var(--border-width-regular) solid var(--ink);
		white-space: nowrap;
	}
	.editions tbody th {
		font-weight: var(--font-weight-regular);
	}
	.editions tbody th .muted {
		display: block;
		font-size: var(--type-caption-size);
	}
	.end {
		text-align: right !important;
	}
	.meter-col {
		min-width: 14rem;
	}
	.meter {
		display: block;
		height: 0.6rem;
		border: var(--border-width-hairline) solid var(--ink);
		margin-bottom: var(--space-025);
	}
	.fill {
		display: block;
		height: 100%;
		background: var(--primary-cyan);
	}
	.nums {
		font-size: var(--type-caption-size);
	}
</style>
