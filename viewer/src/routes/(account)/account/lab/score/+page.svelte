<script lang="ts">
	// Dev-only draft of the score view (research/SCORES.md §4, FEATURES 10.97): a MusicXML file from the local CDN
	// (`media/scutoons/scores/`, a dev copy) in the shared ScoreView – before scores exist as catalog items.
	import { ScoreView } from '@scutoons/reader';
	import { asset } from '$lib/utils/cdn.ts';

	const isDev = import.meta.env.DEV;
	const SCORES = [
		{ id: 'cloud9-en', title: 'Cloud 9' },
		{ id: 'likeababy-en', title: 'Like A Baby' }
	];
	let open = $state<(typeof SCORES)[number] | null>(SCORES[0]);
</script>

{#if isDev}
	<div class="view lab">
		<h1>Score view (draft)</h1>
		{#each SCORES as score (score.id)}
			<button class="btn-secondary" onclick={() => (open = score)} data-testid="score-open">Open "{score.title}"</button>
		{/each}
	</div>
	{#if open}
		{#key open.id}
			<ScoreView
				src={asset(`scutoons/scores/${open.id}.musicxml`)}
				pdf={asset(`scutoons/scores/${open.id}.pdf`)}
				title={open.title}
				onclose={() => (open = null)}
			/>
		{/key}
	{/if}
{/if}

<style>
	.lab {
		padding: var(--space-1);
	}
</style>
