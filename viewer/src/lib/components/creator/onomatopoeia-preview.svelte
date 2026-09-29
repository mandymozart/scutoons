<script lang="ts">
	import { Onomatopoeia, type ActiveOnomatopoeia } from '@scutoons/reader';
	import { TriggerMode } from '@scutoons/content';
	import { asset } from '$lib/utils/cdn.ts';

	/** The reader's onomatopoeia component on a slice-sized stage (replay to see effect + delay again). */
	let { params, background = '', toon }: { params: any; background?: string; toon: string } = $props();

	let run = $state(0);

	const active = $derived<ActiveOnomatopoeia>({
		...($state.snapshot(params) as any),
		id: `preview-${run}`,
		wait: false,
		mode: params?.mode ?? TriggerMode.POLY,
		onComplete: () => {}
	});
</script>

<div class="preview">
	<div class="stage" style:background-image={background ? `url("${asset(`${toon}/${background}`)}")` : null}>
		{#key JSON.stringify(params) + run}
			<Onomatopoeia onomatopoeia={active} />
		{/key}
	</div>
	<button type="button" class="secondary" onclick={() => run++}>↻ replay</button>
</div>

<style>
	.preview {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
		align-items: flex-start;
	}
	.stage {
		position: relative;
		width: 195px;
		aspect-ratio: 390 / 625;
		overflow: hidden;
		background-color: var(--surface-panel);
		background-size: cover;
		border: 1px solid var(--border-subtle);
		/* the reader sizes onomatopoeia in rem; show them at the reader's base size */
		font-size: 16px;
	}
</style>
