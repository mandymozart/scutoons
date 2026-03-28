<script lang="ts">
	import type { MapLocation, MapRoute } from '$lib/models/content.ts';

	let {
		width,
		height,
		routes,
		locations
	}: { width: number; height: number; routes: MapRoute[]; locations: MapLocation[] } = $props();

	function getRouteLocations(route: MapRoute): MapLocation[] {
		return route.locations
			.map((locId) => locations.find((loc) => loc.id === locId))
			.filter((loc): loc is MapLocation => loc !== undefined);
	}

	function createBezierPath(locations: MapLocation[]): string {
		if (locations.length < 2) return '';

		const first = locations[0].coordinates;
		let path = `M ${first.x} ${first.y}`;

		// Handle length factor (0-1, lower = tighter curves)
		const tension = 0.3;

		for (let i = 1; i < locations.length; i++) {
			const curr = locations[i].coordinates;
			const prev = locations[i - 1].coordinates;

			// Calculate tangent direction at previous point
			let prevTangentX, prevTangentY;
			if (i === 1) {
				// First segment: tangent points to current
				prevTangentX = curr.x - prev.x;
				prevTangentY = curr.y - prev.y;
			} else {
				// Middle segments: tangent from prev-prev to current
				const prevPrev = locations[i - 2].coordinates;
				prevTangentX = curr.x - prevPrev.x;
				prevTangentY = curr.y - prevPrev.y;
			}

			// Calculate tangent direction at current point
			let currTangentX, currTangentY;
			if (i === locations.length - 1) {
				// Last segment: tangent from previous
				currTangentX = curr.x - prev.x;
				currTangentY = curr.y - prev.y;
			} else {
				// Middle segments: tangent from previous to next
				const next = locations[i + 1].coordinates;
				currTangentX = next.x - prev.x;
				currTangentY = next.y - prev.y;
			}

			// Normalize and scale tangents
			const prevLen = Math.sqrt(prevTangentX ** 2 + prevTangentY ** 2);
			const currLen = Math.sqrt(currTangentX ** 2 + currTangentY ** 2);

			const segmentLen = Math.sqrt((curr.x - prev.x) ** 2 + (curr.y - prev.y) ** 2);

			// Control points along tangent lines
			const cx1 = prev.x + (prevTangentX / prevLen) * segmentLen * tension;
			const cy1 = prev.y + (prevTangentY / prevLen) * segmentLen * tension;
			const cx2 = curr.x - (currTangentX / currLen) * segmentLen * tension;
			const cy2 = curr.y - (currTangentY / currLen) * segmentLen * tension;

			path += ` C ${cx1} ${cy1}, ${cx2} ${cy2}, ${curr.x} ${curr.y}`;
		}

		return path;
	}
</script>

<svg {width} {height} style="position: absolute; pointer-events: none;">
	{#each routes as route}
		{@const locations = getRouteLocations(route)}
		<path
			d={createBezierPath(locations)}
			stroke="black"
			stroke-width="8"
			fill="none"
			stroke-linecap="round"
			stroke-linejoin="round"
		/>
	{/each}
</svg>
