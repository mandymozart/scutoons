<script lang="ts">
	// One lead to work on: organisation, track / stage, why it is here (`hint`), the last log entry, every phone number
	// and address, the call outcome buttons – on Today's due list and in the weekly portion. `onask` adds the Ask button
	// (the agent prepares the call or drafts a mail).
	import { Badge, Icon } from '@scutoons/ui';
	import type { CallOutcome, DueItem } from '@scutoons/api';
	import { label, relativeDay, telHref } from '$lib/labels.ts';
	import { m } from '$lib/paraglide/messages.js';
	import CallOutcomes from '$lib/components/call-outcomes.svelte';

	let {
		item,
		hint = '',
		busy = false,
		onlog,
		onask,
		askLabel = '',
		showDue = true
	}: {
		item: DueItem;
		hint?: string;
		busy?: boolean;
		onlog: (outcome: CallOutcome, nextAt?: string) => void;
		onask?: () => void;
		askLabel?: string;
		/** the "due today / N days late" note (not for a quiet contact or a first contact) */
		showDue?: boolean;
	} = $props();

	const lastText = (item: DueItem) => {
		const a = item.last_activity;
		if (!a) return '';
		const what = a.kind === 'call' && a.outcome ? `${label('kind', a.kind)} (${label('outcome', a.outcome)})` : label('kind', a.kind);
		return m['today.last']({ what, when: relativeDay(a.at.slice(0, 10)) });
	};
</script>

<li class="card" data-testid="due-card">
	<div class="head">
		<a class="name" href={`/orgs/${item.org.id}`}>{item.org.name}</a>
		<span class="muted caption">{[item.org.city, item.org.country].filter(Boolean).join(', ')}</span>
	</div>
	<div class="row badges">
		<Badge tone="ink">{label('track', item.lead.track)}</Badge>
		<Badge>{label('stage', item.lead.stage)}</Badge>
		{#if item.org.tier}<Badge tone="accent">{item.org.tier}</Badge>{/if}
		{#if showDue}
			<span class="caption" class:late={item.overdue_days > 0}>
				{item.overdue_days > 0 ? m['today.overdue']({ days: item.overdue_days }) : m['today.due_today']()}
			</span>
		{/if}
	</div>
	{#if hint}<p class="caption hint">{hint}</p>{/if}
	{#if item.last_activity}
		<p class="caption muted last">{lastText(item)}{item.last_activity.note ? ` – ${item.last_activity.note}` : ''}</p>
	{/if}
	<ul class="contacts">
		{#if item.org.phone}
			<li><a class="tel" href={telHref(item.org.phone)}><Icon name="phone" /> {item.org.phone}</a> <span class="muted caption">{item.org.name}</span></li>
		{/if}
		{#each item.people as person (person.id)}
			{#if person.phone || person.email}
				<li>
					{#if person.phone}<a class="tel" href={telHref(person.phone)}><Icon name="phone" /> {person.phone}</a>{/if}
					{#if person.email}<a href={`mailto:${person.email}`}><Icon name="mail" /> {person.email}</a>{/if}
					<span class="muted caption">{person.name}{person.role ? ` · ${person.role}` : ''}</span>
				</li>
			{/if}
		{/each}
		{#if item.org.email}
			<li><a href={`mailto:${item.org.email}`}><Icon name="mail" /> {item.org.email}</a></li>
		{/if}
		{#if !item.org.phone && !item.org.email && !item.people.some((p) => p.phone || p.email)}
			<li class="muted caption">{m['today.no_contact']()}</li>
		{/if}
	</ul>
	<div class="actions">
		<CallOutcomes disabled={busy} onlog={(outcome, nextAt) => onlog(outcome, nextAt)} />
		{#if onask}<button type="button" class="sc-round" title={askLabel} aria-label={askLabel} onclick={onask}><Icon name="message-square" /></button>{/if}
	</div>
</li>

<style>
	.head {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: var(--space-05);
	}
	.name {
		font-weight: var(--font-weight-regular);
		font-size: 1.1em;
	}
	.badges {
		margin: var(--space-05) 0;
	}
	.late {
		color: var(--state-error);
		font-weight: var(--font-weight-regular);
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
		font-weight: var(--font-weight-regular);
	}
	.hint {
		margin: 0 0 var(--space-05);
	}
	.actions {
		display: flex;
		align-items: flex-start;
		gap: var(--space-05);
	}
	.actions > :global(:first-child) {
		flex: 1;
	}
</style>
