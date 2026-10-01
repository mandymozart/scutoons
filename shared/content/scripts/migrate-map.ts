// One-time migration to the SCUTOONS world map (agents/MAP.md §2.6, phase M0): catpope's per-locale map files
//   <seed>/catpope/{de,en}/locations.json, routes.json   +   toons.json catpope `map` (layers.terrain, width, height, center)
// → <seed>/scutoons/_/map.json (entity `map`) and the toons' start views in toons.json (`map: { center }`).
//
//   node --experimental-transform-types --no-warnings shared/content/scripts/migrate-map.ts [--seed=shared/content/seed] [--no-import] [--force]
//
// Then (the script runs it at the end, `--no-import` to skip):
//   php server/bin/console import-content shared/content/seed --only=map --update, then the store's toon entries get their
//   start view (export-content → only the `map` blocks rewritten → import-content --only=toon --update)
// Production SQL: regenerate shared/content/seed/content-seed.mysql.sql (only the `map` + changed `toon:*` statements differ).
//
// Every old value survives (`migrateLegacyMap`): ids, coordinates, de/en labels verbatim (incl. "Medival Street"), the
// route's `title` / `titel`, the waypoint order. The old files stay in the seed for one release (the library computes
// the old answer from the world map; tests/entities-compare.php proves they are equal).
//
// Owner decisions (2026-10-01), applied on top:
//   - catpope's places unlock over time: every place is `hidden` with the point where the story reaches it (`unlock`:
//     chapter, reading position, screenplay scene) – derived from the screenplay (cat-pope-awakening de/screenplay.txt,
//     its "# <n>. Kapitel" markers and scene headings); `at` = the scene heading's line offset in its chapter (rounded).
//   - the places the script has and the map lacked are added (hidden, unlocked at their first scene); positions are a
//     best guess on the terrain – the owner places them in the builder.
//   - "garden" (season-1 route, unknown until now) = "Garten Eden" / "Garden of Eden", scene 38, the end of chapter 5.
//   - en names of the new places are translations of the de script headings (the en script covers scenes 1–10 only):
//     flagged for review in agents/MAP.md "Built".
// The script refuses to run twice (the old toon map is gone after the first run) unless --force.
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { checkWorldMap, migrateLegacyMap, type MapPlace, type PlaceUnlock, type ToonIndex, type WorldMap } from '../src/index.ts';

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, '../../..');
const args = Object.fromEntries(process.argv.slice(2).map((a) => a.replace(/^--/, '').split('=')).map(([k, v]) => [k, v ?? true]));
const SEED = resolve(repo, String(args.seed ?? 'shared/content/seed'));
const OUT = join(SEED, 'scutoons/_/map.json');

const read = (path: string) => JSON.parse(readFileSync(path, 'utf8'));
const write = (path: string, data: unknown) => {
	mkdirSync(dirname(path), { recursive: true });
	writeFileSync(path, JSON.stringify(data, null, '\t') + '\n');
};

/** Where the story reaches each catpope place (screenplay scene headings, chapter markers) */
const STORY: Record<string, PlaceUnlock> = {
	apartment: { toon: 'catpope', chapter: 1, at: 0, scene: 2 },
	'church-square': { toon: 'catpope', chapter: 1, at: 0.85, scene: 12 },
	'waterfall-tower': { toon: 'catpope', chapter: 1, at: 0.86, scene: 13 },
	crossroads: { toon: 'catpope', chapter: 1, at: 0.87, scene: 14 },
	'medival-street': { toon: 'catpope', chapter: 1, at: 0.88, scene: 15 },
	'infinity-pool': { toon: 'catpope', chapter: 1, at: 0.89, scene: 16 },
	cathedral: { toon: 'catpope', chapter: 2, at: 0, scene: 17 },
	station: { toon: 'catpope', chapter: 2, at: 0.67, scene: 21 },
	'mountain-village': { toon: 'catpope', chapter: 3, at: 0, scene: 25 },
	'beyond-the-crossroads': { toon: 'catpope', chapter: 3, at: 0.05, scene: 26 },
	'mountain-path': { toon: 'catpope', chapter: 3, at: 0.07, scene: 27 },
	'forest-clearing': { toon: 'catpope', chapter: 3, at: 0.11, scene: 28 },
	monastery: { toon: 'catpope', chapter: 3, at: 0.18, scene: 28 },
	library: { toon: 'catpope', chapter: 3, at: 0.19, scene: 29 },
	crypt: { toon: 'catpope', chapter: 4, at: 0, scene: 35 },
	graveyard: { toon: 'catpope', chapter: 4, at: 0.2, scene: 36 },
	well: { toon: 'catpope', chapter: 4, at: 0.58, scene: 37 },
	garden: { toon: 'catpope', chapter: 5, at: 1, scene: 38 }
};

/** Places the script has and the map lacked (positions: best guess near their neighbours on the terrain) */
const NEW_PLACES: { id: string; de: string; en: string; x: number; y: number; after: string }[] = [
	{ id: 'church-square', de: 'Alterlaa Kirchenvorplatz', en: 'Alterlaa church square', x: 1224, y: 284, after: 'apartment' },
	{ id: 'waterfall-tower', de: 'Brutalistisches Hochhaus mit Wasserfall', en: 'Brutalist tower with waterfall', x: 1240, y: 272, after: 'church-square' },
	{ id: 'crossroads', de: 'Kreuzwege', en: 'Crossroads', x: 1268, y: 290, after: 'waterfall-tower' },
	{ id: 'infinity-pool', de: 'Brutalistischer Infinity Pool', en: 'Brutalist infinity pool', x: 1226, y: 336, after: 'medival-street' },
	{ id: 'mountain-village', de: 'Winterliches Bergdorf', en: 'Wintry mountain village', x: 1478, y: 516, after: 'station' },
	{ id: 'beyond-the-crossroads', de: 'Hinter der Wegkreuzung', en: 'Beyond the crossroads', x: 1464, y: 532, after: 'mountain-village' },
	{ id: 'forest-clearing', de: 'Waldlichtung', en: 'Forest clearing', x: 1416, y: 566, after: 'mountain-path' },
	{ id: 'crypt', de: 'Gruft', en: 'Crypt', x: 1456, y: 578, after: 'library' },
	{ id: 'well', de: 'Am Brunnen', en: 'At the well', x: 1462, y: 596, after: 'graveyard' },
	{ id: 'garden', de: 'Garten Eden', en: 'Garden of Eden', x: 1404, y: 640, after: 'well' }
];

if (existsSync(OUT) && !args.force) {
	console.log(`${OUT} exists – already migrated (--force to run again from the old files).`);
} else {
	const toons: (ToonIndex & { map?: any })[] = read(join(SEED, 'toons.json'));
	const catpope = toons.find((t) => t.id === 'catpope');
	if (!catpope?.map?.layers) throw new Error('toons.json: catpope has no old map (layers/width/height/center) – nothing to migrate (--force needs the old files)');
	const locales = ['de', 'en'];
	const { map, view } = migrateLegacyMap({
		toon: 'catpope',
		toonMap: catpope.map,
		source: catpope.sourceLanguage ?? 'de',
		locations: Object.fromEntries(locales.map((l) => [l, read(join(SEED, `catpope/${l}/locations.json`))])),
		routes: Object.fromEntries(locales.map((l) => [l, read(join(SEED, `catpope/${l}/routes.json`))]))
	});

	// the script's places the map lacked, in story order next to their neighbours
	for (const p of NEW_PLACES) {
		const place: MapPlace = { id: p.id, name: { de: p.de, en: p.en }, x: p.x, y: p.y, toons: ['catpope'], chapter: { toon: 'catpope', number: STORY[p.id].chapter } };
		const at = map.locations.findIndex((x) => x.id === p.after);
		map.locations.splice(at < 0 ? map.locations.length : at + 1, 0, place);
	}
	// unlock over time: hidden until the story reaches the place; the card links the chapter that introduces it
	for (const place of map.locations) {
		const unlock = STORY[place.id];
		place.hidden = true;
		if (unlock) {
			place.unlock = unlock;
			place.chapter = { toon: 'catpope', number: unlock.chapter };
		}
	}
	// the old files' chapter links were all "chapter 1" (a placeholder: the map jumped to chapter 1 with a bogus label);
	// the card now links the chapter that introduces the place (airport: not in the script → no unlock, no link)
	const airport = map.locations.find((p) => p.id === 'airport');
	if (airport && !STORY.airport) delete airport.chapter;

	// toon start views: catpope (+ the sandbox's copy of it) keep their centre; toptoy / yellowmouse had an empty placeholder
	// toons.json is edited in place (its hand-kept formatting stays; only the `map` blocks change)
	let text = readFileSync(join(SEED, 'toons.json'), 'utf8');
	for (const t of toons) {
		if (!t.map?.layers) continue;
		const keep = Number(t.map.width) > 0 ? { center: { x: t.map.center.x, y: t.map.center.y } } : null;
		text = replaceMapBlock(text, t.id, keep);
		if (keep) t.map = keep;
		else delete t.map;
	}
	write(OUT, map satisfies WorldMap);
	writeFileSync(join(SEED, 'toons.json'), text);
	console.log(`wrote ${OUT}: ${map.locations.length} places, ${map.routes.length} routes; toons.json start views: ${toons.filter((t) => t.map).map((t) => t.id).join(', ')} (catpope ${JSON.stringify(view.center)})`);
	for (const p of checkWorldMap(map, { toons: toons.map((t) => t.id), languages: locales })) console.log(`  ${p.level.padEnd(7)} ${p.path.join('.')}: ${p.message}`);
}

/** toons.json text with one toon's `"map": { … }` block replaced (null: removed with its leading comma) */
function replaceMapBlock(text: string, toon: string, map: object | null): string {
	const entry = text.indexOf(`"id": "${toon}"`);
	const key = text.indexOf('"map": {', entry);
	const next = text.indexOf('"id": "', entry + 1);
	if (entry < 0 || key < 0 || (next >= 0 && key > next)) throw new Error(`toons.json: no map block for ${toon}`);
	let depth = 0;
	let end = text.indexOf('{', key);
	for (; end < text.length; end++) {
		if (text[end] === '{') depth++;
		else if (text[end] === '}' && --depth === 0) break;
	}
	if (map === null) return text.slice(0, text.lastIndexOf(',', key)) + text.slice(end + 1);
	const lineStart = text.lastIndexOf('\n', key) + 1;
	const indent = text.slice(lineStart, key);
	const body = JSON.stringify(map, null, 2).split('\n').join(`\n${indent}`);
	return text.slice(0, key) + `"map": ${body}` + text.slice(end + 1);
}

if (!args['no-import']) {
	const php = (...a: string[]) => {
		const run = spawnSync(process.env.PHP_BINARY || 'php', ['server/bin/console', ...a], { cwd: repo, encoding: 'utf8' });
		process.stdout.write(run.stdout ?? '');
		if (run.status !== 0) {
			console.error(run.stderr || run.error?.message || `${a[0]} failed`);
			process.exit(1);
		}
	};
	// 1. the world map from the seed
	php('import-content', SEED, '--only=map', '--update');
	// 2. the store's toon entries: only their `map` becomes the start view (everything else stays as the store has it – the
	//    owner may have changed an entry since the seed): export the published entries, rewrite the map blocks, import back
	const tmp = mkdtempSync(join(tmpdir(), 'scutoons-map-'));
	php('export-content', tmp);
	const exported = join(tmp, 'toons.json');
	if (existsSync(exported)) {
		let text = readFileSync(exported, 'utf8');
		for (const t of JSON.parse(text) as (ToonIndex & { map?: any })[]) {
			if (t.map?.layers) text = replaceMapBlock(text, t.id, Number(t.map.width) > 0 ? { center: { x: t.map.center.x, y: t.map.center.y } } : null);
		}
		writeFileSync(exported, text);
		php('import-content', tmp, '--only=toon', '--update');
	}
	rmSync(tmp, { recursive: true, force: true });
}
