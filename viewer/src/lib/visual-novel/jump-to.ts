import type { JumpToTarget } from '@scutoons/content';

export function buildJumpToUrl(target: JumpToTarget, toonId: string): string | null {
	let url: string;
	console.log('Building jumpTo URL for target', target, 'and toonId', toonId);
	switch (target.type) {
		case 'chapter':
			url = `/${toonId}/chapter/${target.value}`;
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
	
	if (target.label) {
		url += `?label=${target.label}`;
	}
	
	return url;
}
