<script lang="ts">
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import type { AvatarSlot, Entry } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import ItemIcon from '$lib/components/items/item-icon.svelte';

	/**
	 * Default reader avatar: ink-line silhouette with one tappable zone per catalog slot.
	 * Coordinates are in the SVG viewBox (360 × 400); zones are positioned in % of it.
	 */
	const W = 360;
	const H = 400;
	const TILE = 70;

	type Zone = { slot: AvatarSlot; x: number; y: number; ax: number; ay: number };
	/** where an equipped item is drawn on the body: centre + size (viewBox units) */
	const WORN: Record<AvatarSlot, { x: number; y: number; size: number }> = {
		head: { x: 180, y: 30, size: 64 },
		face: { x: 180, y: 50, size: 40 },
		neck: { x: 180, y: 100, size: 44 },
		torso: { x: 180, y: 160, size: 130 },
		hands: { x: 121, y: 236, size: 36 },
		legs: { x: 180, y: 300, size: 110 },
		feet: { x: 180, y: 380, size: 60 },
		back: { x: 180, y: 150, size: 120 },
		accessory: { x: 239, y: 236, size: 36 }
	};
	// draw order: back first, face/head last
	const WORN_ORDER: AvatarSlot[] = ['back', 'legs', 'feet', 'torso', 'hands', 'accessory', 'neck', 'head', 'face'];

	const LAYOUT: Record<AvatarSlot, Omit<Zone, 'slot'>> = {
		head: { x: 8, y: 6, ax: 168, ay: 36 },
		face: { x: 8, y: 84, ax: 176, ay: 54 },
		hands: { x: 8, y: 162, ax: 121, ay: 236 },
		legs: { x: 8, y: 240, ax: 166, ay: 300 },
		feet: { x: 8, y: 318, ax: 160, ay: 382 },
		neck: { x: 282, y: 6, ax: 189, ay: 84 },
		torso: { x: 282, y: 84, ax: 196, ay: 150 },
		back: { x: 282, y: 162, ax: 224, ay: 118 },
		accessory: { x: 282, y: 240, ax: 239, ay: 236 }
	};

	const inventory = getInventoryContext();
	const slots = $derived<AvatarSlot[]>(
		(inventory.catalog?.slots ?? (Object.keys(LAYOUT) as AvatarSlot[])).filter((s) => s in LAYOUT)
	);
	const zones = $derived<Zone[]>(slots.map((slot) => ({ slot, ...LAYOUT[slot] })));

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

<div class="avatar">
	<div class="figure" style="aspect-ratio: {W} / {H}">
		<svg viewBox="0 0 {W} {H}" aria-hidden="true">
			{#each zones as zone}
				{@const on = !!inventory.equippedEntry(zone.slot)}
				<line
					x1={zone.x < W / 2 ? zone.x + TILE : zone.x}
					y1={zone.y + TILE / 2}
					x2={zone.ax}
					y2={zone.ay}
					class:on
					class:active={selected === zone.slot}
				/>
				<circle cx={zone.ax} cy={zone.ay} r="3" class="anchor" class:on />
			{/each}
			<g class="body">
				<circle cx="180" cy="48" r="26" />
				<path
					d="M171,72 L171,90 L142,98 Q128,102 126,118 L114,226 L128,228 L138,136 L144,240 L150,372 L172,372 L178,262 L182,262 L188,372 L210,372 L216,240 L222,136 L232,228 L246,226 L234,118 Q232,102 218,98 L189,90 L189,72"
				/>
				<path d="M150,372 L144,386 L174,386 L172,372 M188,372 L186,386 L216,386 L210,372" />
				<circle cx="121" cy="236" r="7" />
				<circle cx="239" cy="236" r="7" />
			</g>
		</svg>

		{#each WORN_ORDER as slot}
			{@const worn = slots.includes(slot) ? inventory.equippedEntry(slot) : undefined}
			{#if worn}
				{@const w = WORN[slot]}
				<div
					class="worn"
					style="left:{((w.x - w.size / 2) / W) * 100}%; top:{((w.y - w.size / 2) / H) * 100}%; width:{(w.size /
						W) *
						100}%"
				>
					<ItemIcon srcs={[inventory.iconOf(worn)]} name={inventory.nameOf(worn)} />
				</div>
			{/if}
		{/each}

		{#each zones as zone}
			{@const entry = inventory.equippedEntry(zone.slot)}
			<button
				class="zone"
				class:equipped={!!entry}
				class:active={selected === zone.slot}
				style="left:{(zone.x / W) * 100}%; top:{(zone.y / H) * 100}%; width:{(TILE / W) * 100}%; height:{(TILE /
					H) *
					100}%"
				onclick={() => (selected = selected === zone.slot ? null : zone.slot)}
				aria-label={slotLabel(zone.slot)}
			>
				{#if entry}
					<ItemIcon srcs={[inventory.iconOf(entry)]} name={inventory.nameOf(entry)} />
				{/if}
				<span class="label">{slotLabel(zone.slot)}</span>
			</button>
		{/each}
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
	.figure {
		position: relative;
		width: 100%;
		max-width: 420px;
		margin: 0 auto;
	}
	svg {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
	}
	.body {
		fill: none;
		stroke: black;
		stroke-width: 2;
		stroke-linejoin: miter;
	}
	line {
		stroke: #ccc;
		stroke-width: 1;
		stroke-dasharray: 3 3;
	}
	line.on,
	line.active {
		stroke: black;
		stroke-dasharray: none;
	}
	.anchor {
		fill: white;
		stroke: #ccc;
		stroke-width: 1;
	}
	.anchor.on {
		fill: var(--primary-color);
		stroke: black;
	}
	.worn {
		position: absolute;
		pointer-events: none;
	}
	.zone {
		position: absolute;
		box-sizing: border-box;
		padding: 0.125rem;
		background: white;
		color: black;
		border: 2px solid #ccc;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: flex-end;
		touch-action: manipulation;
		transition: border-color 0.1s ease;
	}
	.zone :global(.item-icon) {
		position: absolute;
		inset: 0.125rem 0.125rem 0.75rem;
		width: auto !important;
	}
	.zone:hover,
	.zone.active {
		border-color: black;
		background: white;
	}
	.zone.equipped {
		border-color: black;
	}
	.label {
		font-size: 0.5rem;
		line-height: 1;
		position: relative;
		background: white;
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
