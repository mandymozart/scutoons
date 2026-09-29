// Migrates the old per-locale characters of the toons into the characters model of characters.ts and adds the
// characters the owner delivered images for:
//   <api>/<toon>/{de,en}/characters.json  ([{ name, label, image, color, images: { <mood>: { avatar, portrait, full } },
//                                            description, collection }] – one file per locale)
//   <cdn>/characters/<id>/<id>[_<look>]_<variant>-<mood>.png, <id>_<artwork>.png   (convention folder per character)
// → <out>/scutoons/_/characters.json (global registry), <out>/scutoons/_/groups.json (affiliations),
//   <out>/<toon>/_/cast.json (+ report on stdout).
//
//   node --experimental-transform-types --no-warnings shared/content/scripts/migrate-characters.ts \
//        [--api=viewer/static/api] [--cdn=cdn/root] [--out=shared/content/seed] [--json]
//
// Then import (published version 1, idempotent; `--update` for a changed seed):
//   php server/bin/console import-content shared/content/seed --only=characters.json,groups.json,cast.json
// Production SQL:
//   php server/bin/console import-content shared/content/seed --only=characters.json,groups.json,cast.json --sql
//
// Rules: characters are matched by `name` (the id) across locales; `label` / `description` become { de, en } texts;
// color / collection (→ group) / image paths come from the toon's source language (de). Image paths get the toon
// folder in front (registry paths are from the CDN root); when the locales disagree on a path, the one that exists on
// the local CDN copy wins, the source language first (reported). Old images go to the look `default`.
// A character with a convention folder on the CDN gets its images from there (all looks and moods found, replacing the
// old per-mood paths of that look; the old single `image` stays the last fallback); other files in the folder
// (`<id>_keyvisual.png`) become artwork.
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
	CAST_FILE,
	CHARACTER_IMAGE_ROOT,
	CHARACTER_IMAGE_VARIANTS,
	CHARACTERS_FILE,
	CREDITS_SCOPE,
	DEFAULT_LOOK,
	GROUPS_FILE,
	LEGACY_CHARACTERS_FILE,
	isKnownMood,
	lookIds,
	resolveCast,
	validateCast,
	validateCharacters,
	validateGroups,
	type Character,
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

const GROUPS: GroupVocabulary = {
	groups: [{ id: 'biker-gang', name: { de: 'Biker-Gang', en: 'Biker gang' } }]
};

const WITH_HELMET_LOOKS = [
	{ id: DEFAULT_LOOK, label: { de: 'ohne Helm', en: 'without helmet' }, default: true },
	{ id: 'helmet', label: { de: 'mit Helm', en: 'with helmet' } }
];

/** Changes to migrated characters. */
const UPDATES: Record<string, Partial<RegistryCharacter>> = {
	// the Cat Pope is the protagonist of his toon (his biker persona inherits it)
	catpope: { own_toon: 'catpope' }
};

/** New registry characters and the toon casts they join. */
const NEW: { casts: string[]; character: RegistryCharacter }[] = [
	{
		casts: ['catpope'],
		character: {
			id: 'catpope-biker',
			persona_of: 'catpope',
			persona: {
				label: { de: 'Biker', en: 'Biker' },
				introduced: { toon: 'catpope' },
				note: { de: 'Der Katzenpapst, nachdem er sich der Biker-Gang angeschlossen hat.', en: 'Cat Pope after he joins the biker gang.' }
			},
			group: 'lead',
			looks: WITH_HELMET_LOOKS,
			images: {},
			affiliations: [{ group: 'biker-gang', role: { de: 'Mitglied', en: 'Member' }, since: { toon: 'catpope' } }]
		}
	},
	{
		casts: ['yellowmouse'],
		character: {
			id: 'yellowmouse',
			name: { de: 'Mausboss', en: 'Yellow Mouse' },
			description: {
				de: 'Die Anführerin der Biker-Gang – das war sie schon immer. Als Biker zeigt sie ihr wahres Ich.',
				en: 'The head of the biker gang – she always has been. As the biker she shows her true self.'
			},
			color: 'yellow',
			group: 'lead',
			own_toon: 'yellowmouse',
			mannequin: 'mouse',
			image: 'yellowmouse/avatar.webp',
			images: {},
			affiliations: [{ group: 'biker-gang', role: { de: 'Anführerin', en: 'Leader' } }]
		}
	},
	{
		casts: ['yellowmouse'],
		character: {
			id: 'yellowmouse-biker',
			persona_of: 'yellowmouse',
			// no `introduced`: she has always been the head of the gang
			persona: { label: { de: 'Biker', en: 'Biker' } },
			group: 'lead',
			looks: WITH_HELMET_LOOKS,
			images: {},
			affiliations: [{ group: 'biker-gang', role: { de: 'Anführerin', en: 'Leader' } }]
		}
	},
	{
		casts: ['toptoy'],
		character: {
			id: 'toptoy',
			name: { de: 'Toptoy', en: 'Toptoy' },
			description: { de: 'Das Duo', en: 'The duo' },
			color: 'black',
			group: 'lead',
			own_toon: 'toptoy',
			tags: ['duo'],
			image: 'toptoy/avatar.webp',
			images: { [DEFAULT_LOOK]: { neutral: {} } }
		}
	}
];

const ARTWORK_LABELS: Record<string, LocaleText> = { keyvisual: { de: 'Keyvisual', en: 'Key visual' } };

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
const addToCast = (toon: string, id: string) => {
	casts[toon] ??= { members: [] };
	if (!casts[toon].members.some((m) => m.character === id)) casts[toon].members.push({ character: id, order: casts[toon].members.length + 1 });
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
			for (const variant of CHARACTER_IMAGE_VARIANTS) {
				const path = pickPath(
					toon,
					`${toon}/${id} ${mood} ${variant}`,
					Object.fromEntries(LOCALES.map((l) => [l, (byLocale[l]?.images?.[mood] as Record<string, string> | undefined)?.[variant]]))
				);
				if (path) slots[variant] = path;
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
			images: { [DEFAULT_LOOK]: moods },
			...(UPDATES[id] ?? {})
		};
		if (registry.characters.some((c) => c.id === id)) ambiguous.push(`${toon}/${id}: id already in the registry from another toon – kept the first`);
		else registry.characters.push(character);
		addToCast(toon, id);
	}
	notes.push(`${toon}: ${ids.length} characters (${ids.join(', ')}) from ${LOCALES.filter((l) => read(toon, l)).map((l) => `${l}/${LEGACY_CHARACTERS_FILE}`).join(' + ')}`);
}

for (const { casts: into, character } of NEW) {
	if (!registry.characters.some((c) => c.id === character.id)) registry.characters.push(character);
	for (const toon of into) addToCast(toon, character.id);
	notes.push(`${character.id}: new${character.persona_of ? ` persona of ${character.persona_of}` : ''}, cast ${into.join(', ')}`);
}

// ── Convention folders on the CDN (the owner's delivered images) ─────────────────────────────────────────────────

const byLookFound: Record<string, Record<string, number>> = {};
for (const character of registry.characters) {
	const folder = join(CDN, CHARACTER_IMAGE_ROOT, character.id);
	if (!existsSync(folder)) continue;
	const escaped = character.id.replace(/[-]/g, '\\-');
	const expression = new RegExp(`^${escaped}(?:_([a-z0-9-]+))?_(${CHARACTER_IMAGE_VARIANTS.join('|')})-([a-z0-9-]+)\\.png$`);
	const artwork = new RegExp(`^${escaped}_([a-z0-9-]+)\\.(png|jpe?g|webp)$`);
	const found: Record<string, Record<string, Record<string, string>>> = {};
	const extras = new Set<string>();
	for (const file of readdirSync(folder).sort()) {
		const path = `${CHARACTER_IMAGE_ROOT}/${character.id}/${file}`;
		const e = file.match(expression);
		if (e) {
			const [, look = DEFAULT_LOOK, variant, mood] = e;
			((found[look] ??= {})[mood] ??= {})[variant] = path;
			if (!isKnownMood(mood)) extras.add(mood);
			continue;
		}
		const a = file.match(artwork);
		if (a) {
			character.artwork ??= [];
			if (!character.artwork.some((x) => x.id === a[1])) character.artwork.push({ id: a[1], path, ...(ARTWORK_LABELS[a[1]] ? { label: ARTWORK_LABELS[a[1]] } : {}) });
			continue;
		}
		ambiguous.push(`${character.id}: ${path} matches no convention (ignored)`);
	}
	for (const [look, moods] of Object.entries(found)) {
		if (!lookIds(character).includes(look)) {
			character.looks ??= [{ id: DEFAULT_LOOK, label: { de: 'Standard', en: 'Default' }, default: true }];
			character.looks.push({ id: look, label: { de: look, en: look } });
			ambiguous.push(`${character.id}: look "${look}" found on the CDN without a label – label it in the creator`);
		}
		const replaced = Object.keys(character.images[look] ?? {}).length;
		character.images[look] = moods;
		byLookFound[character.id] = { ...(byLookFound[character.id] ?? {}), [look]: Object.values(moods).reduce((n, s) => n + Object.keys(s).length, 0) };
		notes.push(
			`${character.id}: look "${look}" from ${CHARACTER_IMAGE_ROOT}/${character.id}/ – ${Object.keys(moods).length} moods, ${byLookFound[character.id][look]} images` +
				(replaced ? ` (replaces ${replaced} old mood entries; old single image stays: ${character.image ?? '–'})` : '')
		);
	}
	if (extras.size) notes.push(`${character.id}: extra moods ${[...extras].join(', ')}`);
	// looks declared without images keep an empty neutral entry (the grid shows them as missing)
	for (const look of lookIds(character)) character.images[look] ??= {};
}
// stable key order: declared looks first
for (const c of registry.characters) {
	const ordered: RegistryCharacter['images'] = {};
	for (const look of lookIds(c)) ordered[look] = c.images[look] ?? {};
	c.images = ordered;
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
		a.forEach((c, i) => {
			const r = b.find((x) => x.name === c.name);
			if (!r) return diffs.push(`${c.name} missing`);
			for (const key of Object.keys(c) as (keyof typeof c)[]) {
				if (JSON.stringify(c[key]) !== JSON.stringify(r[key])) diffs.push(`${c.name}.${key}: ${JSON.stringify(c[key])} → ${JSON.stringify(r[key])}`);
			}
		});
		const added = b.filter((r) => !a.some((c) => c.name === r.name)).map((r) => r.name);
		checks.push(
			`${toon}/${locale}: ${diffs.length ? `${diffs.length} differences – ${diffs.join('; ')}` : 'identical to the old characters.json'}${added.length ? ` (+ new: ${added.join(', ')})` : ''}`
		);
	}
}

const allToons = [...new Set([...toons, ...Object.keys(casts)])];
const problems = [
	...validateGroups(GROUPS, LOCALES),
	...validateCharacters(registry, { languages: LOCALES, toons: allToons, groups: GROUPS.groups.map((g) => g.id) }),
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
	...Object.entries(casts).map(([toon, cast]) => write(toon, CAST_FILE, cast))
];

const summary = {
	characters: registry.characters.map(
		(c) =>
			`${c.id} (${Object.values(c.name ?? {}).join(' / ') || `persona of ${c.persona_of}`}${c.persona ? ` · ${Object.values(c.persona.label).join(' / ')}` : ''}, ${c.group}${c.own_toon ? `, own toon ${c.own_toon}` : ''}, looks ${lookIds(c).join('+')}${c.artwork?.length ? `, artwork ${c.artwork.map((a) => a.id).join(', ')}` : ''}${c.affiliations?.length ? `, ${c.affiliations.map((a) => `${a.group} (${a.role?.en ?? '–'})`).join(', ')}` : ''})`
	),
	groups: GROUPS.groups.map((g) => g.id),
	casts: Object.fromEntries(Object.entries(casts).map(([t, c]) => [t, c.members.map((m) => m.character)])),
	ambiguous,
	notes,
	checks,
	problems: problems.map((p) => `${p.level} ${p.file} ${p.path.join('.')}: ${p.message}`),
	files: written
};

if (process.argv.includes('--json')) console.log(JSON.stringify(summary, null, 2));
else {
	console.log(
		`characters migration: ${registry.characters.length} registry characters, ${GROUPS.groups.length} group(s), casts: ${Object.entries(casts).map(([t, c]) => `${t} ${c.members.length}`).join(', ')}`
	);
	for (const [title, lines] of [
		['Characters', summary.characters],
		['Casts', Object.entries(summary.casts).map(([t, ids]) => `${t}: ${ids.join(', ')}`)],
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
