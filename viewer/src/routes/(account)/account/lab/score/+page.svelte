<script lang="ts">
	// Dev-only draft of the score view (research/SCORES.md §4, FEATURES 10.97): a MusicXML file from the local CDN
	// (`media/scutoons/scores/`, a dev copy) in the shared ScoreView – before scores exist as catalog items. Also the
	// lettered cover (§1) and the DIY book's ageing (§2a) at a chosen age.
	import { ageLook } from '@scutoons/content';
	import { CoverLettering, ScoreView } from '@scutoons/reader';
	import { asset } from '$lib/utils/cdn.ts';

	const isDev = import.meta.env.DEV;
	const SCORES = [
		{ id: 'cloud9-en', title: 'Cloud 9' },
		{ id: 'likeababy-en', title: 'Like A Baby' },
		{ id: 'nohesaid-en', title: 'No He Said' },
		{ id: 'notmycity-en', title: 'Not My City' },
		{ id: 'oncetwicethrice-en', title: 'Once, Twice, Thrice' },
		{ id: 'roarlittlekitty-en', title: 'Roar Little Kitty' }
	];
	const DIY_AGES = { tint_days: 14, spots_days: 30, full_days: 90 };
	let open = $state<(typeof SCORES)[number] | null>(SCORES[0]);
	/** the DIY book's age in days (0 = new) */
	let days = $state(0);
	const look = $derived(ageLook(DIY_AGES, 0, days * 86_400_000));
</script>

{#if isDev}
	<div class="view lab">
		<h1>Score view (draft)</h1>
		<p>
			<label>DIY book age: {days} days <input type="range" min="0" max="120" bind:value={days} data-testid="score-age-days" /></label>
		</p>
		<div class="covers">
			{#each SCORES as score (score.id)}
				<button class="cover" onclick={() => (open = score)} data-testid="score-open" title={score.title}>
					<CoverLettering text={score.title} />
				</button>
			{/each}
		</div>
	</div>
	{#if open}
		{#key open.id}
			<ScoreView
				src={asset(`scutoons/scores/${open.id}.musicxml`)}
				pdf={asset(`scutoons/scores/${open.id}.pdf`)}
				title={open.title}
				age={days > 0 ? { ...look, seed: `lab-${open.id}` } : null}
				onclose={() => (open = null)}
			/>
		{/key}
	{/if}
{/if}

<style>
	.lab {
		padding: var(--space-1);
	}
	.covers {
		display: flex;
		gap: var(--space-1);
	}
	/* a stand-in for the shared blank songsheet icon (the owner's art comes later) */
	.cover {
		position: relative;
		width: 8rem;
		aspect-ratio: 3 / 4;
		background: repeating-linear-gradient(transparent 0 0.9rem, rgb(0 0 0 / 0.15) 0.9rem calc(0.9rem + 1px)), var(--paper);
		border: var(--border-width-regular) solid var(--ink);
		cursor: pointer;
	}
</style>
