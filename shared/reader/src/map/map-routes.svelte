<script lang="ts" module>
	export interface DrawnRoute {
		id: string;
		waypoints: (import('@scutoons/content').Waypoint | import('@scutoons/content').ReaderWaypoint)[];
		style?: import('@scutoons/content').MapPathStyle;
		/** a ride line (research/CAVE.md D8): drawn in its network's look */
		network?: import('@scutoons/content').MapNetwork;
		/** draw up to this waypoint index (default: all) */
		to?: number;
		/** drawn solid up to this index (travelled; the rest of a revealed route is dashed) – reader state, M3 */
		travelled?: number;
		/** greyed (the builder's toon filter / reader view) */
		dimmed?: boolean;
		selected?: boolean;
	}
</script>

<script lang="ts">
	import { routeGeometry, routePath, routeStroke, type RoutePlace } from '@scutoons/content';

	/**
	 * The routes of the world map as one SVG over the map's viewport (agents/MAP.md §4.4 styles): the paths are in world
	 * units under the viewport's transform, strokes keep their screen width at every zoom. Places the reader can't see
	 * are skipped (`visible`), unknown ones too (catpope's old `garden`). A route runs into a place's diorama at its entry
	 * gate and out at its exit gate (owner 2026-10-12, BACKLOG 10.66 (4), variant A – `routeGeometry`): `layer="lines"`
	 * (under the places) draws the lines, `layer="gates"` (over them) the gate marks.
	 */
	let {
		routes,
		places,
		zoom,
		x,
		y,
		visible = () => true,
		onroute,
		layer = 'lines'
	}: {
		routes: DrawnRoute[];
		places: RoutePlace[];
		/** lines (under the places) or the gate marks (over them) */
		layer?: 'lines' | 'gates';
		zoom: number;
		x: number;
		y: number;
		visible?: (placeId: string) => boolean;
		/** a click on a route's line (the builder selects it) */
		onroute?: (id: string, event: MouseEvent) => void;
	} = $props();

	const shown = $derived(places.filter((p) => visible(p.id)));

	const geometry = (route: DrawnRoute) => routeGeometry(shown, route.waypoints, 0, route.to ?? Infinity);
</script>

<svg class="map-routes" aria-hidden="true">
	<g transform="translate({x} {y}) scale({zoom})">
		{#each routes as route (route.id)}
			{@const stroke = routeStroke(route.style, route.network)}
			{@const shape = geometry(route)}
			{#if layer === 'gates'}
				<g class="route" class:dimmed={route.dimmed} data-route-gates={route.id}>
					{#each shape.gates as g, gi (gi)}
						<circle class="gate" cx={g.at.x} cy={g.at.y} r={4 / zoom} stroke={stroke.color} data-gate="{g.place}:{g.which}" />
					{/each}
				</g>
			{:else if shape.segments.length}
				<g class="route" class:dimmed={route.dimmed} class:selected={route.selected} data-route={route.id}>
					{#each shape.segments as nodes, si (si)}
						{@const d = routePath(nodes, stroke.tension)}
						{#if onroute}
							<!-- a wide invisible line makes the route easy to hit in the builder -->
							<path class="hit" {d} role="presentation" onclick={(e) => onroute(route.id, e)} />
						{/if}
						{#if route.selected}
							<path class="halo" {d} style:stroke-width="{stroke.width + 8}px" />
						{/if}
						{#if stroke.outline}
							<path class="line" {d} stroke={stroke.outline} style:stroke-width="{stroke.width + 3}px" />
						{/if}
						<path class="line" {d} stroke={stroke.color} stroke-dasharray={stroke.dash || undefined} style:stroke-width="{stroke.width}px" />
					{/each}
				</g>
			{/if}
		{/each}
	</g>
</svg>

<style>
	.map-routes {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		overflow: visible;
		pointer-events: none;
	}
	path {
		fill: none;
		vector-effect: non-scaling-stroke;
		stroke-linecap: round;
		stroke-linejoin: round;
	}
	.hit {
		stroke: transparent;
		stroke-width: 16px;
		pointer-events: stroke;
		cursor: pointer;
	}
	.halo {
		stroke: var(--primary-cyan, #00d1ff);
		opacity: 0.6;
	}
	/* a gate: a small round mark where the route meets the diorama (paper inside, the route's colour around) */
	.gate {
		fill: var(--paper, #fff);
		stroke-width: 2.5px;
		vector-effect: non-scaling-stroke;
	}
	.dimmed {
		opacity: 0.25;
	}
</style>
