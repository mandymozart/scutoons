<script lang="ts">
	// Draft (research/CAVE.md D1, D2): the reader's avatar walks along revealed routes to unlocked places; at home it
	// enters for free, elsewhere "Walk home" or the VR headset. Fake places / routes – the real map comes later.
	import { asset } from '$lib/utils/cdn.ts';

	let { vr, onenter }: { vr: boolean; onenter: (how: 'there' | 'vr') => void } = $props();

	type Place = { id: string; name: string; x: number; y: number; image: string; unlocked: boolean };
	const PLACES: Place[] = [
		{ id: 'home', name: 'Home', x: 18, y: 72, image: 'apartment-blocks', unlocked: true },
		{ id: 'square', name: 'Church square', x: 42, y: 52, image: 'church-square', unlocked: true },
		{ id: 'cathedral', name: 'Cathedral', x: 70, y: 30, image: 'cathedral', unlocked: true },
		{ id: 'station', name: 'Station', x: 78, y: 74, image: 'station', unlocked: true },
		{ id: 'crypt', name: 'Crypt', x: 44, y: 18, image: 'crypt', unlocked: false }
	];
	/** revealed routes (both ways); the crypt's route is not revealed yet */
	const ROUTES: [string, string][] = [
		['home', 'square'],
		['square', 'cathedral'],
		['square', 'station']
	];
	const byId = (id: string) => PLACES.find((p) => p.id === id)!;

	let at = $state('cathedral');
	let selected = $state<string | null>('home');
	let walking = $state(false);
	let pos = $state({ x: byId('cathedral').x, y: byId('cathedral').y });

	/** shortest path over revealed routes (BFS) – null when there is none */
	function path(from: string, to: string): string[] | null {
		const prev = new Map<string, string>([[from, from]]);
		const queue = [from];
		while (queue.length) {
			const cur = queue.shift()!;
			if (cur === to) break;
			for (const [a, b] of ROUTES) {
				const next = a === cur ? b : b === cur ? a : null;
				if (next && !prev.has(next)) {
					prev.set(next, cur);
					queue.push(next);
				}
			}
		}
		if (!prev.has(to)) return null;
		const out = [to];
		while (out[0] !== from) out.unshift(prev.get(out[0])!);
		return out;
	}

	async function walk(to: string) {
		const steps = path(at, to);
		if (!steps) return;
		walking = true;
		const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;
		for (const id of steps.slice(1)) {
			pos = { x: byId(id).x, y: byId(id).y };
			if (!reduced) await new Promise((r) => setTimeout(r, 1200));
			at = id;
		}
		walking = false;
	}

	const sel = $derived(selected ? byId(selected) : null);
	const canWalk = $derived(!!sel && sel.id !== at && !!path(at, sel.id));
</script>

<div class="map" data-testid="cave-map">
	<svg class="routes" viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true">
		{#each ROUTES as [a, b] (a + b)}
			<line x1={byId(a).x} y1={byId(a).y} x2={byId(b).x} y2={byId(b).y} />
		{/each}
	</svg>
	{#each PLACES as place (place.id)}
		<button
			class="place"
			class:locked={!place.unlocked}
			class:selected={selected === place.id}
			style:left="{place.x}%"
			style:top="{place.y}%"
			disabled={!place.unlocked}
			title={place.unlocked ? place.name : 'Not discovered yet'}
			onclick={() => (selected = place.id)}
			data-testid="cave-map-place"
		>
			<img src={asset(`catpope/map/places/${place.image}.webp`)} alt="" />
			{#if place.unlocked}<span class="label">{place.name}</span>{/if}
		</button>
	{/each}
	<img class="avatar" class:walking src={asset('catpope/avatar.webp')} alt="Your avatar" style:left="{pos.x}%" style:top="{pos.y}%" />
</div>

{#if sel}
	<div class="card" data-testid="cave-map-card">
		<strong>{sel.name}</strong>
		{#if sel.id === 'home'}
			{#if at === 'home'}
				<span class="muted">You are home.</span>
				<button class="primary" onclick={() => onenter('there')} data-testid="cave-enter">Enter</button>
			{:else}
				{#if vr}<button onclick={() => onenter('vr')} data-testid="cave-enter-vr">Enter with the VR headset</button>{/if}
				<button class="primary" disabled={!canWalk || walking} onclick={() => walk('home')} data-testid="cave-walk-home">
					{walking ? 'Walking …' : 'Walk home'}
				</button>
			{/if}
		{:else if sel.id === at}
			<span class="muted">Your avatar is here.</span>
		{:else}
			<button class="primary" disabled={!canWalk || walking} onclick={() => walk(sel.id)}>{walking ? 'Walking …' : 'Walk here'}</button>
		{/if}
	</div>
{/if}

<style>
	.map {
		position: relative;
		aspect-ratio: 4 / 3;
		width: min(100%, 40rem, calc(60vh * 4 / 3));
		margin-inline: auto;
		background: var(--paper);
		border: var(--border-width-regular) solid var(--ink);
		overflow: hidden;
	}
	.routes {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
	}
	.routes line {
		stroke: var(--ink);
		stroke-width: 0.6;
		stroke-dasharray: 2 1.5;
		vector-effect: non-scaling-stroke;
	}
	.place {
		position: absolute;
		translate: -50% -85%;
		display: grid;
		justify-items: center;
		padding: 0;
		border: 0;
		background: none;
		color: var(--ink);
		cursor: pointer;
	}
	.place img {
		width: 4.5rem;
	}
	.place.locked {
		opacity: 0.25;
		filter: grayscale(1);
		cursor: default;
	}
	.place.selected .label {
		background: var(--ink);
		color: var(--paper);
	}
	.label {
		padding: 0 0.25rem;
		background: var(--paper);
		font-size: 0.8rem;
	}
	.avatar {
		position: absolute;
		width: 2.5rem;
		translate: -50% -110%;
		border-radius: var(--radius-full);
		border: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
		transition:
			left 1.2s linear,
			top 1.2s linear;
		pointer-events: none;
	}
	@media (prefers-reduced-motion: reduce) {
		.avatar {
			transition: none;
		}
	}
	.card {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5rem;
		margin-top: 0.5rem;
	}
	.muted {
		color: var(--muted);
	}
	.card button {
		min-height: var(--control-height);
		padding: 0 0.75rem;
		border: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
		color: var(--ink);
		font: inherit;
		cursor: pointer;
	}
	.card button.primary {
		background: var(--ink);
		color: var(--paper);
	}
	.card button:disabled {
		opacity: 0.4;
		cursor: default;
	}
</style>
