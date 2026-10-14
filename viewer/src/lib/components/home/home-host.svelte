<script lang="ts">
	import {
		fitsIn,
		pickQualityVariant,
		type Entry,
		type HomeScene,
		type ItemDef
	} from '@scutoons/content';
	import {
		HomeView,
		type HomeLabels,
		type HomePiece,
		type HomeRecipe,
		type HomeSceneFiles,
		type HomeThing
	} from '@scutoons/reader';
	import type { HomeVisitResult } from '@scutoons/api';
	import { fetchHomeVisit } from '$lib/api.ts';
	import { getHomesContext, type Home } from '$lib/contexts/homes.svelte.ts';
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import { getWorldContext } from '$lib/contexts/world.svelte.ts';
	import { mediaQuality } from '$lib/media-quality.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';

	/**
	 * The home view (agents/research/CAVE.md), mounted once in the root layout: the reader's own home (its avatar's –
	 * there, or by the VR headset) or a friend's (a visit link, read-only + the workbench). Builds the shared HomeView's
	 * input from the catalog and the inventory; furniture stays in its home (inventory.place).
	 */
	const homes = getHomesContext();
	const inventory = getInventoryContext();
	const world = getWorldContext();

	const request = $derived(homes.request);
	let note = $state('');
	let visit = $state<HomeVisitResult | null>(null);

	// a visit link: the friend's home, read-only
	$effect(() => {
		const r = request;
		note = '';
		visit = null;
		if (r?.kind !== 'visit') return;
		fetchHomeVisit(r.token).then(
			(v) => (visit = v),
			() => {
				homes.close();
				inventory.notifyMessage(m['avatar_home.visit_invalid']());
			}
		);
	});

	const own = $derived<Home | undefined>(
		request?.kind === 'own' ? homes.homeOf(inventory.currentBody?.character) : undefined
	);
	const sceneId = $derived(request?.kind === 'visit' ? visit?.home.scene : own?.scene);
	const placeId = $derived(request?.kind === 'visit' ? visit?.home.place : own?.place);
	const placeName = $derived(world?.places?.find((p) => p.id === placeId)?.name ?? placeId ?? '');

	/** the scene's files at the reader's media quality: 1080p, 4K on "high" (research/CAVE.md "Data model") */
	const scene = $derived.by<HomeSceneFiles>(() => {
		const def = (inventory.catalog?.home_scenes ?? []).find((s: HomeScene) => s.id === sceneId);
		const level = mediaQuality.level ?? 'medium';
		const file = (
			media: { src: string; variants?: Record<string, string> | null } | null | undefined
		) => (media?.src ? asset(pickQualityVariant(media, level)) : '');
		return { image: file(def?.image), video: file(def?.video) || null };
	});

	const item = (id: string): ItemDef | undefined => inventory.item(id);
	const thing = (e: Entry): HomeThing => ({
		id: String(e.id),
		name: inventory.nameOf(e),
		icon: asset(inventory.iconOf(e)),
		furniture: !!item(e.item_id)?.furniture
	});
	const piece = (e: Entry, contents: HomeThing[]): HomePiece => {
		const def = item(e.item_id);
		return {
			id: String(e.id),
			name: inventory.nameOf(e),
			image: asset(def?.furniture?.image || inventory.iconOf(e)),
			size: def?.furniture?.size ?? 15,
			x: e.home?.x ?? 50,
			y: e.home?.y ?? 80,
			flip: !!e.home?.flip,
			workshop: !!def?.furniture?.workshop,
			holds: def?.holds ? { max: def.holds.max ?? null } : null,
			contents
		};
	};

	const entryById = (id: string) => inventory.entries.find((e) => String(e.id) === id);

	const furniture = $derived.by<HomePiece[]>(() => {
		if (request?.kind === 'visit') {
			return (visit?.furniture ?? []).map((f, i) => {
				const e = {
					id: `visit-${i}`,
					item_id: f.item_id,
					variant: f.variant,
					home: { id: 'visit', x: f.x, y: f.y, flip: f.flip }
				} as unknown as Entry;
				const contents = f.contents.map((c, k) =>
					thing({
						id: `visit-${i}-${k}`,
						item_id: c.item_id,
						variant: c.variant
					} as unknown as Entry)
				);
				return piece(e, contents);
			});
		}
		if (!own) return [];
		return inventory.entries
			.filter((e) => e.home?.id === own.id)
			.map((e) => piece(e, inventory.contentsOf(e).map(thing)));
	});

	/** what the reader carries: loose, not worn, not standing in a home */
	const carried = $derived(
		inventory.entries.filter((e) => !e.home && e.in == null && !inventory.isEquipped(e)).map(thing)
	);

	const workshop = $derived(
		request?.kind === 'visit' ? !!visit?.workshop : furniture.some((p) => p.workshop)
	);
	const recipes = $derived<HomeRecipe[]>(
		workshop
			? (inventory.catalog?.recipes ?? [])
					.filter((r) => r.at === 'workshop')
					.map((r) => ({
						id: r.id,
						name: inventory.nameOf({ item_id: r.output.item, variant: null }),
						possible: inventory.canCraft(r)
					}))
			: []
	);

	const labels = $derived<HomeLabels>({
		leave: m['avatar_home.leave'](),
		visitLeave: m['avatar_home.visit_leave'](),
		inventory: m['avatar_home.inventory'](),
		visiting: m['avatar_home.visiting'](),
		count: m['avatar_home.count']({ count: '{count}', max: '{max}' }),
		putInHint: m['avatar_home.put_in_hint']({ name: '{name}' }),
		placeHint: m['avatar_home.place_hint'](),
		turn: m['avatar_home.turn'](),
		workshop: m['avatar_home.workshop'](),
		noRecipes: m['avatar_home.no_recipes'](),
		dragHint: m['avatar_home.drag_hint']({ name: '{name}' }),
		room: m['avatar_home.room']()
	});

	const where = $derived(
		request?.kind === 'visit'
			? m['avatar_home.where_visit']({ place: placeName })
			: request?.how === 'vr'
				? m['avatar_home.where_vr']({ place: placeName })
				: m['avatar_home.where_own']({ place: placeName })
	);

	const ERRORS: Record<string, () => string> = {
		furniture_stays: m['avatar_home.err_furniture_stays'],
		does_not_fit: m['avatar_home.err_does_not_fit'],
		container_full: m['avatar_home.err_container_full'],
		worn: m['avatar_home.err_worn'],
		in_container: m['avatar_home.err_in_container'],
		needs_workshop: m['avatar_home.err_needs_workshop']
	};
	const say = (result: { ok: boolean; error?: string }) =>
		(note = result.ok ? '' : (ERRORS[result.error ?? ''] ?? m['avatar_home.err_failed'])());

	async function place(t: HomeThing | HomePiece, x: number, y: number, flip = false) {
		const e = entryById(t.id);
		if (e && own) say(await inventory.place(e, { id: own.id, x, y, flip }));
	}
	async function putIn(t: HomeThing, p: HomePiece) {
		const [e, into] = [entryById(t.id), entryById(p.id)];
		if (e && into) say(await inventory.move(e, into));
	}
	async function takeOut(t: HomeThing) {
		const e = entryById(t.id);
		if (e) say(await inventory.move(e, null));
	}
	async function craft(r: HomeRecipe) {
		const recipe = (inventory.catalog?.recipes ?? []).find((x) => x.id === r.id);
		if (!recipe) return;
		const at =
			request?.kind === 'visit' ? { visit: request.token } : own ? { home: own.id } : undefined;
		const result = await inventory.craft(recipe, {}, at);
		say(result);
		if (result.ok) note = m['avatar_home.repaired']({ name: r.name });
	}
	const fits = (t: HomeThing, p: HomePiece) => {
		const [e, into] = [entryById(t.id), entryById(p.id)];
		return !!e && !!into && fitsIn(item(into.item_id), item(e.item_id));
	};

	// the reader's avatar has no home place yet
	$effect(() => {
		if (request?.kind === 'own' && inventory.registryLoaded && !own) {
			homes.close();
			inventory.notifyMessage(m['avatar_home.no_home']());
		}
	});
</script>

{#if request && (request.kind === 'own' ? own : visit)}
	<HomeView
		{scene}
		avatar={request.kind === 'own' && inventory.currentBody
			? asset(inventory.currentBody.path)
			: null}
		{furniture}
		{carried}
		{recipes}
		{where}
		visitor={request.kind === 'visit'}
		{note}
		{labels}
		{fits}
		onplace={(t, x, y) => place(t, x, y)}
		onmove={(p, x, y, flip) => place(p, x, y, flip)}
		onputin={putIn}
		ontakeout={(t) => takeOut(t)}
		oncraft={craft}
		onclose={homes.close}
	/>
{/if}
