// The shot list of a chapter's storyboards, grouped by place and by what each shot wants (photo / unreal / direct /
// generated / art): the sheet for matching the Figma selection (S1C3 page), for planning a shooting, and – for the
// gaps – for prompting. Markdown.
//
//   node scripts/storyboard/shotlist.mjs storyboards/catpope/s1c3 [--out storyboards/_out]

import { mkdirSync, writeFileSync } from 'node:fs';
import { basename, resolve } from 'node:path';
import { ROOT, textOf } from './lib.mjs';
import { loadStoryboards, measureShot, shotsOf } from './storyboard.mjs';

const args = process.argv.slice(2);
const oi = args.indexOf('--out');
const outRoot = resolve(ROOT, oi >= 0 ? args.splice(oi, 2)[1] : 'storyboards/_out');
const src = args[0];
if (!src) {
	console.error('usage: shotlist.mjs <storyboard folder>');
	process.exit(2);
}
const list = loadStoryboards(src);
const name = basename(resolve(src));
const rows = [];
for (const { sb } of list) {
	const places = Object.fromEntries((sb.locations ?? []).map((l) => [l.ref, l]));
	const cast = Object.fromEntries((sb.cast ?? []).map((c) => [c.ref, c]));
	for (const { shot } of shotsOf(sb)) {
		const loc = places[shot.location];
		const m = measureShot(shot);
		rows.push({
			scene: sb.source?.scene, id: shot.id, place: loc?.place ?? loc?.new ?? loc?.ref ?? '–', time: loc?.time ?? '',
			want: shot.source?.want ?? '?', format: `${shot.format} ${shot.aspect}`, px: shot.format === 'loop360' ? `${m.panoScreens.toFixed(1)} screens wide` : `390 × ${m.h}`,
			framing: [shot.shot?.size, shot.shot?.angle, shot.projection !== 'perspective' ? shot.projection : ''].filter(Boolean).join(' '),
			who: (shot.figures ?? []).map((f) => `${cast[f.cast]?.new ?? f.cast}${f.mood ? ` (${f.mood})` : ''}`).join(', '),
			action: textOf(shot.action)
		});
	}
}
const wants = ['photo', 'direct', 'unreal', 'generated', 'art', '?'];
const md = [`# Shot list ${name} – ${rows.length} shots`, '',
	'What each shot needs, by place. Match against the Figma selection on the chapter page; the shooting pages only for gaps.',
	'`want`: photo = from a shooting · direct = to be photographed · unreal = render · generated = prompt it (Luma, manual) · art = drawn / graphic.', '',
	'| want | shots |', '|---|---|', ...wants.map((w) => `| ${w} | ${rows.filter((r) => r.want === w).length} |`).filter((l) => !l.endsWith('| 0 |')), ''];
const byPlace = [...new Set(rows.map((r) => r.place))];
for (const place of byPlace) {
	const rs = rows.filter((r) => r.place === place);
	md.push(`## ${place} (${rs.length})`, '', '| shot | want | format | size | framing | who | what the image shows |', '|---|---|---|---|---|---|---|');
	for (const r of rs) md.push(`| ${r.id} | ${r.want} | ${r.format} | ${r.px} | ${r.framing} | ${r.who} | ${r.action.replace(/\|/g, '/')} |`);
	md.push('');
}
mkdirSync(outRoot, { recursive: true });
const file = resolve(outRoot, `${name}-shotlist.md`);
writeFileSync(file, md.join('\n'));
console.log(`${rows.length} shots → ${file}`);
for (const w of wants) {
	const n = rows.filter((r) => r.want === w).length;
	if (n) console.log(`  ${w}: ${n}`);
}
