<script lang="ts">
	import type { Point } from '@scutoons/content';
	import type { MapViewport } from '@scutoons/reader';

	/**
	 * The reader's own avatar on the map (research/CAVE.md D1 – the walking mode): a round marker with the avatar's
	 * picture where it stands; a walk moves it along the route's points at a steady pace (≈ 1.2 s per stretch), then
	 * `ondone`. Reduced motion: it jumps there.
	 */
	let {
		view,
		at,
		walk = null,
		image,
		ondone
	}: {
		view: MapViewport;
		/** where it stands (world units) */
		at: Point | null;
		/** a walk to show: its points (world units) and how long it takes (ms) */
		walk?: { points: Point[]; ms: number; carrier?: string | null } | null;
		image: string;
		ondone: () => void;
	} = $props();

	let current = $state<Point | null>(null);

	const length = (pts: Point[]) => pts.slice(1).reduce((n, p, i) => n + Math.hypot(p.x - pts[i].x, p.y - pts[i].y), 0);
	/** the point `d` world units along the polyline */
	function along(pts: Point[], d: number): Point {
		for (let i = 1; i < pts.length; i++) {
			const seg = Math.hypot(pts[i].x - pts[i - 1].x, pts[i].y - pts[i - 1].y);
			if (d <= seg) {
				const t = seg ? d / seg : 1;
				return { x: pts[i - 1].x + (pts[i].x - pts[i - 1].x) * t, y: pts[i - 1].y + (pts[i].y - pts[i - 1].y) * t };
			}
			d -= seg;
		}
		return pts[pts.length - 1];
	}

	$effect(() => {
		const w = walk;
		if (!w || w.points.length < 2 || matchMedia('(prefers-reduced-motion: reduce)').matches) {
			current = null;
			if (w) ondone();
			return;
		}
		const total = length(w.points);
		const start = performance.now();
		let frame = requestAnimationFrame(function step(now) {
			const t = Math.min(1, (now - start) / w.ms);
			current = along(w.points, total * t);
			if (t < 1) frame = requestAnimationFrame(step);
			else {
				current = null;
				ondone();
			}
		});
		return () => cancelAnimationFrame(frame);
	});

	const where = $derived(current ?? at);
	const s = $derived(where ? view.toScreen(where) : null);
</script>

{#if s && current && walk?.carrier}
	<!-- the creature airlift: its carrier carries the avatar (research/CAVE.md D8) -->
	<img class="carrier" src={walk.carrier} alt="" draggable="false" style:transform="translate({s.x}px, {s.y}px)" data-testid="map-carrier" />
{/if}
{#if s}
	<img class="avatar" class:walking={!!current} src={image} alt="" draggable="false" style:transform="translate({s.x}px, {s.y}px)" data-testid="map-avatar" />
{/if}

<style>
	/* the carrier hovers above the avatar while it flies */
	.carrier {
		position: absolute;
		left: 0;
		top: 0;
		width: 5rem;
		height: 5rem;
		margin: -6.5rem 0 0 -2.5rem;
		object-fit: contain;
		pointer-events: none;
		z-index: 4;
	}
	/* above the place markers, below the card; the point is its foot */
	.avatar {
		position: absolute;
		left: 0;
		top: 0;
		width: 2.5rem;
		height: 2.5rem;
		margin: -2.75rem 0 0 -1.25rem;
		object-fit: cover;
		object-position: top;
		border: var(--border-width-regular) solid var(--ink);
		border-radius: var(--radius-full);
		background: var(--paper);
		pointer-events: none;
		z-index: 3;
	}
</style>
