<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import type { ReaderMap } from '@scutoons/content';
	import { fetchMap } from '$lib/api.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { getNavigationContext } from '$lib/contexts/navigation.svelte';
	import { getWorldContext } from '$lib/contexts/world.svelte.ts';
	import mapIcon from '$lib/icons/map.svg';
	import MapOverlay from './map-overlay.svelte';
	import { onPlaceUnlocked, replayPlaceReveal, type RevealRequest } from './map-reveal.ts';
	import { onMigrate, type MigrateRequest } from './region-welcome.ts';
	import RegionWelcome from './region-welcome.svelte';
	import { fetchRegistry } from '$lib/api.ts';
	import { characterImage, isPlaceVisible, mapName, type CharacterRegistry, type ReaderRegion } from '@scutoons/content';
	import { getCurrentInventory } from '$lib/contexts/inventory.svelte.ts';

	/**
	 * The world map in the toon pages (agents/MAP.md M1): the round toggle (key M) with a badge while something new is on
	 * the map, and the overlay – mounted the first time it opens (the terrain SVG is 337 KB). The world's data
	 * (`/library/map`, a few KB) loads when the browser is idle: the map's unlock points turn the reading progress into
	 * discoveries (`unlocksFromProgress` → the world state, universe-wide) before the story events exist (M3).
	 */
	const nav = getNavigationContext();
	const world = getWorldContext();
	const toon = $derived(page.params.toonId as string);
	const open = $derived(nav.isOpen('map'));

	let data = $state<ReaderMap | null>(null);
	let failed = $state(false);
	let mounted = $state(false);
	let loading: Promise<void> | null = null;
	let loadedFor = '';

	function load(): Promise<void> {
		const key = `${toon}:${getLocale()}`;
		if (loading && loadedFor === key) return loading;
		loadedFor = key;
		failed = false;
		loading = fetchMap(getLocale(), toon)
			.then((answer) => {
				if (loadedFor === key) data = answer;
			})
			.catch(() => {
				if (loadedFor === key) failed = true;
			});
		return loading;
	}

	onMount(() => {
		const idle = (globalThis as { requestIdleCallback?: (cb: () => void) => number }).requestIdleCallback;
		if (idle) idle(() => load());
		else setTimeout(load, 1500);
	});
	// another toon or language: the toon's view (and names) change
	$effect(() => {
		void toon;
		if (loadedFor && loadedFor !== `${toon}:${getLocale()}`) load();
	});
	$effect(() => {
		if (open) {
			mounted = true;
			load();
		}
	});

	// the places' names for the texts (map.total / map.here)
	$effect(() => {
		if (data && world) world.setPlaces(data.locations);
	});

	$effect(() => {
		function onKey(e: KeyboardEvent) {
			const target = e.target as HTMLElement | null;
			if (target?.closest('input, textarea, select, [contenteditable]')) return;
			if ((e.key === 'm' || e.key === 'M') && !e.ctrlKey && !e.metaKey && !e.altKey) nav.toggleOverlay('map');
		}
		window.addEventListener('keydown', onKey);
		return () => window.removeEventListener('keydown', onKey);
	});

	// ── the story unlocked a place (map-reveal.ts, agents/MAP.md §8.5a): the toast, and – when the run may open the map –
	//    the reveal; one after another, the story waits for each ─────────────────────────────────────────────────
	let reveal = $state<RevealRequest | null>(null);
	/** the map was open when a replay started (a new place tapped in it): it stays open afterwards */
	let stayOpen = $state(false);
	const queue: RevealRequest[] = [];
	$effect(() =>
		onPlaceUnlocked((request) => {
			queue.push(request);
			void next();
		})
	);
	async function next() {
		if (reveal || !queue.length) return;
		const request = queue.shift()!;
		await load();
		const place = data?.locations.find((p) => p.id === request.id);
		// the story's setAvatarLocation: the avatar stands there – only at a place the reader can see (no spoilers)
		if (request.avatar) {
			if (place && world && isPlaceVisible(place, world.map)) world.avatarTo({ location: place.id });
			request.done();
			return void next();
		}
		// the reader asked to see an unlocked place again (its toast, the new place in the map): no unlock, no toast
		if (request.replay) {
			if (!place || !world?.map.places[place.id]) {
				request.done();
				return void next();
			}
			stayOpen = open;
			reveal = request;
			mounted = true;
			nav.openOverlay('map');
			return;
		}
		// not on the map (deleted, mistyped – or the map could not load): the event fizzles, nothing happens
		const added = place && world ? world.unlock([place.id]) : [];
		if (!added.length || request.silent) {
			request.done();
			return void next();
		}
		getCurrentInventory()?.notifyPlace(place!.name, place!.image?.src, place!.id);
		if (!request.reveal) {
			request.done();
			return void next();
		}
		reveal = request;
		mounted = true;
		nav.openOverlay('map');
	}
	/** `kept`: the reader touched the map during the reveal – it stays open (owner 2026-10-12) */
	function revealed(kept = false) {
		const request = reveal;
		reveal = null;
		if (!stayOpen && !kept) nav.closeOverlay();
		stayOpen = false;
		request?.done();
		void next();
	}

	// ── the story's `migrate` (agents/MAP.md §8.10): registered in the save game, then the welcome – the region's sign,
	//    its representative (a registry character) saying the greeting, or the narrator; one after another ──────────
	let welcome = $state<{ region: ReaderRegion; representative: { name: string; portrait: string; color?: string } | null; done: () => void } | null>(null);
	const moves: MigrateRequest[] = [];
	let registry: Promise<CharacterRegistry | null> | null = null;
	$effect(() =>
		onMigrate((request) => {
			moves.push(request);
			void nextMove();
		})
	);
	async function nextMove() {
		if (welcome || !moves.length) return;
		const request = moves.shift()!;
		await load();
		const region = data?.regions?.find((r) => r.id === request.region);
		// not on the map (deleted, mistyped – or the map could not load): the event fizzles
		if (!region || !world) {
			request.done();
			return void nextMove();
		}
		world.migrate(request.who, region.id);
		if (request.silent) {
			request.done();
			return void nextMove();
		}
		registry ??= fetchRegistry().catch(() => null);
		const reg = await registry;
		const rep = region.representative ? reg?.characters.find((c) => c.id === region.representative) : undefined;
		const portrait = rep ? characterImage(rep, {}, 'portrait') : '';
		welcome = {
			region,
			representative: rep && portrait ? { name: mapName(rep.name as Record<string, string>, getLocale()) || rep.id, portrait, color: rep.color } : null,
			done: request.done
		};
	}
	function welcomed() {
		const done = welcome?.done;
		welcome = null;
		done?.();
		void nextMove();
	}

	function toggle(e: MouseEvent) {
		e.stopPropagation();
		nav.toggleOverlay('map');
	}
</script>

<button
	class="map-toggle"
	onclick={toggle}
	aria-label={open ? m['map.close']() : m['map.open']()}
	aria-expanded={open}
	title={world?.hasNew ? m['map.new_badge']() : m['map.title']()}
	data-testid="map-toggle"
>
	<img src={mapIcon} alt="" />
	{#if world?.hasNew}<span class="badge" data-testid="map-badge"></span>{/if}
</button>

{#if welcome}
	<RegionWelcome region={welcome.region} representative={welcome.representative} ondone={welcomed} />
{/if}

{#if mounted}
	<MapOverlay {data} {failed} {open} {toon} onclose={() => nav.closeOverlay()} reveal={reveal?.id ?? null} revealStays={stayOpen} onrevealed={revealed} onreplay={(id) => replayPlaceReveal(id)} />
{/if}

<style>
	.map-toggle {
		position: fixed;
		top: 0.5rem;
		/* the 3rd of the top-right group (main-menu.svelte): one control height (owner 2026-10-02) */
		right: calc(0.5rem + 3 * (var(--control-height) + 0.5rem));
		z-index: 1001;
		background: white;
		border: 2px solid black;
		padding: 0;
		border-radius: 50%;
		box-sizing: border-box;
		height: var(--control-height);
		width: var(--control-height);
		display: flex;
		cursor: pointer;
		transition: background 0.2s ease;
		align-items: center;
		justify-content: center;
		touch-action: manipulation;
		img {
			width: 1rem;
		}
		&:hover {
			background: var(--primary-color);
		}
	}
	/* something new on the map: a cyan dot with an ink border (no animation, agents/MAP.md §3.4) */
	.badge {
		position: absolute;
		top: -3px;
		right: -3px;
		width: 10px;
		height: 10px;
		border-radius: 50%;
		background: var(--primary-cyan, #00d1ff);
		border: 2px solid var(--ink, #000);
		box-sizing: content-box;
	}
</style>
