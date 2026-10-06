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
