<script lang="ts">
	// A band's lineage lines (#125; VARIANTS-CANVAS §3.2): the segments storyboard-board-layout.ts computed – a comb above
	// each row with a tick into every result, a drop from the parent – as ONE path in an SVG under the cards. The viewBox
	// is the band's body in rem (1 unit = 1rem), the stroke 2px ink whatever the scale (non-scaling-stroke). Decorative:
	// the tooltips name each card's parents.
	let { lines, width, height }: { lines: [number, number, number, number][]; width: number; height: number } = $props();
	const d = $derived(lines.map(([x1, y1, x2, y2]) => `M${x1} ${y1}L${x2} ${y2}`).join(''));
</script>

<svg class="lines" viewBox="0 0 {width} {height}" style:width="{width}rem" style:height="{height}rem" aria-hidden="true"><path {d} /></svg>

<style>
	.lines {
		position: absolute;
		left: 0;
		top: 0;
		overflow: visible;
		pointer-events: none;
	}
	path {
		fill: none;
		stroke: var(--ink);
		stroke-width: var(--border-width-regular);
		stroke-linecap: square;
		vector-effect: non-scaling-stroke;
	}
</style>
