#!/usr/bin/env node
// The language-neutral conversion on the LOCAL dev database (agents/NEUTRAL.md "Migration"): every chapter, scene and
// season stored per locale (`chapter:<toon>:<n>:<l>` …) → one neutral document (`chapter:<toon>:<n>` …) with
// @scutoons/content `neutralize*` – the same function the creator's Convert page uses for production.
//
//   npm run dev:neutralize                     dry run: the report per entity (what the source locale overrides)
//   npm run dev:neutralize -- --apply          … and store the neutral documents as NEW entity versions
//   npm run dev:neutralize -- --toon catpope   one toon only
//   npm run dev:neutralize -- --json <file>    the full report as JSON
//
// Per entity: the published per-locale versions → a published neutral version; when a locale has a newer draft (or the
// entity was never published) also the newest versions → a draft neutral version after it (creators' draft mode reads
// the newest). Idempotent: nothing is stored when the newest neutral version already has that content. The per-locale
// rows are never changed (rollback = delete the neutral rows). Database: scripts/dev/db-env.mjs (SCUTOONS_DB_NAME).
import { execFileSync } from 'node:child_process';
import { writeFileSync } from 'node:fs';
import { localSql, MARIADB, phpDbEnv, phpMysqlArgs } from './db-env.mjs';
import { neutralizeDocs, parseEntityKey, toonLanguages } from '../../shared/content/src/index.ts';

const args = process.argv.slice(2);
const apply = args.includes('--apply');
const option = (name) => (args.includes(name) ? args[args.indexOf(name) + 1] : null);
const onlyToon = option('--toon');
const jsonFile = option('--json');
const KINDS = ['chapter', 'scene', 'season'];

/** Statements with positional parameters through PHP's PDO, the parameters on stdin (documents exceed the env limit) */
function write(statements) {
	const php = `$d=new PDO('mysql:host='.getenv('QR_DB_HOST').';port='.getenv('QR_DB_PORT').';dbname='.getenv('QR_DB_NAME').';charset=utf8mb4',getenv('QR_DB_USER'),getenv('QR_DB_PASS'));$d->setAttribute(PDO::ATTR_ERRMODE,PDO::ERRMODE_EXCEPTION);$d->beginTransaction();$ids=[];foreach(json_decode(file_get_contents('php://stdin'),true) as [$s,$p]){$d->prepare($s)->execute($p);$ids[]=(int)$d->lastInsertId();}$d->commit();echo json_encode($ids);`;
	return JSON.parse(execFileSync('php', [...phpMysqlArgs(), '-r', php], { env: { ...process.env, ...phpDbEnv() }, input: JSON.stringify(statements), encoding: 'utf8' }));
}

const rows = localSql([
	`SELECT v.id, v.entity_key, v.kind, v.toon, v.locale, v.status, v.json FROM entity_versions v
	 JOIN (SELECT entity_key, MAX(id) AS newest, MAX(CASE WHEN status = 'published' THEN id END) AS published FROM entity_versions
	       WHERE kind IN ('chapter', 'scene', 'season', 'toon') GROUP BY entity_key) g ON v.id = g.newest OR v.id = g.published
	 ORDER BY v.id`
])[0];

/** entity → { published: { locale: row }, newest: { locale: row } } (per-locale rows); neutral: entity → { published, newest } */
const entities = new Map();
const neutral = new Map();
const toonEntries = new Map();
for (const row of rows) {
	const ref = parseEntityKey(row.entity_key);
	if (!ref) continue;
	if (ref.kind === 'toon') {
		if (!toonEntries.has(ref.toon) || row.status === 'published') toonEntries.set(ref.toon, JSON.parse(row.json));
		continue;
	}
	if (onlyToon && ref.toon !== onlyToon) continue;
	if (!ref.locale) {
		if (!neutral.has(row.entity_key)) neutral.set(row.entity_key, { published: null, newest: null });
		const n = neutral.get(row.entity_key);
		if (row.status === 'published') n.published = row;
		if (!n.newest || n.newest.id < row.id) n.newest = row;
		continue;
	}
	const entity = row.entity_key.slice(0, -(ref.locale.length + 1));
	if (!entities.has(entity)) entities.set(entity, { kind: ref.kind, toon: ref.toon, published: {}, newest: {} });
	const e = entities.get(entity);
	if (row.status === 'published') e.published[ref.locale] = row;
	if (!e.newest[ref.locale] || e.newest[ref.locale].id < row.id) e.newest[ref.locale] = row;
}

const short = (value) => {
	const text = JSON.stringify(value) ?? 'undefined';
	return text.length > 140 ? `${text.slice(0, 137)}…` : text;
};

/** the per-locale rows in the toon's language order (source first, then the rest) */
function ordered(byLocale, languages) {
	const locales = [...languages.filter((l) => byLocale[l]), ...Object.keys(byLocale).filter((l) => !languages.includes(l)).sort()];
	return Object.fromEntries(locales.map((l) => [l, byLocale[l]]));
}

const report = [];
const inserts = [];
let unchanged = 0;
console.log(`Neutral conversion on ${MARIADB.name}${apply ? '' : ' (dry run – --apply stores)'}\n`);
for (const [entity, e] of [...entities].sort(([a], [b]) => a.localeCompare(b, 'en', { numeric: true }))) {
	const { source, languages } = toonLanguages(toonEntries.get(e.toon));
	const variants = [];
	if (Object.keys(e.published).length) variants.push(['published', ordered(e.published, languages)]);
	const newerDraft = Object.entries(e.newest).some(([l, row]) => row.id !== e.published[l]?.id);
	if (newerDraft) variants.push(['draft', ordered(e.newest, languages)]);
	let { published, newest } = neutral.get(entity) ?? { published: null, newest: null };
	for (const [status, byLocale] of variants) {
		const docs = Object.fromEntries(Object.entries(byLocale).map(([l, row]) => [l, JSON.parse(row.json)]));
		const result = neutralizeDocs(e.kind, docs, source);
		const versions = Object.entries(byLocale).map(([l, row]) => `${l}#${row.id}`).join(' ');
		report.push({ entity, status, source, versions, report: result.report });
		console.log(`${entity}  ${status}  (${versions})${result.report.length ? `  – ${result.report.length} difference(s)` : ''}`);
		for (const r of result.report) {
			const values = Object.entries(r.values).map(([l, v]) => `${l}=${short(v)}`).join('  ');
			console.log(`    ${r.kind.padEnd(14)} ${r.path || '(document)'}  ${values}${r.missing ? `  (missing in ${r.missing.join(', ')})` : ''}`);
		}
		const json = JSON.stringify(result.doc);
		if ((status === 'published' ? published : newest)?.json === json) {
			unchanged++;
			continue;
		}
		const message = `neutral conversion (${status}) from ${versions}`;
		const ref = parseEntityKey(entity);
		inserts.push([`INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES (?, ?, ?, NULL, ?, NULL, UTC_TIMESTAMP(), 'draft', ?)`,
			[entity, ref.kind, ref.toon, json, message]]);
		if (status === 'published') {
			// the only published version of the neutral entity (EntityStore::setPublished); per-locale rows are never touched
			inserts.push([`UPDATE entity_versions SET status = CASE WHEN id = LAST_INSERT_ID() THEN 'published' ELSE 'draft' END WHERE entity_key = ? AND (status = 'published' OR id = LAST_INSERT_ID())`, [entity]]);
		}
		newest = { json };
		if (status === 'published') published = newest;
	}
}

const differences = report.reduce((n, r) => n + r.report.length, 0);
console.log(`\n${entities.size} entities, ${report.length} conversions (published + newer drafts), ${differences} reported difference(s); ${unchanged} already converted.`);
if (jsonFile) {
	writeFileSync(jsonFile, JSON.stringify(report, null, '\t') + '\n');
	console.log(`Report: ${jsonFile}`);
}
const versions = inserts.filter(([s]) => s.startsWith('INSERT')).length;
if (!apply) {
	console.log(versions ? `Would store ${versions} neutral version(s) – run again with --apply.` : 'Nothing to store.');
} else if (versions) {
	write(inserts);
	console.log(`Stored ${versions} neutral version(s).`);
} else {
	console.log('Nothing to store.');
}
