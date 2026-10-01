<script lang="ts">
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import {
		AVATAR_SLOTS,
		SENSITIVE_COVER,
		WEAR_ORDER,
		pickText,
		wardrobeBase,
		type AvatarBody,
		type AvatarSlot,
		type AvatarSlotInstance,
		type Entry
	} from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { asset } from '$lib/utils/cdn.ts';
	import ItemIcon from '$lib/components/items/item-icon.svelte';

	/**
	 * Reader avatar: a character's A-pose (CDN, 768 × 1152) with a tile per wear slot of that body in a column on each
	 * side. The bodies come from the character registry (`avatarBodies`, loaded by the inventory context), filtered to
	 * what the reader has unlocked (members: `/me/ avatar.unlocked`; guests: the default unlocks); locked characters are
	 * not shown. A body's wear slots are slot instances (`apose.looks[].slots`: id, slot type, anchor) – a duo has every
	 * slot type twice ("Kopf links" / "Kopf rechts").
	 *
	 * Wardrobe (ITEMS.md "Wardrobe"): the base image is the character's naked A-pose when the reader may see it ("Hide
	 * sensitive content" off, or every legs instance wears an item with an overlay – and only once those overlays have
	 * loaded), else the safe look (swim, pants painted in). On top: worn items with an overlay for this body as full-body
	 * layers in WEAR_ORDER (legs → feet → torso → back → hands → accessory → neck → head → face), then the items without
	 * one as icons at their slot anchors. The picker shows one tile per character (nakedness is the swim pants coming off,
	 * not a look). ITEMS.md "Reader avatar bodies & character unlocks", "Slots per avatar body".
	 */
	const W = 768;
	const H = 1152;

	const inventory = getInventoryContext();
	$effect(() => {
		inventory.loadRegistry();
	});

	// every visible look of the unlocked characters; looks marked `sensitive` (naked) are left out while "Hide sensitive
	// content" is on (members: account page, on by default; guests: always) – ITEMS.md "Sensitive content". A hidden
	// chosen body shows as the character's first visible look (inventory context; the server moves it on save).
	const body = $derived<AvatarBody | undefined>(inventory.currentBody);
	/** one tile per character: its current body, else its first visible look (the default first) */
	const bodies = $derived<AvatarBody[]>(
		inventory.bodies.filter((b, i, all) => all.findIndex((x) => x.character === b.character) === i).map((b) => (body?.character === b.character ? body : b))
	);
	let bodyError = $state<string | null>(null);

	async function chooseBody(b: AvatarBody) {
		bodyError = null;
		try {
			await inventory.setBody({ character: b.character, look: b.look });
		} catch (e) {
			console.warn('Avatar body not saved', e);
			bodyError = m['items.avatar_body_error']();
		}
	}

	// icons without an overlay: back first, face/head last (drawn above every overlay)
	const WORN_ORDER: AvatarSlot[] = ['back', 'legs', 'feet', 'torso', 'hands', 'accessory', 'neck', 'head', 'face'];
	// tiles of a single figure: left column top→bottom, right column top→bottom
	const LEFT: AvatarSlot[] = ['head', 'face', 'hands', 'legs', 'feet'];
	const RIGHT: AvatarSlot[] = ['neck', 'torso', 'back', 'accessory'];
	// tiles of a multi-figure body (a slot type more than once): each instance in the column of its anchor, top→bottom
	const TYPE_ORDER: AvatarSlot[] = ['head', 'face', 'neck', 'torso', 'back', 'hands', 'accessory', 'legs', 'feet'];

	/** the catalog's slot types (instances of other types are not shown) */
	const types = $derived<AvatarSlot[]>((inventory.catalog?.slots ?? AVATAR_SLOTS).filter((s) => (AVATAR_SLOTS as string[]).includes(s)));
	const instances = $derived<AvatarSlotInstance[]>(inventory.slots.filter((s) => types.includes(s.slot)));
	const multi = $derived(new Set(instances.map((s) => s.slot)).size < instances.length);
	const byType = (list: AvatarSlotInstance[], order: AvatarSlot[]) =>
		[...list].sort((a, b) => (order.indexOf(a.slot) + 1 || 99) - (order.indexOf(b.slot) + 1 || 99));
	const left = $derived(
		multi ? byType(instances.filter((s) => s.x < W / 2), TYPE_ORDER) : byType(instances.filter((s) => LEFT.includes(s.slot)), LEFT)
	);
	const right = $derived(
		multi ? byType(instances.filter((s) => s.x >= W / 2), TYPE_ORDER) : byType(instances.filter((s) => !LEFT.includes(s.slot)), RIGHT)
	);
	/** worn items with an overlay for this body (full-body layers, WEAR_ORDER) */
	const overlays = $derived(
		byType(instances, WEAR_ORDER).flatMap((s) => {
			const entry = inventory.equippedEntry(s.id);
			const src = entry ? inventory.overlayOf(entry, s.id) : null;
			return entry && src ? [{ slot: s, entry, src }] : [];
		})
	);
	/** worn items without an overlay: icons at the anchors */
	const drawn = $derived(byType(instances.filter((s) => s.size > 0 && !overlays.some((o) => o.slot.id === s.id)), WORN_ORDER));
	// the base: naked only when allowed; while hidden it waits until the legs overlays are there (never a naked frame)
	let loaded = $state<Record<string, boolean>>({});
	const coverSrcs = $derived(overlays.filter((o) => SENSITIVE_COVER.includes(o.slot.slot)).map((o) => asset(o.src)));
	const coverReady = $derived(coverSrcs.every((src) => loaded[src] === true));
	const wanted = $derived(wardrobeBase(inventory.unlockedBodies, body, { hide: inventory.hideSensitive, covered: inventory.covered }));
	const safe = $derived(wardrobeBase(inventory.unlockedBodies, body, { hide: true, covered: false }));
	const showNaked = $derived(!!wanted?.naked && (!inventory.hideSensitive || coverReady));
	const base = $derived(showNaked ? wanted : safe);
	/** the newest swap note (back in the inventory / put back on) for the status line */
	const note = $derived([...inventory.notices].reverse().find((n) => n.kind === 'returned' || n.kind === 'restored'));
	const noteText = $derived(
		note ? (note.kind === 'returned' ? m['items.returned'] : m['items.restored'])({ name: inventory.nameOf({ item_id: note.item_id, variant: null }) }) : ''
	);

	let selectedId = $state<string | null>(null);
	const selected = $derived(instances.find((s) => s.id === selectedId));
	let working = $state(false);

	const wearables = $derived<Entry[]>(
		selected ? inventory.entries.filter((e) => inventory.item(e.item_id)?.slot === selected.slot) : []
	);

	const typeLabel = (slot: AvatarSlot) => (m as any)[`items.slot_${slot}`]?.() ?? slot;
	const slotLabel = (s: AvatarSlotInstance) => pickText(s.label, getLocale(), ['en', 'de']) || typeLabel(s.slot);

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

{#snippet tile(slot: AvatarSlotInstance)}
	{@const entry = inventory.equippedEntry(slot.id)}
	<button
		class="tile"
		class:equipped={!!entry}
		class:active={selectedId === slot.id}
		onclick={() => (selectedId = selectedId === slot.id ? null : slot.id)}
		aria-pressed={selectedId === slot.id}
		data-slot={slot.id}
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
	{#if bodies.length > 1}
	<div class="bodies" role="group" aria-label={m['items.avatar_body']()} data-testid="avatar-bodies">
		{#each bodies as b (b.character)}
			{@const active = b.character === body?.character}
			<button
				class="body-choice"
				class:active
				aria-pressed={active}
				title={b.name}
				data-body="{b.character}/{b.look}"
				onclick={() => chooseBody(b)}
			>
				<img src={asset(b.path)} alt="" loading="lazy" draggable="false" />
				<span class="body-name">{b.name}</span>
			</button>
		{/each}
	</div>
	{/if}
	{#if bodyError}<p class="note" role="alert">{bodyError}</p>{/if}
	{#if inventory.registryLoaded && !body}<p class="note">{m['items.avatar_body_none']()}</p>{/if}
	<div class="stage" class:dense={multi}>
		<div class="column">
			{#each left as slot (slot.id)}{@render tile(slot)}{/each}
		</div>
		<div class="figure" style="aspect-ratio: {W} / {H}" data-base={showNaked ? 'naked' : 'safe'}>
			{#if body && safe && (!showNaked || inventory.hideSensitive)}
				<!-- the safe look (swim): under the naked base while hidden, so nothing naked shows if a layer fails -->
				<img class="apose" src={asset(safe.path)} alt={showNaked ? '' : body.name} draggable="false" data-layer="base-safe" />
			{/if}
			{#if body && showNaked && wanted}
				<img class="apose" src={asset(wanted.path)} alt={body.name} draggable="false" data-layer="base-naked" />
			{/if}
			{#each overlays as o (o.slot.id)}
				<img
					class="apose"
					src={asset(o.src)}
					alt=""
					draggable="false"
					data-overlay={o.slot.id}
					data-item={o.entry.item_id}
					onload={() => (loaded[asset(o.src)] = true)}
					onerror={() => (loaded[asset(o.src)] = false)}
				/>
			{/each}
			{#each drawn as w (w.id)}
				{@const worn = inventory.equippedEntry(w.id)}
				{#if worn}
					<div
						class="worn"
						data-worn={w.id}
						style="left:{((w.x - w.size / 2) / W) * 100}%; top:{((w.y - w.size / 2) / H) * 100}%; width:{(w.size / W) * 100}%"
					>
						<ItemIcon srcs={[inventory.iconOf(worn)]} name={inventory.nameOf(worn)} />
					</div>
				{/if}
			{/each}
		</div>
		<div class="column">
			{#each right as slot (slot.id)}{@render tile(slot)}{/each}
		</div>
	</div>

	{#if noteText}<p class="note swap" role="status" data-testid="avatar-note">{noteText}</p>{/if}

	{#if selected}
		{@const current = inventory.equippedEntry(selected.id)}
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
						<span class="name">{inventory.nameOf(entry)}{#if on}<span class="worn-tag"> · {m['items.worn']()}</span>{/if}</span>
						<button
							class={on ? '' : 'btn-secondary'}
							class:disabled={working}
							disabled={working}
							onclick={() => run(() => (on ? inventory.unequip(selected.id) : inventory.equip(entry, selected.id)))}
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
		width: 5.5rem;
		padding: 0.25rem;
		background: var(--paper);
		color: var(--ink);
		border: 2px solid var(--border-muted);
		cursor: pointer;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.125rem;
	}
	.body-choice img {
		width: 100%;
		aspect-ratio: 768 / 1152;
		height: auto;
		object-fit: contain;
		user-select: none;
	}
	.body-name {
		font-weight: 700;
		line-height: 1.1;
		text-align: center;
	}
	.body-choice:hover,
	.body-choice.active {
		border-color: var(--ink);
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
	.apose {
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
		background: var(--paper);
		color: var(--ink);
		border: 2px solid var(--border-muted);
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
		border-color: var(--ink);
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
	.dense .column {
		gap: 0.25rem;
	}
	.dense .tile {
		padding: 0.125rem;
		gap: 0.125rem;
	}
	.dense .tile .thumb {
		width: 1.75rem;
		height: 1.75rem;
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
		border: 2px solid var(--border-muted);
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
		color: var(--text-muted);
	}
	.note.swap {
		color: var(--ink);
		text-align: center;
	}
	.worn-tag {
		color: var(--text-muted);
	}
</style>
