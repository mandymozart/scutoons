import { shared } from '$lib/utils/shared.ts';

/**
 * Where this member was last (CREATOR-NAV.md §2a): the phone menu's "Recent" and the dashboard's "Continue". Local
 * first (this browser, `localStorage`) – one entry per page (pathname), the newest first; the chapter editor's entry
 * keeps the group / slice of its URL, so Continue opens the slice. The layout records every place below an area's top.
 */
export interface Place {
	/** pathname + search */
	href: string;
	/** the path's labels below the area (Cat Pope / Chapter 2 / scene-17-24 / Slice 20) */
	trail: string[];
	/** the toon, if the place belongs to one */
	toon: string | null;
	/** ms since the epoch */
	at: number;
}

const KEY = 'sc-creator-recent';
const MAX = 8;

function load(): Place[] {
	try {
		const list = JSON.parse(localStorage.getItem(KEY) ?? '[]');
		return Array.isArray(list) ? list.filter((p) => typeof p?.href === 'string' && Array.isArray(p?.trail)).slice(0, MAX) : [];
	} catch {
		return [];
	}
}

const path = (href: string) => href.split('?')[0];

class Recent {
	list = $state<Place[]>(typeof localStorage === 'undefined' ? [] : load());

	visit(place: Omit<Place, 'at'>) {
		if (!place.trail.length) return;
		const first = this.list[0];
		// the same page with the same path: nothing new (a re-render, a reload)
		if (first && first.href === place.href && first.trail.join('/') === place.trail.join('/')) return;
		this.list = [{ ...place, at: Date.now() }, ...this.list.filter((p) => path(p.href) !== path(place.href))].slice(0, MAX);
		try {
			localStorage.setItem(KEY, JSON.stringify(this.list));
		} catch {
			// private mode, full storage: Recent stays for this visit only
		}
	}
}

export const recent = shared('recent', () => new Recent());
