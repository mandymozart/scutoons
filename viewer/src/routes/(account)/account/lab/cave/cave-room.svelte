<script lang="ts">
	// Draft (research/CAVE.md D3–D6): one drawn room with fixed furniture spots; racks are containers (`holds`), the
	// workbench runs workshop recipes; select an item, then tap a rack to put it in. As a visitor: read-only, but the
	// workbench works with your own items. Local state only – nothing is saved.
	import { asset } from '$lib/utils/cdn.ts';
	import Glyph from './glyph.svelte';

	let { visitor, onleave }: { visitor: boolean; onleave: () => void } = $props();

	type Thing = { uid: number; item: string; name: string; kind: 'cd' | 'tape' | 'vinyl' | 'book' | 'broken_book' | 'furniture' };
	type Furniture = Thing & { holds?: { kinds: Thing['kind'][]; max: number; label: string }; workshop?: boolean };
	type Spot = { id: string; x: number; y: number; wall: boolean; furniture: Furniture | null; contents: Thing[] };

	let uid = 100;
	const t = (item: string, name: string, kind: Thing['kind']): Thing => ({ uid: uid++, item, name, kind });
	const FURNITURE: Record<string, Omit<Furniture, 'uid'>> = {
		cd_rack: { item: 'cd_rack', name: 'CD rack', kind: 'furniture', holds: { kinds: ['cd'], max: 12, label: 'CDs' } },
		cassette_rack: { item: 'cassette_rack', name: 'Cassette rack', kind: 'furniture', holds: { kinds: ['tape'], max: 12, label: 'cassettes' } },
		record_shelf: { item: 'record_shelf', name: 'Record shelf', kind: 'furniture', holds: { kinds: ['vinyl'], max: 20, label: 'records' } },
		workbench: { item: 'workbench', name: 'Workbench', kind: 'furniture', workshop: true }
	};
	const furn = (id: string): Furniture => ({ uid: uid++, ...FURNITURE[id] });

	// six fixed spots (D3): two against each wall, two on the floor in front
	let spots = $state<Spot[]>([
		{ id: 'wall-left-1', x: 16, y: 50, wall: true, furniture: furn('record_shelf'), contents: [t('awakening_vinyl', 'Awakening (vinyl)', 'vinyl')] },
		{ id: 'wall-left-2', x: 35, y: 42, wall: true, furniture: null, contents: [] },
		{ id: 'wall-right-1', x: 65, y: 42, wall: true, furniture: furn('cd_rack'), contents: [t('cd', 'Awakening (CD)', 'cd'), t('cd', 'Like A Baby (CD)', 'cd')] },
		{ id: 'wall-right-2', x: 84, y: 50, wall: true, furniture: null, contents: [] },
		{ id: 'floor-left', x: 24, y: 80, wall: false, furniture: furn('workbench'), contents: [] },
		{ id: 'floor-right', x: 76, y: 80, wall: false, furniture: null, contents: [] }
	]);
	// what the reader carries (the inventory): placeholders for the items to come
	let carried = $state<Thing[]>([
		t('cd', 'Cloud 9 (CD)', 'cd'),
		t('tape', 'Mixtape (cassette)', 'tape'),
		t('awakening_test_press', 'Awakening (test press)', 'vinyl'),
		t('songbook_diy_broken', 'Broken song book', 'broken_book'),
		furn('cassette_rack')
	]);

	let selected = $state<number | null>(null);
	let open = $state<string | null>(null);
	let note = $state('');
	const spot = $derived(spots.find((s) => s.id === open) ?? null);
	const sel = $derived(carried.find((c) => c.uid === selected) ?? null);

	function tapSpot(s: Spot) {
		note = '';
		if (sel && s.furniture?.holds && !visitor) return putIn(s, sel);
		if (sel?.kind === 'furniture' && !s.furniture && !visitor) return place(s, sel as Furniture);
		open = s.id;
	}
	function putIn(s: Spot, thing: Thing) {
		const holds = s.furniture!.holds!;
		if (!holds.kinds.includes(thing.kind)) return void (note = `${thing.name} doesn't fit – the ${s.furniture!.name} takes ${holds.label}.`);
		if (s.contents.length >= holds.max) return void (note = `The ${s.furniture!.name} is full.`);
		s.contents.push(thing);
		carried = carried.filter((c) => c.uid !== thing.uid);
		selected = null;
		open = s.id;
	}
	function takeOut(s: Spot, thing: Thing) {
		s.contents = s.contents.filter((c) => c.uid !== thing.uid);
		carried.push(thing);
	}
	function place(s: Spot, f: Furniture) {
		s.furniture = f;
		carried = carried.filter((c) => c.uid !== f.uid);
		selected = null;
		open = s.id;
	}
	function putAway(s: Spot) {
		carried.push(s.furniture!); // its contents stay in it (D4) – the draft keeps them on the spot
		s.furniture = null;
		open = null;
	}
	function repair() {
		const broken = carried.find((c) => c.kind === 'broken_book');
		if (!broken) return;
		carried = carried.map((c) => (c.uid === broken.uid ? t('songbook_diy', 'DIY song book', 'book') : c));
		note = 'Repaired – the song book is back in your inventory.';
	}
</script>

<div class="cave" class:visitor>
	<div class="room" data-testid="cave-room">
		<svg class="shell" viewBox="0 0 100 100" aria-hidden="true">
			<polygon class="wall" points="0,30 50,8 50,58 0,80" />
			<polygon class="wall" points="50,8 100,30 100,80 50,58" />
			<polygon class="floor" points="0,80 50,58 100,80 50,100" />
		</svg>
		<img class="avatar" src={asset('catpope/avatar.webp')} alt="Your avatar" />
		{#each spots as s (s.id)}
			<button
				class="spot"
				class:empty={!s.furniture}
				class:open={open === s.id}
				class:target={!!sel && !visitor && ((!!s.furniture?.holds && s.furniture.holds.kinds.includes(sel.kind)) || (sel.kind === 'furniture' && !s.furniture))}
				style:left="{s.x}%"
				style:top="{s.y}%"
				title={s.furniture?.name ?? 'Empty spot'}
				disabled={visitor && !s.furniture}
				onclick={() => tapSpot(s)}
				data-testid="cave-spot"
			>
				{#if s.furniture}
					<Glyph kind={s.furniture.item} count={s.contents.length} />
				{:else}
					<span class="plus" aria-hidden="true">+</span>
				{/if}
			</button>
		{/each}
	</div>

	<aside class="panel" data-testid="cave-panel">
		{#if note}<p class="note" role="status">{note}</p>{/if}
		{#if spot?.furniture}
			<h2>{spot.furniture.name}{#if spot.furniture.holds}<span class="muted">{` · `}{spot.contents.length} / {spot.furniture.holds.max}</span>{/if}</h2>
			{#if spot.furniture.holds}
				<div class="grid">
					{#each spot.contents as c (c.uid)}
						<button class="thing" disabled={visitor} title={visitor ? c.name : `Take out: ${c.name}`} onclick={() => takeOut(spot, c)}>
							<Glyph kind={c.item} /><span>{c.name}</span>
						</button>
					{/each}
				</div>
				{#if !visitor}<p class="muted">Select something in your inventory, then tap the {spot.furniture.name}.</p>{/if}
			{:else if spot.furniture.workshop}
				<p>Workshop recipes</p>
				<button class="primary" disabled={!carried.some((c) => c.kind === 'broken_book')} onclick={repair} data-testid="cave-repair">
					Repair the broken song book
				</button>
			{/if}
			{#if !visitor}<button onclick={() => putAway(spot)}>Put away</button>{/if}
		{:else if spot}
			<h2>Empty spot</h2>
			<p class="muted">Select a piece of furniture in your inventory, then tap the spot.</p>
		{/if}

		<h2>{visitor ? 'Your inventory (you are visiting)' : 'Your inventory'}</h2>
		<div class="grid">
			{#each carried as c (c.uid)}
				<button class="thing" class:selected={selected === c.uid} disabled={visitor} onclick={() => (selected = selected === c.uid ? null : c.uid)}>
					<Glyph kind={c.item} /><span>{c.name}</span>
				</button>
			{/each}
		</div>
		<button onclick={onleave} data-testid="cave-leave">{visitor ? 'Leave the visit' : 'Leave the cave'}</button>
	</aside>
</div>

<style>
	.cave {
		display: grid;
		gap: 1rem;
	}
	@media (min-width: 64rem) {
		.cave {
			grid-template-columns: minmax(0, 1fr) 22rem;
			align-items: start;
		}
	}
	/* the 2.5D diorama: two walls meeting in a corner, the floor in front (MAP.md §5.1 three-quarter view) */
	.room {
		position: relative;
		aspect-ratio: 1;
		margin-inline: auto;
		width: min(100%, 40rem, 75vh);
	}
	.shell {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
	}
	.shell polygon {
		stroke: var(--ink);
		stroke-width: 0.5;
		stroke-linejoin: round;
	}
	.wall {
		fill: var(--paper);
	}
	.floor {
		fill: rgb(0 0 0 / 0.06);
	}
	.avatar {
		position: absolute;
		left: 50%;
		top: 46%;
		width: 20%;
		translate: -50% 0;
		pointer-events: none;
	}
	.spot {
		position: absolute;
		translate: -50% -50%;
		width: 22%;
		aspect-ratio: 1;
		padding: 0;
		border: 0;
		background: none;
		color: var(--ink);
		cursor: pointer;
	}
	.spot.empty {
		width: 14%;
		border: var(--border-width-regular) dashed var(--ink);
		background: var(--paper);
	}
	.visitor .spot.empty {
		visibility: hidden;
	}
	.spot.open,
	.spot.target {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: 0.25rem;
	}
	.spot.target {
		outline-style: dashed;
	}
	.plus {
		font-size: 1.5rem;
	}
	.panel {
		display: grid;
		gap: 0.5rem;
		align-content: start;
	}
	h2 {
		margin: 0.5rem 0 0;
		font-size: 1rem;
	}
	.muted {
		color: var(--muted);
	}
	.note {
		margin: 0;
		padding: 0.5rem;
		border: var(--border-width-regular) solid var(--ink);
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(5.5rem, 1fr));
		gap: 0.5rem;
	}
	.thing {
		display: grid;
		justify-items: center;
		gap: 0.25rem;
		padding: 0.25rem;
		border: var(--border-width-regular) solid transparent;
		background: none;
		color: var(--ink);
		font: inherit;
		font-size: 0.75rem;
		cursor: pointer;
	}
	.thing.selected {
		border-color: var(--ink);
	}
	.thing:disabled {
		cursor: default;
	}
	.panel > button {
		justify-self: start;
		min-height: var(--control-height);
		padding: 0 0.75rem;
		border: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
		color: var(--ink);
		font: inherit;
		cursor: pointer;
	}
	.panel > button.primary {
		background: var(--ink);
		color: var(--paper);
	}
	.panel > button:disabled {
		opacity: 0.4;
	}
</style>
