<script lang="ts" module>
	export interface Group {
		id: string;
		name: string;
		colour: string;
		/** ink or paper – whichever reads on the colour */
		text: string;
		/** the group's current instruction (its last `say`) */
		instruction: string;
	}
</script>

<script lang="ts">
	// #231 draft: the audience phone's vote – in the group's colour, under the group's current instruction (the last
	// `say` per group, B.6 snapshot); the options as buttons, the chosen one inverted and changeable until the close
	// (`vote.cast` again), then the result. A phone gets `vote.tally` only when the vote shows it (`show`), so without it
	// the countdown runs locally from `vote.open.seconds`. Fed by the fake relay (src/lib/live-vote-sim.ts).
	import type { VoteOpen, VoteResult, VoteTally } from '@scutoons/live';
	import type { FakeVoteRelay } from '$lib/live-vote-sim.ts';

	let { relay, group, question, client = 'me' }: { relay: FakeVoteRelay; group: Group; question: string; client?: string } =
		$props();

	let open = $state<VoteOpen | null>(null);
	let tally = $state<VoteTally | null>(null);
	let result = $state<VoteResult | null>(null);
	let mine = $state<number | null>(null);
	let endsAt = $state(0);
	let now = $state(Date.now());

	$effect(() =>
		relay.on(
			'audience',
			(e) => {
				if (e.kind === 'vote.open') {
					[open, tally, result, mine] = [e.p, null, null, null];
					endsAt = Date.now() + e.p.seconds * 1000;
				} else if (e.kind === 'vote.tally') tally = e.p;
				else if (e.kind === 'vote.result') result = e.p;
			},
			group.id
		)
	);
	$effect(() => {
		const id = setInterval(() => (now = Date.now()), 250);
		return () => clearInterval(id);
	});

	const left = $derived(Math.max(0, Math.ceil((tally ? tally.left : endsAt - now) / 1000)));
	const total = $derived(tally ? Math.max(1, tally.voters) : 1);

	function choose(i: number) {
		if (!open || result) return;
		mine = i;
		relay.cast(client, i, group.id);
	}

	const HOW = {
		rule: { majority: 'Die meisten Stimmen.', groups: 'Die meisten Gruppen.', first: 'Zuerst genug Stimmen.' },
		fallback: 'Niemand hat abgestimmt – es bleibt dabei.',
		override: 'Die Regie hat entschieden.'
	} as const;
	const howText = $derived(!result || !open ? '' : result.how === 'rule' ? HOW.rule[open.rule] : HOW[result.how]);
</script>

<section class="phone" style:--group={group.colour} style:--on-group={group.text} aria-label="Vote (audience phone)">
	<header>
		<span class="group">{group.name}</span>
		{#if open && !result}<span class="left">noch {left} s</span>{/if}
	</header>
	<p class="instruction">{group.instruction}</p>

	{#if result && open}
		<div class="card">
			<span class="label">Ergebnis</span>
			<span class="winner">{open.options[result.option]}</span>
			<span class="how">{howText}</span>
			{#if mine !== null}
				<span class="mine">Deine Stimme: {open.options[mine]}{mine === result.option ? ' ✓' : ''}</span>
			{/if}
		</div>
	{:else if open}
		<div class="card">
			<span class="label">Abstimmung</span>
			<h2 class="question">{question}</h2>
			<div class="options" role="group" aria-label={question}>
				{#each open.options as option, i (i)}
					<button type="button" class="option" class:chosen={mine === i} aria-pressed={mine === i} onclick={() => choose(i)}>
						<span>{option}</span>
						{#if tally}<span class="n">{tally.counts[i]}</span>{/if}
						{#if tally}<span class="share" style:width="{(tally.counts[i] / total) * 100}%"></span>{/if}
					</button>
				{/each}
			</div>
			<p class="hint">
				{mine === null ? 'Tipp auf deine Wahl.' : 'Du kannst bis zum Schluss wechseln.'}
			</p>
		</div>
	{/if}
</section>

<style>
	.phone {
		display: flex;
		flex-direction: column;
		gap: var(--space-15);
		box-sizing: border-box;
		flex: 1;
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
	.winner {
		font-size: var(--type-title-size);
	}
	.mine {
		padding-left: var(--space-05);
		border-left: 0.5rem solid var(--group);
	}
</style>
