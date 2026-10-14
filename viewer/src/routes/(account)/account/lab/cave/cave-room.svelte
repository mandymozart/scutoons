<script lang="ts">
	// Draft (research/CAVE.md D3–D6): a full-screen view (owner 2026-10-14: reused by live mode – built, it moves to
	// @scutoons/reader like the ScoreView). A home scene (`cave` / `tatami_room` – research/CAVE.md "Terms"), furniture placed freely (drag it; select
	// one in the inventory, tap the room), furniture never leaves the room; racks are containers (select, then tap the
	// rack); every home has a workbench. As a visitor: read-only, the workbench works with your own items. Nothing is saved.
	import { asset } from '$lib/utils/cdn.ts';
	import CavePanel from './cave-panel.svelte';
	import Glyph from './glyph.svelte';
	import RoomBackdrop from './room-backdrop.svelte';
	import { depth, furn, type Placed, type Thing, thing } from './cave-things.ts';

	let { visitor, where, onleave }: { visitor: boolean; where: string; onleave: () => void } =
		$props();

	let scene = $state<'cave' | 'tatami_room'>('cave');
	let placed = $state<Placed[]>([
		{ f: furn('workbench'), x: 28, y: 90, contents: [] },
		{
			f: furn('record_shelf'),
			x: 18,
			y: 50,
			contents: [thing('awakening_vinyl', 'Awakening (vinyl)', 'vinyl')]
		},
		{
			f: furn('cd_rack'),
			x: 70,
			y: 64,
			contents: [thing('cd', 'Awakening (CD)', 'cd'), thing('cd', 'Like A Baby (CD)', 'cd')]
		}
	]);
	let carried = $state<Thing[]>([
		thing('cd', 'Cloud 9 (CD)', 'cd'),
		thing('tape', 'Mixtape (cassette)', 'tape'),
		thing('awakening_test_press', 'Awakening (test press)', 'vinyl'),
		thing('songbook_diy_broken', 'Broken song book', 'broken_book'),
		furn('cassette_rack'),
		furn('sofa')
	]);
	let selected = $state<number | null>(null);
	let open = $state<number | null>(null);
	let note = $state('');
	let room: HTMLDivElement;
	const sel = $derived(carried.find((c) => c.uid === selected) ?? null);

	/** a pointer position in % of the room, kept inside it */
	function at(e: PointerEvent | MouseEvent) {
		const r = room.getBoundingClientRect();
		const clamp = (v: number) => Math.min(95, Math.max(5, v));
		return {
			x: clamp(((e.clientX - r.left) / r.width) * 100),
			y: clamp(((e.clientY - r.top) / r.height) * 100)
		};
	}

	/** tap on the room itself: a selected furniture item is placed there (D4) – it stays in the room from now on */
	function tapRoom(e: MouseEvent) {
		if (visitor || sel?.kind !== 'furniture' || (e.target as Element).closest('.furniture')) return;
		placed.push({ f: sel, ...at(e), contents: [] });
		carried = carried.filter((c) => c.uid !== sel.uid);
		open = placed[placed.length - 1].f.uid;
		selected = null;
		note = '';
	}

	// drag = free placement; a press without movement = a tap (open it, or put the selected item in)
	let drag: { p: Placed; moved: boolean } | null = null;
	function down(e: PointerEvent, p: Placed) {
		if (visitor) return;
		drag = { p, moved: false };
		(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
	}
	function move(e: PointerEvent) {
		if (!drag) return;
		const pos = at(e);
		if (!drag.moved && Math.hypot(pos.x - drag.p.x, pos.y - drag.p.y) < 2) return;
		drag.moved = true;
		Object.assign(drag.p, pos);
	}
	function up(p: Placed) {
		const moved = drag?.moved;
		drag = null;
		if (moved) return;
		note = '';
		if (sel && p.f.holds && !visitor) return putIn(p, sel);
		open = p.f.uid;
	}
	function putIn(p: Placed, t: Thing) {
		const holds = p.f.holds!;
		if (!holds.kinds.includes(t.kind))
			return void (note = `${t.name} doesn't fit – the ${p.f.name} takes ${holds.label}.`);
		if (p.contents.length >= holds.max) return void (note = `The ${p.f.name} is full.`);
		p.contents.push(t);
		carried = carried.filter((c) => c.uid !== t.uid);
		selected = null;
		open = p.f.uid;
	}
</script>

<div
	class="cave"
	class:visitor
	role="dialog"
	aria-modal="true"
	aria-label={where}
	data-testid="cave-view"
>
	<header>
		<span data-testid="cave-where">{where}</span>
		{#if !visitor}
			<!-- draft only: compare the home scenes (built: set per avatar in the character definition – CAVE.md D3) -->
			<span class="seg" role="group" aria-label="Home scene">
				<button aria-pressed={scene === 'cave'} onclick={() => (scene = 'cave')}>Cave</button>
				<button
					aria-pressed={scene === 'tatami_room'}
					onclick={() => (scene = 'tatami_room')}
					data-testid="home-scene-tatami">Tatami room</button
				>
			</span>
		{/if}
		<button onclick={onleave} data-testid="cave-leave"
			>{visitor ? 'Leave the visit' : 'Leave home'}</button
		>
	</header>
	<div class="body">
		<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions (draft: placing by pointer) -->
		<div
			class="room"
			class:placing={sel?.kind === 'furniture'}
			bind:this={room}
			onclick={tapRoom}
			data-testid="cave-room"
		>
			<RoomBackdrop {scene} />
			<img class="avatar" src={asset('catpope/avatar.webp')} alt="Your avatar" />
			{#each placed as p (p.f.uid)}
				<button
					class="furniture"
					class:open={open === p.f.uid}
					class:target={!!sel && !visitor && !!p.f.holds?.kinds.includes(sel.kind)}
					style:left="{p.x}%"
					style:width="{p.f.size * depth(p.y)}%"
					style:z-index={Math.round(p.y)}
					style:top="{p.y}%"
					title={visitor ? p.f.name : `${p.f.name} – drag to move`}
					onpointerdown={(e) => down(e, p)}
					onpointermove={move}
					onpointerup={() => up(p)}
					data-testid="cave-furniture"
				>
					<Glyph kind={p.f.item} count={p.contents.length} />
				</button>
			{/each}
		</div>
		<CavePanel
			{visitor}
			{note}
			open={placed.find((p) => p.f.uid === open) ?? null}
			bind:carried
			bind:selected
			ontakeout={(p, t) => {
				p.contents = p.contents.filter((c) => c.uid !== t.uid);
				carried.push(t);
			}}
			onnote={(n) => (note = n)}
		/>
	</div>
</div>

<style>
	/* full screen above the app (like the ScoreView): a bar, then the room and the panel */
	.cave {
		position: fixed;
		inset: 0;
		z-index: 2080; /* above the app, like the ScoreView */
		display: grid;
		grid-template-rows: auto minmax(0, 1fr);
		background: var(--paper);
		color: var(--ink);
	}
	header {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem 1rem;
		padding: 0.5rem 1rem;
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	header button {
		min-height: var(--control-height);
		padding: 0 0.75rem;
		border: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
		color: var(--ink);
		font: inherit;
		cursor: pointer;
	}
	/* grouped = square, joined borders */
	.seg {
		display: inline-flex;
	}
	.seg button + button {
		margin-left: calc(-1 * var(--border-width-regular));
	}
	.seg button[aria-pressed='true'] {
		background: var(--ink);
		color: var(--paper);
	}
	/* phone: the room on top, the panel scrolls below; desktop: the room fills the height, the panel at the side */
	.body {
		display: grid;
		grid-template-rows: auto minmax(0, 1fr);
		min-height: 0;
	}
	/* the 2.5D diorama (MAP.md §5.1 three-quarter view) */
	.room {
		position: relative;
		aspect-ratio: 1;
		margin: 0.5rem auto;
		width: min(100% - 1rem, 55vh);
		align-self: center;
		touch-action: none;
	}
	.room.placing {
		cursor: crosshair;
	}
	@media (min-width: 64rem) {
		.body {
			grid-template-rows: none;
			grid-template-columns: minmax(0, 1fr) 22rem;
		}
		.body .room {
			width: min(100% - 2rem, 100vh - 6rem);
		}
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
	/* the point is where it stands (its foot), its width = its size × the depth (cave-things.ts) */
	.furniture {
		position: absolute;
		translate: -50% -90%;
		aspect-ratio: 1;
		padding: 0;
		border: 0;
		background: none;
		color: var(--ink);
		cursor: grab;
		touch-action: none;
	}
	.visitor .furniture {
		cursor: pointer;
	}
	.furniture.open,
	.furniture.target {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: 0.25rem;
	}
	.furniture.target {
		outline-style: dashed;
	}
</style>
