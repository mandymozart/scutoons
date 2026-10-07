<script lang="ts">
	import { Panel, useNodesInitialized, useStore, useSvelteFlow, useViewport } from '@xyflow/svelte';
	import { Icon } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';

	/** zoom / fit as the creator's round icon buttons (instead of Svelte Flow's small square controls) */
	let { focus, entry, onzoom }: { focus?: string; entry?: string; onzoom?: (zoom: number) => void } = $props();
	const flow = useSvelteFlow();
	const store = useStore();
	const viewport = useViewport();
	$effect(() => onzoom?.(viewport.current.zoom));

	/** below this zoom the boxes' caption type is too small to read: "Show everything" stops there (the minimap shows the rest) */
	const READABLE = 0.5;

	// the first view never shrinks the text: 1:1 around what's open (`focus`: the current box, centred) or else the
	// entry (the chapter's first group, the season's first chapter, the first toon), near the top below the buttons; "Show everything" fits the graph down to READABLE
	const ready = useNodesInitialized();
	let started = $state(false);
	$effect(() => {
		if (!ready.current || started) return;
		started = true;
		const nodes = flow.getNodes();
		if (!nodes.length) return;
		const box = (n: (typeof nodes)[number]) => ({ w: n.measured?.width ?? n.width ?? 0, h: n.measured?.height ?? n.height ?? 0 });
		const current = focus ? nodes.find((n) => n.id === focus) : undefined;
		if (current) {
			const s = box(current);
			return void flow.setCenter(current.position.x + s.w / 2, current.position.y + s.h / 2, { zoom: 1 });
		}
		const top = nodes.find((n) => n.id === entry) ?? [...nodes].sort((a, b) => a.position.y - b.position.y || a.position.x - b.position.x)[0];
		const rem = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
		const s = box(top);
		// the entry's top edge 4rem below the canvas' top (the zoom buttons sit there)
		void flow.setCenter(top.position.x + s.w / 2, top.position.y - 4 * rem + store.height / 2, { zoom: 1 });
	});
	// later: another box becomes the current one (a new selection, the graph updated in place) – the zoom stays, the
	// view only pans to it when it is out of sight (owner 2026-10-06, BACKLOG M.6)
	// svelte-ignore state_referenced_locally
	let shown = focus;
	$effect(() => {
		const id = focus;
		if (!started || !ready.current || id === shown) return;
		shown = id;
		const n = id ? flow.getNode(id) : undefined;
		if (!n) return;
		const w = n.measured?.width ?? n.width ?? 0;
		const h = n.measured?.height ?? n.height ?? 0;
		const { x, y, zoom } = viewport.current;
		const left = n.position.x * zoom + x;
		const top = n.position.y * zoom + y;
		const inside = left >= 0 && top >= 0 && left + w * zoom <= store.width && top + h * zoom <= store.height;
		if (!inside) void flow.setCenter(n.position.x + w / 2, n.position.y + h / 2, { zoom, duration: 300 });
	});
	const fit = () => flow.fitView({ padding: 0.1, minZoom: READABLE, maxZoom: 1 });
</script>

<Panel position="top-left">
	<!-- a toolbar = grouped = square (owner 2026-10-07, DESIGN.md "Round vs square"): one segmented frame -->
	<div class="controls sc-seg">
		<button type="button" title={m['creator.flow_zoom_in']()} aria-label={m['creator.flow_zoom_in']()} onclick={() => flow.zoomIn()} data-testid="flow-zoom-in"><Icon name="zoom-in" /></button>
		<button type="button" title={m['creator.flow_zoom_out']()} aria-label={m['creator.flow_zoom_out']()} onclick={() => flow.zoomOut()} data-testid="flow-zoom-out"><Icon name="zoom-out" /></button>
		<button type="button" title={m['creator.flow_fit']()} aria-label={m['creator.flow_fit']()} onclick={fit} data-testid="flow-fit"><Icon name="maximize" /></button>
	</div>
</Panel>

<style>
	.controls > button {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: var(--control-height);
		padding: 0;
	}
</style>
