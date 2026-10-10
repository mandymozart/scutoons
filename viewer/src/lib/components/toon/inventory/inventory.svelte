<script lang="ts">
	import { onMount } from 'svelte';
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
	import ItemIcon from '$lib/components/items/item-icon.svelte';
	import ItemDetail from '$lib/components/items/item-detail.svelte';
	import RecipePreview from '$lib/components/items/recipe-preview.svelte';
	import Avatar from '$lib/components/avatar/avatar.svelte';
	import ReaderStats from '$lib/components/items/reader-stats.svelte';

	const nav = getNavigationContext();
	const inventory = getInventoryContext();
	const ui = getItemsUiContext();
	// one slide-out, two top buttons (owner 2026-10-03): "inventory" opens it on Items, "avatar" on the Avatar tab
	const inventoryVisible = $derived(nav.isOpen('inventory') || nav.isOpen('avatar'));
	const tab = $derived<'items' | 'avatar'>(nav.isOpen('avatar') ? 'avatar' : 'items');

	let selected = $state<Entry | null>(null);
	let combineWith = $state<Entry | null>(null);
	let combineSecond = $state<Entry | null>(null);

	const entries = $derived(
		[...inventory.entries].sort((a, b) => (a.acquired_at < b.acquired_at ? -1 : 1))
	);
	// worn items are not listed (owner 2026-10-01): they live in the Avatar tab's slot tiles; a stackable entry shows
	// the pieces that are not worn (an entry is worn at most once). Taking an item off brings it back here.
	const listed = $derived(
		entries.flatMap((e) => {
			const left = e.qty - (inventory.isEquipped(e) ? 1 : 0);
			return left > 0 ? [{ entry: e, qty: left }] : [];
		})
	);
	// keep the detail sheet in sync with inventory updates (e.g. qty change, entry consumed)
	$effect(() => {
		if (selected && !inventory.entries.some((e) => String(e.id) === String(selected!.id))) selected = null;
	});

	function iconsFor(entry: Entry): string[] {
		return [inventory.iconOf(entry), inventory.item(entry.item_id)?.icon ?? ''];
	}

	function reset() {
		selected = null;
		combineWith = null;
		combineSecond = null;
	}

	function pick(entry: Entry) {
		if (combineWith) {
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

	onMount(() => {
		const param = page.url.searchParams.get('inventory');
		if (param === 'open' || param === '1' || param === 'true') {
			nav.openOverlay('inventory');
			const url = new URL(window.location.href);
			url.searchParams.delete('inventory');
			goto(url.pathname + url.search, { replaceState: true, noScroll: true, keepFocus: true });
		}
	});

	function toggle(e: MouseEvent, overlay: 'inventory' | 'avatar') {
		e.stopPropagation();
		reset();
		nav.toggleOverlay(overlay);
	}

	function showTab(overlay: 'inventory' | 'avatar') {
		reset();
		nav.openOverlay(overlay);
	}
</script>

<button class="top-toggle inventory-toggle" onclick={(e) => toggle(e, 'inventory')} aria-label={m['items.tab_items']()} aria-pressed={nav.isOpen('inventory')} aria-controls="inventory-panel">
	<img src={inventoryIcon} alt="" />
</button>
<button class="top-toggle avatar-toggle" onclick={(e) => toggle(e, 'avatar')} aria-label={m['items.tab_avatar']()} aria-pressed={nav.isOpen('avatar')} aria-controls="inventory-panel" data-testid="avatar-toggle">
	<img src={avatarIcon} alt="" />
</button>

<PullPanel
	open={inventoryVisible}
	onclose={() => nav.closeOverlay()}
	label={tab === 'avatar' ? m['items.tab_avatar']() : m['items.tab_items']()}
	id="inventory-panel"
	{keepOpen}
	testid="inventory"
>
	{#snippet header()}
	<div class="tabs" role="tablist">
		<!-- in the top row's order (avatar left of the bag, owner 2026-10-10: the tabs were "verdreht" – the other way round) -->
		<button
			role="tab"
			aria-selected={tab === 'avatar'}
			class:active={tab === 'avatar'}
			onclick={() => showTab('avatar')}><img src={avatarIcon} alt="" /><span class="tab-label">{m['items.tab_avatar']()}</span></button
		>
		<button
			role="tab"
			aria-selected={tab === 'items'}
			class:active={tab === 'items'}
			onclick={() => showTab('inventory')}><img src={inventoryIcon} alt="" /><span class="tab-label">{m['items.tab_items']()}</span></button
		>
		<!-- the reader's stats (appetite, testo) small at the right of the tab row, seen on both tabs (owner 2026-10-03) -->
		<div class="tab-stats"><ReaderStats compact /></div>
	</div>
	{/snippet}

	{#if !inventory.isAccount}
		<p class="guest">
			{m['items.guest_hint']()}
			<a href={localizeHref('/account')}>{m['items.sign_in']()}</a>
		</p>
	{/if}

	{#if tab === 'avatar'}
		<Avatar />
	{:else if combineWith && combineSecond}
		<RecipePreview
			first={combineWith}
			second={combineSecond}
			oncancel={() => (combineSecond = null)}
			ondone={() => reset()}
		/>
	{:else if selected && !combineWith}
		<ItemDetail entry={selected} onback={() => (selected = null)} oncombine={(e) => (combineWith = e)} />
	{:else if !inventory.catalog && inventory.entries.length === 0}
		<p class="empty">{inventory.catalogError ? m['items.unavailable']() : m['items.loading']()}</p>
	{:else}
		{#if listed.length === 0}
			<p class="empty">{m['inventory.empty']()}</p>
		{/if}
		{#if combineWith}
			<div class="combine-bar">
				<span>{m['items.combine_hint']({ name: inventory.nameOf(combineWith) })}</span>
				<button class="btn-secondary" onclick={() => (combineWith = null)}>{m['common.cancel']()}</button>
			</div>
		{/if}
		{#if listed.length}
			<section class="items" data-testid="inventory-items">
				{#each listed as { entry, qty } (entry.id)}
					<button
						class="item"
						data-control-exempt="inventory item card (picture + name)"
						class:source={combineWith && String(combineWith.id) === String(entry.id)}
						onclick={() => pick(entry)}
						title={inventory.nameOf(entry)}
						data-item={entry.item_id}
					>
						<ItemIcon srcs={iconsFor(entry)} name={inventory.nameOf(entry)} />
						{#if qty > 1}
							<div class="count">{qty}</div>
						{/if}
						<span class="name">{inventory.nameOf(entry)}</span>
					</button>
				{/each}
			</section>
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

	/* the panel's header: the grip above, the tabs and the stats */
	.tabs {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
		padding: 0 1rem 1rem;
	}
	/* the stats wrap below the tabs (still right) only where they don't fit beside them (≈ 320 px phones) */
	.tab-stats {
		margin-left: auto;
		align-self: center;
		flex-shrink: 0;
		color: var(--ink);
	}
	/* the switch uses the secondary (outlined) button look of the panel, like "Kombinieren" (owner 2026-09-30):
	   inactive = muted outline, active = ink outline */
	.tabs button {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0 1rem;
		background: transparent;
		color: var(--ink);
		border: 2px solid var(--border-muted);
		cursor: pointer;
	}
	.tabs button:hover,
	.tabs button.active {
		background: transparent;
		border-color: var(--ink);
	}
	/* the top buttons' icons, white on the dark panel */
	.tabs img {
		width: 1rem;
		filter: invert(1);
	}
	/* phones: icon-only tabs (the label stays for screen readers), so the stats fit beside them */
	@media (max-width: 479px) {
		.tabs button {
			width: var(--control-height);
			justify-content: center;
			padding: 0;
		}
		.tab-label {
			position: absolute;
			width: 1px;
			height: 1px;
			overflow: hidden;
			clip-path: inset(50%);
			white-space: nowrap;
		}
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

	.items {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(5rem, 1fr));
		gap: 0.5rem;
	}
	.item {
		position: relative;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.25rem;
		padding: 0.25rem;
		background: var(--paper);
		color: var(--ink);
		border: 2px solid var(--border-muted);
		cursor: pointer;
		touch-action: manipulation;
		transition: border-color 0.1s ease;
	}
	.item:hover {
		background: var(--paper);
		border-color: var(--ink);
	}
	.item.source {
		border-color: var(--ink);
		filter: drop-shadow(0 0 6px var(--primary-color));
	}
	.name {
		font-size: 0.625rem;
		line-height: 1.2;
		text-align: center;
		overflow-wrap: anywhere;
	}
	.count {
		position: absolute;
		left: 0.25rem;
		top: 0.25rem;
		padding: 0 0.25rem;
		font-size: 0.75rem;
		background: var(--paper);
		border: 1px solid black;
		border-radius: 0.5rem;
	}
</style>
