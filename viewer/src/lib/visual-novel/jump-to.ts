import type { JumpToTarget } from '$lib/models/content.ts';

export function buildJumpToUrl(target: JumpToTarget, toonId: string): string | null {
	let url: string;
	
	switch (target.type) {
		case 'chapter':
			url = `/${toonId}/chapter/${target.value}`;
			break;
		case 'song':
			url = `/${toonId}/song/${target.value}`;
			break;
		case 'season':
			url = `/${toonId}/season/${target.value}`;
			break;
		case 'toon':
			url = `/${toonId}`;
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
	
	if (target.label) {
		url += `?label=${target.label}`;
	}
	
	return url;
}
