// A shot hands over (research/redesign/lap2/STORYBOARD.md "Where a shot goes"): its asset as a Library address, what it
// was made into (the deck scene, the XR experience), its own address for the deck, and where a new slice cut from it goes
// in the chapter. The patterns are the ones types.ts gives the generated schema (test/storyboard-handover.test.ts holds
// them equal). Pure – no Node, no Svelte.

/** the file kinds a shot's slot takes */
export const SHOT_ASSET_KINDS = ['image', 'video'] as const;
export type ShotAssetKind = (typeof SHOT_ASSET_KINDS)[number];

/** `Shot.asset`: a file's `image:<path>` / `video:<path>` or a generator's `generator:<id>` (types.ts `@pattern`) */
export const SHOT_ASSET = /^((image|video):[^\s]+|generator:[a-z0-9][a-z0-9._-]*)$/;
/** `Shot.made`: which kind of address each place takes (types.ts `@pattern`) */
export const SHOT_MADE = { deck: /^scene:[^\s]+$/, xr: /^experience:[a-z][a-z0-9-]{1,63}$/ } as const;
export type ShotMadeAt = keyof typeof SHOT_MADE;

/** what a shot's asset address names */
export type ShotAsset = { at: 'file'; kind: ShotAssetKind; path: string; address: string } | { at: 'generator'; id: string; address: string };

/** a shot's asset address read; not one: null */
export function readShotAsset(address: unknown): ShotAsset | null {
	if (typeof address !== 'string' || !SHOT_ASSET.test(address)) return null;
	const at = address.indexOf(':');
	const kind = address.slice(0, at), rest = address.slice(at + 1);
	return kind === 'generator' ? { at: 'generator', id: rest, address } : { at: 'file', kind: kind as ShotAssetKind, path: rest, address };
}

/** the asset address of a Library file (its kind and its main file's CDN-root path); a kind the slot doesn't take: null */
export function fileShotAsset(kind: string, path: string): string | null {
	const address = `${kind}:${path}`;
	return (SHOT_ASSET_KINDS as readonly string[]).includes(kind) && SHOT_ASSET.test(address) ? address : null;
}

/** a made thing's address fits its place */
export const fitsMade = (at: ShotMadeAt, address: unknown): address is string => typeof address === 'string' && SHOT_MADE[at].test(address);

/**
 * The shot's own address, `shot:<toon>:<chapter>:<id>`: what a link carries to the deck while the shot has no deck scene,
 * so the deck makes one from it (#873) and writes its `scene:<id>` into the shot's `made.deck`.
 */
export const shotAddress = (toon: string, chapter: number, shot: string): string => `shot:${toon}:${chapter}:${shot}`;

/** `shot:<toon>:<chapter>:<id>` read; anything else: null */
export function readShotAddress(address: string): { toon: string; chapter: number; shot: string } | null {
	const m = /^shot:([a-z0-9][a-z0-9_-]*):(0|[1-9]\d*):(sb-[^\s:]+)$/.exec(address);
	return m ? { toon: m[1], chapter: Number(m[2]), shot: m[3] } : null;
}

/** the parts of a chapter a slice placement reads: its groups (named by their scene, `scene-<n>`), their slices and the shot each was cut from */
export interface SliceGroups {
	groups: readonly { id?: string; slices?: readonly { shot?: string }[] }[];
}

/** where a slice sits: its group's index and its index in the group */
export interface SliceSpot {
	group: number;
	slice: number;
}

/** the first slice cut from `shot`; none: null */
export function sliceOfShot(chapter: SliceGroups, shot: string): SliceSpot | null {
	for (const [group, g] of chapter.groups.entries()) {
		const slice = (g.slices ?? []).findIndex((s) => s.shot === shot);
		if (slice >= 0) return { group, slice };
	}
	return null;
}

/**
 * Where a new slice cut from `shot` goes: after the last slice of the nearest earlier shot (the storyboard's `order`) that
 * has one, else before the first slice of the nearest later shot that has one, else at the end of the shot's scene's
 * group (`scene`: its name, validate.ts sceneName), else at the end of the last group. A chapter without groups: null.
 */
export function sliceSpotFor(chapter: SliceGroups, order: readonly string[], shot: string, scene?: string): SliceSpot | null {
	if (!chapter.groups.length) return null;
	const at = order.indexOf(shot);
	const last = (id: string): SliceSpot | null => {
		for (let group = chapter.groups.length - 1; group >= 0; group--) {
			const slices = chapter.groups[group].slices ?? [];
			for (let slice = slices.length - 1; slice >= 0; slice--) if (slices[slice].shot === id) return { group, slice: slice + 1 };
		}
		return null;
	};
	if (at >= 0) {
		for (let i = at - 1; i >= 0; i--) {
			const spot = last(order[i]);
			if (spot) return spot;
		}
		for (let i = at + 1; i < order.length; i++) {
			const spot = sliceOfShot(chapter, order[i]);
			if (spot) return spot;
		}
	}
	const own = scene ? chapter.groups.findIndex((g) => g.id === scene) : -1;
	const group = own >= 0 ? own : chapter.groups.length - 1;
	return { group, slice: (chapter.groups[group].slices ?? []).length };
}
