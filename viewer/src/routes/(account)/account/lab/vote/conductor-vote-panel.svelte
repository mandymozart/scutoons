<script lang="ts">
	// #231 draft: the conductor's vote panel – question, rule, fallback, the countdown (`vote.tally.left`), live bars,
	// Close now / Override, then the result and how it was decided (`vote.result.how`). Dark like the 2026-10-07 draft
	// (a conductor sits in a dark room). Fed by the fake relay (src/lib/live-vote-sim.ts); nothing here is the protocol.
	import { Button } from '@scutoons/ui';
	import type { VoteOpen, VoteResult, VoteTally } from '@scutoons/live';
	import type { FakeVoteRelay } from '$lib/live-vote-sim.ts';

	let { relay, question, groupNames }: { relay: FakeVoteRelay; question: string; groupNames: Record<string, string> } = $props();

	let open = $state<VoteOpen | null>(null);
	let tally = $state<VoteTally | null>(null);
	let result = $state<VoteResult | null>(null);
	let overriding = $state(false);

	$effect(() =>
		relay.on('conductor', (e) => {
			if (e.kind === 'vote.open') [open, tally, result, overriding] = [e.p, null, null, false];
			else if (e.kind === 'vote.tally') tally = e.p;
			else if (e.kind === 'vote.result') [result, overriding] = [e.p, false];
		})
	);

	const counts = $derived(result?.counts ?? tally?.counts ?? open?.options.map(() => 0) ?? []);
	const voters = $derived(result ? result.counts.reduce((a, b) => a + b, 0) : (tally?.voters ?? 0));
	const leftMs = $derived(result ? 0 : (tally?.left ?? (open ? open.seconds * 1000 : 0)));
	const leading = $derived(result ? result.option : counts.indexOf(Math.max(...counts)));
	/** a bar's scale: share of the voters; first to n: the way to n */
	const scale = $derived(open?.rule === 'first' ? Math.max(open.n ?? 1, ...counts) : Math.max(1, voters));
	const RULES = { majority: 'majority', groups: 'per group', first: 'first to n' } as const;
	const ruleText = $derived(
		!open
			? ''
			: open.rule === 'first'
				? `first to ${open.n ?? 1}`
				: open.rule === 'groups'
					? `per group – each group's majority is one vote`
					: 'majority – most votes'
	);
	const groupsText = $derived(open?.groups ? open.groups.map((g) => groupNames[g] ?? g).join(', ') : 'all audience');
	const fallbackLabel = $derived(open ? open.options[open.fallback ?? 0] : '');
	const HOW = { rule: 'by the rule', fallback: 'by the fallback – nobody voted', override: 'by override – the conductor chose' } as const;
</script>

<section class="conductor" aria-label="Vote (conductor)">
	{#if open}
		<header>
			<span class="label">Vote · {open.vote}</span>
			<span class="state" class:closed={result}>{result ? 'Closed' : 'Open'}</span>
		</header>
		<h2 class="question">{question}</h2>
		<dl class="meta">
			<div><dt>Rule</dt><dd title={ruleText}>{RULES[open.rule]}{open.rule === 'first' ? ` · n ${open.n ?? 1}` : ''}</dd></div>
			<div><dt>Fallback</dt><dd>{fallbackLabel}</dd></div>
			<div><dt>Voting</dt><dd>{groupsText}</dd></div>
			<div><dt>Audience sees tally</dt><dd>{open.show ? 'yes' : 'no'}</dd></div>
		</dl>

		{#if !result}
			<div class="countdown" aria-live="off">
				<span class="seconds">{Math.ceil(leftMs / 1000)}<small>s</small></span>
				<span class="track"><span class="fill" style:width="{(leftMs / (open.seconds * 1000)) * 100}%"></span></span>
			</div>
		{:else}
			<div class="result">
				<span class="label">Result</span>
				<span class="winner">{open.options[result.option]}</span>
				<span class="how">{HOW[result.how]}{result.how === 'rule' ? ` (${RULES[open.rule]})` : ''}</span>
			</div>
		{/if}

		<ul class="bars">
			{#each open.options as option, i (i)}
				<li class:lead={i === leading && (voters > 0 || result)}>
					<span class="name">{option}</span>
					<span class="bar">
						<span class="fill" style:width="{(counts[i] / scale) * 100}%"></span>
						{#if open.rule === 'first'}<span class="goal" title="n = {open.n}"></span>{/if}
					</span>
					<span class="n">{counts[i]}</span>
				</li>
			{/each}
		</ul>
		<p class="caption">
			{voters} voted{open.rule === 'groups' ? ' · the tally has totals only, not per group' : ''}
		</p>

		{#if !result}
			<div class="actions">
				{#if overriding}
					<span class="label">Override with</span>
					{#each open.options as option, i (i)}
						<Button variant="secondary" size="sm" onclick={() => relay.close(i)}>{option}</Button>
					{/each}
					<Button variant="clear" size="sm" onclick={() => (overriding = false)}>Cancel</Button>
				{:else}
					<Button variant="secondary" size="sm" onclick={() => (overriding = true)}>Override …</Button>
					<Button variant="primary" size="sm" onclick={() => relay.close()}>Close now</Button>
				{/if}
			</div>
			<p class="caption note">Hold is not in the protocol (no message) – only Close now and Override.</p>
		{/if}
	{:else}
		<p class="caption">No vote open.</p>
	{/if}
</section>

<style>
	.conductor {
		--fg: var(--paper);
		--muted: #a0a0a0;
		--rule: #6b6b6b;
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		box-sizing: border-box;
		padding: var(--space-15);
		background: var(--ink);
		color: var(--fg);
		font-family: var(--font-mono);
		border: var(--border-width-regular) solid var(--ink);
	}
	header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: var(--space-1);
	}
	.label,
	dt {
		font-size: var(--type-caption-size, 0.75rem);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label, 0.08em);
		color: var(--muted);
	}
	.state {
		font-size: var(--type-caption-size, 0.75rem);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label, 0.08em);
		padding: var(--space-025) var(--space-05);
		background: var(--badge-published);
		color: var(--badge-published-text);
	}
	.state.closed {
		background: var(--paper);
		color: var(--ink);
	}
	.question {
		margin: 0;
		font-size: var(--type-title-size, 1.5rem);
		font-weight: var(--font-weight-regular);
		line-height: 1.2;
	}
	.meta {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(9rem, 1fr));
		gap: var(--space-05) var(--space-1);
		margin: 0;
	}
	dd {
		margin: 0;
	}
	.countdown {
		display: flex;
		align-items: center;
		gap: var(--space-1);
	}
	.seconds {
		font-size: var(--type-title-size, 1.5rem);
		min-width: 3.5rem;
	}
	.seconds small {
		margin-left: var(--space-025);
		font-size: 1rem;
		color: var(--muted);
	}
	.track,
	.bar {
		position: relative;
		flex: 1;
		height: 0.5rem;
		background: #262626;
	}
	.track .fill,
	.bar .fill {
		position: absolute;
		inset: 0 auto 0 0;
		background: var(--fg);
		transition: width 0.25s linear;
	}
	.result {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: var(--space-05) var(--space-1);
		padding: var(--space-05) 0 var(--space-05) var(--space-1);
		border-left: 0.5rem solid var(--badge-published);
	}
	.winner {
		font-size: var(--type-title-size, 1.5rem);
	}
	.how {
		color: var(--muted);
	}
	.bars {
		display: flex;
		flex-direction: column;
		gap: var(--space-075);
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.bars li {
		display: grid;
		grid-template-columns: minmax(4rem, 8rem) 1fr 3rem;
		align-items: center;
		gap: var(--space-1);
		color: var(--muted);
	}
	.bars li .bar {
		height: 1.5rem;
	}
	.bars li .fill {
		background: var(--rule);
	}
	.bars li.lead {
		color: var(--fg);
	}
	.bars li.lead .fill {
		background: var(--fg);
	}
	.goal {
		position: absolute;
		inset: -0.25rem 0 -0.25rem auto;
		border-right: var(--border-width-regular) dashed var(--badge-published);
	}
	.n {
		text-align: right;
	}
	.caption {
		margin: 0;
		font-size: var(--type-caption-size, 0.75rem);
		color: var(--muted);
	}
	.note {
		border-left: 0.5rem solid var(--badge-warning);
		padding-left: var(--space-05);
	}
	.actions {
		display: flex;
		flex-wrap: wrap;
		justify-content: flex-end;
		align-items: center;
		gap: var(--space-05);
	}
	/* the ui Button on ink: primary = paper, secondary = an ink field with a grey frame → paper on hover */
	.actions :global(.sc-button.primary) {
		background: var(--paper);
		color: var(--ink);
		border-color: var(--paper);
	}
	.actions :global(.sc-button.secondary) {
		background: var(--ink);
		color: var(--paper);
		border-color: var(--rule);
	}
	.actions :global(.sc-button.secondary:hover) {
		border-color: var(--paper);
	}
	.actions :global(.sc-button.clear) {
		color: var(--paper);
	}
</style>
