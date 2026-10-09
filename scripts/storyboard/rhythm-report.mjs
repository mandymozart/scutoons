// Rhythm report over existing chapters (STORYBOARD.md phase 1a): per slice length, text, reading + look time; per
// screen the density (walls); silent stretches; the chapter's minutes. Writes a Markdown summary and an HTML page
// with the rail (slice thumbnails from the media dev copy, if present).
//
//   node scripts/storyboard/rhythm-report.mjs catpope:1 catpope:2 [--out storyboards/_out] [--fit 17.5]
//
// Inputs: "toon:n" = the published chapter from the local dev DB (read-only), or a chapter JSON file.
// --fit M  solves the careful look time per screen so that the FIRST chapter takes M minutes (owner: ch1 = 15–20).

import { mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { RHYTHM, ROOT, S, TARGET, W, esc, fmtMin, loadChapter, mediaRoot, readSeconds, textOf, words } from './lib.mjs';

const args = process.argv.slice(2);
const opt = (name, dflt) => {
	const i = args.indexOf(name);
	if (i < 0) return dflt;
	const v = args[i + 1];
	args.splice(i, 2);
	return v;
};
const outDir = resolve(ROOT, opt('--out', 'storyboards/_out'));
const fit = opt('--fit', null);
const refs = args.length ? args : ['catpope:1', 'catpope:2'];

/** flatten a chapter into measured units: slices of the reading stack + VN messages */
export function measure(ch) {
	const units = [];
	let y = 0;
	for (const g of ch.groups ?? []) {
		const type = g.renderer?.type ?? 'vertical';
		const slices = g.slices ?? [];
		const msgs = (g.events ?? []).filter((e) => e.type === 'showMessage');
		const vn = msgs.length > 0;
		slices.forEach((sl, i) => {
			const h = sl.width ? (sl.height * W) / sl.width : sl.height || S;
			const boxes = (sl.dialog ?? []).map((d) => ({ y: y + ((d.rect?.y ?? 0) / 100) * h, n: words(d.text), text: textOf(d.text) }));
			// a horizontal group is one screen of scroll; each slide is one more look
			const scrollScreens = type === 'horizontal' ? (i === 0 ? 1 : 0) : h / S;
			units.push({
				kind: 'slice', group: g.label ?? '(no label)', type, vn, i, image: sl.image, video: !!sl.video, h, y,
				screens: h / S, scrollScreens, boxes, words: boxes.reduce((a, b) => a + b.n, 0), items: (sl.items ?? []).length
			});
			if (type !== 'horizontal' || i === slices.length - 1) y += type === 'horizontal' ? S : h;
		});
		if (vn) {
			let since = 0;
			let maxRun = 0;
			for (const e of g.events ?? []) {
				if (e.type === 'showMessage') since++;
				if (e.type === 'showChoices' || e.type === 'joinScene' || e.type === 'leaveScene') since = 0;
				maxRun = Math.max(maxRun, since);
			}
			units.push({
				kind: 'vn', group: g.label, messages: msgs.length, words: msgs.reduce((a, m) => a + words(m.params?.text), 0),
				choices: (g.events ?? []).filter((e) => e.type === 'addChoice').length, maxRun
			});
		}
	}
	return { units, height: y };
}

export function time(m, look) {
	let scroll = 0, lookT = 0, text = 0, vn = 0;
	for (const u of m.units) {
		if (u.kind === 'slice') {
			scroll += RHYTHM.scrollPerScreen * u.scrollScreens;
			lookT += look * u.screens;
			text += u.boxes.reduce((a, b) => a + readSeconds(b.n), 0);
		} else vn += u.messages * (RHYTHM.perBox + RHYTHM.perClick) + u.words / RHYTHM.wordsPerSecond;
	}
	return { scroll, look: lookT, text, vn, total: scroll + lookT + text + vn };
}

/** sliding windows of one screen, a quarter screen apart, over the reading stack */
export function windows(m) {
	const boxes = m.units.filter((u) => u.kind === 'slice').flatMap((u) => u.boxes);
	const out = [];
	for (let top = 0; top < Math.max(1, m.height - S / 2); top += S / 4) {
		const inside = boxes.filter((b) => b.y >= top && b.y < top + S);
		out.push({ top, words: inside.reduce((a, b) => a + b.n, 0), boxes: inside.length });
	}
	return out;
}

function silentRuns(slices) {
	const runs = [];
	let start = -1;
	slices.forEach((u, k) => {
		if (u.words === 0 && start < 0) start = k;
		if ((u.words > 0 || k === slices.length - 1) && start >= 0) {
			const end = u.words > 0 ? k - 1 : k;
			const screens = slices.slice(start, end + 1).reduce((a, s) => a + s.screens, 0);
			if (end - start + 1 >= 4) runs.push({ from: start + 1, to: end + 1, slices: end - start + 1, screens });
			start = -1;
		}
	});
	return runs.sort((a, b) => b.screens - a.screens);
}

const median = (xs) => {
	const s = [...xs].sort((a, b) => a - b);
	return s.length ? s[Math.floor(s.length / 2)] : 0;
};

function analyse(ref, look) {
	const ch = loadChapter(ref);
	const m = measure(ch);
	const slices = m.units.filter((u) => u.kind === 'slice');
	const vns = m.units.filter((u) => u.kind === 'vn');
	const win = windows(m);
	const walls = win.filter((w) => w.words > RHYTHM.wall.words || w.boxes > RHYTHM.wall.boxes);
	const t = time(m, look.careful);
	const tSkim = time(m, look.skim);
	return {
		ref, title: textOf(ch.title), number: ch.number, m, slices, vns, win, walls, t, tSkim,
		screens: m.height / S,
		words: slices.reduce((a, s) => a + s.words, 0),
		boxes: slices.reduce((a, s) => a + s.boxes.length, 0),
		silent: silentRuns(slices),
		vnWords: vns.reduce((a, v) => a + v.words, 0),
		vnMessages: vns.reduce((a, v) => a + v.messages, 0)
	};
}

// --- fit the careful look time on the first chapter ---
const look = { ...RHYTHM.lookPerScreen };
if (fit) {
	const m = measure(loadChapter(refs[0]));
	const base = time(m, 0);
	const screens = m.units.filter((u) => u.kind === 'slice').reduce((a, u) => a + u.screens, 0);
	look.careful = Math.max(0, (Number(fit) * 60 - base.total) / screens);
}

const reports = refs.map((r) => analyse(r, look));

// --- Markdown ---
const md = [];
md.push(`# Rhythm report – ${refs.join(', ')}`, '');
md.push(`Model (STORYBOARD.md §6): scroll ${RHYTHM.scrollPerScreen} s/screen · look **${look.careful.toFixed(1)} s/screen careful**, ${look.skim} s skim${fit ? ` (careful fitted: ${refs[0]} = ${fit} min)` : ''} · text ${RHYTHM.perBox} s + words/${RHYTHM.wordsPerSecond} per box · VN message + ${RHYTHM.perClick} s tap. Screen = ${W} × ${S}. Target per chapter: ${TARGET.screens.join('–')} screens, ${TARGET.minutes.join('–')} min careful.`, '');
md.push('| | ' + reports.map((r) => `${r.ref} "${r.title}"`).join(' | ') + ' |');
md.push('|---|' + reports.map(() => '---').join('|') + '|');
const row = (label, f) => md.push(`| ${label} | ` + reports.map(f).join(' | ') + ' |');
row('slices (reading stack)', (r) => r.slices.length);
row('screens', (r) => r.screens.toFixed(1));
row('text boxes / words (de)', (r) => `${r.boxes} / ${r.words}`);
row('words per screen (median window)', (r) => median(r.win.map((w) => w.words)));
row(`walls (> ${RHYTHM.wall.words} words or > ${RHYTHM.wall.boxes} boxes per screen)`, (r) => `${r.walls.length} of ${r.win.length} windows, worst ${Math.max(0, ...r.win.map((w) => w.words))} words`);
row('silent stretches ≥ 4 slices (longest)', (r) => (r.silent.length ? `${r.silent.length}; slices ${r.silent[0].from}–${r.silent[0].to} = ${r.silent[0].screens.toFixed(1)} screens` : '–'));
row('VN groups / messages / words', (r) => `${r.vns.length} / ${r.vnMessages} / ${r.vnWords}`);
row('VN longest run without a choice', (r) => Math.max(0, ...r.vns.map((v) => v.maxRun)));
row('time: look / text / scroll / VN', (r) => `${fmtMin(r.t.look)} / ${fmtMin(r.t.text)} / ${fmtMin(r.t.scroll)} / ${fmtMin(r.t.vn)}`);
row('**careful total**', (r) => `**${fmtMin(r.t.total)}**`);
row('skim total', (r) => fmtMin(r.tSkim.total));
md.push('');
for (const r of reports) {
	md.push(`## ${r.ref} – words per slice (reading order)`, '', '`' + r.slices.map((s) => s.words).join(' ') + '`', '');
	if (r.walls.length) {
		const worst = [...r.walls].sort((a, b) => b.words - a.words).slice(0, 5);
		md.push('Worst walls (window top in screens): ' + worst.map((w) => `${(w.top / S).toFixed(1)} → ${w.words} words / ${w.boxes} boxes`).join(' · '), '');
	}
}
mkdirSync(outDir, { recursive: true });
const mdPath = resolve(outDir, 'rhythm-report.md');
writeFileSync(mdPath, md.join('\n'));

// --- HTML rail ---
const media = mediaRoot();
const toon = (ref) => (ref.includes(':') ? ref.split(':')[0] : 'catpope');
const SCALE = 0.25; // px of the rail per px of the canvas
const rem = (px) => `${(px / 16).toFixed(3)}rem`;
function rail(r) {
	const maxSec = 40;
	const rows = r.slices.map((s, k) => {
		const h = s.screens * S * SCALE;
		const lookS = look.careful * s.screens;
		const textS = s.boxes.reduce((a, b) => a + readSeconds(b.n), 0);
		const src = media && s.image ? pathToFileURL(resolve(media, toon(r.ref), s.image)).href : '';
		const dense = s.words > RHYTHM.wall.words;
		return `<div class="row" style="height:${rem(Math.max(h, 6))}" title="${esc(s.group)} · slice ${k + 1} · ${s.words} words · ${(lookS + textS).toFixed(0)} s">
  <div class="img">${src ? `<img loading="lazy" src="${src}" alt="">` : ''}</div>
  <div class="bar"><span class="look" style="width:${((lookS / maxSec) * 100).toFixed(1)}%"></span><span class="text${dense ? ' dense' : ''}" style="width:${((textS / maxSec) * 100).toFixed(1)}%"></span></div>
  <div class="lbl">${k + 1}${s.type === 'horizontal' ? ' ↔' : ''}${s.vn ? ' VN' : ''}${s.words ? ` · ${s.words}w` : ''}</div>
</div>`;
	});
	return `<section><h2>${esc(r.ref)} “${esc(r.title)}” – ${r.screens.toFixed(1)} screens · ${fmtMin(r.t.total)} careful · ${fmtMin(r.tSkim.total)} skim</h2><div class="rail" style="--screen:${rem(S * SCALE)}">${rows.join('')}</div></section>`;
}
const html = `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Rhythm report</title><style>
:root{--bg:#fafaf7;--fg:#1d1d1b;--muted:#77756e;--look:#c9c4b5;--text:#3b6fb6;--dense:#d0452b;--line:#0001}
@media (prefers-color-scheme:dark){:root{--bg:#161614;--fg:#eceae4;--muted:#9a978e;--look:#4a473f;--text:#6f9be0;--dense:#ff7a5c;--line:#fff1}}
body{background:var(--bg);color:var(--fg);font:0.875rem/1.4 system-ui,sans-serif;margin:0;padding:1rem}
main{display:flex;gap:2rem;flex-wrap:wrap;align-items:flex-start}
h1{font-size:1.25rem}h2{font-size:0.875rem;font-weight:600;max-width:22rem}
.rail{width:22rem;background:repeating-linear-gradient(to bottom,transparent 0 calc(var(--screen) - 0.0625rem),var(--line) calc(var(--screen) - 0.0625rem) var(--screen))}
.row{display:grid;grid-template-columns:6.1rem 1fr 4.5rem;gap:0.375rem;align-items:start;overflow:hidden;border-top:0.0625rem solid var(--line)}
.img img{width:100%;display:block}.bar{display:flex;height:0.5rem;margin-top:0.25rem}
.look{background:var(--look)}.text{background:var(--text)}.text.dense{background:var(--dense)}
.lbl{font-size:0.6875rem;color:var(--muted);white-space:nowrap}
.legend span{display:inline-block;width:0.75rem;height:0.5rem;margin:0 0.25rem 0 0.75rem}
</style></head><body><h1>Rhythm report</h1>
<p class="legend">Bar per slice (40 s = full width): <span style="background:var(--look)"></span>look <span style="background:var(--text)"></span>text <span style="background:var(--dense)"></span>text wall · faint line = one phone screen (${S} px). Careful look time ${look.careful.toFixed(1)} s/screen.</p>
<main>${reports.map(rail).join('')}</main></body></html>`;
const htmlPath = resolve(outDir, 'rhythm-report.html');
writeFileSync(htmlPath, html);

console.log(md.join('\n'));
console.log(`\n→ ${mdPath}\n→ ${htmlPath}${media ? '' : '\n(no media dev copy found – rail without thumbnails)'}`);
