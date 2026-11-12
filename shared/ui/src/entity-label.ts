/**
 * The ONE place where a content entity's key (agents/DATA.md: `chapter:catpope:2`, `season:catpope:1:de`,
 * `toon-settings:catpope`, `characters` …) becomes the name people see (owner 2026-09-30:
 * no file names in the UI). A locale at the end of a key is ignored. Apps pass their translations; defaults are English.
 */
export interface EntityLabels {
	chapter: (number: number) => string;
	season: (number: number) => string;
	/** the toon's per-language settings (title, description, covers) */
	toon: string;
	/** the toon's index entry (status, names, languages + the language-neutral settings: map, tile …) */
	entry: string;
	characters: string;
	moods: string;
	/** the custom collage presets (`colorations`, platform-wide – research/COLLAGE.md §12) */
	colorations: string;
	requests: string;
	cast: string;
	credits: string;
	people: string;
	roles: string;
	/** the SCUTOONS world map (entity `map`; the old per-toon `locations` / `routes` too) */
	map: string;
	/** one place / route of the world map (`place:<id>`, `route:<id>` – agents/MAP-DOCUMENTS.md); callers that know its name pass it */
	place: (id: string) => string;
	route: (id: string) => string;
	region: (id: string) => string;
	/** one character of the registry (`character:<id>`) */
	character: (id: string) => string;
	/** one deck scene (`scene:<id>`, research/SCENES.md §9) */
	scene: (id: string) => string;
	events: string;
	/** a chapter's storyboard (`storyboard:<toon>:<n>`, creator only) */
	storyboard: (number: number) => string;
	/** a season's / a chapter's screenplay (`season-script:<toon>:<n>`, `chapter-script:<toon>:<n>`, creator only) */
	seasonScript: (number: number) => string;
	chapterScript: (number: number) => string;
	/** a Kanon text (`kanon:<subject>`, research/KANON.md): the subject segment, e.g. `character.catpope.human` */
	kanon: (subject: string) => string;
	/** a catalog entry's document (`item:camera` → kind `item`, id `camera`, `module:walk`, `rig:character.catpope`; agents/CATALOG-IN-CREATOR.md) */
	catalog: (kind: string, id: string) => string;
}

export const ENTITY_LABELS: EntityLabels = {
	chapter: (n) => `Chapter ${n}`,
	season: (n) => `Season ${n}`,
	toon: 'Toon settings',
	entry: 'Toon entry',
	characters: 'Characters',
	moods: 'Moods',
	colorations: 'Collage presets',
	requests: 'Wanted images',
	cast: 'Cast',
	credits: 'Credits',
	people: 'People',
	roles: 'Roles',
	map: 'Map',
	place: (id) => `Place ${id}`,
	route: (id) => `Route ${id}`,
	region: (id) => `Region ${id}`,
	character: (id) => `Character ${id}`,
	scene: (id) => `Scene ${id}`,
	events: 'Events',
	storyboard: (n) => `Storyboard · Chapter ${n}`,
	seasonScript: (n) => `Script · Season ${n}`,
	chapterScript: (n) => `Script · Chapter ${n}`,
	kanon: (subject) => `Kanon · ${subject.split('.').slice(1).join(' · ')}`,
	catalog: (kind, id) => `${kind[0].toUpperCase()}${kind.slice(1)} ${id}`
};

/** the catalog documents `<kind>:<id>` (server EntityKey::CATALOG_KINDS) */
const CATALOG = ['item', 'recipe', 'album', 'product', 'edition', 'home-scene', 'airlift-carrier', 'instrument', 'preset', 'rig', 'module'];

const NAMED: Record<string, keyof EntityLabels> = {
	'toon-settings': 'toon',
	toon: 'entry',
	characters: 'characters',
	moods: 'moods',
	colorations: 'colorations',
	'asset-requests': 'requests',
	cast: 'cast',
	credits: 'credits',
	people: 'people',
	roles: 'roles',
	locations: 'map',
	routes: 'map',
	map: 'map',
	events: 'events'
};

/** `chapter:catpope:2` → "Chapter 2", `season:catpope:1:de` → "Season 1" … */
export function entityLabel(key: string, labels: Partial<EntityLabels> = {}): string {
	const l = { ...ENTITY_LABELS, ...labels };
	const [kind, id, number] = key.split(':');
	if (CATALOG.includes(kind) && id) return l.catalog(kind, id);
	if (kind === 'chapter' && number) return l.chapter(Number(number));
	if (kind === 'season' && number) return l.season(Number(number));
	if (kind === 'storyboard' && number) return l.storyboard(Number(number));
	if (kind === 'season-script' && number) return l.seasonScript(Number(number));
	if (kind === 'chapter-script' && number) return l.chapterScript(Number(number));
	if (kind === 'kanon' && id) return l.kanon(id);
	if ((kind === 'place' || kind === 'route' || kind === 'region' || kind === 'character' || kind === 'scene') && id) return l[kind](id);
	const named = NAMED[kind];
	if (named) return l[named] as string;
	return kind;
}
