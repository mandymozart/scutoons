<script lang="ts">
	// The AI usage on the Dashboard (owner 2026-10-05): what the Ask agents cost (GET /api/admin/agent-usage, the Ask
	// service logs every Anthropic call) – this month per app and per person, and per day in the dashboard's period
	// (one bar series per app). Days are UTC days, as the log stores them.
	import { ApiError, type AgentUsageOverview, type AgentUsageRow } from '@scutoons/api';
	import { Chart, Panel, Skeleton } from '@scutoons/ui';
	import { admin } from '$lib/api.ts';
	import { errorText, number, shortDay } from '$lib/format.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';

	let { days }: { days: number } = $props();

	const APPS: AgentUsageRow['app'][] = ['creator', 'embassy', 'admin'];
	const appLabel = (app: AgentUsageRow['app']) => (app === 'creator' ? m['dashboard.app_creator']() : app === 'embassy' ? m['dashboard.app_embassy']() : m['dashboard.app_admin']());
	const usd = new Intl.NumberFormat(getLocale(), { style: 'currency', currency: 'USD', minimumFractionDigits: 2, maximumFractionDigits: 2 });
	const cost = (value: number) => usd.format(value);
	/** the chart's axis and tooltip: the caption says USD */
	const plain = new Intl.NumberFormat(getLocale(), { minimumFractionDigits: 2, maximumFractionDigits: 2 });
	const amount = (value: number) => plain.format(value);

	const isoDay = (date: Date) => date.toISOString().slice(0, 10);

	let month = $state<AgentUsageOverview | null>(null);
	let daily = $state<AgentUsageOverview | null>(null);
	let error = $state<string | null>(null);
	let unavailable = $state(false);

	$effect(() => {
		const today = new Date();
		const from = new Date(today.getTime() - (days - 1) * 86_400_000);
		error = null;
		Promise.all([
			admin.agentUsage({ from: `${isoDay(today).slice(0, 8)}01`, to: isoDay(today), group: 'month' }),
			admin.agentUsage({ from: isoDay(from), to: isoDay(today), group: 'day' })
		])
			.then(([a, b]) => {
				month = a;
				daily = b;
			})
			.catch((e) => {
				if (e instanceof ApiError && e.status === 503) unavailable = true;
				else error = errorText(e);
			});
	});

	const sum = (rows: AgentUsageRow[], match: (r: AgentUsageRow) => boolean) => rows.filter(match).reduce((total, r) => total + r.cost_usd, 0);
	const calls = (rows: AgentUsageRow[], match: (r: AgentUsageRow) => boolean) => rows.filter(match).reduce((total, r) => total + r.calls, 0);

	const tiles = $derived(
		month
			? [
					{ label: m['dashboard.ai_month'](), value: cost(month.totals.cost_usd), note: m['dashboard.ai_calls']({ count: number(month.totals.calls) }) },
					...APPS.map((app) => ({ label: appLabel(app), value: cost(sum(month!.rows, (r) => r.app === app)), note: m['dashboard.ai_calls']({ count: number(calls(month!.rows, (r) => r.app === app)) }) }))
				]
			: []
	);

	/** every day of the period (zero-filled), one series per app */
	const chart = $derived.by(() => {
		if (!daily) return null;
		const labels: string[] = [];
		for (let d = new Date(`${daily.from}T00:00:00Z`); isoDay(d) <= daily.to; d = new Date(d.getTime() + 86_400_000)) labels.push(isoDay(d));
		const rows = daily.rows;
		return {
			labels,
			series: APPS.map((app) => ({ label: appLabel(app), values: labels.map((day) => Number(sum(rows, (r) => r.app === app && r.period === day).toFixed(4))) }))
		};
	});

	/** this month per person: cost per app + total, most expensive first */
	const people = $derived.by(() => {
		if (!month) return [];
		const byUser = new Map<string, { email: string; apps: Record<string, number>; total: number }>();
		for (const r of month.rows) {
			const row = byUser.get(r.user_id) ?? { email: r.email ?? r.user_id, apps: {}, total: 0 };
			row.apps[r.app] = (row.apps[r.app] ?? 0) + r.cost_usd;
			row.total += r.cost_usd;
			byUser.set(r.user_id, row);
		}
		return [...byUser.values()].sort((a, b) => b.total - a.total);
	});
</script>

<Panel title={m['dashboard.ai_title']()}>
	{#if unavailable}
		<p class="muted">{m['dashboard.ai_unavailable']()}</p>
	{:else if error}
		<p class="error-box" role="alert">{error}</p>
	{:else if !month || !chart}
		<Skeleton variant="block" height="10rem" label={m['shell.loading']()} />
	{:else}
		<ul class="tiles" data-testid="ai-usage">
			{#each tiles as tile (tile.label)}
				<li>
					<span class="label">{tile.label}</span>
					<span class="value">{tile.value}</span>
					<span class="label">{tile.note}</span>
				</li>
			{/each}
		</ul>
		<p class="caption">{m['dashboard.ai_chart']()}</p>
		<Chart kind="bar" labels={chart.labels} series={chart.series} ariaLabel={m['dashboard.ai_chart']()} formatLabel={shortDay} formatValue={amount} height={140} />
		<p class="caption">{m['dashboard.ai_people']()}</p>
		{#if people.length}
			<table class="people">
				<thead>
					<tr>
						<th scope="col">{m['dashboard.ai_person']()}</th>
						<th scope="col" class="end">{m['dashboard.ai_total']()}</th>
					</tr>
				</thead>
				<tbody>
					{#each people as p (p.email)}
						<tr>
							<td class="who">
								{p.email}
								<span class="split">{APPS.filter((app) => p.apps[app]).map((app) => `${appLabel(app)} ${cost(p.apps[app])}`).join(' · ')}</span>
							</td>
							<td class="end">{cost(p.total)}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		{:else}
			<p class="muted">{m['dashboard.ai_none']()}</p>
		{/if}
	{/if}
</Panel>

<style>
	.tiles {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(9rem, 1fr));
		margin: 0 0 var(--space-1);
		padding: 0;
		list-style: none;
		border: var(--border-width-regular) solid var(--ink);
	}
	.tiles li {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		padding: var(--space-075) var(--space-1);
		border-right: var(--border-width-hairline) solid var(--border-subtle);
		border-bottom: var(--border-width-hairline) solid var(--border-subtle);
	}
	.label,
	.caption {
		color: var(--text-muted);
		font-size: var(--type-caption-size);
	}
	.caption {
		margin: var(--space-1) 0 var(--space-05);
	}
	.value {
		font-size: 1.6em;
		font-variant-numeric: tabular-nums;
	}
	.people {
		width: 100%;
		border-collapse: collapse;
		font-variant-numeric: tabular-nums;
	}
	.people th,
	.people td {
		padding: var(--space-05) var(--space-075);
		text-align: left;
		border-bottom: var(--border-width-hairline) solid var(--border-subtle);
	}
	.people thead th {
		border-bottom: var(--border-width-regular) solid var(--ink);
		white-space: nowrap;
	}
	.who {
		overflow-wrap: anywhere;
	}
	.split {
		display: block;
		color: var(--text-muted);
		font-size: var(--type-caption-size);
	}
	.people td {
		vertical-align: top;
	}
	.end {
		text-align: right !important;
	}
	.muted {
		color: var(--text-muted);
	}
</style>
