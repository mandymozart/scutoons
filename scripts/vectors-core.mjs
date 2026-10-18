// The vector library → the asset manager, the part both ways share (agents/research/VECTORS.md §6 step 5):
// scripts/vectors-import.mjs (the local dev API, as the test user) and the SCUTOONS MCP tool `import_vectors` (any API,
// as the signed-in member – production). Every entry of shared/tokens/vectors/vectors.json is shrunk with svgo (owner
// 2026-10-17: "shrink is fine"), uploaded to `catpope/vectors/<slug>.svg` and made a shape (title, tags, shape row).
// Plan first, then write: a path taken by an asset that is NO shape stops the whole run before anything is written;
// a shape with other bytes is replaced only where the caller allows it (`replace` – the local dev copy), else it is
// reported and left alone. A shape whose bytes, title, tags and shape row are already right is not touched.
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { optimize } from 'svgo';

export const VECTORS_DIR = path.resolve(import.meta.dirname, '../shared/tokens/vectors');
const TOON = 'catpope';

/** svgo's defaults, but the viewBox and the size stay (masks and layout scale by them) */
export function shrink(svg) {
	const out = optimize(svg, { multipass: true, plugins: [{ name: 'preset-default', params: { overrides: { removeViewBox: false } } }] }).data;
	if (!/<svg\b[^>]*\b(width="[\d.]+"[^>]*\bheight="[\d.]+"|viewBox=)/.test(out)) throw new Error('svgo dropped the size');
	return out;
}

/** the API's tag rule (AssetRegistry::encodeTags): lower case a–z 0–9 space _ . - */
const tagsOf = (v) => v.tags.filter((t) => /^[a-z0-9 _.-]{1,40}$/.test(t));
const sha = (bytes) => createHash('sha256').update(bytes).digest('hex');
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);

/** what the asset should look like: title, tags, the shape row */
function wanted(manifest, v) {
	const figma = v.figma ? { ...(manifest.sources[v.figma.source] ?? {}), ...v.figma } : null;
	if (figma) delete figma.source;
	return { title: v.title, tags: tagsOf(v), shape: { tile: v.tile ?? null, mask: Boolean(v.mask), colors: v.colors ?? [], source: figma, derived: v.derived ?? null } };
}

/**
 * @param {{
 *   find: (key: string) => Promise<null | { id: string, sha256: string | null, title: string, tags: string[], shape: object | null }>,
 *   upload: (bytes: Buffer, key: string, title: string, tags: string[]) => Promise<string>,
 *   replaceFile?: (bytes: Buffer, key: string) => Promise<void>,
 *   save: (id: string, body: { title: string, tags: string[], shape: object }) => Promise<void>,
 *   dryRun?: boolean, only?: Set<string>, limit?: number, log?: (line: string) => void
 * }} io
 */
export async function importVectors(io) {
	const log = io.log ?? (() => {});
	const manifest = JSON.parse(fs.readFileSync(path.join(VECTORS_DIR, 'vectors.json'), 'utf8'));
	const plan = [];
	let before = 0;
	let after = 0;
	for (const v of manifest.vectors) {
		if (io.only?.size && !io.only.has(v.slug)) continue;
		const raw = fs.readFileSync(path.join(VECTORS_DIR, `${v.slug}.svg`), 'utf8');
		const bytes = Buffer.from(shrink(raw));
		before += Buffer.byteLength(raw);
		after += bytes.length;
		const key = `${TOON}/vectors/${v.slug}.svg`;
		const found = await io.find(key);
		const want = wanted(manifest, v);
		let action;
		if (!found) action = 'upload';
		else if (!found.shape) action = 'collision';
		else if (found.sha256 !== sha(bytes)) action = io.replaceFile ? 'replace' : 'conflict';
		else action = same({ title: found.title, tags: found.tags, shape: found.shape }, want) ? 'unchanged' : 'update';
		plan.push({ slug: v.slug, key, bytes, want, found, action });
		if (plan.length % 50 === 0) log(`${plan.length} planned …`);
	}
	const count = (a) => plan.filter((p) => p.action === a).length;
	const summary = {
		shapes: plan.length,
		upload: count('upload'),
		replace: count('replace'),
		update: count('update'),
		unchanged: count('unchanged'),
		conflict: plan.filter((p) => p.action === 'conflict').map((p) => p.key),
		collision: plan.filter((p) => p.action === 'collision').map((p) => p.key),
		kb: { before: Math.round(before / 1024), after: Math.round(after / 1024) }
	};
	// a path held by something that is no shape: nothing is written at all
	if (summary.collision.length || io.dryRun) return { ...summary, written: 0, failed: [], left: 0 };

	const todo = plan.filter((p) => ['upload', 'replace', 'update'].includes(p.action));
	const batch = io.limit ? todo.slice(0, io.limit) : todo;
	const failed = [];
	let written = 0;
	for (const p of batch) {
		try {
			let id = p.found?.id;
			if (p.action === 'upload') id = await io.upload(p.bytes, p.key, p.want.title, p.want.tags);
			if (p.action === 'replace') await io.replaceFile(p.bytes, p.key);
			// the upload already set title and tags – the shape row is what remains (and every field on an update)
			await io.save(id, p.want);
			written++;
		} catch (err) {
			failed.push(`${p.slug}: ${err.message ?? err}`);
		}
		if ((written + failed.length) % 25 === 0) log(`${written + failed.length} of ${batch.length} written …`);
	}
	return { ...summary, written, failed, left: todo.length - batch.length };
}
