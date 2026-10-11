// The location dioramas (owner 2026-10-11, "Characters(13).zip"): every place image in 1x / 2x / 3x (256 / 512 / 1024,
// WebP – normalized by the session's script: each painted / plaster pair cropped to one shared square frame) uploaded
// through the SCUTOONS MCP (the member's own rights; upload_asset never overwrites – an existing path is reported and
// skipped), then a new draft of the world map: each place's image = the painted 1x (+ the plaster 1x), three new places
// for the pictures that had none (ore pit, ore mountain, Vienna subway – hidden: an unlockLocation event unlocks them).
//   node scripts/places-upload.mjs <folder with catpope/map/places/*.webp> [--read] [--assets-only] [--draft-only] [--variants]
// --variants: also the 2x / 3x files in `variants` / `plasterVariants` (only once the API knows them – after the deploy).
import { readFileSync, readdirSync } from 'node:fs';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { Client } from '@modelcontextprotocol/client';
import { StdioClientTransport } from '@modelcontextprotocol/client/stdio';

const args = process.argv.slice(2);
const flag = (f) => { const i = args.indexOf(f); return i >= 0 ? (args.splice(i, 1), true) : false; };
const readOnly = flag('--read'), assetsOnly = flag('--assets-only'), draftOnly = flag('--draft-only'), withVariants = flag('--variants');
const src = args[0];
if (!src) { console.error('usage: places-upload.mjs <folder> [--read] [--assets-only] [--draft-only] [--variants]'); process.exit(2); }
const DIR = resolve(src, 'catpope/map/places');
const FOLDER = 'catpope/map/places';

/** the pictures → places (owner's zip names; the three without a place get a new one) */
const PLACES = ['cathedral', 'mountain-path', 'ore-pit', 'monastery', 'infinity-pool', 'station', 'waterfall-tower', 'church-square', 'ore-mountain', 'graveyard', 'library', 'well', 'crypt', 'vienna-subway', 'apartment-blocks'];
const NEW_PLACES = [
	{ id: 'ore-pit', name: { de: 'Erz-Tagebau', en: 'Ore pit' }, x: 1492, y: 540 },
	{ id: 'ore-mountain', name: { de: 'Erzberg', en: 'Ore mountain' }, x: 1500, y: 506 },
	{ id: 'vienna-subway', name: { de: 'U-Bahn-Station', en: 'Subway station' }, x: 1214, y: 318 }
];

const cfg = JSON.parse(readFileSync(resolve(homedir(), '.claude.json'), 'utf8')).mcpServers?.scutoons;
if (!cfg) throw new Error('no "scutoons" MCP entry in ~/.claude.json');
const client = new Client({ name: 'scutoons-places-upload', version: '1.0.0' });
await client.connect(new StdioClientTransport({ command: cfg.command, args: cfg.args, env: { ...process.env, ...cfg.env }, stderr: 'ignore' }));
const call = async (name, a) => {
	const r = await client.callTool({ name, arguments: a });
	const text = (r.content ?? []).map((c) => c.text ?? '').join('\n');
	let data; try { data = JSON.parse(text); } catch { data = text; }
	return { error: !!r.isError, data };
};
const files = (id, plaster) => {
	const base = `${FOLDER}/${id}${plaster ? '-plaster' : ''}`;
	return { low: `${base}.webp`, medium: `${base}.2x.webp`, high: `${base}.3x.webp` };
};

try {
	const cur = await call('read_entity', { key: 'map' });
	const map = cur.data?.content ?? cur.data?.draft ?? cur.data;
	const version = cur.data?.version?.id ?? null;
	console.log(`map: ${cur.error ? `error – ${String(cur.data).slice(0, 300)}` : `version ${version ?? '?'} (${cur.data?.version?.status ?? '?'}), ${map?.locations?.length ?? '?'} places`}`);
	if (readOnly || cur.error) process.exit(cur.error ? 1 : 0);

	if (!draftOnly) {
		let done = 0, skipped = 0, failed = 0;
		for (const name of readdirSync(DIR).filter((f) => f.endsWith('.webp')).sort()) {
			const r = await call('upload_asset', { file: join(DIR, name), path: `${FOLDER}/${name}` });
			const text = typeof r.data === 'string' ? r.data : JSON.stringify(r.data);
			if (!r.error) done++;
			else if (/exists|409/i.test(text)) skipped++;
			else { failed++; console.log(`  ${name}: ${text.slice(0, 200)}`); }
		}
		console.log(`uploads: ${done} new, ${skipped} already there, ${failed} failed`);
		if (failed) process.exit(1);
	}
	if (assetsOnly) process.exit(0);

	const next = structuredClone(map);
	for (const p of NEW_PLACES) if (!next.locations.some((l) => l.id === p.id)) next.locations.push({ ...p, toons: ['catpope'], hidden: true });
	for (const id of PLACES) {
		const place = next.locations.find((l) => l.id === id);
		if (!place) { console.log(`  no place ${id}`); continue; }
		const painted = files(id, false), plaster = files(id, true);
		place.image = { ...(place.image ?? {}), src: painted.low, plaster: plaster.low };
		if (withVariants) {
			place.image.variants = painted;
			place.image.plasterVariants = plaster;
		}
	}
	const r = await call('save_draft', { key: 'map', base: version, content: next, message: `Places: the dioramas (painted + plaster${withVariants ? ', 1x / 2x / 3x' : ''}) for ${PLACES.length} places; new: ${NEW_PLACES.map((p) => p.id).join(', ')}` });
	console.log(`save_draft: ${r.error ? 'refused – ' : ''}${typeof r.data === 'string' ? r.data.slice(0, 800) : JSON.stringify(r.data).slice(0, 800)}`);
} finally {
	await client.close();
}
