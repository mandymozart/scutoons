<script lang="ts">
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import type { AvatarSlot, Entry } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';
	import ItemIcon from '$lib/components/items/item-icon.svelte';

	/**
	 * Default reader avatar: the sphynx mannequin (CDN, 768 × 1152) with named slot tiles in a column on each side.
	 * Equipped items are drawn on the body; positions are in % of the mannequin image.
	 */
	const W = 768;
	const H = 1152;

	type Worn = Record<AvatarSlot, { x: number; y: number; size: number }>;
	/** where an equipped item sits on the sphynx body: centre + width (image px) */
	const SPHYNX_WORN: Worn = {
		head: { x: 370, y: 70, size: 170 },
		face: { x: 372, y: 105, size: 110 },
		neck: { x: 372, y: 200, size: 110 },
		torso: { x: 372, y: 340, size: 330 },
		hands: { x: 150, y: 575, size: 90 },
		legs: { x: 372, y: 760, size: 300 },
		feet: { x: 372, y: 1040, size: 330 },
		back: { x: 372, y: 330, size: 360 },
		accessory: { x: 598, y: 575, size: 90 }
	};
	/** mouse: big head, shorter body */
	const MOUSE_WORN: Worn = {
		head: { x: 385, y: 130, size: 330 },
		face: { x: 385, y: 230, size: 200 },
		neck: { x: 385, y: 330, size: 120 },
		torso: { x: 385, y: 450, size: 260 },
		hands: { x: 175, y: 625, size: 90 },
		legs: { x: 385, y: 770, size: 260 },
		feet: { x: 385, y: 1010, size: 340 },
		back: { x: 385, y: 450, size: 300 },
		accessory: { x: 595, y: 625, size: 90 }
	};

	/** Avatar bodies (768 × 1152 each). The first one is the default. */
	const BODIES = [
		{ id: 'sphynx', image: 'catpope/avatar/mannequin.webp', worn: SPHYNX_WORN },
		{ id: 'sphynx_nude', image: 'catpope/avatar/mannequin-nude.webp', worn: SPHYNX_WORN },
		{ id: 'mouse', image: 'yellowmouse/avatar/mannequin.webp', worn: MOUSE_WORN }
	] as const;
	type BodyId = (typeof BODIES)[number]['id'];
	// Body choice is a per-device preference for now (no server field yet)
	const BODY_KEY = 'scutoons_avatar_body';

	function readBody(): BodyId {
		try {
			const stored = localStorage.getItem(BODY_KEY);
			if (BODIES.some((b) => b.id === stored)) return stored as BodyId;
		} catch {
			// storage unavailable → default body
		}
		return BODIES[0].id;
	}

	let bodyId = $state<BodyId>(readBody());
	const body = $derived(BODIES.find((b) => b.id === bodyId) ?? BODIES[0]);
	const WORN = $derived(body.worn);

	function chooseBody(id: BodyId) {
		bodyId = id;
		try {
			localStorage.setItem(BODY_KEY, id);
		} catch {
			// not persisted, still switches for this visit
		}
	}

	// draw order: back first, face/head last
	const WORN_ORDER: AvatarSlot[] = ['back', 'legs', 'feet', 'torso', 'hands', 'accessory', 'neck', 'head', 'face'];
	// tiles: left column top→bottom, right column top→bottom
	const LEFT: AvatarSlot[] = ['head', 'face', 'hands', 'legs', 'feet'];
	const RIGHT: AvatarSlot[] = ['neck', 'torso', 'back', 'accessory'];

	const inventory = getInventoryContext();
	const slots = $derived<AvatarSlot[]>(
		(inventory.catalog?.slots ?? (Object.keys(SPHYNX_WORN) as AvatarSlot[])).filter((s) => s in SPHYNX_WORN)
	);
	const left = $derived(LEFT.filter((s) => slots.includes(s)));
	const right = $derived([...RIGHT, ...slots.filter((s) => !LEFT.includes(s) && !RIGHT.includes(s))].filter((s) => slots.includes(s)));

	let selected = $state<AvatarSlot | null>(null);
	let working = $state(false);

	const wearables = $derived<Entry[]>(
		selected ? inventory.entries.filter((e) => inventory.item(e.item_id)?.slot === selected) : []
	);

	const slotLabel = (slot: AvatarSlot) => (m as any)[`items.slot_${slot}`]?.() ?? slot;

	async function run(fn: () => Promise<void>) {
		working = true;
		try {
			await fn();
		} catch (e) {
			console.warn('Avatar update failed', e);
		} finally {
			working = false;
		}
	}
</script>

{#snippet tile(slot: AvatarSlot)}
	{@const entry = inventory.equippedEntry(slot)}
	<button
		class="tile"
		class:equipped={!!entry}
		class:active={selected === slot}
		onclick={() => (selected = selected === slot ? null : slot)}
		aria-pressed={selected === slot}
	>
		<span class="thumb">
			{#if entry}
				<ItemIcon srcs={[inventory.iconOf(entry)]} name={inventory.nameOf(entry)} />
			{/if}
		</span>
		<span class="label">{slotLabel(slot)}</span>
	</button>
{/snippet}

<div class="avatar">
	<div class="bodies" role="group" aria-label={m['items.avatar_body']()}>
		{#each BODIES as b (b.id)}
			<button
				class="body-choice"
				class:active={b.id === bodyId}
				aria-pressed={b.id === bodyId}
				onclick={() => chooseBody(b.id)}
			>
				{(m as any)[`items.avatar_body_${b.id}`]?.() ?? b.id}
			</button>
		{/each}
	</div>
	<div class="stage">
		<div class="column">
			{#each left as slot (slot)}{@render tile(slot)}{/each}
		</div>
		<div class="figure" style="aspect-ratio: {W} / {H}">
			<img class="mannequin" src={asset(body.image)} alt="" draggable="false" />
			{#each WORN_ORDER as slot}
				{@const worn = slots.includes(slot) ? inventory.equippedEntry(slot) : undefined}
				{#if worn}
					{@const w = WORN[slot]}
					<div
						class="worn"
						style="left:{((w.x - w.size / 2) / W) * 100}%; top:{((w.y - w.size / 2) / H) * 100}%; width:{(w.size / W) * 100}%"
					>
						<ItemIcon srcs={[inventory.iconOf(worn)]} name={inventory.nameOf(worn)} />
					</div>
				{/if}
			{/each}
		</div>
		<div class="column">
			{#each right as slot (slot)}{@render tile(slot)}{/each}
		</div>
	</div>

	{#if selected}
		{@const current = inventory.equippedEntry(selected)}
		<div class="picker">
			<h3>{slotLabel(selected)}</h3>
			{#if wearables.length === 0}
				<p class="note">{m['items.nothing_to_wear']()}</p>
			{/if}
			<ul>
				{#each wearables as entry (entry.id)}
					{@const on = current && String(current.id) === String(entry.id)}
					<li>
						<div class="thumb"><ItemIcon srcs={[inventory.iconOf(entry)]} name={inventory.nameOf(entry)} /></div>
						<span class="name">{inventory.nameOf(entry)}</span>
						<button
							class={on ? '' : 'btn-secondary'}
							class:disabled={working}
							disabled={working}
							onclick={() =>
								run(() => (on ? inventory.unequip(selected!) : inventory.equip(entry)))}
						>
							{on ? m['items.take_off']() : m['items.put_on']()}
						</button>
					</li>
				{/each}
			</ul>
		</div>
	{/if}
</div>

<style>
	.avatar {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	.bodies {
		display: flex;
		flex-wrap: wrap;
		justify-content: center;
		gap: 0.5rem;
	}
	.body-choice {
		font: inherit;
		font-size: 0.75rem;
		padding: 0.25rem 0.75rem;
		background: white;
		color: black;
		border: 2px solid #ccc;
		cursor: pointer;
	}
	.body-choice:hover,
	.body-choice.active {
		border-color: black;
	}
	.stage {
		display: grid;
		grid-template-columns: 4.5rem minmax(0, 1fr) 4.5rem;
		align-items: stretch;
		gap: 0.5rem;
		max-width: 560px;
		width: 100%;
		margin: 0 auto;
	}
	.column {
		display: flex;
		flex-direction: column;
		justify-content: space-around;
		gap: 0.5rem;
	}
	.figure {
		position: relative;
		width: 100%;
	}
	.mannequin {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		object-fit: contain;
		user-select: none;
	}
	.worn {
		position: absolute;
		pointer-events: none;
	}
	.tile {
		box-sizing: border-box;
		padding: 0.25rem;
		background: white;
		color: black;
		border: 2px solid #ccc;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.25rem;
		touch-action: manipulation;
		transition: border-color 0.1s ease;
		cursor: pointer;
	}
	.tile:hover,
	.tile.active,
	.tile.equipped {
		border-color: black;
	}
	.tile.active {
		background: var(--primary-color);
	}
	.thumb {
		position: relative;
		width: 2.5rem;
		height: 2.5rem;
		display: block;
	}
	.thumb :global(.item-icon) {
		position: absolute;
		inset: 0;
		width: auto !important;
	}
	.label {
		font-size: 0.625rem;
		line-height: 1.1;
		text-align: center;
		overflow-wrap: anywhere;
	}
	.picker h3 {
		margin: 0 0 0.5rem;
		font-size: 1rem;
		font-weight: 700;
	}
	.picker ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.picker li {
		display: grid;
		grid-template-columns: 3rem 1fr auto;
		align-items: center;
		gap: 0.5rem;
		border: 2px solid #ccc;
		padding: 0.25rem;
	}
	.picker li button {
		padding: 0.5rem 1rem;
	}
	.name {
		font-size: 0.75rem;
		line-height: 1.3;
	}
	.note {
		margin: 0;
		font-size: 0.75rem;
		color: #6b6b6b;
	}
</style>
