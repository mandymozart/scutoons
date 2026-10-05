<script lang="ts">
	import { Button, Icon } from '@scutoons/ui';
	import { askAbout } from '@scutoons/ask';
	import type { CallOutcome, CheckinAnswer, DueItem, PaceReminder, Portion, PortionItem, TeamMember, TeamPaceRow } from '@scutoons/api';
	import { ApiError, embassy } from '$lib/api.ts';
	import { getEmbassySession } from '$lib/session.svelte.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { hintText, label, relativeDay, shortDate } from '$lib/labels.ts';
	import { pace, setPace } from '$lib/pace.svelte.ts';
	import { changed } from '$lib/ask-page.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import DueCard from '$lib/components/due-card.svelte';
	import PaceChoice from '$lib/components/pace-choice.svelte';

	/**
	 * Today. Mine: this week's portion at my own pace (owner 2026-10-05: "consumable portions per day or per week …
	 * liaisons stay on their own pace") – today's slice first, the rest of the week below, what doesn't fit waits for
	 * next week; every 4 weeks one question whether the pace is right. Nobody's / everyone's / a team member's: the due
	 * list as before, and for ambassadors everyone's pace.
	 */
	const session = getEmbassySession();
	const toaster = getToaster();

	let owner = $state('me');
	let due = $state<DueItem[]>([]);
	let unscheduled = $state(0);
	let team = $state<TeamMember[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let busy = $state<string | null>(null);
	let portion = $state<Portion | null>(null);
	let teamPace = $state<TeamPaceRow[]>([]);
	const isAmbassador = $derived(session.hasRole('ambassador') || session.hasRole('admin'));
	// the weekly portion needs the API's pace table (migration 2026-10-05-embassy-pace); without it: the due list
	const portionMode = $derived(owner === 'me' && !!pace.value);

	async function load() {
		loading = true;
		error = null;
		try {
			if (portionMode) {
				portion = await embassy.portion('me');
				return;
			}
			if (owner === 'all' && isAmbassador) embassy.teamPace().then((r) => (teamPace = r.items), () => (teamPace = []));
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
		void portionMode;
		void changed.count;
		load();
	});

	// ── the pace: change it here; every 4 weeks "is your pace right?" ──
	let editing = $state(false);
	let perWeek = $state(10);
	let reminder = $state<PaceReminder>('weekly');
	function editPace() {
		perWeek = pace.value?.per_week ?? 10;
		reminder = pace.value?.reminder ?? 'weekly';
		editing = !editing;
	}
	async function savePace() {
		try {
			setPace((await embassy.setPace({ per_week: Math.max(1, Math.min(100, Math.round(perWeek) || 10)), reminder })).pace);
			editing = false;
			toaster.addToast({ type: 'success', message: m['pace.saved']() });
			await load();
		} catch (e) {
			toaster.addToast({ type: 'error', message: e instanceof Error ? e.message : String(e) });
		}
	}
	async function checkin(answer: CheckinAnswer) {
		try {
			const { pace: next } = await embassy.checkin(answer);
			setPace(next);
			toaster.addToast({ type: 'success', message: m['pace.checkin_thanks']({ pace: next.per_week }) });
			await load();
		} catch (e) {
			toaster.addToast({ type: 'error', message: e instanceof Error ? e.message : String(e) });
		}
	}

	const GROUPS = { due: m['portion.due'], quiet: m['portion.quiet'], first: m['portion.first'] };
	const todays = $derived(portion ? portion.items.slice(0, portion.today_count) : []);
	const later = $derived(portion ? portion.items.slice(portion.today_count) : []);
	const waiting = $derived(portion ? portion.more.due + portion.more.quiet + portion.more.first : 0);
	const hintOf = (item: PortionItem) => `${GROUPS[item.group]()} · ${hintText(item.hint)}`;
	function prepare(item: PortionItem) {
		askAbout(m['portion.prepare_text']({ org: item.org.name, track: label('track', item.lead.track), hint: hintText(item.hint) }));
	}
	// replies in the shared mailbox land in the log (IMAP; the API reads at most every 2 minutes) and take their leads
	// off the due list – look when the list opens
	$effect(() => {
		embassy.syncInbox().then((r) => void (r.new_activities && load()), () => {});
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

</script>

<h1>{m['today.title']()}</h1>
{#if !portionMode}<p class="muted caption intro">{m['today.intro']()}</p>{/if}

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

{#if portionMode}
	{#if pace.value?.checkin_due}
		<div class="card checkin" data-testid="pace-checkin">
			<p>{m['pace.checkin']({ weeks: 4, pace: pace.value.per_week })}</p>
			<div class="row">
				<Button variant="secondary" size="sm" onclick={() => checkin('less')}>{m['pace.less']()}</Button>
				<Button variant="secondary" size="sm" onclick={() => checkin('fine')}>{m['pace.fine']()}</Button>
				<Button variant="secondary" size="sm" onclick={() => checkin('more')}>{m['pace.more']()}</Button>
			</div>
		</div>
	{/if}
	{#if portion}
		<div class="week" data-testid="pace-week">
			<span>{m['pace.this_week']()}</span>
			<span class="meter" role="meter" aria-valuemin="0" aria-valuemax={portion.week.pace} aria-valuenow={Math.min(portion.week.done, portion.week.pace)} aria-label={m['pace.this_week']()}>
				<i style:width="{Math.min(100, (portion.week.done / Math.max(1, portion.week.pace)) * 100)}%"></i>
			</span>
			<span>{m['pace.done']({ done: portion.week.done, pace: portion.week.pace })}</span>
			<button type="button" class="sc-round" class:is-active={editing} title={m['pace.label']()} aria-label={m['pace.label']()} aria-pressed={editing} onclick={editPace}><Icon name="sliders" /></button>
			{#if portion.items.length}
				<button type="button" class="sc-round" title={m['portion.ask']()} aria-label={m['portion.ask']()} onclick={() => askAbout(m['portion.ask_text']({ count: portion?.items.length ?? 0, date: portion?.today ?? '' }))}><Icon name="message-square" /></button>
			{/if}
		</div>
		{#if editing}
			<div class="card edit">
				<PaceChoice bind:perWeek bind:reminder compact />
				<div><Button size="sm" onclick={savePace}>{m['org.save']()}</Button></div>
			</div>
		{/if}
	{/if}
{/if}

{#if error}
	<p class="error-box">{error}</p>
{:else if portionMode}
	{#if !portion}
		<p class="muted">{m['shell.loading']()}</p>
	{:else if !portion.owned}
		<p class="card">{m['portion.none_owned']()} <a href="/orgs/new">{m['portion.add']()}</a> · <a href="/leads?owner=none">{m['portion.unowned']()}</a></p>
	{:else if !portion.items.length}
		<p class="card">{m['portion.done_week']()}</p>
	{/if}
	{#if todays.length}
		<h2>{m['portion.today']()}</h2>
		<ul class="due" data-testid="portion-today">
			{#each todays as item (item.lead.id)}
				<DueCard {item} hint={hintOf(item)} showDue={item.group === 'due'} busy={busy === item.lead.id} onlog={(outcome, nextAt) => logCall(item, outcome, nextAt)} onask={() => prepare(item)} askLabel={m['portion.prepare']()} />
			{/each}
		</ul>
	{/if}
	{#if later.length}
		<h2>{m['portion.later']()}</h2>
		<ul class="due" data-testid="portion-later">
			{#each later as item (item.lead.id)}
				<DueCard {item} hint={hintOf(item)} showDue={item.group === 'due'} busy={busy === item.lead.id} onlog={(outcome, nextAt) => logCall(item, outcome, nextAt)} onask={() => prepare(item)} askLabel={m['portion.prepare']()} />
			{/each}
		</ul>
	{/if}
	{#if portion && waiting}
		<p class="caption muted">{m['portion.more']({ count: waiting })} <a href="/leads?owner=me">{m['portion.take']()}</a></p>
	{/if}
{:else if loading && !due.length}
	<p class="muted">{m['shell.loading']()}</p>
{:else if !due.length}
	<p class="card">{m['today.empty']()}</p>
{/if}

{#if !portionMode && owner === 'all' && teamPace.length}
	<h2>{m['team.title']()}</h2>
	<div class="team">
		<table data-testid="team-pace">
			<thead><tr><th>{m['team.person']()}</th><th>{m['team.pace']()}</th><th>{m['team.weeks']()}</th><th>{m['team.owned']()}</th><th>{m['team.checkin']()}</th></tr></thead>
			<tbody>
				{#each teamPace as row (row.user.id)}
					<tr>
						<td>{row.user.email}</td>
						<td>{row.pace ? `${row.pace.per_week} ${m['pace.unit']()}` : m['team.not_onboarded']()}</td>
						<td>{row.weeks.map((w) => w.done).join(' · ')}</td>
						<td>{row.owned}</td>
						<td>{row.last_checkin ? `${shortDate(row.last_checkin.at.slice(0, 10))}: ${m[`pace.${row.last_checkin.answer}`]()} (${row.last_checkin.per_week_before} → ${row.last_checkin.per_week_after})` : m['team.none']()}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
{/if}

{#if !portionMode}
<ul class="due" data-testid="due-list">
	{#each due as item (item.lead.id)}
		<DueCard {item} busy={busy === item.lead.id} onlog={(outcome, nextAt) => logCall(item, outcome, nextAt)} />
	{/each}
</ul>

{/if}

{#if !portionMode && unscheduled}
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
	.checkin,
	.edit {
		display: flex;
		flex-direction: column;
		gap: var(--space-075);
		margin-bottom: var(--space-1);
	}
	.checkin p {
		margin: 0;
	}
	.week {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-05) var(--space-1);
		margin-bottom: var(--space-1);
	}
	.meter {
		position: relative;
		flex: 1;
		min-width: 6rem;
		height: 0.5rem;
		border: var(--border-width-hairline) solid var(--ink);
	}
	.meter i {
		position: absolute;
		inset: 0 auto 0 0;
		background: var(--ink);
	}
	.team {
		overflow-x: auto;
		margin-bottom: var(--space-15);
	}
	.team table {
		border-collapse: collapse;
		width: 100%;
	}
	.team :is(th, td) {
		text-align: left;
		padding: var(--space-025) var(--space-05);
		border-bottom: var(--border-width-hairline) solid var(--border-subtle);
		white-space: nowrap;
	}
	.team th {
		font-size: var(--type-caption-size);
		color: var(--text-muted);
		font-weight: var(--font-weight-regular);
	}
	.due {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		margin: 0 0 var(--space-15);
		padding: 0;
		list-style: none;
	}
</style>
