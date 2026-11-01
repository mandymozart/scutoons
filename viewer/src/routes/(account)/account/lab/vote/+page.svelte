<script lang="ts">
	// Dev-only drafts of the Live vote screens (#231, research/LIVE.md B.2 / B.6 / B.9): the conductor's vote panel and
	// an audience phone, both on one fake relay (src/lib/live-vote-sim.ts – the protocol's vote.open / vote.tally every
	// 250 ms / vote.result, the Rust relay's rules). The phone's taps count in the conductor's tally. Nothing is sent.
	// Query: ?view=both|conductor|audience · rule=majority|groups|first · group=balkon|parkett|rang · lang=de|en · voting=balkon (only one group votes) · crowd=0 (nobody else votes → the fallback) · seconds=N.
	import { onMount } from 'svelte';
	import type { VoteOpen, VoteRule } from '@scutoons/live';
	import { FakeVoteRelay, type CrowdGroup } from '$lib/live-vote-sim.ts';
	import { deviceLocale } from '$lib/live-vote.ts';
	import ConductorVote from '$lib/components/live-vote/conductor-vote.svelte';
	import AudienceVote, { type Group } from '$lib/components/live-vote/audience-vote.svelte';

	const isDev = import.meta.env.DEV;
	const query = typeof location === 'undefined' ? new URLSearchParams() : new URLSearchParams(location.search);

	/** the phone follows the device's language (owner 2026-11-01); `?lang=en|de` overrides it. The question and the labels
	 *  come with the vote as locale maps (VoteOpen.question / labels). */
	const LOCALE = query.get('lang') ?? deviceLocale(typeof navigator === 'undefined' ? undefined : navigator.languages);
	const GROUPS: Group[] = [
		{ id: 'balkon', name: LOCALE === 'de' ? 'Balkon' : 'Balcony', colour: 'var(--badge-published)', text: 'var(--ink)' },
		{ id: 'parkett', name: LOCALE === 'de' ? 'Parkett' : 'Stalls', colour: 'var(--badge-warning)', text: 'var(--ink)' },
		{ id: 'rang', name: LOCALE === 'de' ? 'Rang' : 'Circle', colour: 'var(--badge-soon)', text: 'var(--paper)' }
	];
	const INSTRUCTION: Record<string, Record<string, string>> = {
		de: { balkon: 'Steht auf und haltet eure Handys hoch.', parkett: 'Flüstert der Katze eure Wahl zu.', rang: 'Klatscht zweimal, dann wählt.' },
		en: { balkon: 'Stand up and hold your phones high.', parkett: 'Whisper your choice to the cat.', rang: 'Clap twice, then vote.' }
	};
	const GROUP_NAMES = Object.fromEntries(GROUPS.map((g) => [g.id, g.name]));
	const CROWD: CrowdGroup[] = [
		{ group: 'balkon', size: 18, lean: [3, 1] },
		{ group: 'parkett', size: 26, lean: [1, 2] },
		{ group: 'rang', size: 14, lean: [2, 1] }
	];

	type View = 'both' | 'conductor' | 'audience';
	let view = $state<View>((query.get('view') as View) || 'both');
	let rule = $state<VoteRule>((query.get('rule') as VoteRule) || 'groups');
	let groupId = $state(query.get('group') ?? 'balkon');
	let onlyBalkon = $state(query.get('voting') === 'balkon');
	let crowd = $state(query.get('crowd') !== '0');
	const seconds = Number(query.get('seconds')) || 30;
	const group = $derived(GROUPS.find((g) => g.id === groupId) ?? GROUPS[0]);

	/** One relay per run; the phone gets the same vote with its labels translated (the options are indices) */
	let relay = $state<FakeVoteRelay | null>(null);
	let run = 0;
	function start() {
		relay?.dispose();
		const expected = (crowd ? CROWD.filter((c) => !onlyBalkon || c.group === 'balkon').reduce((n, c) => n + c.size, 0) : 0) + (view !== 'conductor' && (!onlyBalkon || groupId === 'balkon') ? 1 : 0);
		const r = new FakeVoteRelay(undefined, crowd ? CROWD : [], 7, expected);
		relay = r;
		run++;
		const p: VoteOpen = {
			vote: `blotter-${run}`,
			options: ['Yes', 'No'],
			question: { en: 'Trade the blotter?', de: 'Tauscht die Katze das Löschblatt?' },
			labels: { en: ['Yes', 'No'], de: ['Ja', 'Nein'] },
			rule,
			n: rule === 'first' ? 20 : undefined,
			seconds,
			fallback: 0,
			groups: onlyBalkon ? ['balkon'] : undefined,
			// the phones get the tally, but a voter sees it only after voting
			show: true
		};
		r.open(p);
	}
	onMount(() => {
		start();
		return () => relay?.dispose();
	});

	const VIEWS: [View, string][] = [['both', 'Both'], ['conductor', 'Conductor'], ['audience', 'Audience']];
	const RULES: [VoteRule, string][] = [['majority', 'Majority'], ['groups', 'Per group'], ['first', 'First to 20']];
</script>

{#if isDev}
	<div class="lab" data-view={view}>
		<h1>Live vote (draft)</h1>
		<details class="controls" open={query.get('controls') === '1'}>
			<summary>Draft controls – not part of the design · fake relay, #231</summary>
			<div class="seg" role="group" aria-label="View">
				{#each VIEWS as [v, label] (v)}
					<button type="button" aria-pressed={view === v} onclick={() => (view = v)}>{label}</button>
				{/each}
			</div>
			<div class="seg" role="group" aria-label="Rule">
				{#each RULES as [r, label] (r)}
					<button type="button" aria-pressed={rule === r} onclick={() => ((rule = r), start())}>{label}</button>
				{/each}
			</div>
			<div class="seg" role="group" aria-label="The phone's group">
				{#each GROUPS as g (g.id)}
					<button type="button" aria-pressed={groupId === g.id} onclick={() => (groupId = g.id)}>{g.name}</button>
				{/each}
			</div>
			<label><input type="checkbox" bind:checked={onlyBalkon} onchange={start} /> Only Balkon votes</label>
			<label><input type="checkbox" bind:checked={crowd} onchange={start} /> Crowd votes</label>
			<button type="button" class="restart" onclick={start}>Open a new vote</button>
		</details>

		{#if relay}
			<div class="screens">
				{#if view !== 'audience'}
					<div class="screen conductor-screen">
						<span class="tag">Conductor</span>
						{#key relay}<ConductorVote source={relay} groupNames={GROUP_NAMES} />{/key}
					</div>
				{/if}
				{#if view !== 'conductor'}
					<div class="screen phone-screen">
						<span class="tag">Audience phone</span>
						<div class="device">
							{#key relay}{#key group.id}<AudienceVote source={relay} {group} locale={LOCALE} instruction={(INSTRUCTION[LOCALE] ?? INSTRUCTION.en)[group.id]} />{/key}{/key}
						</div>
					</div>
				{/if}
			</div>
		{/if}
	</div>
{/if}

<style>
	.lab {
		box-sizing: border-box;
		max-width: 80rem;
		margin: 0 auto;
		padding: 5rem var(--space-1) var(--space-3);
		font-family: var(--font-mono);
	}
	h1 {
		margin: 0 0 var(--space-1);
		font-size: var(--type-title-size);
	}
	.controls[open] {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-05) var(--space-1);
		padding-bottom: var(--space-1);
	}
	.controls {
		margin: 0 0 var(--space-15);
		padding: 0 var(--space-1);
		border: var(--border-width-regular) dashed var(--border-muted);
	}
	summary {
		flex-basis: 100%;
		display: flex;
		align-items: center;
		min-height: var(--control-height);
		cursor: pointer;
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--text-muted);
	}
	label {
		white-space: nowrap;
		display: inline-flex;
		align-items: center;
		gap: var(--space-05);
	}
	/* the viewer has no tools.css: a square segmented group (DESIGN "Round vs square"), locally like ai-connections */
	.seg {
		display: inline-flex;
		border: var(--border-width-regular) solid var(--ink);
	}
	.seg button,
	.restart {
		box-sizing: border-box;
		min-height: var(--control-height);
		padding: 0 var(--space-1);
		background: var(--paper);
		color: var(--ink);
		border: 0;
		border-radius: var(--radius-none);
		font: inherit;
		line-height: var(--type-button-line);
		cursor: pointer;
	}
	.seg button + button {
		border-left: var(--border-width-regular) solid var(--ink);
	}
	.seg button[aria-pressed='true'] {
		background: var(--ink);
		color: var(--paper);
	}
	.restart {
		border: var(--border-width-regular) solid var(--border-muted);
	}
	.restart:hover {
		border-color: var(--ink);
	}
	.screens {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-start;
		gap: var(--space-15);
	}
	.screen {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
	}
	.conductor-screen {
		flex: 2 1 28rem;
	}
	.phone-screen {
		flex: 1 1 18rem;
	}
	[data-view='audience'] .phone-screen {
		flex-basis: 100%;
	}
	.tag {
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--text-muted);
	}
	/* the phone as a device frame next to the conductor; alone it is the page */
	.device {
		display: flex;
		flex-direction: column;
		height: 44rem;
		max-width: 24rem;
		overflow: auto;
		border: var(--border-width-regular) solid var(--ink);
	}
	[data-view='audience'] .device {
		max-width: none;
		height: auto;
		min-height: 36rem;
	}
	@media (max-width: 479px) {
		.lab {
			padding-inline: 0;
		}
		h1,
		.controls,
		.tag {
			margin-inline: var(--space-1);
		}
		.device {
			max-width: none;
			height: auto;
			min-height: 36rem;
			border-inline: 0;
		}
	}
</style>
