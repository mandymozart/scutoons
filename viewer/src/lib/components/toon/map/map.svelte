<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { unlocksFromProgress, type ReaderMap } from '@scutoons/content';
	import { fetchMap } from '$lib/api.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { getNavigationContext } from '$lib/contexts/navigation.svelte';
	import { getGameContext } from '$lib/contexts/game.svelte.ts';
	import { getWorldContext } from '$lib/contexts/world.svelte.ts';
	import mapIcon from '$lib/icons/map.svg';
	import MapOverlay from './map-overlay.svelte';

	/**
	 * The world map in the toon pages (agents/MAP.md M1): the round toggle (key M) with a badge while something new is on
	 * the map, and the overlay – mounted the first time it opens (the terrain SVG is 337 KB). The world's data
	 * (`/library/map`, a few KB) loads when the browser is idle: the map's unlock points turn the reading progress into
	 * discoveries (`unlocksFromProgress` → the world state, universe-wide) before the story events exist (M3).
	 */
	const nav = getNavigationContext();
	const game = getGameContext();
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

	// the places' unlock slices for the reading tracker (a seen slice unlocks its places, world.sliceSeen)
	$effect(() => {
		if (data && world) world.setUnlockRules(data.locations);
	});

	// reading progress → discoveries (set operations: nothing happens twice)
	$effect(() => {
		if (!data || !world || !game) return;
		const ids = unlocksFromProgress(data.locations, toon, game.state.progress);
		if (ids.some((id) => !world.map.places[id])) world.unlock(ids);
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

{#if mounted}
	<MapOverlay {data} {failed} {open} {toon} onclose={() => nav.closeOverlay()} />
{/if}

<style>
	.map-toggle {
		position: fixed;
		top: 0.5rem;
		/* the 3rd of the top-right group (main-menu.svelte): one control height (owner 2026-10-02) */
		right: calc(0.5rem + 2 * (var(--control-height) + 0.5rem));
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
