// Generates the JSON Schemas in ../schema from the TypeScript types, and the copy the API ships in server/schema/content/
// (the server checks every content save against them: server/src/Domain/Content/ContentSchemas.php).
//   node scripts/schema.js          write schema/*.schema.json
//   node scripts/schema.js --check  fail when a committed schema is out of date (part of `npm run check`)
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createGenerator } from 'ts-json-schema-generator';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
/** the API deploys only server/, so it carries its own copy */
const serverCopy = join(root, '../../server/schema/content');

/** file name → root type. Content files: `<toon>/<locale>/{toon,season1,chapterN,<chapter>/scene-N}.json`;
 *  credits: `<toon>/_/credits.json`, `scutoons/_/people.json`, `scutoons/_/roles.json`;
 *  characters: `scutoons/_/characters.json`, `scutoons/_/moods.json`, `<toon>/_/cast.json`; the screenplay:
 *  `<toon>/<locale>/script.json`; the world map: `scutoons/_/map.json`. */
const SCHEMAS = {
	chapter: 'Chapter',
	season: 'Season',
	toon: 'ToonSettings',
	'toon-index': 'ToonIndex',
	group: 'Group',
	// the language-neutral documents chapter:<toon>:<n>, scene:<toon>:<chapter>:<name>, season:<toon>:<n> (neutral.ts)
	'chapter-doc': 'ChapterDoc',
	'group-doc': 'GroupDoc',
	'season-doc': 'SeasonDoc',
	catalog: 'Catalog',
	credits: 'ToonCredits',
	people: 'PeopleRegistry',
	roles: 'RoleVocabulary',
	// the registry as documents (agents/MAP-DOCUMENTS.md): `characters` = its own fields, `character:<id>` each
	characters: 'CharacterRegistrySettings',
	'registry-character': 'CharacterDoc',
	moods: 'MoodVocabulary',
	'asset-requests': 'AssetRequests',
	cast: 'ToonCast',
	script: 'ToonScript',
	// the world map as documents (agents/MAP-DOCUMENTS.md): `map` = its settings, `place:<id>`, `route:<id>`
	map: 'MapSettings',
	'map-place': 'PlaceDoc',
	'map-route': 'RouteDoc',
	'map-region': 'RegionDoc',
	events: 'EventList'
};

// every object stays open (unknown keys pass), except a type whose doc comment says `@additionalProperties false`
// (RendererParams: a stale creator tab or MCP copy must not write the Swiper's keys back – FEATURES 10.100 §8)
const generator = createGenerator({
	path: join(root, 'src/index.ts'),
	tsconfig: join(root, 'tsconfig.json'),
	expose: 'export',
	topRef: true,
	jsDoc: 'extended',
	extraTags: ['additionalProperties'],
	skipTypeCheck: true,
	additionalProperties: true
});

const check = process.argv.includes('--check');
const stale = [];

for (const [name, type] of Object.entries(SCHEMAS)) {
	const schema = {
		$id: `https://scutoons.com/schema/${name}.schema.json`,
		title: type,
		...generator.createSchema(type)
	};
	// doc comments carry the checkout's line endings (CRLF on Windows): the same schema on every machine
	const text = JSON.stringify(schema, (_key, value) => (typeof value === 'string' ? value.replace(/\r\n/g, '\n') : value), '\t') + '\n';
	for (const file of [join(root, 'schema', `${name}.schema.json`), join(serverCopy, `${name}.schema.json`)]) {
		if (check) {
			if (!existsSync(file) || readFileSync(file, 'utf8').replace(/\r\n/g, '\n') !== text) stale.push(file);
		} else {
			mkdirSync(dirname(file), { recursive: true });
			writeFileSync(file, text);
		}
	}
	if (!check) console.log(`schema/${name}.schema.json (+ server copy) ← ${type}`);
}

if (stale.length) {
	console.error(`Out of date (run \`npm run schema -w @scutoons/content\`):\n${stale.join('\n')}`);
	process.exit(1);
}
if (check) console.log(`@scutoons/content: ${Object.keys(SCHEMAS).length} JSON Schemas up to date`);
