// The places' sizes on the hex grid (owner 2026-10-11: "wie in SimCity – Brunnen ist die Hälfte der Krypta, die Krypta
// die Hälfte vom Pool, die U-Bahn doppelt so groß wie die Krypta, Kirche / Bahnhof / Kloster gleich (Bahnhof etwas
// länger), die kleine Kirche die Hälfte vom Dom, Apartment-Blocks und Friedhof größer, Erzberg und Erzloch die größten")
// as a new map draft: each place gets its `size` (world units), then every place is snapped once to the nearest free
// spot of the grid – the big ones first – so no two overlap. Production through the SCUTOONS MCP, or the local DB.
// BACKLOG 10.67 (owner 2026-10-11, "das Größte ≈ 12"): the sizes ÷8, a cell of 1; the places snap from the positions of
// `--from <version id>` (the map before the first snap: the author's own placement), else from the latest.
//   node --experimental-transform-types scripts/places-sizes.mjs [--local] [--dry] [--from <version id>]
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { resolve } from 'node:path';
import { snapPlace, placeSize } from '../shared/content/src/map.ts';

const SIZES = {
	well: 0.75, crypt: 1.5, 'church-square': 2, 'infinity-pool': 3, 'vienna-subway': 3,
	cathedral: 4, monastery: 4, library: 4, 'mountain-path': 4, station: 4.5,
	'apartment-blocks': 6, graveyard: 6, 'waterfall-tower': 6, 'ore-pit': 12, 'ore-mountain': 12
};
const args = process.argv.slice(2);
const local = args.includes('--local'), dry = args.includes('--dry');
const from = args.includes('--from') ? Number(args[args.indexOf('--from') + 1]) : null;

/** `map` = the latest; `origin` = the map whose positions the places snap from (default the latest) */
function arrange(map, origin = map) {
	const next = structuredClone(map);
	for (const p of next.locations) {
		const o = origin.locations.find((x) => x.id === p.id);
		if (o) (p.x = o.x), (p.y = o.y);
		if (SIZES[p.id]) p.size = SIZES[p.id];
		else delete p.size; // the default
		if (p.image) delete p.image.size; // one size: the place's
		// a place's own view zoom ×4, like PLACE_ZOOM 2 → 8 (the places shrank ÷8)
		if (p.view?.zoom) p.view.zoom = Math.min(32, p.view.zoom * 4); // run once per map
	}
	const placed = [];
	const moves = [];
	for (const p of [...next.locations].sort((a, b) => placeSize(b) - placeSize(a))) {
		const spot = snapPlace({ width: next.width, height: next.height, locations: placed }, p, placeSize(p));
		if (Math.hypot(spot.x - p.x, spot.y - p.y) > 0.01) moves.push(`${p.id} ${Math.round(Math.hypot(spot.x - p.x, spot.y - p.y))}`);
		p.x = spot.x;
		p.y = spot.y;
		placed.push(p);
	}
	return { next, moves };
}

if (local) {
	const sql = (q) => execFileSync('docker', ['exec', '-i', 'scutoons-mariadb-1', 'sh', '-c', 'mariadb -uroot -p"$MARIADB_ROOT_PASSWORD" -N -r scutoons'], { input: q, maxBuffer: 1 << 26 }).toString();
	const read = (where) => JSON.parse(sql(`SELECT json FROM entity_versions WHERE entity_key='map' ${where} ORDER BY id DESC LIMIT 1;`).trim());
	const map = read('');
	const { next, moves } = arrange(map, from ? read(`AND id=${from}`) : map);
	console.log('moved (units):', moves.join(', '));
	if (!dry) {
		const BS = String.fromCharCode(92);
		const json = JSON.stringify(next).split(BS).join(BS + BS).split("'").join(BS + "'");
		sql(`INSERT INTO entity_versions (entity_key, kind, toon, locale, json, created_at, status, message) VALUES ('map', 'map', NULL, NULL, '${json}', NOW(), 'draft', 'Places: sizes ÷8 on a finer hex grid (cell 1), snapped from the author placement');`);
		console.log('local draft saved');
	}
} else {
	const { Client } = await import('@modelcontextprotocol/client');
	const { StdioClientTransport } = await import('@modelcontextprotocol/client/stdio');
	const cfg = JSON.parse(readFileSync(resolve(homedir(), '.claude.json'), 'utf8')).mcpServers.scutoons;
	const client = new Client({ name: 'scutoons-places-sizes', version: '1.0.0' });
	await client.connect(new StdioClientTransport({ command: cfg.command, args: cfg.args, env: { ...process.env, ...cfg.env }, stderr: 'ignore' }));
	const call = async (name, a) => { const r = await client.callTool({ name, arguments: a }); const t = (r.content ?? []).map((c) => c.text ?? '').join('\n'); let d; try { d = JSON.parse(t); } catch { d = t; } return { error: !!r.isError, data: d }; };
	try {
		const cur = await call('read_entity', { key: 'map' });
		const map = cur.data?.content ?? cur.data;
		const origin = from ? await call('read_entity', { key: 'map', version: from }) : null;
		const { next, moves } = arrange(map, origin ? (origin.data?.content ?? origin.data) : map);
		console.log(`map v${cur.data?.version?.id}: moved (units):`, moves.join(', '));
		if (!dry) {
			const r = await call('save_draft', { key: 'map', base: cur.data?.version?.id ?? null, content: next, message: 'Places: sizes ÷8 (well 0.75 … ore mountain 12) on a finer hex grid (cell 1), every place snapped to a free spot' });
			console.log('save_draft:', JSON.stringify(r.data).slice(0, 400));
		}
	} finally {
		await client.close();
	}
}
