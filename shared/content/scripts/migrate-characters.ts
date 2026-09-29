// Migrates the old per-locale characters of the toons into the characters model of characters.ts and adds what the
// owner delivered (2026-09-29):
//   <api>/<toon>/{de,en}/characters.json  ([{ name, label, image, color, images: { <mood>: { avatar, portrait, full } },
//                                            description, collection }] – one file per locale)
//   <cdn>/characters/<character>[-<persona>]/<character>[-<persona>][_<look>]_<view>-<mood>.png   (convention folders)
//   <cdn>/characters/<character>[-<persona>]/<character>[-<persona>]_transition-<from>-to-<to>.png
// → <out>/scutoons/_/{characters,groups,moods,asset-requests}.json, <out>/<toon>/_/cast.json (+ report on stdout).
//
//   node --experimental-transform-types --no-warnings shared/content/scripts/migrate-characters.ts \
//        [--api=viewer/static/api] [--cdn=cdn/root] [--out=shared/content/seed] [--json]
//
// Then import (published version 1, idempotent; `--update` after a seed change):
//   php server/bin/console import-content shared/content/seed --only=characters.json,groups.json,moods.json,asset-requests.json,cast.json
// Production SQL: the same with `--sql` (committed as shared/content/seed/characters-seed.mysql.sql).
//
// Rules: characters are matched by `name` (the id) across locales; `label` / `description` become { de, en } texts;
// color / collection (→ group) / image paths come from the toon's source language (de). Old image paths get the toon
// folder in front (registry paths are from the CDN root); when the locales disagree on a path, the one that exists on
// the local CDN copy wins, the source language first (reported). Old images become the regular persona's default look.
// A persona with a convention folder on the CDN gets its looks from there (every look / mood / view found, replacing
// the old per-mood paths of that look; the old single `image` stays the last fallback); transition files become
// `transitions`, other files in the folder artwork.
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
	ASSET_REQUESTS_FILE,
	CAST_FILE,
	CHARACTER_IMAGE_ROOT,
	CHARACTER_IMAGE_VARIANTS,
	CHARACTERS_FILE,
	CREDITS_SCOPE,
	DEFAULT_LOOK,
	GROUPS_FILE,
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
	validateGroups,
	validateMoods,
	type Character,
	type CharacterLook,
	type CharacterPersona,
	type CharacterRegistry,
	type GroupVocabulary,
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

const GROUPS: GroupVocabulary = { groups: [{ id: 'biker-gang', name: { de: 'Biker-Gang', en: 'Biker gang' } }] };
const MOODS = defaultMoodVocabulary();

const helmetLooks = (): CharacterLook[] => [
	{ id: DEFAULT_LOOK, label: { de: 'ohne Helm', en: 'without helmet' }, default: true, images: {} },
	{ id: 'helmet', label: { de: 'mit Helm', en: 'with helmet' }, images: {} }
];

/** Personas / fields added to migrated characters. */
const UPDATES: Record<string, (c: RegistryCharacter) => void> = {
	catpope: (c) => {
		c.own_toon = 'catpope';
		c.personas.push(
			{ id: 'human', label: { de: 'Mensch', en: 'Human' }, looks: [{ id: DEFAULT_LOOK, default: true, images: {} }] },
			{
				id: 'biker',
				label: { de: 'Biker', en: 'Biker' },
				// `introduced` stays empty until the owner sets where he joins the gang
				description: { de: 'Der Katzenpapst, nachdem er sich der Biker-Gang angeschlossen hat.', en: 'Cat Pope after he joins the biker gang.' },
				// only the biker is in the gang
				affiliations: [{ group: 'biker-gang', role: { de: 'Mitglied', en: 'Member' } }],
				looks: helmetLooks()
			}
		);
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
		group: 'lead',
		own_toon: 'yellowmouse',
		mannequin: 'mouse',
		image: 'yellowmouse/avatar.webp',
		affiliations: [{ group: 'biker-gang', role: { de: 'Anführerin', en: 'Leader' } }],
		// one persona, two looks: the kimono (default) and the biker helmet
		personas: [{ id: REGULAR_PERSONA, looks: helmetLooks() }]
	},
	{
		id: 'toptoy',
		name: { de: 'Toptoy', en: 'Toptoy' },
		description: { de: 'Das Duo', en: 'The duo' },
		color: 'black',
		group: 'lead',
		own_toon: 'toptoy',
		tags: ['duo'],
		image: 'toptoy/avatar.webp',
		personas: [{ id: REGULAR_PERSONA, looks: [{ id: DEFAULT_LOOK, default: true, images: {} }] }]
	}
];

/** Files in a convention folder that are transitions although they don't follow the transition naming. */
const TRANSITION_FILES: Record<string, { persona: string; from: string; to: string; label: LocaleText }> = {
	'characters/yellowmouse/yellowmouse_keyvisual-helmet-off.png': { persona: REGULAR_PERSONA, from: 'helmet', to: DEFAULT_LOOK, label: { de: 'Helm ab', en: 'Taking the helmet off' } }
};

/** Casts per toon and season (character → personas), in order. Legacy characters.json casts are season 1. */
const CASTS: Record<string, Record<string, [string, string[]][]>> = {
	catpope: { '1': [['catpope', [REGULAR_PERSONA, 'human']]] },
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
			group: source.collection ?? 'cast',
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
			look.images = moods;
			notes.push(
				`${character.id} · ${persona.id} · ${lookId}: ${lookPortraitMoods(look).length} moods from ${CHARACTER_IMAGE_ROOT}/${name}/` +
					(replaced ? ` (replaces ${replaced} old mood entries; the old single image stays: ${character.image ?? '–'})` : '')
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
const REQUESTS = emptyAssetRequests();
const problems = [
	...validateGroups(GROUPS, LOCALES),
	...validateMoods(MOODS, LOCALES),
	...validateCharacters(registry, { languages: LOCALES, toons: allToons, groups: GROUPS.groups.map((g) => g.id), moods: MOODS }),
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
	write(CREDITS_SCOPE, GROUPS_FILE, GROUPS),
	write(CREDITS_SCOPE, MOODS_FILE, MOODS),
	write(CREDITS_SCOPE, ASSET_REQUESTS_FILE, REQUESTS),
	...Object.entries(casts).map(([toon, cast]) => write(toon, CAST_FILE, cast))
];

const personaSummary = (p: CharacterPersona) =>
	`${p.id}[${p.looks.map((l) => `${l.id}${l.default ? '*' : ''}:${lookPortraitMoods(l).length}`).join(' ')}${p.transitions?.length ? ` +${p.transitions.length} transition` : ''}${p.affiliations ? ` ${p.affiliations.map((a) => a.group).join(',')}` : ''}]`;
const summary = {
	characters: registry.characters.map(
		(c) =>
			`${c.id} (${Object.values(c.name).join(' / ')}, ${c.group}${c.own_toon ? `, own toon ${c.own_toon}` : ''}${c.affiliations?.length ? `, ${c.affiliations.map((a) => `${a.group} ${a.role?.en ?? ''}`).join(', ')}` : ''}) personas ${c.personas.map(personaSummary).join(' ')}`
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
		`characters migration: ${registry.characters.length} characters, ${registry.characters.reduce((n, c) => n + c.personas.length, 0)} personas, ${MOODS.moods.length} moods, ${GROUPS.groups.length} group(s), casts: ${Object.entries(casts).map(([t, c]) => `${t} ${castEntries(c).length}`).join(', ')}`
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
