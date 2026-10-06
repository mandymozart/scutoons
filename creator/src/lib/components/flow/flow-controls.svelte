<script lang="ts">
	import { Panel, useNodesInitialized, useSvelteFlow } from '@xyflow/svelte';
	import { Icon } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';

	/** zoom / fit as the creator's round icon buttons (instead of Svelte Flow's small square controls) */
	const flow = useSvelteFlow();

	// the first view: the chapter's start, readable (at most 1:1) – a long chapter isn't shrunk to a strip of dots;
	// "Show everything" fits the whole graph
	const ready = useNodesInitialized();
	let started = false;
	$effect(() => {
		if (!ready.current || started) return;
		started = true;
		const top = [...flow.getNodes()].sort((a, b) => a.position.y - b.position.y || a.position.x - b.position.x).slice(0, 6);
		flow.fitView({ nodes: top, padding: 0.1, maxZoom: 1 });
	});
</script>

<Panel position="top-left">
	<div class="controls">
		<button type="button" class="sc-round" title={m['creator.flow_zoom_in']()} aria-label={m['creator.flow_zoom_in']()} onclick={() => flow.zoomIn()} data-testid="flow-zoom-in"><Icon name="zoom-in" /></button>
		<button type="button" class="sc-round" title={m['creator.flow_zoom_out']()} aria-label={m['creator.flow_zoom_out']()} onclick={() => flow.zoomOut()} data-testid="flow-zoom-out"><Icon name="zoom-out" /></button>
		<button type="button" class="sc-round" title={m['creator.flow_fit']()} aria-label={m['creator.flow_fit']()} onclick={() => flow.fitView({ padding: 0.1 })} data-testid="flow-fit"><Icon name="maximize" /></button>
	</div>
</Panel>

<style>
	.controls {
		display: flex;
		gap: var(--space-025);
	}
	.controls .sc-round {
		background: var(--paper);
	}
	.controls .sc-round:hover,
	.controls .sc-round:focus-visible {
		background: var(--ink);
	}
</style>
