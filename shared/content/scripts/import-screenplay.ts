// Writes a screenplay of the screenplay editor's layout (`<dir>/screenplay.txt` + `<dir>/config.json` → `meta`) as the
// file of a season's or a chapter's script entity (agents/DATA.md "Script", research/KANON.md §3):
// `<out>/<toon>/<locale>/season-script<n>.json` / `chapter-script<n>.json` (default out:
// logs/dev/screenplay, gitignored – production is the content's source of truth; the creator edits scripts there)
// = `{ text, meta }`, the text byte for byte (no trimming, no line-end or whitespace changes; LF stays LF).
//
//   node --experimental-transform-types --no-warnings shared/content/scripts/import-screenplay.ts \
//        --dir=C:/Development/cat-pope-agent/external-repos/cat-pope-awakening/de --toon=catpope --locale=de [--season=1 | --chapter=<n>]
//
// Then into the local store (published, idempotent; --update replaces a stored, different one with a new version):
//   npm run dev:db -- console import-content ../logs/dev/screenplay --toon=catpope --only=season-script [--update]
// Scripts are creator-only: `import-content --sql` (the production seed) leaves them out unless --only names the kind.
import { mkdirSync, readFileSync, writeFileSync, existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { SCRIPT_META_FIELDS, validateFile, type ScriptMeta, type ToonScript } from '../src/index.ts';

const args = Object.fromEntries(process.argv.slice(2).map((a) => a.replace(/^--/, '').split('=') as [string, string]));
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const dir = args.dir;
const toon = args.toon;
const locale = args.locale;
const out = resolve(args.out ?? join(root, '../../logs/dev/screenplay'));
// a season's script (default: season 1 – how Cat Pope was written) or a chapter's (how Top Toy is written)
const kind = args.chapter !== undefined ? 'chapter-script' : 'season-script';
const number = args.chapter ?? args.season ?? '1';
if (!dir || !toon || !locale || !/^[a-z0-9_-]{1,64}$/.test(toon) || !/^[a-z]{2}$/.test(locale) || !/^(0|[1-9]\d{0,5})$/.test(number) || (kind === 'season-script' && number === '0')) {
	console.error('usage: import-screenplay.ts --dir=<folder with screenplay.txt [+ config.json]> --toon=<id> --locale=<xx> [--season=<n> | --chapter=<n>] [--out=logs/dev/screenplay]');
	process.exit(2);
}

const bytes = readFileSync(join(dir, 'screenplay.txt'));
const text = bytes.toString('utf8');
if (!Buffer.from(text, 'utf8').equals(bytes)) throw new Error('screenplay.txt is not valid UTF-8 (it would not round-trip)');

const script: ToonScript = { text };
const configFile = join(dir, 'config.json');
if (existsSync(configFile)) {
	const source = JSON.parse(readFileSync(configFile, 'utf8'))?.meta ?? {};
	const meta: ScriptMeta = {};
	for (const field of SCRIPT_META_FIELDS) if (typeof source[field] === 'string' && source[field]) meta[field] = source[field];
	if (Object.keys(meta).length) script.meta = meta;
}

const problems = validateFile(kind, script, locale, { file: `${kind}:${toon}:${number}` } as never).filter((p) => p.level === 'error');
if (problems.length) throw new Error(problems.map((p) => p.message).join('\n'));

const file = join(out, toon, locale, `${kind}${number}.json`);
mkdirSync(dirname(file), { recursive: true });
writeFileSync(file, JSON.stringify(script, null, '\t') + '\n');
const check = JSON.parse(readFileSync(file, 'utf8')) as ToonScript;
if (check.text !== text) throw new Error('round trip failed');
console.log(`${file}: ${(text.match(/\n/g) ?? []).length} lines, ${bytes.length} bytes, meta ${Object.keys(script.meta ?? {}).join(', ') || '–'} (text identical)`);
