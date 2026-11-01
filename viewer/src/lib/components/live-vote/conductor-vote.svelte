<script lang="ts">
	// The conductor's vote panel (#231, research/LIVE.md B.2 / B.6): the question, the rule, the fallback, the countdown
	// (`vote.tally.left`), live bars with counts and percentages, for the rule "per group" each group's own split and
	// pick, who has voted of how many devices, Hold (the countdown stops, votes still count) / Close now / Override, then
	// the result and how it was decided (`vote.result.how`). Ink on paper (owner 2026-11-01); a dark mode for the whole
	// Live section comes later. The source is the real relay (live-vote-link.ts) or the draft's fake (live-vote-sim.ts).
	import { Button } from '@scutoons/ui';
	import type { VoteOpen, VoteResult, VoteTally } from '@scutoons/live';
	import { percent, pickText, type VoteSource } from '$lib/live-vote.ts';
	import { winner } from '$lib/live-vote-sim.ts';

	let { source, groupNames = {} }: { source: VoteSource; groupNames?: Record<string, string> } = $props();

	let open = $state<VoteOpen | null>(null);
	let tally = $state<VoteTally | null>(null);
	let result = $state<VoteResult | null>(null);
	let overriding = $state(false);
	/** the countdown does not tick between tallies: the tally says where it stands */
	let held = $state(false);

	$effect(() =>
		source.on('conductor', (e) => {
			if (e.kind === 'vote.open') [open, tally, result, overriding, held] = [e.p, null, null, false, false];
			else if (e.kind === 'vote.tally') [tally, held] = [e.p, e.p.paused === true];
			else if (e.kind === 'vote.result') [result, overriding, held] = [e.p, false, false];
		})
	);

	const labels = $derived(open?.options ?? []);
	const question = $derived(open ? pickText(open.question, 'en') : '');
	const counts = $derived(result?.counts ?? tally?.counts ?? labels.map(() => 0));
	const voters = $derived(counts.reduce((a, b) => a + b, 0));
	const leftMs = $derived(result ? 0 : (tally?.left ?? (open ? open.seconds * 1000 : 0)));
	const leading = $derived(result ? result.option : counts.indexOf(Math.max(...counts)));
	/** first to n: the bar is the way to n, else the share of the voters */
	const scale = $derived(open?.rule === 'first' ? Math.max(open.n ?? 1, ...counts) : Math.max(1, voters));
	const RULES = { majority: 'majority', groups: 'per group', first: 'first to n' } as const;
	const RULE_HELP = {
		majority: 'The option with the most votes wins.',
		groups: "Each group's own majority counts as one vote; the option with the most groups wins.",
		first: 'The first option to reach n votes wins at once.'
	} as const;
	const groupsText = $derived(open?.groups ? open.groups.map((g) => groupNames[g] ?? g).join(', ') : 'all audience');
	const HOW = { rule: 'by the rule', fallback: 'by the fallback – nobody voted', override: 'by override – the conductor chose' } as const;
	const byGroup = $derived(result?.groups ?? tally?.groups ?? {});
	/** per group: the group's own pick (its majority; a tie with the fallback goes to the fallback) and the groups' tally */
	const groupRows = $derived(
		open?.rule === 'groups'
			? Object.entries(byGroup).map(([g, c]) => ({ g, c, total: c.reduce((a, b) => a + b, 0), pick: winner(c, open!.fallback ?? 0) }))
			: []
	);
	const groupVotes = $derived(labels.map((_, i) => groupRows.filter((r) => r.pick === i).length));
</script>

<section class="conductor" aria-label="Vote (conductor)">
	{#if open}
		<header>
			<span class="label">Vote · {open.vote}</span>
			<span class="state" class:closed={result} class:held={held && !result}>{result ? 'Closed' : held ? 'Held' : 'Open'}</span>
		</header>
		{#if question}<h2 class="question">{question}</h2>{/if}
		<dl class="meta">
			<div><dt>Rule</dt><dd title={RULE_HELP[open.rule]}>{RULES[open.rule]}{open.rule === 'first' ? ` · n ${open.n ?? 1}` : ''}</dd></div>
			<div><dt>Fallback</dt><dd>{labels[open.fallback ?? 0]}</dd></div>
			<div><dt>Voting</dt><dd>{groupsText}</dd></div>
		</dl>
		<p class="caption">{RULE_HELP[open.rule]} Closes when the timer is up or every connected device has voted.</p>

		{#if !result}
			<div class="countdown">
				<span class="seconds">{Math.ceil(leftMs / 1000)}<small>s</small></span>
				<span class="track"><span class="fill" style:width="{(leftMs / (open.seconds * 1000)) * 100}%"></span></span>
			</div>
		{:else}
			<div class="result">
				<span class="label">Result</span>
				<span class="winner">{labels[result.option]}</span>
				<span class="how">{HOW[result.how]}{result.how === 'rule' ? ` (${RULES[open.rule]})` : ''}</span>
			</div>
		{/if}

		<h3 class="label">{open.rule === 'groups' ? 'All votes' : 'Votes'} · {voters}{tally?.expected && !result ? ` of ${tally.expected} devices` : ''}</h3>
		<ul class="bars">
			{#each labels as option, i (i)}
				<li class:lead={i === leading && voters > 0}>
					<span class="name">{option}</span>
					<span class="bar">
						<span class="fill" style:width="{(counts[i] / scale) * 100}%"></span>
						{#if open.rule === 'first'}<span class="goal" title="n = {open.n}"></span>{/if}
					</span>
					<span class="n">{counts[i]}<small>{percent(counts[i], counts)}%</small></span>
				</li>
			{/each}
		</ul>

		{#if open.rule === 'groups'}
			<h3 class="label">Per group · {groupRows.length} groups, one vote each</h3>
			<ul class="groups">
				{#each groupRows as r (r.g)}
					<li>
						<span class="name">{groupNames[r.g] ?? r.g}</span>
						<span class="pair">
							{#each labels as option, i (i)}
								<span class="cell" class:pick={r.pick === i}>{option} {percent(r.c[i], r.c)}%<small>{r.c[i]}</small></span>
							{/each}
						</span>
					</li>
				{/each}
				<li class="sum">
					<span class="name">Group votes</span>
					<span class="pair">
						{#each labels as option, i (i)}<span class="cell">{option} {groupVotes[i]}</span>{/each}
					</span>
				</li>
			</ul>
		{/if}

		{#if !result}
			<div class="actions">
				{#if overriding}
					<span class="label">Override with</span>
					{#each labels as option, i (i)}
						<Button variant="secondary" size="sm" onclick={() => source.close(i)}>{option}</Button>
					{/each}
					<Button variant="clear" size="sm" onclick={() => (overriding = false)}>Cancel</Button>
				{:else}
					<Button variant="secondary" size="sm" onclick={() => source.hold(!held)}>{held ? 'Run on' : 'Hold'}</Button>
					<Button variant="secondary" size="sm" onclick={() => (overriding = true)}>Override …</Button>
					<Button variant="primary" size="sm" onclick={() => source.close()}>Close now</Button>
				{/if}
			</div>
			<p class="caption">Hold stops the countdown; votes still count. A held vote does not close when everyone has voted.</p>
		{/if}
	{:else}
		<p class="caption">No vote open.</p>
	{/if}
</section>

<style>
	.conductor {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		box-sizing: border-box;
		padding: var(--space-15);
		background: var(--paper);
		color: var(--ink);
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
		margin: 0;
		font-size: var(--type-caption-size);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	.state {
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		padding: var(--space-025) var(--space-05);
		background: var(--badge-published);
		color: var(--badge-published-text);
	}
	.state.held {
		background: var(--badge-warning);
		color: var(--badge-warning-text);
	}
	.state.closed {
		background: var(--ink);
		color: var(--paper);
	}
	.question {
		margin: 0;
		font-size: var(--type-title-size);
		font-weight: var(--font-weight-regular);
		line-height: var(--type-title-line);
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
	.caption {
		margin: 0;
		font-size: var(--type-caption-size);
		color: var(--text-muted);
	}
	.countdown {
		display: flex;
		align-items: center;
		gap: var(--space-1);
	}
	.seconds {
		font-size: var(--type-title-size);
		min-width: 3.5rem;
	}
	.seconds small {
		margin-left: var(--space-025);
		font-size: 1rem;
		color: var(--text-muted);
	}
	.track,
	.bar {
		position: relative;
		flex: 1;
		height: 0.5rem;
		border: var(--border-width-hairline) solid var(--ink);
		box-sizing: content-box;
	}
	.track .fill,
	.bar .fill {
		position: absolute;
		inset: 0 auto 0 0;
		background: var(--ink);
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
		font-size: var(--type-title-size);
	}
	.how {
		color: var(--text-muted);
	}
	.bars,
	.groups {
		display: flex;
		flex-direction: column;
		gap: var(--space-075);
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.bars li {
		display: grid;
		grid-template-columns: minmax(4rem, 8rem) 1fr 6rem;
		align-items: center;
		gap: var(--space-1);
		color: var(--text-muted);
	}
	.bars li .bar {
		height: 1.5rem;
	}
	.bars li .fill {
		background: var(--border-muted);
	}
	.bars li.lead {
		color: var(--ink);
	}
	.bars li.lead .fill {
		background: var(--ink);
	}
	.goal {
		position: absolute;
		inset: -0.25rem 0 -0.25rem auto;
		border-right: var(--border-width-regular) dashed var(--badge-published);
	}
	.n {
		text-align: right;
	}
	.n small {
		margin-left: var(--space-05);
		font-size: var(--type-caption-size);
		color: var(--text-muted);
	}
	.groups li {
		display: grid;
		grid-template-columns: minmax(4rem, 8rem) 1fr;
		align-items: center;
		gap: var(--space-1);
	}
	.pair {
		display: flex;
		gap: var(--space-05);
	}
	.cell {
		flex: 1;
		padding: var(--space-025) var(--space-05);
		border: var(--border-width-hairline) solid var(--border-muted);
		color: var(--text-muted);
	}
	.cell.pick {
		border: var(--border-width-regular) solid var(--ink);
		background: var(--ink);
		color: var(--paper);
	}
	.cell small {
		margin-left: var(--space-05);
		font-size: var(--type-caption-size);
		opacity: 0.7;
	}
	.sum {
		padding-top: var(--space-05);
		border-top: var(--border-width-hairline) solid var(--ink);
	}
	.sum .cell {
		border-color: transparent;
		color: var(--ink);
	}
	.actions {
		display: flex;
		flex-wrap: wrap;
		justify-content: flex-end;
		align-items: center;
		gap: var(--space-05);
	}
</style>
