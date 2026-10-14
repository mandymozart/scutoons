<script lang="ts">
	// Draft placeholders for the home scenes (research/CAVE.md "Terms", D3 – the owner's art comes later): the cave and an empty
	// Japanese monk's room with tatami, both in the map places' three-quarter view (MAP.md §5.1). viewBox 0 0 100 100;
	// the floor is the rhombus left (0,80) – back (50,58) – right (100,80) – front (50,102).
	let { scene }: { scene: 'cave' | 'tatami_room' } = $props();

	/** a point on the floor: u from left→back edge … right, v from left→front edge (both 0 … 1) */
	const floor = (u: number, v: number) => `${u * 50 + v * 50},${80 - u * 22 + v * 22}`;
	/** a point on the left wall (u along the wall to the corner, h height 0 … 1) / the right wall */
	const left = (u: number, h: number) => `${u * 50},${80 - u * 22 - h * 50}`;
	const right = (u: number, h: number) => `${50 + u * 50},${58 + u * 22 - h * 50}`;
	const thirds = [1 / 3, 2 / 3];
	const quarters = [0.25, 0.5, 0.75];
</script>

<svg viewBox="0 0 100 100" aria-hidden="true" data-testid="room-backdrop" data-scene={scene}>
	{#if scene === 'tatami_room'}
		<polygon class="wall" points="0,30 50,8 50,58 0,80" />
		<polygon class="wall" points="50,8 100,30 100,80 50,58" />
		<!-- shoji: a lattice on both walls -->
		{#each quarters as u (u)}
			<polyline class="thin" points="{left(u, 0)} {left(u, 1)}" />
			<polyline class="thin" points="{right(u, 0)} {right(u, 1)}" />
		{/each}
		{#each [0.35, 0.7] as h (h)}
			<polyline class="thin" points="{left(0, h)} {left(1, h)} {right(1, h)}" />
		{/each}
		<polygon class="mat" points="{floor(0, 0)} {floor(1, 0)} {floor(1, 1)} {floor(0, 1)}" />
		<!-- tatami: two rows of mats, the middle seam offset (a 4½-mat feel) -->
		<polyline class="seam" points="{floor(0.5, 0)} {floor(0.5, 1)}" />
		{#each thirds as v (v)}
			<polyline class="seam" points="{floor(0, v)} {floor(0.5, v)}" />
		{/each}
		<polyline class="seam" points="{floor(0.5, 0.5)} {floor(1, 0.5)}" />
	{:else}
		<!-- the cave: a rock vault, a worn floor, a few stalactites -->
		<path class="rock" d="M0 100 L0 42 Q2 14 24 8 Q38 1 52 5 Q66 1 80 9 Q98 16 100 44 L100 100 Z" />
		<path class="crack" d="M14 30 Q20 40 16 52 M84 28 Q78 38 86 50 M50 12 Q46 24 52 34" />
		{#each [[30, 9, 6], [44, 5, 9], [62, 5, 7], [74, 8, 5]] as [x, y, h] (x)}
			<polygon class="rock" points="{x - 2},{y} {x + 2},{y} {x},{y + h}" />
		{/each}
		<polygon class="ground" points="{floor(0, 0)} {floor(1, 0)} {floor(1, 1)} {floor(0, 1)}" />
	{/if}
</svg>

<style>
	svg {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		overflow: visible;
	}
	polygon,
	path,
	polyline {
		stroke: var(--ink);
		stroke-width: 0.5;
		stroke-linejoin: round;
		fill: none;
	}
	.wall,
	.rock {
		fill: var(--paper);
	}
	.mat {
		fill: rgb(150 160 90 / 0.18);
	}
	.ground {
		fill: rgb(0 0 0 / 0.08);
	}
	.thin,
	.seam,
	.crack {
		stroke-width: 0.3;
	}
</style>
