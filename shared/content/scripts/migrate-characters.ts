// Migrates the old per-locale characters of the toons into the characters model of characters.ts:
//   <api>/<toon>/{de,en}/characters.json  ([{ name, label, image, color, images: { <mood>: { avatar, portrait, full } },
//                                            description, collection }] – one file per locale)
// → <out>/scutoons/_/characters.json (global registry) + <out>/<toon>/_/cast.json (+ report on stdout).
//
//   node --experimental-transform-types --no-warnings shared/content/scripts/migrate-characters.ts \
//        [--api=viewer/static/api] [--cdn=cdn/root] [--out=shared/content/seed] [--json]
//
// Then import (published version 1, idempotent):
//   php server/bin/console import-content shared/content/seed --only=characters.json,cast.json
// Production SQL:
//   php server/bin/console import-content shared/content/seed --only=characters.json,cast.json --sql
//
// Rules: characters are matched by `name` (the id) across locales; `label` / `description` become { de, en } texts;
// color / collection (→ group) / image paths come from the toon's source language (de). Image paths get the toon
// folder in front (registry paths are from the CDN root); when the locales disagree on a path, the one that exists on
// the local CDN copy (`cdn/root`) wins, the source language first (reported). Toons without a characters.json get the
// registry characters of their own (EXTRA below) in their cast.
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
	CAST_FILE,
	CHARACTER_IMAGE_VARIANTS,
	CHARACTERS_FILE,
	CREDITS_SCOPE,
	LEGACY_CHARACTERS_FILE,
	resolveCast,
	validateCast,
	validateCharacters,
	type Character,
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

/** Characters of toons that have no characters.json yet (owner, 2026-09-29): the protagonists of their own toons. */
const EXTRA: { toon: string; character: RegistryCharacter }[] = [
	{
		toon: 'yellowmouse',
		character: {
			id: 'mouse',
			name: { de: 'Mausboss', en: 'Yellow Mouse' },
			color: 'yellow',
			group: 'lead',
			own_toon: 'yellowmouse',
			mannequin: 'mouse',
			image: 'yellowmouse/avatar.webp',
			images: { neutral: {} }
		}
	},
	{
		toon: 'toptoy',
		character: {
			id: 'toptoy',
			name: { de: 'Toptoy', en: 'Toptoy' },
			description: { de: 'Das Duo', en: 'The duo' },
			color: 'black',
			group: 'lead',
			own_toon: 'toptoy',
			tags: ['duo'],
			image: 'toptoy/avatar.webp',
			images: { neutral: {} }
		}
	}
];

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
const toons = readdirSync(API, { withFileTypes: true })
	.filter((d) => d.isDirectory() && LOCALES.some((l) => existsSync(join(API, d.name, l, LEGACY_CHARACTERS_FILE))))
	.map((d) => d.name)
	.sort();

for (const toon of toons) {
	const files = Object.fromEntries(LOCALES.map((l) => [l, read(toon, l) ?? []]));
	const ids: string[] = [];
	for (const l of [SOURCE, ...LOCALES.filter((x) => x !== SOURCE)]) for (const c of files[l]) if (c?.name && !ids.includes(c.name)) ids.push(c.name);
	const members: ToonCast['members'] = [];
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
		const images: RegistryCharacter['images'] = {};
		const moods = [...new Set(LOCALES.flatMap((l) => Object.keys(byLocale[l]?.images ?? {})))];
		for (const mood of moods) {
			const slots: Record<string, string> = {};
			for (const variant of CHARACTER_IMAGE_VARIANTS) {
				const path = pickPath(
					toon,
					`${toon}/${id} ${mood} ${variant}`,
					Object.fromEntries(LOCALES.map((l) => [l, (byLocale[l]?.images?.[mood] as Record<string, string> | undefined)?.[variant]]))
				);
				if (path) slots[variant] = path;
			}
			images[mood] = slots;
		}
		const image = pickPath(toon, `${toon}/${id} image`, Object.fromEntries(LOCALES.map((l) => [l, byLocale[l]?.image])));
		const name = text('label') ?? { [SOURCE]: id };
		const description = text('description');
		const character: RegistryCharacter = {
			id,
			name,
			...(description ? { description } : {}),
			color: source.color,
			group: source.collection ?? 'cast',
			...(image ? { image } : {}),
			images
		};
		if (registry.characters.some((c) => c.id === id)) ambiguous.push(`${toon}/${id}: id already in the registry from another toon – kept the first`);
		else registry.characters.push(character);
		members.push({ character: id, order: members.length + 1 });
	}
	casts[toon] = { members };
	notes.push(`${toon}: ${members.length} characters (${ids.join(', ')}) from ${LOCALES.filter((l) => read(toon, l)).map((l) => `${l}/${LEGACY_CHARACTERS_FILE}`).join(' + ')}`);
}

for (const { toon, character } of EXTRA) {
	if (registry.characters.some((c) => c.id === character.id)) continue;
	registry.characters.push(character);
	casts[toon] ??= { members: [] };
	if (!casts[toon].members.some((m) => m.character === character.id)) casts[toon].members.push({ character: character.id, order: casts[toon].members.length + 1 });
	notes.push(`${toon}: registry character "${character.id}" (own toon, neutral images empty, image placeholder ${character.image})`);
}

// ── Check: the resolved cast must equal the old files (apart from the reported path choices) ────────────────────
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
			const r = b[i];
			if (!r) return diffs.push(`${c.name} missing`);
			for (const key of Object.keys(c) as (keyof typeof c)[]) {
				if (JSON.stringify(c[key]) !== JSON.stringify(r[key])) diffs.push(`${c.name}.${key}: ${JSON.stringify(c[key])} → ${JSON.stringify(r[key])}`);
			}
		});
		checks.push(`${toon}/${locale}: ${diffs.length ? `${diffs.length} differences – ${diffs.join('; ')}` : 'identical to the old characters.json'}`);
	}
}

const problems = [
	...validateCharacters(registry, { languages: LOCALES, toons: [...toons, ...EXTRA.map((e) => e.toon)] }),
	...Object.entries(casts).flatMap(([toon, cast]) => validateCast(cast, { registry, languages: LOCALES, file: `${toon}/_/${CAST_FILE}` }))
].filter((p) => p.level !== 'info');

const write = (toon: string, file: string, data: unknown) => {
	const path = join(OUT, toon, '_', file);
	mkdirSync(dirname(path), { recursive: true });
	writeFileSync(path, JSON.stringify(data, null, '\t') + '\n');
	return path.replace(repo + '\\', '').replace(repo + '/', '');
};
const written = [write(CREDITS_SCOPE, CHARACTERS_FILE, registry), ...Object.entries(casts).map(([toon, cast]) => write(toon, CAST_FILE, cast))];

const summary = {
	characters: registry.characters.map((c) => `${c.id} (${Object.values(c.name).join(' / ')}, ${c.group}${c.own_toon ? `, own toon ${c.own_toon}` : ''})`),
	casts: Object.fromEntries(Object.entries(casts).map(([t, c]) => [t, c.members.map((m) => m.character)])),
	ambiguous,
	notes,
	checks,
	problems: problems.map((p) => `${p.level} ${p.file} ${p.path.join('.')}: ${p.message}`),
	files: written
};

if (process.argv.includes('--json')) console.log(JSON.stringify(summary, null, 2));
else {
	console.log(`characters migration: ${registry.characters.length} registry characters, casts: ${Object.entries(casts).map(([t, c]) => `${t} ${c.members.length}`).join(', ')}`);
	for (const [title, lines] of [['Characters', summary.characters], ['Ambiguous (check)', ambiguous], ['Notes', notes], ['Resolved vs old files', checks], ['Problems', summary.problems], ['Written', written]] as const) {
		if (!lines.length) continue;
		console.log(`\n${title}:`);
		for (const line of lines) console.log(`  - ${line}`);
	}
}
