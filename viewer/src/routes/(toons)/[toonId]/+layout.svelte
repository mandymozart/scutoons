<script lang="ts">
	import SequenceNavigation from '$lib/components/toon/sequence-navigation.svelte';
	import { onDestroy } from 'svelte';
	import { setAudioContext } from '$lib/contexts/audio.svelte.ts';
	import { setVisualNovelContext } from '$lib/contexts/visual-novel.svelte';
	import Map from '$lib/components/toon/map/map.svelte';
	import Inventory from '$lib/components/toon/inventory/inventory.svelte';
	import ReaderEffect from '$lib/components/items/reader-effect.svelte';
	import { setGameContext } from '$lib/contexts/game.svelte.ts';
	import { setNavigationContext } from '$lib/contexts/navigation.svelte.ts';
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { getMembershipContext } from '$lib/contexts/membership.svelte.ts';
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import { getWorldContext } from '$lib/contexts/world.svelte.ts';
	import { createReaderExpressions, setReaderExpressions } from '$lib/expressions/resolver.svelte.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { isReadable } from '$lib/draft-preview.svelte.ts';
	import type { LoadedSequenceItem } from '@scutoons/content';

	let { children, data, params } = $props();
	let sequence = data.sequence || [];

	const audio = setAudioContext(params.toonId);
	const vn = setVisualNovelContext();
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
				places: () => world?.places
			},
			game,
			() => getLocale()
		)
	);

	vn.setCharacters(data.characters || []);

	onDestroy(async () => {
		await audio.unload();
	});
</script>

{@render children()}

<SequenceNavigation {sequence} {params} />
<Map />
<Inventory />
<ReaderEffect />
