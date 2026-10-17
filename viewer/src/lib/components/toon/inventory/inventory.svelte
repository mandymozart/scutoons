<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { localizeHref } from '$lib/paraglide/runtime.js';
	import { getNavigationContext } from '$lib/contexts/navigation.svelte';
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import { getItemsUiContext } from '$lib/contexts/items-ui.svelte.ts';
	import type { Entry } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import PullPanel from '$lib/components/panel/pull-panel.svelte';
	import inventoryIcon from '$lib/icons/inventory.svg';
	import avatarIcon from '$lib/icons/avatar.svg';
	import ItemDetail from '$lib/components/items/item-detail.svelte';
	import RecipePreview from '$lib/components/items/recipe-preview.svelte';
	import Avatar from '$lib/components/avatar/avatar.svelte';
	import ReaderStats from '$lib/components/items/reader-stats.svelte';
	import InvitesPanel from '$lib/components/items/invites-panel.svelte';
	import { warmModel } from '$lib/components/items/model-warmup.ts';
	import InventoryGrid from './inventory-grid.svelte';
	import { createPackMode } from './pack-mode.svelte.ts';

	const nav = getNavigationContext();
	const inventory = getInventoryContext();
	const ui = getItemsUiContext();
	// two top buttons, two panels (owner 2026-10-12: avatar and inventory "auf eigene Panels", the tab buttons in the
	// shared panel out – it was 2026-10-03's one slide-out with Items / Avatar tabs)

	let selected = $state<Entry | null>(null);
	let combineWith = $state<Entry | null>(null);
	let combineSecond = $state<Entry | null>(null);
	/** a container opened: its own grid (research/SCORES.md §2) */
	let openBox = $state<Entry | null>(null);
	// the header's "Back": from an item's detail to the list, from a container's view out of it (the recipe has its own)
	const backTo = $derived.by(() => {
		if (combineWith && combineSecond) return undefined;
		if (selected && !combineWith) return { label: m['items.back'](), go: () => (selected = null) };
		if (openBox && !pack.packing) return { label: m['items.container_back'](), go: () => (openBox = null) };
		return undefined;
	});

	const entries = $derived(
		[...inventory.entries].sort((a, b) => (a.acquired_at < b.acquired_at ? -1 : 1))
	);
	// worn items are not listed (owner 2026-10-01): they live in the Avatar tab's slot tiles; a stackable entry shows
	// the pieces that are not worn (an entry is worn at most once). Taking an item off brings it back here.
	// what lies in a container is only in that container's grid
	const shown = $derived(openBox ? inventory.contentsOf(openBox) : entries.filter((e) => !inventory.containerOf(e)));
	const listed = $derived(
		shown.flatMap((e) => {
			const left = e.qty - (inventory.isEquipped(e) ? 1 : 0);
			return left > 0 ? [{ entry: e, qty: left }] : [];
		})
	);
	// keep the detail sheet in sync with inventory updates (e.g. qty change, entry consumed)
	$effect(() => {
		if (selected && !inventory.entries.some((e) => String(e.id) === String(selected!.id))) selected = null;
		if (openBox && !inventory.entries.some((e) => String(e.id) === String(openBox!.id))) openBox = null;
	});

	// the inventory shows a record with a 3D model: its player card may follow – warm three.js and the model in idle time
	$effect(() => {
		if (!nav.isOpen('inventory')) return;
		for (const { entry } of listed) {
			if (inventory.item(entry.item_id)?.album) warmModel(inventory.modelOf(entry));
		}
	});

	/** a container's fill: "3 / 25" */
	function fill(entry: Entry): string {
		const max = inventory.item(entry.item_id)?.holds?.max;
		return max != null ? `${inventory.countIn(entry)} / ${max}` : String(inventory.countIn(entry));
	}

	// the pack mode and the container view's take-out (research/PACKING.md §2)
	let gridEl = $state<HTMLElement | null>(null);
	const pack = createPackMode(inventory, async (id) => {
		await tick();
		gridEl?.querySelector<HTMLElement>(`[data-entry="${CSS.escape(id)}"]`)?.focus();
	});
	const boxOpen = $derived(!!openBox && !combineWith && !pack.packing);
	const boxAtHome = $derived(!!openBox && !!inventory.item(openBox.item_id)?.furniture);
	function tileOf(entry: Entry) {
		if (pack.packing) return pack.packTile(entry);
		return boxOpen && openBox ? pack.boxTile(entry, openBox) : null;
	}
	function hold(entry: Entry, qty: number) {
		if (pack.packing) pack.hold(entry, qty);
		else if (boxOpen && !boxAtHome) void pack.takeOut(entry, qty);
	}
	function startPacking(entry: Entry) {
		selected = null;
		openBox = null;
		pack.start(entry);
	}
	// Escape leaves the pack mode first (before the panel closes on it), back to the item's detail
	$effect(() => {
		if (!pack.packing) return;
		const entry = pack.packing;
		const onKey = (e: KeyboardEvent) => {
			if (e.key !== 'Escape') return;
			e.stopImmediatePropagation();
			e.preventDefault();
			pack.cancel();
			selected = entry;
		};
		window.addEventListener('keydown', onKey, true);
		return () => window.removeEventListener('keydown', onKey, true);
	});

	function reset() {
		selected = null;
		openBox = null;
		combineWith = null;
		combineSecond = null;
		pack.cancel();
	}

	/** two of the same kind with no recipe between them go onto one field (owner 2026-10-17, research/PACKING.md §6) */
	const stacksWith = (a: Entry, b: Entry) =>
		inventory.twinsOf(a).some((t) => String(t.id) === String(b.id)) && inventory.recipesFor(a.item_id, b.item_id).length === 0;
	async function stackOnto(entry: Entry, onto: Entry) {
		combineWith = null;
		const result = await inventory.stack(entry, onto);
		if (!result.ok) return pack.say(result.error === 'own_piece' ? m['items.stack_own_piece']() : m['items.move_failed']());
		const now = inventory.entries.find((e) => String(e.id) === String(onto.id));
		pack.say(m['items.stack_done']({ qty: now?.qty ?? onto.qty + entry.qty, name: inventory.nameOf(onto) }));
		await tick();
		gridEl?.querySelector<HTMLElement>(`[data-entry="${CSS.escape(String(onto.id))}"]`)?.focus();
	}

	function pick(entry: Entry) {
		if (pack.packing) return pack.tap(entry);
		if (boxOpen && !boxAtHome) return void pack.takeOut(entry);
		if (combineWith) {
			if (stacksWith(combineWith, entry)) return void stackOnto(combineWith, entry);
			if (String(entry.id) === String(combineWith.id) && entry.qty < 2) return;
			combineSecond = entry;
			return;
		}
		selected = entry;
	}

	// overlays opened from the inventory (camera, claim, a minigame) live outside of it: their taps / Escape are theirs
	const keepOpen = () => !!(ui.camera || ui.claimCode || ui.minigame);

	// the panel closes itself on Escape; "i" toggles it
	$effect(() => {
		function handleKeyDown(e: KeyboardEvent) {
			const target = e.target as HTMLElement | null;
			if (target?.closest('input, textarea, select, [contenteditable]')) return;
			if (ui.camera || ui.claimCode) return;
			if ((e.key === 'i' || e.key === 'I') && !e.ctrlKey && !e.metaKey && !e.altKey) nav.toggleOverlay('inventory');
		}

		window.addEventListener('keydown', handleKeyDown);
		return () => window.removeEventListener('keydown', handleKeyDown);
	});

	// `?inventory=open` (QR landing page) or a claim that wants the inventory shown
	$effect(() => {
		if (ui.inventoryRequested) {
			nav.openOverlay('inventory');
			ui.requestInventory(false);
		}
	});

	// `?avatar=open`: the account page's "Open avatar" (owner 2026-10-16) – the same for the avatar panel
	onMount(() => {
		const opened = (name: string) => ['open', '1', 'true'].includes(page.url.searchParams.get(name) ?? '');
		const overlay = opened('inventory') ? 'inventory' : opened('avatar') ? 'avatar' : null;
		if (overlay) {
			nav.openOverlay(overlay);
			const url = new URL(window.location.href);
			url.searchParams.delete('inventory');
			url.searchParams.delete('avatar');
			goto(url.pathname + url.search, { replaceState: true, noScroll: true, keepFocus: true });
		}
	});

	function toggle(e: MouseEvent, overlay: 'inventory' | 'avatar') {
		e.stopPropagation();
		reset();
		nav.toggleOverlay(overlay);
	}

</script>

<button class="top-toggle inventory-toggle" onclick={(e) => toggle(e, 'inventory')} aria-label={m['items.tab_items']()} aria-pressed={nav.isOpen('inventory')} aria-controls="inventory-panel">
	<img src={inventoryIcon} alt="" />
</button>
<button class="top-toggle avatar-toggle" onclick={(e) => toggle(e, 'avatar')} aria-label={m['items.tab_avatar']()} aria-pressed={nav.isOpen('avatar')} aria-controls="avatar-panel" data-testid="avatar-toggle">
	<img src={avatarIcon} alt="" />
</button>

{#snippet head(icon: string, label: string, back?: { label: string; go: () => void })}
	<!-- the reader's stats (appetite, testo) small at the right, in both panels (owner 2026-10-03); no title – the top
	     button says which panel it is (owner 2026-10-16: "die Überschriften ganz oben am Panel … redundant"); a "Back"
	     on the left in this fixed part, not in the scrolling body (owner 2026-10-17) -->
	<div class="head" title={label} data-icon={icon}>
		{#if back}<button class="btn-back btn-clear" onclick={back.go} data-testid="panel-back">← {back.label}</button>{/if}
		<div class="stats"><ReaderStats compact /></div>
	</div>
{/snippet}

{#snippet guest()}
	{#if !inventory.isAccount}
		<p class="guest">
			{m['items.guest_hint']()}
			<a href={localizeHref('/account')}>{m['items.sign_in']()}</a>
		</p>
	{/if}
{/snippet}

<PullPanel open={nav.isOpen('avatar')} onclose={() => nav.closeOverlay()} label={m['items.tab_avatar']()} id="avatar-panel" {keepOpen} testid="avatar-panel">
	{#snippet header()}{@render head(avatarIcon, m['items.tab_avatar']())}{/snippet}
	{@render guest()}
	<Avatar />
</PullPanel>

<PullPanel open={nav.isOpen('inventory')} onclose={() => nav.closeOverlay()} label={m['items.tab_items']()} id="inventory-panel" {keepOpen} testid="inventory">
	{#snippet header()}{@render head(inventoryIcon, m['items.tab_items'](), backTo)}{/snippet}
	{@render guest()}

	{#if combineWith && combineSecond}
		<RecipePreview
			first={combineWith}
			second={combineSecond}
			oncancel={() => (combineSecond = null)}
			ondone={() => reset()}
		/>
	{:else if selected && !combineWith}
		<ItemDetail entry={selected} onback={() => (selected = null)} oncombine={(e) => (combineWith = e)} onopencontainer={(c) => ((openBox = c), (selected = null), pack.clearNote())} onpack={startPacking} />
	{:else if !inventory.catalog && inventory.entries.length === 0}
		<p class="empty">{inventory.catalogError ? m['items.unavailable']() : m['items.loading']()}</p>
	{:else}
		<!-- gift invites: invite with a campaign's item, the open invites, a gift's shipping address (research/GIFT-INVITES.md) -->
		{#if !combineWith && !openBox && !pack.packing}<InvitesPanel />{/if}
		{#if pack.packing}
			<div class="combine-bar" data-testid="pack-bar">
				<span>{m['items.pack_where']({ name: inventory.nameOf(pack.packing) })}</span>
				<button class="btn-secondary" onclick={() => ((selected = pack.packing), pack.cancel())} data-testid="pack-cancel">{m['common.cancel']()}</button>
			</div>
		{:else if openBox}
			<div class="box-bar" data-testid="container-view">
				<span>{inventory.nameOf(openBox)} · {fill(openBox)}</span>
			</div>
			{#if openBox.overfilled_at}<p class="empty">{m['items.container_overfilled']()}</p>{/if}
			{#if inventory.countIn(openBox) > 0}<p class="hint">{boxAtHome ? m['items.container_at_home']() : m['items.container_tap_hint']()}</p>{/if}
		{/if}
		{#if pack.note}<p class="empty" role="status" data-testid="pack-note">{pack.note}</p>{/if}
		<p class="sr-only" aria-live="polite">{pack.live}</p>
		{#if listed.length === 0 && !openBox}
			<p class="empty">{m['inventory.empty']()}</p>
		{/if}
		{#if combineWith}
			<div class="combine-bar">
				<span>{(inventory.twinsOf(combineWith).length ? m['items.combine_hint_stack'] : m['items.combine_hint'])({ name: inventory.nameOf(combineWith) })}</span>
				<button class="btn-secondary" onclick={() => (combineWith = null)}>{m['common.cancel']()}</button>
			</div>
		{/if}
		{#if listed.length}
			<div bind:this={gridEl}>
				<InventoryGrid
					{listed}
					tile={tileOf}
					source={combineWith}
					draggable={!openBox && !combineWith && !pack.packing}
					onpick={pick}
					onhold={hold}
					oncount={(n) => (pack.live = String(n))}
					ondropinto={(entry, target) => (stacksWith(entry, target) ? stackOnto(entry, target) : pack.drop(entry, target))}
				/>
			</div>
		{/if}
	{/if}
</PullPanel>

<style>
	/* the menu button is right-most (main-menu.svelte), then inventory, avatar, map, chapters; one control height
	   (owner 2026-10-02) */
	.top-toggle {
		position: fixed;
		top: 0.5rem;
		right: calc(0.5rem + var(--control-height) + 0.5rem);
		z-index: 1001;
		background: var(--paper);
		border: 2px solid black;
		padding: 0;
		border-radius: 50%;
		box-sizing: border-box;
		height: var(--control-height);
		width: var(--control-height);
		display: flex;
		cursor: pointer;
		transition: all 0.2s ease;
		align-items: center;
		justify-content: center;
		touch-action: manipulation;
	}
	.avatar-toggle {
		right: calc(0.5rem + 2 * (var(--control-height) + 0.5rem));
	}
	.top-toggle img {
		width: 1rem;
	}
	.top-toggle:hover {
		background: var(--primary-color);
	}

	/* the panel's header: the grip above, its name and the stats */
	.head {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5rem;
		min-height: 2rem;
		padding: 0 1rem 0.5rem;
		color: var(--ink);
	}
	/* the top buttons' icons, white on the dark panel */
	.head img {
		width: 1rem;
		filter: invert(1);
	}
	/* the stats wrap below the name (still right) only where they don't fit beside it (≈ 320 px phones) */
	.stats {
		margin-left: auto;
		flex-shrink: 0;
	}

	.guest {
		margin: 0 0 1rem;
		font-size: 0.75rem;
		line-height: 1.3;
		color: var(--text-muted);
	}
	.guest a {
		color: var(--ink);
	}

	.empty {
		margin: 0;
	}

	.box-bar {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0 1rem 0.5rem;
		color: var(--ink);
	}
	.head .btn-back {
		color: var(--ink);
		font-size: 0.75rem;
		white-space: nowrap;
		flex-shrink: 0;
	}
	.combine-bar {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-bottom: 1rem;
		font-size: 0.75rem;
		line-height: 1.3;
	}
	.combine-bar button {
		padding: 0 1rem;
		flex: none;
	}

	.hint {
		margin: 0 0 0.5rem;
		font-size: 0.75rem;
		line-height: 1.3;
		color: var(--text-muted);
	}
	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip-path: inset(50%);
		white-space: nowrap;
	}
</style>
