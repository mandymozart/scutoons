/**
 * The ONE place where a content entity's storage key (today a content-store path like `chapter2.json`) becomes the
 * name people see (owner 2026-09-30: no file names in the UI – content moves to the API/database, the data refactor
 * can switch the keys here without touching any label). Apps pass their translations; defaults are English.
 */
export interface EntityLabels {
	chapter: (number: number) => string;
	season: (number: number) => string;
	/** a scene stored on its own (a chapter's referenced group) */
	scene: (label: string) => string;
	/** the toon's settings (title, covers, map, downloads) */
	toon: string;
	/** the toon's languages (its index entry) */
	languages: string;
	characters: string;
	moods: string;
	requests: string;
	cast: string;
	credits: string;
	people: string;
	roles: string;
}

export const ENTITY_LABELS: EntityLabels = {
	chapter: (n) => `Chapter ${n}`,
	season: (n) => `Season ${n}`,
	scene: (label) => `Scene ${label}`,
	toon: 'Toon settings',
	languages: 'Languages',
	characters: 'Characters',
	moods: 'Moods',
	requests: 'Wanted images',
	cast: 'Cast',
	credits: 'Credits',
	people: 'People',
	roles: 'Roles'
};

const NAMED: Record<string, keyof EntityLabels> = {
	'toon.json': 'toon',
	'index.json': 'languages',
	'characters.json': 'characters',
	'moods.json': 'moods',
	'asset-requests.json': 'requests',
	'cast.json': 'cast',
	'credits.json': 'credits',
	'people.json': 'people',
	'roles.json': 'roles'
};

/** `chapter2.json` → "Chapter 2", `season1.json` → "Season 1", `chapter2/scene-14.json` → "Scene 14" … */
export function entityLabel(key: string, labels: Partial<EntityLabels> = {}): string {
	const l = { ...ENTITY_LABELS, ...labels };
	// a toon / locale prefix (`catpope/_/cast.json`, `de/chapter1.json`) is not part of the entity
	const path = key.replace(/^(?:[a-z0-9_-]+\/)?(?:[a-z]{2}|_)\/(?=[^/]+(?:\/[^/]+)?\.json$)/, '');
	let match = /^chapter(\d+)\.json$/.exec(path);
	if (match) return l.chapter(Number(match[1]));
	match = /^season(\d+)\.json$/.exec(path);
	if (match) return l.season(Number(match[1]));
	// platform-wide and toon-level entities (also with a scope prefix, e.g. `scutoons/characters.json`)
	const named = NAMED[path.split('/').pop() ?? path];
	if (named) return l[named] as string;
	match = /^[a-z0-9_-]+\/([a-z0-9_-]+)\.json$/.exec(path);
	if (match) return l.scene(match[1].replace(/^scene-/, ''));
	return path.replace(/\.json$/, '');
}
