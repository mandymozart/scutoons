<script lang="ts">
	// Dev-only prototype of a place's 3D room (agents/research/LOCATION-SCENES.md §7, issue #74): the cathedral from the
	// owner's Unreal level (tools/rooms), a pickup, a resident, the quality levels and what each costs. Nothing is saved.
	import { onDestroy } from 'svelte';
	import { asset } from '$lib/utils/cdn.ts';
	import { mountRoom, type RoomStage, type RoomStats } from './room-stage.js';

	const isDev = import.meta.env.DEV;
	const levels = ['low', 'medium', 'high'] as const;
	let level = $state<(typeof levels)[number]>('medium');
	let canvas = $state<HTMLCanvasElement>();
	let stats = $state<RoomStats | null>(null);
	let notice = $state('');
	let error = $state('');
	let taken = $state<string[]>([]);
	let room: RoomStage | null = null;

	const things = [
		{
			id: 'record',
			spot: 'altar',
			kind: 'pickup' as const,
			model: asset('catpope/items/awakening_vinyl.v2.set.glb'),
			label: 'Awakening LP'
		},
		{
			id: 'cucumber',
			spot: 'pew',
			kind: 'pickup' as const,
			icon: asset('catpope/items/cucumber.webp'),
			height: 0.25,
			label: 'Gurke'
		},
		{
			id: 'catpope',
			spot: 'aisle',
			kind: 'resident' as const,
			model: asset('characters/catpope/catpope-regular.glb'),
			height: 1.1,
			label: 'Cat Pope'
		}
	];

	async function load(l: string) {
		room?.dispose();
		room = null;
		stats = null;
		error = '';
		taken = [];
		if (!canvas) return;
		try {
			room = await mountRoom(canvas, {
				url: asset(`scutoons/rooms/cathedral.${l}.glb`),
				things,
				onstats: (s) => (stats = s),
				ontap: (hit) => {
					const t = things.find((x) => x.id === hit.thing);
					if (t?.kind === 'pickup' && !taken.includes(t.id)) {
						taken = [...taken, t.id];
						room?.take(t.id);
						notice = `${t.label} ist jetzt in deiner Tasche.`;
					} else if (t) notice = `${t.label} wohnt hier.`;
					else notice = `Punkt im Raum: ${hit.point.join(', ')}`;
				}
			});
			(window as unknown as { __room?: unknown }).__room = room;
		} catch (e) {
			error = String(e);
		}
	}
	$effect(() => {
		if (canvas) load(level);
	});
	onDestroy(() => room?.dispose());
</script>

{#if isDev}
	<div class="lab">
		<canvas bind:this={canvas} onpointerdown={() => room?.allowTilt()} data-testid="room-canvas"
		></canvas>
		<div class="bar">
			<span class="where">Kathedrale</span>
			<div class="sc-seg" role="group" aria-label="Qualität">
				{#each levels as l (l)}
					<button
						class:active={level === l}
						aria-pressed={level === l}
						onclick={() => (level = l)}
						data-testid="room-level-{l}">{l}</button
					>
				{/each}
			</div>
		</div>
		{#if notice}<p class="notice" data-testid="room-notice">{notice}</p>{/if}
		{#if error}<p class="notice sc-error">{error}</p>{/if}
		{#if stats}
			<pre class="stats" data-testid="room-stats">{JSON.stringify(stats, null, 1)}</pre>
		{/if}
	</div>
{/if}

<style>
	.lab {
		position: fixed;
		inset: 0;
		background: #000;
		z-index: 50;
	}
	canvas {
		width: 100%;
		height: 100%;
		display: block;
		touch-action: none;
	}
	.bar {
		position: absolute;
		inset: auto 1rem 1rem 1rem;
		display: flex;
		gap: 1rem;
		align-items: center;
		justify-content: space-between;
		color: #fff;
		pointer-events: none; /* taps go through to the room, only the buttons take them */
	}
	.bar .sc-seg {
		pointer-events: auto;
	}
	.notice {
		position: absolute;
		left: 1rem;
		right: 1rem;
		bottom: 4.5rem;
		margin: 0;
		color: #fff;
		text-shadow: 0 0 0.5rem #000;
	}
	.stats {
		position: absolute;
		top: 1rem;
		right: 1rem;
		margin: 0;
		padding: 0.5rem;
		font-size: 0.7rem;
		line-height: 1.2;
		color: #9f9;
		background: rgb(0 0 0 / 0.6);
		pointer-events: none;
	}
</style>
