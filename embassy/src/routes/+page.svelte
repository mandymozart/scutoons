<script lang="ts">
	import { Badge } from '@scutoons/ui';
	import type { CallOutcome, DueItem, TeamMember } from '@scutoons/api';
	import { ApiError, embassy } from '$lib/api.ts';
	import { getEmbassySession } from '$lib/session.svelte.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { label, relativeDay, shortDate, telHref } from '$lib/labels.ts';
	import { m } from '$lib/paraglide/messages.js';
	import CallOutcomes from '$lib/components/call-outcomes.svelte';

	/** Today: the due list of the signed-in person (or nobody's / everyone's / a team member's) */
	const session = getEmbassySession();
	const toaster = getToaster();

	let owner = $state('me');
	let due = $state<DueItem[]>([]);
	let unscheduled = $state(0);
	let team = $state<TeamMember[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let busy = $state<string | null>(null);

	async function load() {
		loading = true;
		error = null;
		try {
			const result = await embassy.today(owner);
			due = result.due;
			unscheduled = result.unscheduled;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		void owner;
		load();
	});
	$effect(() => {
		embassy.team().then((r) => (team = r.items.filter((t) => t.id !== session.user?.id)), () => {});
	});

	async function logCall(item: DueItem, outcome: CallOutcome, nextAt?: string) {
		busy = item.lead.id;
		try {
			const result = await embassy.log(item.org.id, {
				kind: 'call',
				track: item.lead.track,
				outcome,
				person_id: item.people.find((p) => p.phone)?.id ?? item.people[0]?.id ?? null,
				...(nextAt ? { next_at: nextAt } : {})
			});
			const next = result.lead?.next_at;
			toaster.addToast({
				type: 'success',
				message: next ? m['today.logged']({ date: `${shortDate(next)} (${relativeDay(next)})` }) : m['today.logged_closed']()
			});
			await load();
		} catch (e) {
			toaster.addToast({ type: 'error', message: e instanceof ApiError ? e.message : String(e) });
		} finally {
			busy = null;
		}
	}

	const lastText = (item: DueItem) => {
		const a = item.last_activity;
		if (!a) return '';
		const what = a.kind === 'call' && a.outcome ? `${label('kind', a.kind)} (${label('outcome', a.outcome)})` : label('kind', a.kind);
		return m['today.last']({ what, when: relativeDay(a.at.slice(0, 10)) });
	};
</script>

<h1>{m['today.title']()}</h1>
<p class="muted caption intro">{m['today.intro']()}</p>

<div class="row owner">
	<label class="field">
		<span>{m['today.owner']()}</span>
		<select bind:value={owner}>
			<option value="me">{m['today.mine']()}</option>
			<option value="none">{m['today.unowned']()}</option>
			<option value="all">{m['today.everyone']()}</option>
			{#each team as member (member.id)}
				<option value={member.id}>{member.email}</option>
			{/each}
		</select>
	</label>
</div>

{#if error}
	<p class="error-box">{error}</p>
{:else if loading && !due.length}
	<p class="muted">{m['shell.loading']()}</p>
{:else if !due.length}
	<p class="card">{m['today.empty']()}</p>
{/if}

<ul class="due" data-testid="due-list">
	{#each due as item (item.lead.id)}
		<li class="card">
			<div class="head">
				<a class="name" href={`/orgs/${item.org.id}`}>{item.org.name}</a>
				<span class="muted caption">{[item.org.city, item.org.country].filter(Boolean).join(', ')}</span>
			</div>
			<div class="row badges">
				<Badge tone="ink">{label('track', item.lead.track)}</Badge>
				<Badge>{label('stage', item.lead.stage)}</Badge>
				{#if item.org.tier}<Badge tone="accent">{item.org.tier}</Badge>{/if}
				<span class="caption" class:late={item.overdue_days > 0}>
					{item.overdue_days > 0 ? m['today.overdue']({ days: item.overdue_days }) : m['today.due_today']()}
				</span>
			</div>
			{#if item.last_activity}
				<p class="caption muted last">{lastText(item)}{item.last_activity.note ? ` – ${item.last_activity.note}` : ''}</p>
			{/if}
			<ul class="contacts">
				{#if item.org.phone}
					<li><a class="tel" href={telHref(item.org.phone)}>☎ {item.org.phone}</a> <span class="muted caption">{item.org.name}</span></li>
				{/if}
				{#each item.people as person (person.id)}
					{#if person.phone || person.email}
						<li>
							{#if person.phone}<a class="tel" href={telHref(person.phone)}>☎ {person.phone}</a>{/if}
							{#if person.email}<a href={`mailto:${person.email}`}>✉ {person.email}</a>{/if}
							<span class="muted caption">{person.name}{person.role ? ` · ${person.role}` : ''}</span>
						</li>
					{/if}
				{/each}
				{#if item.org.email}
					<li><a href={`mailto:${item.org.email}`}>✉ {item.org.email}</a></li>
				{/if}
				{#if !item.org.phone && !item.org.email && !item.people.some((p) => p.phone || p.email)}
					<li class="muted caption">{m['today.no_contact']()}</li>
				{/if}
			</ul>
			<CallOutcomes disabled={busy === item.lead.id} onlog={(outcome, nextAt) => logCall(item, outcome, nextAt)} />
		</li>
	{/each}
</ul>

{#if unscheduled}
	<p><a href="/leads?stage=research&owner={owner === 'all' ? '' : owner}">{m['today.unscheduled']({ count: unscheduled })}</a></p>
{/if}

<style>
	.intro {
		max-width: 40rem;
	}
	.owner {
		margin: var(--space-1) 0;
	}
	.owner .field {
		min-width: 14rem;
	}
	.due {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		margin: 0 0 var(--space-15);
		padding: 0;
		list-style: none;
	}
	.head {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: var(--space-05);
	}
	.name {
		font-weight: var(--font-weight-bold);
		font-size: 1.1em;
	}
	.badges {
		margin: var(--space-05) 0;
	}
	.late {
		color: var(--state-error);
		font-weight: var(--font-weight-bold);
	}
	.last {
		margin: 0 0 var(--space-05);
	}
	.contacts {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		margin: 0 0 var(--space-075);
		padding: 0;
		list-style: none;
	}
	.contacts li {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: var(--space-05);
	}
	.tel {
		font-weight: var(--font-weight-bold);
	}
</style>
