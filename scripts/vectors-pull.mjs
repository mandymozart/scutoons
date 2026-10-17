// The vector library (shared/tokens/vectors, owner 2026-10-17): re-exports every entry of vectors.json that has a
// `figma` node over the SCUTOONS Figma bridge (tools/figma-bridge – the relay running, the plugin open on the file),
// makes the `derived` ones (turned copies), measures each SVG (width, height, colors, bytes, sha256) back into
// vectors.json and writes vectors.css (one `--vector-<slug>: url(…)` each, for CSS masks).
//   node scripts/vectors-pull.mjs            all entries
//   node scripts/vectors-pull.mjs wreath …   only these slugs (the rest is measured as it is)
//   node scripts/vectors-pull.mjs --local    no Figma: derive + measure the files on disk
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';

const root = path.resolve(import.meta.dirname, '..');
const dir = path.join(root, 'shared/tokens/vectors');
const manifestPath = path.join(dir, 'vectors.json');
const args = process.argv.slice(2);
const local = args.includes('--local');
const only = new Set(args.filter((a) => !a.startsWith('--')));
const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
const file = (slug) => path.join(dir, `${slug}.svg`);

const failed = [];
let done = 0;
for (const v of manifest.vectors) {
	if (!v.figma || local || (only.size && !only.has(v.slug))) continue;
	let out;
	try {
		out = execFileSync(process.execPath, [path.join(root, 'tools/figma-bridge/cli.mjs'), 'export', '--node', v.figma.node, '--format', 'svg', '--to', file(v.slug)], { stdio: 'pipe' }).toString();
	} catch (err) {
		out = String(err.stdout ?? '') || JSON.stringify({ error: String(err.message) });
	}
	if (out.includes('"error"')) failed.push(`${v.slug} (${v.figma.node}): ${out.match(/"error":\s*"([^"]*)"/)?.[1] ?? out.trim()}`);
	else if (++done % 25 === 0) console.log(`${done} exported`);
}

for (const v of manifest.vectors) {
	if (!v.derived) continue;
	const svg = fs.readFileSync(file(v.derived.from), 'utf8');
	const [, w, h] = svg.match(/width="([\d.]+)" height="([\d.]+)"/);
	if (v.derived.rotate !== 90) throw new Error(`${v.slug}: only rotate 90 is supported`);
	const body = svg.replace(/^<svg[^>]*>/, '').replace(/<\/svg>\s*$/, '');
	fs.writeFileSync(file(v.slug), `<svg width="${h}" height="${w}" viewBox="0 0 ${h} ${w}" fill="none" xmlns="http://www.w3.org/2000/svg">\n<g transform="translate(${h} 0) rotate(90)">${body}</g>\n</svg>\n`);
}

for (const v of manifest.vectors) {
	if (!fs.existsSync(file(v.slug))) {
		failed.push(`${v.slug}: no file`);
		continue;
	}
	const bytes = fs.readFileSync(file(v.slug));
	const svg = bytes.toString('utf8');
	const [, w, h] = svg.match(/width="([\d.]+)" height="([\d.]+)"/) ?? [];
	const named = { black: '#000000', white: '#FFFFFF' };
	const colors = [...new Set([...svg.matchAll(/(?:fill|stroke|stop-color)="(#[0-9a-fA-F]{3,8}|black|white)"/g)].map((m) => named[m[1]] ?? m[1].toUpperCase()))].sort();
	Object.assign(v, {
		path: `vectors/${v.slug}.svg`,
		width: Number(w),
		height: Number(h),
		colors,
		// one colour (near-blacks count as one): usable as a CSS mask, the page colours it; more (or white parts): an
		// image as it is
		mask: !colors.includes('#FFFFFF') && (colors.length <= 1 || colors.every((c) => parseInt(c.slice(1, 3), 16) < 0x40)),
		bytes: bytes.length,
		sha256: createHash('sha256').update(bytes).digest('hex')
	});
}

// the same bytes twice = one shape (owner 2026-10-17): keep one entry, the other's Figma node goes to its `figma.copies`
const bySha = new Map();
for (const v of manifest.vectors) if (v.sha256) bySha.set(v.sha256, [...(bySha.get(v.sha256) ?? []), v.slug]);
const twins = [...bySha.values()].filter((slugs) => slugs.length > 1);
if (twins.length) console.log(`the same file twice (keep one, move the other's node to figma.copies):\n  ${twins.map((s) => s.join(' = ')).join('\n  ')}`);

fs.writeFileSync(manifestPath, JSON.stringify(manifest, null, '\t') + '\n');
const css = manifest.vectors.map((v) => `\t--vector-${v.slug}: url('./vectors/${v.slug}.svg');`).join('\n');
fs.writeFileSync(
	path.join(root, 'shared/tokens/vectors.css'),
	`/* @scutoons/tokens/vectors.css: the vector library as CSS (written by scripts/vectors-pull.mjs – edit vectors.json).\n   Use as a mask so the colour comes from the page: mask-image: var(--vector-wreath); background: var(--ornament). */\n:root {\n${css}\n}\n`
);
if (failed.length) console.log(`not exported:\n  ${failed.join('\n  ')}`);
console.log(`${manifest.vectors.length} vectors, ${Math.round(manifest.vectors.reduce((n, v) => n + (v.bytes ?? 0), 0) / 1024)} KB`);
