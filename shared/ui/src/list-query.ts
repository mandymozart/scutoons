// URL-driven list state (admin UI pattern): `?page=&page_size=&sort=&q=&<filter>=`. Pure URL helpers, no routing:
// the app reads the state in its `load` / page and renders links built with `listHref`.

export interface ListState {
	page: number;
	pageSize: number;
	sort: string;
	q: string;
	/** every other non-empty query parameter */
	filters: Record<string, string>;
}

export interface ListDefaults {
	pageSize?: number;
	sort?: string;
	/** names of the filter parameters to read (others are ignored) */
	filters?: string[];
}

/** Reads the list state from a URL (page from 1, page size 1…500) */
export function readList(url: URL, defaults: ListDefaults = {}): ListState {
	const p = url.searchParams;
	const int = (value: string | null, fallback: number) => {
		const n = Number.parseInt(value ?? '', 10);
		return Number.isFinite(n) && n > 0 ? n : fallback;
	};
	const filters: Record<string, string> = {};
	for (const name of defaults.filters ?? []) {
		const value = p.get(name);
		if (value) filters[name] = value;
	}
	return {
		page: int(p.get('page'), 1),
		pageSize: Math.min(500, int(p.get('page_size'), defaults.pageSize ?? 50)),
		sort: p.get('sort') || defaults.sort || '',
		q: p.get('q') ?? '',
		filters
	};
}

/**
 * The same URL with some parameters changed (`null` / '' removes one). Changing anything but `page` resets the page,
 * so a new filter never lands on an empty page. Returns path + query (for `href` / `goto`).
 */
export function listHref(url: URL, patch: Record<string, string | number | null | undefined>): string {
	const next = new URL(url);
	for (const [key, value] of Object.entries(patch)) {
		if (value === null || value === undefined || value === '') next.searchParams.delete(key);
		else next.searchParams.set(key, String(value));
	}
	if (!('page' in patch)) next.searchParams.delete('page');
	const query = next.searchParams.toString();
	return next.pathname + (query ? `?${query}` : '');
}

/** The API query of a list state: `{ page, page_size, sort, q, …filters }` */
export function listParams(state: ListState): Record<string, string | number> {
	const params: Record<string, string | number> = { page: state.page, page_size: state.pageSize, ...state.filters };
	if (state.sort) params.sort = state.sort;
	if (state.q) params.q = state.q;
	return params;
}
