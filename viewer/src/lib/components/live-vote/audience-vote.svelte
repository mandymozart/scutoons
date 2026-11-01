<script lang="ts" module>
	export interface Group {
		id: string;
		name: string;
		colour: string;
		/** ink or paper – whichever reads on the colour */
		text: string;
	}
</script>

<script lang="ts">
	// The audience phone's vote (#231, research/LIVE.md B.9): in the group's colour, under the group's current instruction
	// (the last `say`), the question and the options in the device's language, the chosen option inverted and changeable
	// until the close, then the result. The live counts show only AFTER this phone has voted (owner 2026-11-01) – the relay
	// sends the tally to a voter only then, and only when the vote shows it. The countdown shows only while the vote is
	// open (it runs on the phone's clock from `seconds`, with the relay's `left` once the tally comes). The source is the
	// real relay (live-vote-link.ts) or the draft's fake (live-vote-sim.ts).
	import type { VoteOpen, VoteResult, VoteTally } from '@scutoons/live';
	import { howKey, percent, pickLabels, pickText, type VoteSource } from '$lib/live-vote.ts';
	import { wordsFor } from '$lib/live-vote-words.ts';

	let { source, group, locale, client = 'me', instruction = '' }: { source: VoteSource; group: Group; locale: string; client?: string; instruction?: string } = $props();

	let open = $state<VoteOpen | null>(null);
	let tally = $state<VoteTally | null>(null);
	let result = $state<VoteResult | null>(null);
	let mine = $state<number | null>(null);
	let say = $state('');
	let endsAt = $state(0);
	let now = $state(Date.now());
	const t = $derived(wordsFor(locale));

	$effect(() =>
		source.on(
			'audience',
			(e) => {
				if (e.kind === 'vote.open') {
					[open, tally, result, mine] = [e.p, null, null, null];
					endsAt = Date.now() + e.p.seconds * 1000;
				} else if (e.kind === 'vote.tally') tally = e.p;
				else if (e.kind === 'vote.result') result = e.p;
				else if (e.kind === 'say') {
					const text = (e.p as { text?: string | Record<string, string> }).text;
					say = typeof text === 'string' ? text : pickText(text, locale);
				}
			},
			group.id,
			client
		)
	);
	$effect(() => {
		const id = setInterval(() => (now = Date.now()), 250);
		return () => clearInterval(id);
	});

	const labels = $derived(open ? pickLabels(open, locale) : []);
	const question = $derived(open ? pickText(open.question, locale) : '');
	const left = $derived(Math.max(0, Math.ceil((tally ? tally.left : endsAt - now) / 1000)));
	/** counts only after this phone voted */
	const seesCounts = $derived(mine !== null && tally !== null);

	function choose(i: number) {
		if (!open || result) return;
		mine = i;
		source.cast(client, i, group.id);
	}
</script>

<section class="phone" style:--group={group.colour} style:--on-group={group.text} aria-label="Vote">
	<header>
		<span class="group">{group.name}</span>
		{#if open && !result}<span class="left">{tally?.paused ? t.paused : t.left(left)}</span>{/if}
	</header>
	{#if say || instruction}<p class="instruction">{say || instruction}</p>{/if}

	{#if result && open}
		<div class="card">
			<span class="label">{t.result}</span>
			<span class="winner">{labels[result.option]}</span>
			<span class="how">{t.how[howKey(open, result)]}</span>
			{#if mine !== null}
				<span class="mine">{t.yours}: {labels[mine]}{mine === result.option ? ' ✓' : ''}</span>
			{/if}
		</div>
	{:else if open}
		<div class="card">
			<span class="label">{t.vote}</span>
			{#if question}<h2 class="question">{question}</h2>{/if}
			<div class="options" role="group" aria-label={question || t.vote}>
				{#each labels as option, i (i)}
					<button type="button" class="option" class:chosen={mine === i} aria-pressed={mine === i} onclick={() => choose(i)}>
						<span>{option}</span>
						{#if seesCounts && tally}<span class="n">{percent(tally.counts[i], tally.counts)}%</span>{/if}
						{#if seesCounts && tally}<span class="share" style:width="{percent(tally.counts[i], tally.counts)}%"></span>{/if}
					</button>
				{/each}
			</div>
			<p class="hint">{mine === null ? t.pick : t.change}</p>
		</div>
	{:else}
		<p class="hint waiting">{t.waiting}</p>
	{/if}
</section>

<style>
	.phone {
		display: flex;
		flex-direction: column;
		gap: var(--space-15);
		box-sizing: border-box;
		flex: 1;
		min-height: 100%;
		align-items: center;
		padding: var(--space-15) var(--space-1);
		background: var(--group);
		color: var(--on-group);
		font-family: var(--font-mono);
	}
	/* on a wide screen the phone's column stays readable, the group's colour fills the rest */
	.phone > :global(*) {
		box-sizing: border-box;
		width: min(100%, 36rem);
	}
	header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: var(--space-1);
	}
	.group,
	.label {
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	.group {
		padding: var(--space-025) var(--space-05);
		border: var(--border-width-regular) solid currentColor;
	}
	.left {
		font-variant-numeric: tabular-nums;
	}
	.instruction {
		margin: 0;
		font-size: var(--type-title-size);
		line-height: var(--type-title-line);
	}
	.card {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		padding: var(--space-1);
		background: var(--paper);
		color: var(--ink);
		border: var(--border-width-regular) solid var(--ink);
	}
	.question {
		margin: 0;
		font-size: 1rem;
		font-weight: var(--font-weight-regular);
	}
	.options {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
	}
	.option {
		position: relative;
		display: flex;
		justify-content: space-between;
		align-items: center;
		box-sizing: border-box;
		min-height: var(--control-height);
		padding: 0 var(--space-1);
		overflow: hidden;
		background: var(--paper);
		color: var(--ink);
		border: var(--border-width-regular) solid var(--border-muted);
		border-radius: var(--radius-none);
		font: inherit;
		font-size: var(--type-button-size);
		line-height: var(--type-button-line);
		cursor: pointer;
	}
	.option:hover {
		border-color: var(--ink);
	}
	.option.chosen {
		background: var(--ink);
		color: var(--paper);
		border-color: var(--ink);
	}
	.option .share {
		position: absolute;
		inset: auto auto 0 0;
		height: 0.25rem;
		background: var(--group);
	}
	.hint,
	.how {
		margin: 0;
		font-size: var(--type-caption-size);
		color: var(--text-muted);
	}
	.waiting {
		color: inherit;
	}
	.winner {
		font-size: var(--type-title-size);
	}
	.mine {
		padding-left: var(--space-05);
		border-left: 0.5rem solid var(--group);
	}
</style>
