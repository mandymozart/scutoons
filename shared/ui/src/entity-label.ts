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
	/** the toon's per-language settings (title, description, covers) */
	toon: string;
	/** the toon's index entry (status, names, languages + the language-neutral settings: map, tile …) */
	entry: string;
	characters: string;
	moods: string;
	requests: string;
	cast: string;
	credits: string;
	people: string;
	roles: string;
	/** the SCUTOONS world map (entity `map`; the old per-toon `locations` / `routes` too) */
	map: string;
	events: string;
	/** the toon's screenplay (one per language, creator only) */
	script: string;
	/** a catalog entry's document (`item:camera` → kind `item`, id `camera`; agents/CATALOG-IN-CREATOR.md) */
	catalog: (kind: string, id: string) => string;
}

export const ENTITY_LABELS: EntityLabels = {
	chapter: (n) => `Chapter ${n}`,
	season: (n) => `Season ${n}`,
	scene: (label) => `Scene ${label}`,
	toon: 'Toon settings',
	entry: 'Toon entry',
	characters: 'Characters',
	moods: 'Moods',
	requests: 'Wanted images',
	cast: 'Cast',
	credits: 'Credits',
	people: 'People',
	roles: 'Roles',
	map: 'Map',
	events: 'Events',
	script: 'Script',
	catalog: (kind, id) => `${kind[0].toUpperCase()}${kind.slice(1)} ${id}`
};

const CATALOG = ['item', 'recipe', 'album', 'product', 'edition'];

const NAMED: Record<string, keyof EntityLabels> = {
	'toon-settings': 'toon',
	toon: 'entry',
	characters: 'characters',
	moods: 'moods',
	'asset-requests': 'requests',
	cast: 'cast',
	credits: 'credits',
	people: 'people',
	roles: 'roles',
	locations: 'map',
	routes: 'map',
	map: 'map',
	events: 'events',
	script: 'script'
};

/** `chapter:catpope:2` → "Chapter 2", `season:catpope:1:de` → "Season 1", `scene:catpope:2:scene-14` → "Scene 14" … */
export function entityLabel(key: string, labels: Partial<EntityLabels> = {}): string {
	const l = { ...ENTITY_LABELS, ...labels };
	const [kind, id, number, scene] = key.split(':');
	if (CATALOG.includes(kind) && id) return l.catalog(kind, id);
	if (kind === 'chapter' && number) return l.chapter(Number(number));
	if (kind === 'season' && number) return l.season(Number(number));
	if (kind === 'scene' && scene) return l.scene(scene.replace(/^scene-/, ''));
	const named = NAMED[kind];
	if (named) return l[named] as string;
	return kind;
}
