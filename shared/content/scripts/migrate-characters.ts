// Migrates the old per-locale characters of the toons into the characters model of characters.ts and adds what the
// owner delivered (2026-09-29):
//   <api>/<toon>/{de,en}/characters.json  ([{ name, label, image, color, images: { <mood>: { avatar, portrait, full } },
//                                            description, collection }] – one file per locale)
//   <cdn>/characters/<character>[-<persona>]/<character>[-<persona>][_<look>]_<view>-<mood>.png   (convention folders)
//   <cdn>/characters/<character>[-<persona>]/<character>[-<persona>]_transition-<from>-to-<to>.png
// → <out>/scutoons/_/{characters,moods,asset-requests}.json, <out>/<toon>/_/cast.json (+ report on stdout).
//
//   node --experimental-transform-types --no-warnings shared/content/scripts/migrate-characters.ts \
//        [--api=viewer/static/api] [--cdn=cdn/root] [--out=shared/content/seed] [--json]
//
// Then import (published version 1, idempotent; `--update` after a seed change):
//   php server/bin/console import-content shared/content/seed --only=characters.json,moods.json,asset-requests.json,cast.json
// Production SQL: the same with `--sql` (committed as shared/content/seed/characters-seed.mysql.sql).
//
// Rules: characters are matched by `name` (the id) across locales; `label` / `description` become { de, en } texts;
// color / image paths come from the toon's source language (de). Old image paths get the toon
// folder in front (registry paths are from the CDN root); when the locales disagree on a path, the one that exists on
// the local CDN copy wins, the source language first (reported). Old images become the regular persona's default look.
// A persona with a convention folder on the CDN gets its looks from there (every look / mood / view found, replacing
// the old per-mood paths of that look; the old single `image` stays the last fallback); transition files become
// `transitions`, other files in the folder artwork.
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
	APOSE_LOOKS,
	APOSE_VIEW,
	ASSET_REQUESTS_FILE,
	DEFAULT_MOOD,
	aposeConventionPath,
	CAST_FILE,
	CHARACTER_IMAGE_ROOT,
	CHARACTER_IMAGE_VARIANTS,
	CHARACTERS_FILE,
	CREDITS_SCOPE,
	LEGACY_CHARACTER_IDS,
	DEFAULT_LOOK,
	LEGACY_CHARACTERS_FILE,
	MOODS_FILE,
	REGULAR_PERSONA,
	castEntries,
	conventionName,
	defaultMoodVocabulary,
	emptyAssetRequests,
	findPersona,
	isKnownMood,
	lookPortraitMoods,
	resolveCast,
	validateCast,
	validateCharacters,
	validateMoods,
	type Character,
	type CharacterLook,
	type CharacterPersona,
	type CharacterRegistry,
	type LocaleText,
	type RegistryCharacter,
	type ToonCast
} from '../src/index.ts';

const repo = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');
const arg = (name: string, fallback: string) => process.argv.find((a) => a.startsWith(`--${name}=`))?.slice(name.length + 3) ?? fallback;
const API = resolve(repo, arg('api', 'viewer/static/api'));
const CDN = resolve(repo, arg('cdn', 'cdn/root'));
const OUT = resolve(repo, arg('out', 'shared/content/seed'));
const LOCALES = ['de', 'en'];
const SOURCE = 'de';

// ── Owner input (2026-09-29) ──────────────────────────────────────────────────────────────────────────────────────

const MOODS = defaultMoodVocabulary();

/** Wear anchors of the reader's avatar bodies (A-poses, 768 × 1152), from viewer components/avatar/avatar.svelte */
const SPHYNX_WORN = {
	head: { x: 370, y: 70, size: 170 },
	face: { x: 372, y: 105, size: 110 },
	neck: { x: 372, y: 200, size: 110 },
	torso: { x: 372, y: 340, size: 330 },
	hands: { x: 150, y: 575, size: 90 },
	legs: { x: 372, y: 760, size: 300 },
	feet: { x: 372, y: 1040, size: 330 },
	back: { x: 372, y: 330, size: 360 },
	accessory: { x: 598, y: 575, size: 90 }
};
const MOUSE_WORN = {
	head: { x: 385, y: 130, size: 330 },
	face: { x: 385, y: 230, size: 200 },
	neck: { x: 385, y: 330, size: 120 },
	torso: { x: 385, y: 450, size: 260 },
	hands: { x: 175, y: 625, size: 90 },
	legs: { x: 385, y: 770, size: 260 },
	feet: { x: 385, y: 1010, size: 340 },
	back: { x: 385, y: 450, size: 300 },
	accessory: { x: 595, y: 625, size: 90 }
};

const APOSE_LABEL = Object.fromEntries(APOSE_LOOKS.map((l) => [l.id, l.label])) as Record<'swim' | 'naked', LocaleText>;

const helmetLooks = (): CharacterLook[] => [
	{ id: DEFAULT_LOOK, label: { de: 'ohne Helm', en: 'without helmet' }, default: true, images: {} },
	{ id: 'helmet', label: { de: 'mit Helm', en: 'with helmet' }, images: {} }
];

/** Former characters that are personas of another character now (owner, 2026-09-30) – LEGACY_CHARACTER_IDS; labels. */
const PERSONA_LABELS: Record<string, LocaleText | null> = {
	human: { de: 'Mensch', en: 'human' },
	cat: { de: 'Katze', en: 'cat' },
	// the old characters.json labels
	mountaintiger: null
};

/** Personas / fields added to migrated characters. */
const UPDATES: Record<string, (c: RegistryCharacter) => void> = {
	catpope: (c) => {
		c.own_toon = 'catpope';
		findPersona(c, REGULAR_PERSONA)!.looks[0].label = { de: 'päpstlich', en: 'papal' };
		// the reader's avatar bodies (character level): swim pants and naked
		c.apose = {
			looks: [
				{ id: 'swim', label: APOSE_LABEL.swim, default: true, path: 'characters/catpope/catpope_apose-swim.webp', anchors: { ...SPHYNX_WORN } },
				{ id: 'naked', label: APOSE_LABEL.naked, path: 'characters/catpope/catpope_apose-naked.webp', anchors: { ...SPHYNX_WORN } }
			]
		};
		c.artwork = [{ id: 'apose-example-dressed', path: 'characters/catpope/catpope_apose-example-dressed.webp', label: { de: 'A-Pose angezogen (Referenz)', en: 'A-pose dressed (reference)' } }];
		c.personas.push({
			id: 'biker',
			label: { de: 'Biker', en: 'Biker' },
			// `introduced` stays empty until the owner sets where he joins the gang
			description: { de: 'Der Katzenpapst, nachdem er sich der Biker-Gang angeschlossen hat.', en: 'Cat Pope after he joins the biker gang.' },
			looks: helmetLooks()
		});
	}
};

/** New registry characters. */
const NEW: RegistryCharacter[] = [
	{
		id: 'yellowmouse',
		name: { de: 'Mausboss', en: 'Yellow Mouse' },
		description: {
			de: 'Die Anführerin der Biker-Gang – das war sie schon immer, das ist ihr wahres Ich.',
			en: 'The head of the biker gang – she always has been; this is her true self.'
		},
		color: 'yellow',
		own_toon: 'yellowmouse',
		image: 'yellowmouse/avatar.webp',
		// one persona, two looks: the kimono (default) and the biker helmet
		personas: [{ id: REGULAR_PERSONA, looks: helmetLooks() }],
		// avatar body: swim pants; naked is still to be delivered (an open asset request)
		apose: {
			looks: [
				{ id: 'swim', label: APOSE_LABEL.swim, default: true, path: 'characters/yellowmouse/yellowmouse_apose-swim.webp', anchors: { ...MOUSE_WORN } },
				{ id: 'naked', label: APOSE_LABEL.naked }
			]
		}
	},
	{
		id: 'toptoy',
		name: { de: 'Toptoy', en: 'Toptoy' },
		description: { de: 'Das Duo', en: 'The duo' },
		color: 'black',
		own_toon: 'toptoy',
		tags: ['duo'],
		image: 'toptoy/avatar.webp',
		personas: [{ id: REGULAR_PERSONA, looks: [{ id: DEFAULT_LOOK, default: true, images: {} }] }]
	}
];

/** Looks whose old per-mood paths are replaced by the convention folder (owner: catpope's old SVGs only as `image`). */
const REPLACE_OLD_IMAGES = ['catpope/regular/default'];

/** Files in a convention folder that are transitions although they don't follow the transition naming. */
const TRANSITION_FILES: Record<string, { persona: string; from: string; to: string; label: LocaleText }> = {
	'characters/yellowmouse/yellowmouse_keyvisual-helmet-off.png': { persona: REGULAR_PERSONA, from: 'helmet', to: DEFAULT_LOOK, label: { de: 'Helm ab', en: 'Taking the helmet off' } }
};

/** Casts per toon and season (character → personas), in order. Legacy characters.json casts are season 1. */
const CASTS: Record<string, Record<string, [string, string[]][]>> = {
	catpope: { '1': [['catpope', [REGULAR_PERSONA, 'human', 'cat', 'mountaintiger']]] },
	yellowmouse: { '1': [['catpope', [REGULAR_PERSONA, 'biker']], ['yellowmouse', [REGULAR_PERSONA]]] },
	toptoy: { '1': [['toptoy', [REGULAR_PERSONA]]] }
};

// ── Legacy characters.json ────────────────────────────────────────────────────────────────────────────────────────

const notes: string[] = [];
const ambiguous: string[] = [];
const onCdn = (path: string) => existsSync(join(CDN, path));

const read = (toon: string, locale: string): Character[] | null => {
	const path = join(API, toon, locale, LEGACY_CHARACTERS_FILE);
	return existsSync(path) ? JSON.parse(readFileSync(path, 'utf8')) : null;
};

/** The path both locales should share: equal → it; else the source's when it exists on the CDN, else one that does. */
function pickPath(toon: string, where: string, byLocale: Record<string, string | undefined>): string {
	const values = [...new Set(Object.values(byLocale).filter((v): v is string => !!v))];
	if (!values.length) return '';
	const full = (p: string) => `${toon}/${p.replace(/^\/+/, '')}`;
	if (values.length === 1) {
		if (!onCdn(full(values[0]))) notes.push(`${where}: ${full(values[0])} is not on the local CDN copy (kept)`);
		return full(values[0]);
	}
	const ordered = [byLocale[SOURCE], ...LOCALES.filter((l) => l !== SOURCE).map((l) => byLocale[l])].filter((v): v is string => !!v);
	const chosen = ordered.find((p) => onCdn(full(p))) ?? ordered[0];
	ambiguous.push(
		`${where}: locales differ (${LOCALES.map((l) => `${l} ${byLocale[l] ?? '–'}${byLocale[l] && onCdn(full(byLocale[l]!)) ? ' ✓' : byLocale[l] ? ' ✗' : ''}`).join(', ')}) → ${full(chosen)}`
	);
	return full(chosen);
}

const registry: CharacterRegistry = { characters: [] };
const casts: Record<string, ToonCast> = {};
const addToCast = (toon: string, season: string, character: string, personas: string[]) => {
	const list = ((casts[toon] ??= { seasons: {} }).seasons[season] ??= []);
	const entry = list.find((e) => e.character === character);
	if (entry) entry.personas = [...new Set([...entry.personas, ...personas])];
	else list.push({ character, personas: [...personas] });
};
const toons = readdirSync(API, { withFileTypes: true })
	.filter((d) => d.isDirectory() && LOCALES.some((l) => existsSync(join(API, d.name, l, LEGACY_CHARACTERS_FILE))))
	.map((d) => d.name)
	.sort();

for (const toon of toons) {
	const files = Object.fromEntries(LOCALES.map((l) => [l, read(toon, l) ?? []]));
	const ids: string[] = [];
	for (const l of [SOURCE, ...LOCALES.filter((x) => x !== SOURCE)]) for (const c of files[l]) if (c?.name && !ids.includes(c.name)) ids.push(c.name);
	for (const id of ids) {
		const byLocale = Object.fromEntries(LOCALES.map((l) => [l, files[l].find((c) => c.name === id)])) as Record<string, Character | undefined>;
		const source = byLocale[SOURCE] ?? Object.values(byLocale).find(Boolean)!;
		const text = (key: 'label' | 'description'): LocaleText | undefined => {
			const entries = LOCALES.map((l) => [l, byLocale[l]?.[key]] as const).filter(([, v]) => typeof v === 'string' && v.trim());
			return entries.length ? Object.fromEntries(entries) : undefined;
		};
		for (const key of ['color', 'collection'] as const) {
			const values = new Set(LOCALES.map((l) => byLocale[l]?.[key]).filter(Boolean));
			if (values.size > 1) ambiguous.push(`${toon}/${id}: ${key} differs between locales (${[...values].join(' / ')}) → ${source[key]}`);
		}
		const moods: Record<string, Record<string, string>> = {};
		for (const mood of [...new Set(LOCALES.flatMap((l) => Object.keys(byLocale[l]?.images ?? {})))]) {
			const slots: Record<string, string> = {};
			for (const view of CHARACTER_IMAGE_VARIANTS) {
				const path = pickPath(
					toon,
					`${toon}/${id} ${mood} ${view}`,
					Object.fromEntries(LOCALES.map((l) => [l, (byLocale[l]?.images?.[mood] as Record<string, string> | undefined)?.[view]]))
				);
				if (path) slots[view] = path;
			}
			moods[mood] = slots;
		}
		const image = pickPath(toon, `${toon}/${id} image`, Object.fromEntries(LOCALES.map((l) => [l, byLocale[l]?.image])));
		const description = text('description');
		const character: RegistryCharacter = {
			id,
			name: text('label') ?? { [SOURCE]: id },
			...(description ? { description } : {}),
			color: source.color,
			...(image ? { image } : {}),
			personas: [{ id: REGULAR_PERSONA, looks: [{ id: DEFAULT_LOOK, default: true, images: moods }] }]
		};
		UPDATES[id]?.(character);
		if (registry.characters.some((c) => c.id === id)) ambiguous.push(`${toon}/${id}: id already in the registry from another toon – kept the first`);
		else registry.characters.push(character);
		addToCast(toon, '1', id, [REGULAR_PERSONA]);
	}
	notes.push(`${toon}: ${ids.length} characters (${ids.join(', ')}) from ${LOCALES.filter((l) => read(toon, l)).map((l) => `${l}/${LEGACY_CHARACTERS_FILE}`).join(' + ')} → season 1 cast`);
}
for (const c of NEW) if (!registry.characters.some((x) => x.id === c.id)) registry.characters.push(c);

// former characters → personas (plf → catpope · human, cat → catpope · cat, mountain-tiger → catpope · mountaintiger):
// label, description, color and images move over; old scene refs resolve through LEGACY_CHARACTER_IDS
for (const [legacyId, { character, persona }] of Object.entries(LEGACY_CHARACTER_IDS)) {
	const old = registry.characters.find((c) => c.id === legacyId);
	const target = registry.characters.find((c) => c.id === character);
	if (!old || !target) continue;
	registry.characters = registry.characters.filter((c) => c !== old);
	const oldRegular = findPersona(old, REGULAR_PERSONA)!;
	target.personas.push({
		id: persona,
		label: PERSONA_LABELS[persona] ?? old.name,
		...(old.description ? { description: old.description } : {}),
		...(old.color && old.color !== target.color ? { color: old.color } : {}),
		looks: oldRegular.looks
	});
	for (const cast of Object.values(casts))
		for (const list of Object.values(cast.seasons)) {
			const i = list.findIndex((e) => e.character === legacyId);
			if (i >= 0) list.splice(i, 1);
		}
	notes.push(`${legacyId} → ${character} · ${persona} (label ${JSON.stringify(PERSONA_LABELS[persona] ?? old.name)}, color ${old.color}, old image ${old.image ?? '–'} dropped – the persona falls back to ${character}'s)`);
}
// Cat Pope's personas in the owner's order
const PERSONA_ORDER = [REGULAR_PERSONA, 'human', 'cat', 'mountaintiger', 'biker'];
for (const c of registry.characters) c.personas.sort((a, b) => (PERSONA_ORDER.indexOf(a.id) + 1 || 99) - (PERSONA_ORDER.indexOf(b.id) + 1 || 99));
for (const [toon, seasons] of Object.entries(CASTS))
	for (const [season, entries] of Object.entries(seasons)) for (const [character, personas] of entries) addToCast(toon, season, character, personas);

// ── Convention folders on the CDN (the owner's delivered images) ─────────────────────────────────────────────────

for (const character of registry.characters) {
	for (const persona of character.personas) {
		const name = conventionName(character.id, persona.id);
		const folder = join(CDN, CHARACTER_IMAGE_ROOT, name);
		if (!existsSync(folder)) continue;
		const escaped = name.replace(/-/g, '\\-');
		const expression = new RegExp(`^${escaped}(?:_([a-z0-9-]+))?_(${CHARACTER_IMAGE_VARIANTS.join('|')})-([a-z0-9-]+)\\.png$`);
		const transition = new RegExp(`^${escaped}_transition-([a-z0-9]+)-to-([a-z0-9]+)\\.png$`);
		const apose = new RegExp(`^${escaped}_apose-([a-z0-9-]+)\\.(webp|png)$`);
		const other = new RegExp(`^${escaped}_([a-z0-9-]+)\\.(png|jpe?g|webp)$`);
		const found: Record<string, Record<string, Record<string, string>>> = {};
		const extras = new Set<string>();
		for (const file of readdirSync(folder).sort()) {
			const path = `${CHARACTER_IMAGE_ROOT}/${name}/${file}`;
			const e = file.match(expression);
			if (e) {
				const [, look = DEFAULT_LOOK, view, mood] = e;
				((found[look] ??= {})[mood] ??= {})[view] = path;
				if (!isKnownMood(mood, MOODS)) extras.add(mood);
				continue;
			}
			if (file.match(apose)) {
				// A-pose looks / references are set explicitly above
				if (!character.apose?.looks.some((l) => l.path === path) && !character.artwork?.some((a) => a.path === path)) ambiguous.push(`${character.id}: ${path} (an A-pose file) is not referenced`);
				continue;
			}
			const mapped = TRANSITION_FILES[path];
			const t = file.match(transition);
			if (mapped || t) {
				const target = mapped ? (findPersona(character, mapped.persona) ?? persona) : persona;
				(target.transitions ??= []).push({ from: mapped?.from ?? t![1], to: mapped?.to ?? t![2], image: path, ...(mapped?.label ? { label: mapped.label } : {}) });
				notes.push(`${character.id} · ${target.id}: transition ${mapped?.from ?? t![1]} → ${mapped?.to ?? t![2]} (${path})`);
				continue;
			}
			const a = file.match(other);
			if (a) {
				character.artwork ??= [];
				if (!character.artwork.some((x) => x.id === a[1])) character.artwork.push({ id: a[1], path });
				continue;
			}
			ambiguous.push(`${character.id}: ${path} matches no convention (ignored)`);
		}
		for (const [lookId, moods] of Object.entries(found)) {
			let look = persona.looks.find((l) => l.id === lookId);
			if (!look) {
				look = { id: lookId, label: { de: lookId, en: lookId }, images: {} };
				persona.looks.push(look);
				ambiguous.push(`${character.id} · ${persona.id}: look "${lookId}" found on the CDN without a label – label it in the creator`);
			}
			const replaced = Object.keys(look.images).length;
			if (REPLACE_OLD_IMAGES.includes(`${character.id}/${persona.id}/${lookId}`)) look.images = moods;
			else
				// keep the old explicit paths the convention folder doesn't have (e.g. the legacy full svg)
				for (const [mood, views] of Object.entries(moods)) look.images[mood] = { ...(look.images[mood] ?? {}), ...views };
			notes.push(
				`${character.id} · ${persona.id} · ${lookId}: ${lookPortraitMoods(look).length} moods from ${CHARACTER_IMAGE_ROOT}/${name}/` +
					(replaced ? (REPLACE_OLD_IMAGES.includes(`${character.id}/${persona.id}/${lookId}`) ? ` (replaces ${replaced} old mood entries; the old single image stays: ${character.image ?? '–'})` : ' (merged with the old explicit paths)') : '')
			);
		}
		if (extras.size) notes.push(`${character.id} · ${persona.id}: moods not in the vocabulary: ${[...extras].join(', ')}`);
	}
}

// ── Check: the resolved cast vs the old files ────────────────────────────────────────────────────────────────────
const checks: string[] = [];
for (const toon of toons) {
	for (const locale of LOCALES) {
		const old = read(toon, locale);
		if (!old) continue;
		const resolved = resolveCast(registry, casts[toon], toon, locale, { fallbacks: LOCALES.filter((l) => l !== locale) });
		const norm = (list: Character[]) =>
			list.map((c) => ({ name: c.name, label: c.label, image: c.image, color: c.color, images: c.images ?? {}, description: c.description ?? '', collection: c.collection ?? '' }));
		const a = norm(old);
		const b = norm(resolved);
		const diffs: string[] = [];
		a.forEach((c) => {
			const r = b.find((x) => x.name === c.name);
			if (!r) return diffs.push(`${c.name} missing`);
			for (const key of Object.keys(c) as (keyof typeof c)[]) {
				if (JSON.stringify(c[key]) !== JSON.stringify(r[key])) {
					diffs.push(
						key === 'images'
							? `${c.name}.images: ${Object.keys(c.images).length} → ${Object.keys(r.images).length} moods, neutral ${JSON.stringify(r.images.neutral ?? {})}`
							: `${c.name}.${key}: ${JSON.stringify(c[key])} → ${JSON.stringify(r[key])}`
					);
				}
			}
		});
		const added = b.filter((r) => !a.some((c) => c.name === r.name)).map((r) => r.name);
		checks.push(`${toon}/${locale}: ${diffs.length ? `${diffs.length} differences – ${diffs.join('; ')}` : 'identical to the old characters.json'}${added.length ? ` (+ new: ${added.join(', ')})` : ''}`);
	}
}

const allToons = [...new Set([...toons, ...Object.keys(casts)])];
// the missing naked A-pose of the mouse: an open asset request
const REQUESTS = emptyAssetRequests();
REQUESTS.requests.push({
	id: 'yellowmouse-apose-naked',
	character: 'yellowmouse',
	persona: REGULAR_PERSONA,
	look: 'naked',
	mood: DEFAULT_MOOD,
	view: APOSE_VIEW,
	path: aposeConventionPath('yellowmouse', 'naked'),
	note: 'A-pose naked (768 × 1152, like yellowmouse_apose-swim.webp)',
	requested_by: 'migration (owner request 2026-09-30)',
	requested_at: '2026-09-30T00:00:00Z',
	status: 'open'
});
const problems = [
	...validateMoods(MOODS, LOCALES),
	...validateCharacters(registry, { languages: LOCALES, toons: allToons, moods: MOODS }),
	...Object.entries(casts).flatMap(([toon, cast]) => validateCast(cast, { registry, languages: LOCALES, file: `${toon}/_/${CAST_FILE}` }))
].filter((p) => p.level !== 'info');

const write = (toon: string, file: string, data: unknown) => {
	const path = join(OUT, toon, '_', file);
	mkdirSync(dirname(path), { recursive: true });
	writeFileSync(path, JSON.stringify(data, null, '\t') + '\n');
	return path.replace(repo + '\\', '').replace(repo + '/', '');
};
const written = [
	write(CREDITS_SCOPE, CHARACTERS_FILE, registry),
	write(CREDITS_SCOPE, MOODS_FILE, MOODS),
	write(CREDITS_SCOPE, ASSET_REQUESTS_FILE, REQUESTS),
	...Object.entries(casts).map(([toon, cast]) => write(toon, CAST_FILE, cast))
];

const personaSummary = (p: CharacterPersona) =>
	`${p.id}[${p.looks.map((l) => `${l.id}${l.default ? '*' : ''}:${lookPortraitMoods(l).length}`).join(' ')}${p.transitions?.length ? ` +${p.transitions.length} transition` : ''}]`;
const summary = {
	characters: registry.characters.map(
		(c) =>
			`${c.id} (${Object.values(c.name).join(' / ')}${c.own_toon ? `, own toon ${c.own_toon}` : ''}${c.apose ? `, A-pose ${c.apose.looks.map((l) => `${l.id}${l.path ? '' : ' (wanted)'}`).join('/')}` : ''}) personas ${c.personas.map(personaSummary).join(' ')}`
	),
	casts: Object.entries(casts).flatMap(([t, c]) => Object.entries(c.seasons).map(([s, list]) => `${t} season ${s}: ${list.map((e) => `${e.character}(${e.personas.join('+')})`).join(', ')}`)),
	ambiguous,
	notes,
	checks,
	problems: problems.map((p) => `${p.level} ${p.file} ${p.path.join('.')}: ${p.message}`),
	files: written
};

if (process.argv.includes('--json')) console.log(JSON.stringify(summary, null, 2));
else {
	console.log(
		`characters migration: ${registry.characters.length} characters, ${registry.characters.reduce((n, c) => n + c.personas.length, 0)} personas, ${MOODS.moods.length} moods, casts: ${Object.entries(casts).map(([t, c]) => `${t} ${castEntries(c).length}`).join(', ')}`
	);
	for (const [title, lines] of [
		['Characters (persona[look*:portrait moods])', summary.characters],
		['Casts', summary.casts],
		['Ambiguous (check)', ambiguous],
		['Notes', notes],
		['Resolved vs old files', checks],
		['Problems', summary.problems],
		['Written', written]
	] as const) {
		if (!lines.length) continue;
		console.log(`\n${title}:`);
		for (const line of lines) console.log(`  - ${line}`);
	}
}
