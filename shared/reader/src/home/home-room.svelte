<script lang="ts">
	import { furnitureDepth } from '@scutoons/content';
	import { fill, type HomeLabels, type HomePiece, type HomeSceneFiles } from './home-types.ts';

	/**
	 * The room of the home view (research/CAVE.md D3, D4): the home scene (a still, a video loop over it unless the
	 * reader prefers reduced motion), the avatar, the furniture at its foot point – smaller further back, what stands
	 * further front covers what stands behind. Drag a piece to move it (free placement); a press without moving is a
	 * tap (`ontap`); a tap on the room itself reports the point (`onroom`, to place the selected furniture).
	 */
	let {
		scene,
		avatar = null,
		furniture,
		open = null,
		targets = [],
		editable,
		placing,
		labels,
		ontap,
		onroom,
		onmove
	}: {
		scene: HomeSceneFiles;
		/** the avatar's A-pose URL */
		avatar?: string | null;
		furniture: HomePiece[];
		/** the opened piece's id */
		open?: string | null;
		/** pieces the selected thing fits into (drawn dashed) */
		targets?: string[];
		/** the reader may move things (their own home) */
		editable: boolean;
		/** a furniture item is selected: a tap on the room places it */
		placing: boolean;
		labels: HomeLabels;
		ontap: (piece: HomePiece) => void;
		onroom: (x: number, y: number) => void;
		onmove: (piece: HomePiece, x: number, y: number) => void;
	} = $props();

	let room: HTMLDivElement;
	let motion = $state(true);
	let videoFailed = $state(false);
	$effect(() => {
		const query = matchMedia('(prefers-reduced-motion: reduce)');
		motion = !query.matches;
		const change = () => (motion = !query.matches);
		query.addEventListener('change', change);
		return () => query.removeEventListener('change', change);
	});

	/** a pointer position in % of the room, kept inside it */
	function at(e: PointerEvent | MouseEvent) {
		const r = room.getBoundingClientRect();
		const clamp = (v: number) => Math.round(Math.min(98, Math.max(2, v)) * 100) / 100;
		return {
			x: clamp(((e.clientX - r.left) / r.width) * 100),
			y: clamp(((e.clientY - r.top) / r.height) * 100)
		};
	}

	// a drag moves the piece live; it is reported once, on release
	let drag: { piece: HomePiece; moved: boolean; x: number; y: number } | null = $state(null);
	function down(e: PointerEvent, piece: HomePiece) {
		if (!editable) return;
		drag = { piece, moved: false, x: piece.x, y: piece.y };
		(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
	}
	function move(e: PointerEvent) {
		if (!drag) return;
		const pos = at(e);
		if (!drag.moved && Math.hypot(pos.x - drag.piece.x, pos.y - drag.piece.y) < 2) return;
		drag.moved = true;
		drag.x = pos.x;
		drag.y = pos.y;
	}
	function up(piece: HomePiece) {
		const d = drag;
		drag = null;
		if (d?.moved) onmove(piece, d.x, d.y);
		else ontap(piece);
	}
	const where = (p: HomePiece) =>
		drag?.piece.id === p.id && drag.moved ? { x: drag.x, y: drag.y } : { x: p.x, y: p.y };

	function tapRoom(e: MouseEvent) {
		if (!placing || (e.target as Element).closest('.piece')) return;
		const { x, y } = at(e);
		onroom(x, y);
	}
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions (placing is by pointer; the furniture are buttons) -->
<div
	class="room"
	class:placing
	bind:this={room}
	onclick={tapRoom}
	aria-label={labels.room}
	data-testid="home-room"
>
	{#if scene.image}
		<img class="scene" src={scene.image} alt="" />
	{:else}
		<svg class="scene" viewBox="0 0 100 100" aria-hidden="true">
			<polygon class="wall" points="0,30 50,8 50,58 0,80" />
			<polygon class="wall" points="50,8 100,30 100,80 50,58" />
			<polygon class="floor" points="0,80 50,58 100,80 50,102" />
		</svg>
	{/if}
	{#if scene.video && motion && !videoFailed}
		<!-- muted decoration: no captions needed -->
		<video
			class="scene"
			src={scene.video}
			poster={scene.image || undefined}
			autoplay
			muted
			loop
			playsinline
			onerror={() => (videoFailed = true)}
		></video>
	{/if}
	{#if avatar}<img class="avatar" src={avatar} alt="" />{/if}
	{#each furniture as piece (piece.id)}
		{@const p = where(piece)}
		<button
			class="piece"
			class:open={open === piece.id}
			class:target={targets.includes(piece.id)}
			class:flip={piece.flip}
			style:left="{p.x}%"
			style:top="{p.y}%"
			style:width="{piece.size * furnitureDepth(p.y)}%"
			style:z-index={Math.round(p.y)}
			title={editable ? fill(labels.dragHint, { name: piece.name }) : piece.name}
			aria-label={piece.name}
			onpointerdown={(e) => down(e, piece)}
			onpointermove={move}
			onpointerup={() => up(piece)}
			data-testid="home-piece"
		>
			<img src={piece.image} alt="" draggable="false" />
		</button>
	{/each}
</div>

<style>
	/* the 2.5D diorama (MAP.md §5.1 three-quarter view) – square, as large as the space allows */
	.room {
		position: relative;
		aspect-ratio: 1;
		width: 100%;
		touch-action: none;
		overflow: hidden;
	}
	.room.placing {
		cursor: crosshair;
	}
	.scene {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		object-fit: cover;
		pointer-events: none;
	}
	svg.scene polygon {
		stroke: var(--ink);
		stroke-width: 0.5;
		stroke-linejoin: round;
	}
	.wall {
		fill: var(--paper);
	}
	.floor {
		fill: color-mix(in srgb, var(--ink) 8%, transparent);
	}
	.avatar {
		position: absolute;
		left: 50%;
		top: 50%;
		width: 20%;
		translate: -50% 0;
		z-index: 70; /* its feet: in front of what stands behind it */
		pointer-events: none;
	}
	/* the point is the foot; the width = size × depth */
	.piece {
		position: absolute;
		translate: -50% -100%;
		padding: 0;
		border: 0;
		background: none;
		cursor: grab;
		touch-action: none;
	}
	.piece img {
		display: block;
		width: 100%;
		height: auto;
		pointer-events: none;
	}
	.piece.flip img {
		scale: -1 1;
	}
	.piece.open,
	.piece.target {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: 0.25rem;
	}
	.piece.target {
		outline-style: dashed;
	}
</style>
