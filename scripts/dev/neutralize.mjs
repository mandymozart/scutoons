#!/usr/bin/env node
// The language-neutral conversion on the LOCAL dev database (agents/NEUTRAL.md "Migration"): every chapter, scene and
// season stored per locale (`chapter:<toon>:<n>:<l>` …) → one neutral document (`chapter:<toon>:<n>` …) with
// @scutoons/content `neutralize*` – the same function the creator's Convert page uses for production. The per-language
// toon settings (`toon-settings:<toon>:<l>`) are folded into the toon entry `toon:<toon>` (step 4, `planToonConversion`):
// a new entry version with their texts / release / status as locale maps; the old name `i18n.<l>.name` goes into
// `title` (decision 4: one name) – also for entries that have their texts already but still store `i18n`.
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
import { parseEntityKey, planConversion, planToonConversion, toonLanguages } from '../../shared/content/src/index.ts';

const args = process.argv.slice(2);
const apply = args.includes('--apply');
const option = (name) => (args.includes(name) ? args[args.indexOf(name) + 1] : null);
const onlyToon = option('--toon');
const jsonFile = option('--json');
const KINDS = ['chapter', 'season'];

/** Statements with positional parameters through PHP's PDO, the parameters on stdin (documents exceed the env limit) */
function write(statements) {
	const php = `$d=new PDO('mysql:host='.getenv('QR_DB_HOST').';port='.getenv('QR_DB_PORT').';dbname='.getenv('QR_DB_NAME').';charset=utf8mb4',getenv('QR_DB_USER'),getenv('QR_DB_PASS'));$d->setAttribute(PDO::ATTR_ERRMODE,PDO::ERRMODE_EXCEPTION);$d->beginTransaction();$ids=[];foreach(json_decode(file_get_contents('php://stdin'),true) as [$s,$p]){$d->prepare($s)->execute($p);$ids[]=(int)$d->lastInsertId();}$d->commit();echo json_encode($ids);`;
	return JSON.parse(execFileSync('php', [...phpMysqlArgs(), '-r', php], { env: { ...process.env, ...phpDbEnv() }, input: JSON.stringify(statements), encoding: 'utf8' }));
}

const rows = localSql([
	`SELECT v.id, v.entity_key, v.kind, v.toon, v.locale, v.status, v.json FROM entity_versions v
	 JOIN (SELECT entity_key, MAX(id) AS newest, MAX(CASE WHEN status = 'published' THEN id END) AS published FROM entity_versions
	       WHERE kind IN ('chapter', 'season', 'toon', 'toon-settings') GROUP BY entity_key) g ON v.id = g.newest OR v.id = g.published
	 ORDER BY v.id`
])[0];

/** entity → { published: { locale: row }, newest: { locale: row } } (per-locale rows); neutral: entity → { published, newest } */
const entities = new Map();
const neutral = new Map();
const toonEntries = new Map();
/** toon → { published: row, newest: row } of its entry; toon → { published: { locale: row }, newest } of its settings */
const entryRows = new Map();
const settingRows = new Map();
for (const row of rows) {
	const ref = parseEntityKey(row.entity_key);
	if (!ref) continue;
	if (ref.kind === 'toon') {
		if (!toonEntries.has(ref.toon) || row.status === 'published') toonEntries.set(ref.toon, JSON.parse(row.json));
		if (!entryRows.has(ref.toon)) entryRows.set(ref.toon, { published: null, newest: null });
		const e = entryRows.get(ref.toon);
		if (row.status === 'published') e.published = row;
		if (!e.newest || e.newest.id < row.id) e.newest = row;
		continue;
	}
	if (onlyToon && ref.toon !== onlyToon) continue;
	if (ref.kind === 'toon-settings') {
		if (!settingRows.has(ref.toon)) settingRows.set(ref.toon, { published: {}, newest: {} });
		const e = settingRows.get(ref.toon);
		if (row.status === 'published') e.published[ref.locale] = row;
		if (!e.newest[ref.locale] || e.newest[ref.locale].id < row.id) e.newest[ref.locale] = row;
		continue;
	}
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

const report = [];
const inserts = [];
let unchanged = 0;
console.log(`Neutral conversion on ${MARIADB.name}${apply ? '' : ' (dry run – --apply stores)'}\n`);
const version = (row) => ({ id: row.id, content: JSON.parse(row.json) });
const versions = (byLocale) => Object.fromEntries(Object.entries(byLocale).map(([l, row]) => [l, version(row)]));
for (const [entity, e] of [...entities].sort(([a], [b]) => a.localeCompare(b, 'en', { numeric: true }))) {
	const { source, languages } = toonLanguages(toonEntries.get(e.toon));
	const n = neutral.get(entity);
	// the same plan as the creator's Convert page (@scutoons/content planConversion)
	const steps = planConversion({
		kind: e.kind,
		source,
		languages,
		published: versions(e.published),
		newest: versions(e.newest),
		neutralPublished: n?.published ? JSON.parse(n.published.json) : undefined,
		neutralNewest: n?.newest ? JSON.parse(n.newest.json) : undefined
	});
	for (const step of steps) {
		report.push({ entity, status: step.status, source, versions: step.versions, report: step.report });
		console.log(`${entity}  ${step.status}  (${step.versions})${step.report.length ? `  – ${step.report.length} difference(s)` : ''}`);
		for (const r of step.report) {
			const values = Object.entries(r.values).map(([l, v]) => `${l}=${short(v)}`).join('  ');
			console.log(`    ${r.kind.padEnd(14)} ${r.path || '(document)'}  ${values}${r.missing ? `  (missing in ${r.missing.join(', ')})` : ''}`);
		}
		if (step.unchanged) {
			unchanged++;
			continue;
		}
		const message = `neutral conversion (${step.status}) from ${step.versions}`;
		const ref = parseEntityKey(entity);
		inserts.push([`INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES (?, ?, ?, NULL, ?, NULL, UTC_TIMESTAMP(), 'draft', ?)`,
			[entity, ref.kind, ref.toon, JSON.stringify(step.doc), message]]);
		if (step.status === 'published') {
			// the only published version of the neutral entity (EntityStore::setPublished); per-locale rows are never touched
			inserts.push([`UPDATE entity_versions SET status = CASE WHEN id = LAST_INSERT_ID() THEN 'published' ELSE 'draft' END WHERE entity_key = ? AND (status = 'published' OR id = LAST_INSERT_ID())`, [entity]]);
		}
	}
}

// the toon settings → the toon entry (step 4); the old name into the title (decision 4)
const toonIds = [...new Set([...settingRows.keys(), ...entryRows.keys()])].filter((t) => !onlyToon || t === onlyToon).sort();
for (const toon of toonIds) {
	const e = settingRows.get(toon) ?? { published: {}, newest: {} };
	const entity = `toon:${toon}`;
	const { source, languages } = toonLanguages(toonEntries.get(toon));
	const entry = entryRows.get(toon);
	const steps = planToonConversion({
		source,
		languages,
		entryPublished: entry?.published ? version(entry.published) : undefined,
		entryNewest: entry?.newest ? version(entry.newest) : undefined,
		published: versions(e.published),
		newest: versions(e.newest)
	});
	if (!steps.length) {
		unchanged++;
		console.log(`${entity}  ${!entry ? 'no toon entry – nothing to fold the settings into' : settingRows.has(toon) ? 'has its texts (and no i18n) already' : 'nothing to fold (no toon settings, no i18n with texts)'}`);
		continue;
	}
	for (const step of steps) {
		report.push({ entity, status: step.status, source, versions: step.versions, report: step.report });
		console.log(`${entity}  ${step.status}  (${step.versions})${step.report.length ? `  – ${step.report.length} difference(s)` : ''}`);
		for (const r of step.report) {
			const values = Object.entries(r.values).map(([l, v]) => `${l}=${short(v)}`).join('  ');
			console.log(`    ${r.kind.padEnd(14)} ${r.path || '(document)'}  ${values}${r.missing ? `  (missing in ${r.missing.join(', ')})` : ''}${r.invisible ? '  (no reader saw it)' : ''}`);
		}
		const message = step.nameOnly
			? `the old name (i18n) folded into the title (${step.status}) from ${step.versions}`
			: `toon settings folded into the entry (${step.status}) from ${step.versions}`;
		inserts.push([`INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES (?, 'toon', ?, NULL, ?, NULL, UTC_TIMESTAMP(), 'draft', ?)`,
			[entity, toon, JSON.stringify(step.doc), message]]);
		if (step.status === 'published') {
			inserts.push([`UPDATE entity_versions SET status = CASE WHEN id = LAST_INSERT_ID() THEN 'published' ELSE 'draft' END WHERE entity_key = ? AND (status = 'published' OR id = LAST_INSERT_ID())`, [entity]]);
		}
	}
}

const differences = report.reduce((n, r) => n + r.report.length, 0);
console.log(`\n${entities.size} entities, ${report.length} conversions (published + newer drafts), ${differences} reported difference(s); ${unchanged} already converted.`);
if (jsonFile) {
	writeFileSync(jsonFile, JSON.stringify(report, null, '\t') + '\n');
	console.log(`Report: ${jsonFile}`);
}
const stored = inserts.filter(([s]) => s.startsWith('INSERT')).length;
if (!apply) {
	console.log(stored ? `Would store ${stored} neutral version(s) – run again with --apply.` : 'Nothing to store.');
} else if (stored) {
	write(inserts);
	console.log(`Stored ${stored} neutral version(s).`);
} else {
	console.log('Nothing to store.');
}
