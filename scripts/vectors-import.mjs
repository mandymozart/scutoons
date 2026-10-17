// The vector library → the asset manager (agents/research/VECTORS.md §6 step 5): every entry of
// shared/tokens/vectors/vectors.json is shrunk with svgo (owner 2026-10-17: "shrink is fine") and uploaded through the
// SCUTOONS API to `<toon>/vectors/<slug>.svg` (all of the Figma "Vector" page belongs to Cat Pope), then made a shape
// (`PUT /api/assets/{id} { shape }` – tile, mask, colours, Figma provenance, derived). Idempotent: a file whose bytes are
// already there is left alone (only its shape row and tags are written again); other bytes replace it (clean slate –
// nothing uses the shapes yet). LOCAL dev API only for now: as the test user (npm run dev:test-user); production
// waits until the owner asks.
//   node scripts/vectors-import.mjs [--dry-run] [slug …]
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { optimize } from 'svgo';
import { local } from './dev/ports.mjs';

const root = path.resolve(import.meta.dirname, '..');
const dir = path.join(root, 'shared/tokens/vectors');
const manifest = JSON.parse(fs.readFileSync(path.join(dir, 'vectors.json'), 'utf8'));
const args = process.argv.slice(2);
const dry = args.includes('--dry-run');
const only = new Set(args.filter((a) => !a.startsWith('--')));
const TOON = 'catpope';

const API = (process.env.SCUTOONS_API ?? local('api', '/api')).replace(/\/$/, '');
if (!/^http:\/\/(localhost|127\.0\.0\.1)(:\d+)?\//.test(API + '/')) throw new Error(`refusing ${API}: the import runs against the local dev API only (production when the owner asks)`);
const session = path.join(root, 'logs/dev/test-user.json');
if (!fs.existsSync(session)) throw new Error('no test-user session: npm run dev:test-user');
const { token } = JSON.parse(fs.readFileSync(session, 'utf8'));

async function call(method, url, body, type = 'application/json') {
	const res = await fetch(API + url, {
		method,
		headers: { Authorization: `Bearer ${token}`, ...(body !== undefined ? { 'Content-Type': type } : {}) },
		body: body === undefined ? undefined : type === 'application/json' ? JSON.stringify(body) : body
	});
	const json = await res.json().catch(() => ({}));
	if (!res.ok) throw new Error(`${method} ${url} → ${res.status} ${json.error?.code ?? json.code ?? ''} ${json.error?.message ?? json.message ?? ''}`.trim());
	return json;
}

/** svgo's defaults, but the viewBox and the size stay (masks and layout scale by them) */
const shrink = (svg) =>
	optimize(svg, {
		multipass: true,
		plugins: [{ name: 'preset-default', params: { overrides: { removeViewBox: false } } }]
	}).data;

/** the API's tag rule (AssetRegistry::encodeTags): lower case a–z 0–9 space _ . - */
const tagsOf = (v) => v.tags.filter((t) => /^[a-z0-9 _.-]{1,40}$/.test(t));

const sha = (bytes) => createHash('sha256').update(bytes).digest('hex');
const count = { uploaded: 0, replaced: 0, unchanged: 0, failed: 0 };
let before = 0;
let after = 0;
for (const v of manifest.vectors) {
	if (only.size && !only.has(v.slug)) continue;
	const raw = fs.readFileSync(path.join(dir, `${v.slug}.svg`), 'utf8');
	const small = Buffer.from(shrink(raw));
	if (!/<svg\b[^>]*\bwidth="[\d.]+"[^>]*\bheight="[\d.]+"|<svg\b[^>]*\bviewBox=/.test(small.toString())) throw new Error(`${v.slug}: svgo dropped the size`);
	before += Buffer.byteLength(raw);
	after += small.length;
	if (dry) continue;
	const key = `${TOON}/vectors/${v.slug}.svg`;
	try {
		const found = await call('GET', `/assets/lookup/?path=${encodeURIComponent(key)}`);
		let id = found.asset?.asset?.id;
		const current = found.asset?.versions?.find((x) => x.status === 'current');
		if (!id) {
			const up = await call('PUT', `/assets/upload/?path=${encodeURIComponent(key)}&title=${encodeURIComponent(v.title)}&tags=${encodeURIComponent(tagsOf(v).join(','))}`, small, 'image/svg+xml');
			id = up.asset.id;
			count.uploaded++;
		} else if (current?.sha256 !== sha(small)) {
			await call('PUT', `/assets/replace/?path=${encodeURIComponent(key)}&note=${encodeURIComponent('vectors-import')}`, small, 'image/svg+xml');
			count.replaced++;
		} else count.unchanged++;
		const figma = v.figma ? { ...(manifest.sources[v.figma.source] ?? {}), ...v.figma } : null;
		if (figma) delete figma.source;
		await call('PUT', `/assets/${id}`, {
			title: v.title,
			tags: tagsOf(v),
			shape: { tile: v.tile ?? null, mask: Boolean(v.mask), colors: v.colors ?? [], source: figma, derived: v.derived ?? null }
		});
	} catch (err) {
		count.failed++;
		console.log(`${v.slug}: ${err.message}`);
	}
	const done = count.uploaded + count.replaced + count.unchanged + count.failed;
	if (done % 50 === 0) console.log(`${done} …`);
}
console.log(`${dry ? 'dry run: ' : ''}${JSON.stringify(count)} · ${Math.round(before / 1024)} KB → ${Math.round(after / 1024)} KB (svgo)`);
if (count.failed) process.exitCode = 1;
