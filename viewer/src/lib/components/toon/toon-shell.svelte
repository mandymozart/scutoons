<script lang="ts">
	import type { Snippet } from 'svelte';
	import SequenceNavigation from '$lib/components/toon/sequence-navigation.svelte';
	import { onDestroy } from 'svelte';
	import { setAudioContext } from '$lib/contexts/audio.svelte.ts';
	import { setVisualNovelContext } from '$lib/contexts/visual-novel.svelte';
	import Map from '$lib/components/toon/map/map.svelte';
	import Inventory from '$lib/components/toon/inventory/inventory.svelte';
	import { setGameContext } from '$lib/contexts/game.svelte.ts';
	import { setNavigationContext } from '$lib/contexts/navigation.svelte.ts';
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { getMembershipContext } from '$lib/contexts/membership.svelte.ts';
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import { getWorldContext } from '$lib/contexts/world.svelte.ts';
	import { createReaderExpressions, lazyWorldPlaces, setReaderExpressions } from '$lib/expressions/resolver.svelte.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { isReadable } from '$lib/draft-preview.svelte.ts';
	import { eventHistoryKey, type Character, type LoadedSequenceItem } from '@scutoons/content';
	import { afterNavigate } from '$app/navigation';
	import { readerName } from '$lib/reader-name.svelte.ts';
	import { pageView } from '$lib/visual-novel/page-view.ts';
	import { chromeFollowsScroll, holdChrome } from '$lib/chrome.ts';

	/**
	 * One toon's reader shell (its VN, audio, sequence, characters, reader expressions, map, inventory) – the toon layout
	 * mounts it once per toon (`{#key params.toonId}`): a jump into another toon (`JumpToTarget.toon`) gets that toon's
	 * sequence, cast and audio instead of the one it came from. The save game is ONE for every toon (owner 2026-10-06,
	 * contexts/world.svelte.ts): `setGameContext` is only this toon's view on it – the story variables travel along.
	 */
	let { children, data, params }: { children: Snippet; data: { sequence?: LoadedSequenceItem[]; characters?: Character[] }; params: { toonId: string } } = $props();
	// mounted once per toon (the layout's {#key}): the toon's initial values are the right ones
	// svelte-ignore state_referenced_locally
	let sequence = data.sequence || [];

	// svelte-ignore state_referenced_locally
	const audio = setAudioContext(params.toonId);
	const vn = setVisualNovelContext();
	// svelte-ignore state_referenced_locally
	const game = setGameContext(params.toonId);
	const nav = setNavigationContext();

	// expressions in texts (agents/EXPRESSIONS.md §2): the reader's state as the texts may read it – never email, codes or ids
	const session = getSessionContext();
	const membership = getMembershipContext();
	const inventory = getInventoryContext();
	const world = getWorldContext();
	setReaderExpressions(
		createReaderExpressions(
			{
				toon: game.state.toonId,
				variable: (name) => game.getVariable(name),
				signedIn: () => !!session?.isLoggedIn,
				member: () => !!(membership?.isValid ?? session?.hasAccess),
				accountCreated: () => session?.user?.created_at,
				language: () => getLocale(),
				startedAt: () => game.state.created_at,
				checkpoints: () => game.state.progress,
				chapters: () => sequence.filter((item: LoadedSequenceItem) => isReadable(item)).map((item: LoadedSequenceItem) => item.number),
				entries: () => inventory?.entries ?? [],
				wears: (item) => (inventory?.entries ?? []).some((e) => e.item_id === item && inventory.isEquipped(e)),
				body: () => inventory?.currentBody ?? inventory?.body,
				unlockedCharacters: () => inventory?.unlocked.length ?? 0,
				stat: (id) => inventory?.stats.find((s) => s.id === id)?.level,
				effects: () => (inventory?.effects ?? []).map((e) => e.kind),
				map: () => world?.map,
				places: () => world?.places ?? lazyWorldPlaces(getLocale()),
				name: () => readerName.value,
				eventCount: (id) => game.visits(eventHistoryKey(id))
			},
			game,
			() => getLocale()
		)
	);

	// svelte-ignore state_referenced_locally
	vn.setCharacters(data.characters || []);
	// every navigation counts as a page view (events "once per page view", items "every visit") – also coming back
	afterNavigate(() => void pageView());

	// the top row slides away while reading (lib/chrome.ts), never while an overlay is open
	$effect(() => chromeFollowsScroll());
	$effect(() => {
		if (nav.activeOverlay) return holdChrome();
	});

	onDestroy(async () => {
		await audio.unload();
	});
</script>

{@render children()}

<SequenceNavigation {sequence} {params} />
<Map />
<Inventory />
