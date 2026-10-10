// Uploads a production draft (figma-production.mjs → storyboards/_out/<ch>/production/) through the SCUTOONS MCP server
// – the same server and token as Claude Code's `scutoons` MCP entry (~/.claude.json), started here over stdio, so the
// member's own rights apply: upload_asset for every slice (never overwrites – an existing path is reported and
// skipped), then save_draft of the chapter (a new draft version on top of the current one; readers keep the published
// version – publishing stays the owner's button in the creator).
//
//   node scripts/storyboard/production-upload.mjs storyboards/catpope/s1c3 --key chapter:catpope:3 [--read] [--assets-only] [--draft-only]
//     --read   only show the chapter's current state on the server (no change)

import { readFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { basename, resolve } from 'node:path';
import { Client } from '@modelcontextprotocol/client';
import { StdioClientTransport } from '@modelcontextprotocol/client/stdio';
import { ROOT } from './lib.mjs';

const args = process.argv.slice(2);
const take = (f, d) => { const i = args.indexOf(f); return i >= 0 ? args.splice(i, 2)[1] : d; };
const flag = (f) => { const i = args.indexOf(f); return i >= 0 ? (args.splice(i, 1), true) : false; };
const key = take('--key');
const readOnly = flag('--read'), assetsOnly = flag('--assets-only'), draftOnly = flag('--draft-only');
const src = args[0];
if (!src || !key) { console.error('usage: production-upload.mjs <storyboard folder> --key chapter:<toon>:<n> [--read] [--assets-only] [--draft-only]'); process.exit(2); }
const PROD = resolve(ROOT, 'storyboards/_out', basename(resolve(src)), 'production');

const cfg = JSON.parse(readFileSync(resolve(homedir(), '.claude.json'), 'utf8')).mcpServers?.scutoons;
if (!cfg) throw new Error('no "scutoons" MCP entry in ~/.claude.json');
const client = new Client({ name: 'scutoons-production-upload', version: '1.0.0' });
await client.connect(new StdioClientTransport({ command: cfg.command, args: cfg.args, env: { ...process.env, ...cfg.env }, stderr: 'ignore' }));
const call = async (name, a) => {
	const r = await client.callTool({ name, arguments: a });
	const text = (r.content ?? []).map((c) => c.text ?? '').join('\n');
	let data; try { data = JSON.parse(text); } catch { data = text; }
	return { error: !!r.isError, data };
};

try {
	const cur = await call('read_entity', { key });
	const doc = cur.data?.content ?? cur.data?.draft ?? cur.data;
	const version = cur.data?.version?.id ?? null; // the version the new draft builds on
	console.log(`${key}: ${cur.error ? `error – ${String(cur.data).slice(0, 300)}` : `version ${version ?? '?'} (${cur.data?.version?.status ?? '?'}), groups: ${(doc?.groups ?? []).map((g) => `${g.label}(${(g.slices ?? []).length})`).join(', ')}`}`);
	if (readOnly) { console.log(JSON.stringify(cur.data?.version), JSON.stringify(doc).slice(0, 300)); process.exit(0); }

	if (!draftOnly) {
		const uploads = JSON.parse(readFileSync(resolve(PROD, 'uploads.json'), 'utf8'));
		let done = 0, skipped = 0, failed = 0;
		for (const u of uploads) {
			const r = await call('upload_asset', { file: resolve(PROD, u.file), path: u.path });
			if (!r.error) done++;
			else if (/409|exists/i.test(String(r.data))) skipped++;
			else { failed++; console.log(`\n${u.path}: ${String(r.data).slice(0, 200)}`); if (failed > 5) throw new Error('too many upload failures – stopped'); }
			process.stdout.write(`\rassets ${done + skipped + failed}/${uploads.length} (new ${done}, already there ${skipped}, failed ${failed})   `);
		}
		console.log();
	}
	if (!assetsOnly) {
		const { meta, ...chapter } = JSON.parse(readFileSync(resolve(PROD, 'chapter.json'), 'utf8')); // meta: our own bookkeeping, not part of the chapter
		const r = await call('save_draft', { key, base: version, content: chapter, message: `Storyboard first throw from Figma (${meta?.production?.from ?? 'figma-production.mjs'}): ${chapter.groups.length} scene groups, ${chapter.groups.reduce((a, g) => a + g.slices.length, 0)} slices` });
		console.log(`save_draft: ${r.error ? 'refused – ' : ''}${typeof r.data === 'string' ? r.data.slice(0, 600) : JSON.stringify(r.data).slice(0, 600)}`);
	}
} finally {
	await client.close();
}
