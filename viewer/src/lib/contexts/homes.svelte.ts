// The avatar's home (agents/research/CAVE.md): which homes the reader has, who lives where, and the open home view.
// Members: the server (`GET /homes/`, made per avatar on the way); guests: on the device – every unlocked avatar with a
// home place has its own home (`guest:<place>`), no WG, no visit links (they need an account). Furniture placement is
// the inventory's (`inventory.place`, `Entry.home`).
import { getContext, setContext } from 'svelte';
import { characterHome, type RegistryCharacter } from '@scutoons/content';
import type { HomesResult } from '@scutoons/api';
import { fetchHomes } from '$lib/api.ts';
import type { InventoryContext } from './inventory.svelte.ts';

const HOMES_KEY = Symbol('homes');

export interface Home {
	id: string;
	place: string;
	scene: string;
	residents: string[];
}

/** what the home view shows: the reader's own home (there, or by the VR headset) or a friend's (a visit link) */
export type HomeRequest = { kind: 'own'; how: 'there' | 'vr' } | { kind: 'visit'; token: string };

export function createHomesContext(inventory: InventoryContext) {
	let server = $state<HomesResult | null>(null);
	let request = $state<HomeRequest | null>(null);

	const characters = $derived(
		new Map<string, RegistryCharacter>((inventory.registry?.characters ?? []).map((c) => [c.id, c]))
	);

	const homes = $derived.by<Home[]>(() => {
		if (inventory.isAccount) return server?.homes ?? [];
		const out = new Map<string, Home>();
		for (const unlock of inventory.unlocked) {
			const { place, scene } = characterHome(characters.get(unlock.character));
			if (!place) continue;
			const home = out.get(place) ?? { id: `guest:${place}`, place, scene, residents: [] };
			home.residents.push(unlock.character);
			out.set(place, home);
		}
		return [...out.values()];
	});

	/** the members' homes from the server (made for every avatar with a home place) */
	async function load() {
		await inventory.loadRegistry();
		if (inventory.isAccount) server = await fetchHomes().catch(() => server);
	}

	return {
		get homes() {
			return homes;
		},
		get maxResidents() {
			return server?.max_residents ?? 5;
		},
		get mover() {
			return server?.mover ?? { item: 'cucumber', qty: 50 };
		},
		/** where an avatar lives (none = it has no home place yet) */
		homeOf: (character: string | undefined) =>
			homes.find((h) => !!character && h.residents.includes(character)),
		get request() {
			return request;
		},
		load,
		/** "Eintreten" at the home place, or the VR headset from anywhere (research/CAVE.md D2) */
		openOwn: async (how: 'there' | 'vr') => {
			await load();
			request = { kind: 'own', how };
		},
		openVisit: (token: string) => (request = { kind: 'visit', token }),
		close: () => (request = null),
		/** the server's answer after a move-in (WG, mover) */
		apply: (result: HomesResult) => (server = result)
	};
}

export type HomesContext = ReturnType<typeof createHomesContext>;

export function setHomesContext(inventory: InventoryContext) {
	const context = createHomesContext(inventory);
	setContext(HOMES_KEY, context);
	return context;
}

export function getHomesContext() {
	return getContext<HomesContext>(HOMES_KEY);
}
