<script lang="ts">
	// The Nodes tab, read-only (RIG-EDITING.md §2.2, #282): the graph of what the deck plays – its tracks' nodes, the sum,
	// the outlet, the analyser and the being – as boxes with their ports and the links as cables, the live value on each
	// cable. The graph is the core's wiring (the app hands it on from the engine host after every rig turn); `read` gives
	// a cable's value from the engine's analysis frame (NaN: none). Here only placement (nodes.ts) and drawing; the values
	// are read ten times a second while the tab shows. Patching (add a node, drag a link) is #203.
	import { untrack } from 'svelte';
	import { cableTexts, layoutGraph, sameTexts, type GraphWiring } from '../nodes.ts';
	import { m } from '../messages.ts';

	let {
		wiring = { nodes: [], links: [] },
		read,
		active = true
	}: {
		/** none yet: the deck has not played anything */
		wiring?: GraphWiring;
		read: (path: string) => number;
		/** the tab shows: read the values (a hidden tab reads nothing) */
		active?: boolean;
	} = $props();

	const layout = $derived(layoutGraph(wiring));
	let texts = $state<string[]>([]);

	function sample() {
		const next = cableTexts(wiring.links, read);
		if (!sameTexts(next, texts)) texts = next;
	}

	$effect(() => {
		if (!active) return;
		untrack(sample);
		const timer = setInterval(sample, 100);
		return () => clearInterval(timer);
	});
</script>

<div class="nodes" data-testid="designer-nodes">
	{#if layout.nodes.length === 0}
		<p class="muted">{wiring.problem || m['designer.nodes_empty']()}</p>
	{:else}
		<svg viewBox="0 0 {layout.width} {layout.height}" style:max-width="{layout.width}rem" role="img" aria-label={m['designer.nodes_graph']()}>
			<title>{m['designer.nodes_hint']()}</title>
			<!-- the viewBox counts in rem (at full size): a text's size, a stroke's width are rem too -->
			<g font-size="0.6875">
			{#each layout.cables as c (c.index)}
				<path class="cable {c.type}" class:back={c.back} d={c.d} />
			{/each}
			{#each layout.nodes as n (n.id)}
				<g class="node" class:idle={!n.played} data-testid="designer-node">
					<title>{n.id} · {n.kind}{n.track ? ` · ${n.track}` : ''}{n.played ? '' : ` – ${m['designer.nodes_not_played']()}`}</title>
					<rect class="box" x={n.x} y={n.y} width={n.w} height={n.h} />
					<rect class="head" x={n.x} y={n.y} width={n.w} height="1.5" />
					<text class="id" x={n.x + 0.5} y={n.y + 1}>{n.id}</text>
					<text class="kind" x={n.x + n.w - 0.5} y={n.y + 1}>{n.world ? n.world.source : n.kind}</text>
					{#if n.world}
						<text class="port-name" x={n.x + 0.5} y={n.y + n.h - 0.25} data-testid="designer-world-filter">{n.world.listen.join(' ')}</text>
					{/if}
					{#each n.inputs as p (p.name)}
						<circle class="port {p.type}" class:wired={p.wired} cx={p.x} cy={p.y} r="0.1875" />
						<text class="port-name" x={p.x + 0.5} y={p.y + 0.25}>{p.name}</text>
					{/each}
					{#each n.outputs as p (p.name)}
						<circle class="port {p.type}" class:wired={p.wired} cx={p.x} cy={p.y} r="0.1875" />
						<text class="port-name out" x={p.x - 0.5} y={p.y + 0.25}>{p.name}</text>
					{/each}
				</g>
			{/each}
			{#each layout.cables as c (c.index)}
				<text class="value" x={c.labelX} y={c.labelY + 0.25} data-testid="designer-cable-value">{texts[c.index] ?? '–'}</text>
			{/each}
			</g>
		</svg>
	{/if}
</div>

<style>
	.nodes {
		height: 100%;
		overflow: auto;
		padding: var(--space-1);
		color: var(--tool-text);
		font-family: var(--font-sans);
	}
	.muted {
		margin: 0 0 var(--space-05);
		color: var(--tool-muted);
	}
	/* the whole rig at once: at most its own size, smaller when the half is narrower (the viewBox keeps its shape) */
	svg {
		display: block;
		width: 100%;
		height: auto;
	}
	/* cables and pins keep their signal's colour and shape (RULES H11: the deck's one signal table) */
	.cable {
		fill: none;
		stroke: var(--signal-audio);
		stroke-width: 0.0625;
	}
	.cable.control {
		stroke: var(--signal-control);
		stroke-dasharray: 0.25 0.1875;
	}
	.cable.video {
		stroke: var(--signal-picture);
		stroke-width: 0.125;
	}
	.cable.event {
		stroke: var(--signal-event);
		stroke-width: 0.125;
		stroke-dasharray: 0.5 0.1875;
	}
	.cable.back {
		stroke-dasharray: 0.0625 0.1875;
	}
	/* a node: the control ground (see-through), its line; its head the text colour at the head's strength */
	.box {
		fill: var(--tool-control);
		stroke: var(--tool-line);
		stroke-width: 0.0625;
	}
	.head {
		fill: var(--tool-head);
	}
	.id {
		fill: var(--tool-text);
	}
	.kind {
		fill: var(--tool-muted);
		text-anchor: end;
	}
	.idle {
		opacity: 0.45;
	}
	.port {
		fill: var(--tool-ground);
		stroke: var(--signal-audio);
		stroke-width: 0.0625;
	}
	.port.wired {
		fill: var(--signal-audio);
	}
	.port.control {
		stroke: var(--signal-control);
	}
	.port.control.wired {
		fill: var(--signal-control);
	}
	.port.video {
		stroke: var(--signal-picture);
	}
	.port.video.wired {
		fill: var(--signal-picture);
	}
	.port.event {
		stroke: var(--signal-event);
	}
	.port.event.wired {
		fill: var(--signal-event);
	}
	.port-name {
		fill: var(--tool-muted);
	}
	.port-name.out {
		text-anchor: end;
	}
	/* the live value on a cable: the mono face, ringed with the stage so it reads over the line */
	.value {
		fill: var(--tool-text);
		text-anchor: middle;
		font-family: var(--font-mono);
		font-variant-numeric: tabular-nums;
		stroke: var(--tool-ground);
		stroke-width: 0.25;
		paint-order: stroke;
	}
</style>
