/**
 * The ONE place where a content entity's key (agents/DATA.md: `chapter:catpope:2`, `season:catpope:1:de`,
 * `scene:catpope:2:scene-14`, `toon-settings:catpope`, `characters` …) becomes the name people see (owner 2026-09-30:
 * no file names in the UI). A locale at the end of a key is ignored. Apps pass their translations; defaults are English.
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
	/** the map (locations, routes) */
	map: string;
	events: string;
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
	roles: 'Roles',
	map: 'Map',
	events: 'Events'
};

const NAMED: Record<string, keyof EntityLabels> = {
	'toon-settings': 'toon',
	toon: 'languages',
	characters: 'characters',
	moods: 'moods',
	'asset-requests': 'requests',
	cast: 'cast',
	credits: 'credits',
	people: 'people',
	roles: 'roles',
	locations: 'map',
	routes: 'map',
	events: 'events'
};

/** `chapter:catpope:2` → "Chapter 2", `season:catpope:1:de` → "Season 1", `scene:catpope:2:scene-14` → "Scene 14" … */
export function entityLabel(key: string, labels: Partial<EntityLabels> = {}): string {
	const l = { ...ENTITY_LABELS, ...labels };
	const [kind, , number, scene] = key.split(':');
	if (kind === 'chapter' && number) return l.chapter(Number(number));
	if (kind === 'season' && number) return l.season(Number(number));
	if (kind === 'scene' && scene) return l.scene(scene.replace(/^scene-/, ''));
	const named = NAMED[kind];
	if (named) return l[named] as string;
	return kind;
}
