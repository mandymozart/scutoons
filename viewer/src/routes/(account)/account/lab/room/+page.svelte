<script lang="ts">
	// Dev-only prototype of a place's 3D room (agents/research/LOCATION-SCENES.md §7, issue #74): the cathedral from the
	// owner's Unreal levels (tools/rooms; ?room=<id>), pickups, a resident, the quality levels and what each costs. Nothing is
	// saved.
	import { onDestroy } from 'svelte';
	import { asset } from '$lib/utils/cdn.ts';
	import { mountRoom, type RoomStage, type RoomStats, type RoomThing } from './room-stage.js';

	const isDev = import.meta.env.DEV;
	const levels = ['low', 'medium', 'high'] as const;
	let level = $state<(typeof levels)[number]>('medium');
	let canvas = $state<HTMLCanvasElement>();
	let stats = $state<RoomStats | null>(null);
	let notice = $state('');
	let error = $state('');
	let taken = $state<string[]>([]);
	let room: RoomStage | null = null;

	const pope = {
		model: asset('characters/catpope/catpope-regular.glb'),
		height: 1.1,
		label: 'Cat Pope'
	};
	const cucumber = { icon: asset('catpope/items/cucumber.webp'), height: 0.25, label: 'Gurke' };
	const record = {
		model: asset('catpope/items/awakening_vinyl.v2.set.glb'),
		label: 'Awakening LP'
	};
	/** the lab's rooms (tools/rooms/rooms/<id>.json) and what stands at their spots */
	const rooms: Record<
		string,
		{
			label: string;
			things: (RoomThing & { label: string })[];
			/** a splat room (its view + spots: `<id>.splat.json` next to the file) */
			ext?: 'splat';
		}
	> = {
		cathedral: {
			label: 'Kathedrale',
			things: [
				{ id: 'record', spot: 'altar', kind: 'pickup', ...record },
				{ id: 'cucumber', spot: 'pew', kind: 'pickup', ...cucumber },
				{ id: 'catpope', spot: 'aisle', kind: 'resident', ...pope }
			]
		},
		'garden-hut': {
			label: 'Gartenhütte',
			ext: 'splat',
			things: [
				{ id: 'cucumber', spot: 'grass', kind: 'pickup', ...cucumber },
				{ id: 'catpope', spot: 'step', kind: 'resident', ...pope }
			]
		},
		'library-blend': {
			label: 'Bibliothek (Blender)',
			things: [
				{ id: 'record', spot: 'desk', kind: 'pickup', ...record },
				{ id: 'catpope', spot: 'floor', kind: 'resident', ...pope }
			]
		},
		library: {
			label: 'Bibliothek (Splat)',
			ext: 'splat',
			things: [
				{ id: 'record', spot: 'desk', kind: 'pickup', ...record },
				{ id: 'catpope', spot: 'floor', kind: 'resident', ...pope }
			]
		},
		station: {
			label: 'Bahnhof',
			things: [
				{ id: 'cucumber', spot: 'platform', kind: 'pickup', ...cucumber },
				{ id: 'catpope', spot: 'door', kind: 'resident', ...pope }
			]
		}
	};
	let which = $state(
		(typeof location === 'undefined' ? null : new URLSearchParams(location.search).get('room')) ??
			'cathedral'
	);
	const things = $derived(rooms[which]?.things ?? []);

	async function load(id: string, l: string) {
		room?.dispose();
		room = null;
		stats = null;
		error = '';
		taken = [];
		if (!canvas) return;
		try {
			room = await mountRoom(canvas, {
				url: asset(`scutoons/rooms/${id}.${l}.${rooms[id]?.ext ?? 'glb'}`),
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
		if (canvas) load(which, level);
	});
	onDestroy(() => room?.dispose());
</script>

{#if isDev}
	<div class="lab">
		<canvas bind:this={canvas} onpointerdown={() => room?.allowTilt()} data-testid="room-canvas"
		></canvas>
		<div class="bar">
			<div class="sc-seg" role="group" aria-label="Raum">
				{#each Object.entries(rooms) as [id, r] (id)}
					<button
						class:active={which === id}
						aria-pressed={which === id}
						onclick={() => (which = id)}
						data-testid="room-{id}">{r.label}</button
					>
				{/each}
			</div>
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
		flex-wrap: wrap;
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
