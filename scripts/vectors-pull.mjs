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

for (const v of manifest.vectors) {
	if (!v.figma || local || (only.size && !only.has(v.slug))) continue;
	execFileSync(process.execPath, [path.join(root, 'tools/figma-bridge/cli.mjs'), 'export', '--node', v.figma.node, '--format', 'svg', '--to', file(v.slug)], { stdio: 'pipe' });
	console.log('exported', v.slug);
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

fs.writeFileSync(manifestPath, JSON.stringify(manifest, null, '\t') + '\n');
const css = manifest.vectors.map((v) => `\t--vector-${v.slug}: url('./vectors/${v.slug}.svg');`).join('\n');
fs.writeFileSync(
	path.join(root, 'shared/tokens/vectors.css'),
	`/* @scutoons/tokens/vectors.css: the vector library as CSS (written by scripts/vectors-pull.mjs – edit vectors.json).\n   Use as a mask so the colour comes from the page: mask-image: var(--vector-wreath); background: var(--ornament). */\n:root {\n${css}\n}\n`
);
console.log(`${manifest.vectors.length} vectors, ${Math.round(manifest.vectors.reduce((n, v) => n + v.bytes, 0) / 1024)} KB`);
