import { intoOtherToon, pickJumpLabel, type JumpToTarget } from '@scutoons/content';

/**
 * The URL of a jump. `anyOf` (CUTOVER 10.27, the chapter 2 stroll): a random one of these group labels, never
 * `currentLabel` (the group the reader is on). A chapter target with `toon` (owner 2026-10-06, "at label") leads into
 * that toon's chapter (+ `?label=`) – the story variables travel along (one save game for every toon, DATA.md "The save
 * game"); `currentLabel` doesn't count there.
 */
export function buildJumpToUrl(target: JumpToTarget, toonId: string, currentLabel: string | null = null): string | null {
	let url: string;
	switch (target.type) {
		case 'chapter':
			url = `/${intoOtherToon(target, toonId) ? target.toon : toonId}/chapter/${target.value}`;
			break;
		case 'season':
			url = `/${toonId}/season/${target.value}`;
			break;
		case 'toon':
			url = `/${target.value}`;
			break;
		case 'product':
			url = `/shop/product/${target.value}`;
			break;
		case 'xr':
			url = `/xr/${target.value}`;
			break;
		default:
			return null;
	}
	
	const label = target.anyOf?.length ? pickJumpLabel(target.anyOf, intoOtherToon(target, toonId) ? null : currentLabel) : target.label;
	if (label) {
		url += `?label=${encodeURIComponent(label)}`;
	}
	
	return url;
}

/** where a page is (the chapter page tells the VN context): a jump within this chapter can be checked against it */
export interface JumpPlace {
	toon: string;
	chapter: number;
	/** the chapter's group labels (its scenes' included) */
	labels: readonly string[];
}

/**
 * Why a jump can't go where it says – a group label (or every `anyOf` label) that isn't in THIS chapter – or null: it
 * can, or it leads somewhere this page can't judge (another chapter or toon: validation refuses those when they are
 * missing, the page there shows the chapter from its start). The runner then stays (owner 2026-10-06: no dead ends –
 * the follow-up bar comes instead of a page that isn't there).
 */
export function jumpProblem(target: JumpToTarget | null | undefined, place: JumpPlace | null | undefined): string | null {
	if (!target || !place || target.type !== 'chapter') return null;
	if (intoOtherToon(target, place.toon) || Number(target.value) !== place.chapter) return null;
	if (target.anyOf?.length) {
		const there = target.anyOf.filter((l) => place.labels.includes(l));
		return there.length ? null : `none of the groups ${target.anyOf.map((l) => `"${l}"`).join(', ')} is in this chapter`;
	}
	if (target.label && !place.labels.includes(target.label)) return `no group "${target.label}" in this chapter`;
	return null;
}
