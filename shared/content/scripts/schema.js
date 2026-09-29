// Generates the JSON Schemas in ../schema from the TypeScript types (the server validates creator writes with them).
//   node scripts/schema.js          write schema/*.schema.json
//   node scripts/schema.js --check  fail when a committed schema is out of date (part of `npm run check`)
import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createGenerator } from 'ts-json-schema-generator';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');

/** file name → root type. Content files: `<toon>/<locale>/{toon,season1,chapterN,<chapter>/scene-N}.json`. */
const SCHEMAS = {
	chapter: 'Chapter',
	season: 'Season',
	toon: 'Toon',
	'toon-index': 'ToonIndex',
	group: 'Group',
	catalog: 'Catalog'
};

const generator = createGenerator({
	path: join(root, 'src/index.ts'),
	tsconfig: join(root, 'tsconfig.json'),
	expose: 'export',
	topRef: true,
	jsDoc: 'extended',
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
	const text = JSON.stringify(schema, null, '\t') + '\n';
	const file = join(root, 'schema', `${name}.schema.json`);
	if (check) {
		if (!existsSync(file) || readFileSync(file, 'utf8').replace(/\r\n/g, '\n') !== text) stale.push(file);
	} else {
		writeFileSync(file, text);
		console.log(`schema/${name}.schema.json ← ${type}`);
	}
}

if (stale.length) {
	console.error(`Out of date (run \`npm run schema -w @scutoons/content\`):\n${stale.join('\n')}`);
	process.exit(1);
}
if (check) console.log(`@scutoons/content: ${Object.keys(SCHEMAS).length} JSON Schemas up to date`);
