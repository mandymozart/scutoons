<script lang="ts">
	// A running generation on the variants board (#125; VARIANTS-CANVAS §3.3 "Running": "a placeholder where the result
	// will land – the house progress bar and the estimate"): the card's box at the end of its main input's row, the
	// progress over the house bar (queued = the empty frame) and the estimate in euro; the caption the model and the job's
	// state. Its record replaces it when the job ends (the Generate panel polls the jobs). Not a card: no status, no
	// actions, the keys skip it. Cancel and failures stay in the Generate panel's Jobs list.
	import { TOP, type BoardNode } from '$lib/editor/storyboard-board-layout.ts';

	let { node, caption, cost, title, captions }: { node: BoardNode; caption: [string, string]; cost: string; title: string; captions: boolean } = $props();
	const pct = $derived(node.job?.progress != null ? Math.round(node.job.progress * 100) : null);
</script>

<div class="run" data-key={node.key} data-testid="running" role="group" aria-label="{caption.join(', ')}{pct != null ? `, ${pct} %` : ''}" {title} style:left="{node.x}rem" style:top="{node.y + TOP}rem" style:width="{node.w}rem">
	<span class="pic" style:width="{node.pw}rem" style:height="{node.h}rem">
		<span class="pct">{pct != null ? `${pct} %` : caption[1]}</span>
		{#if cost}<span class="cost">{cost}</span>{/if}
		<span class="bar" role="progressbar" aria-label={caption[0]} aria-valuemin="0" aria-valuemax="100" aria-valuenow={pct ?? undefined}><span style:width="{pct ?? 0}%"></span></span>
	</span>
	{#if captions}<span class="m"><span>{caption[0]}</span><span class="muted">{caption[1]}</span></span>{/if}
</div>

<style>
	.run {
		position: absolute;
		font-size: var(--type-caption-size);
		color: var(--ink);
	}
	.pic {
		position: relative;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: var(--space-025);
		box-sizing: border-box;
		border: var(--border-width-regular) dashed var(--ink);
		background: var(--paper);
		overflow: hidden;
	}
	.pct {
		font-variant-numeric: tabular-nums;
	}
	.cost {
		color: var(--text-muted);
		white-space: nowrap;
	}
	.bar {
		position: absolute;
		left: var(--space-025);
		right: var(--space-025);
		bottom: var(--space-025);
		height: 0.75rem;
		box-sizing: border-box;
		border: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
	}
	.bar > span {
		display: block;
		height: 100%;
		background: var(--ink);
	}
	.m {
		display: block;
		margin-top: 0.25rem;
		line-height: 1.25;
	}
	.m > span {
		display: block;
		padding: 0 0.125rem;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.muted {
		color: var(--text-muted);
	}
</style>
