// The storyboard page: all scenes stacked as one column of schematic shots (what the phone shows, scaled), the rhythm
// rail beside it (seconds per shot: look / text / scroll), a screen ruler, the summary and the validator's findings.
// Also writes one SVG per shot (_out/<chapter>/svg/<id>.svg) – the input for the Figma draft frame.
//
//   node scripts/storyboard/render.mjs storyboards/catpope/s1c3 [--out storyboards/_out]

import { mkdirSync, writeFileSync } from 'node:fs';
import { basename, resolve } from 'node:path';
import { ROOT, S, TARGET, W, esc, fmtMin, textOf } from './lib.mjs';
import { shotSvg } from './schematic.mjs';
import { loadStoryboards, measureShot, measureVn, summarize, validate } from './storyboard.mjs';

const args = process.argv.slice(2);
const oi = args.indexOf('--out');
const outRoot = resolve(ROOT, oi >= 0 ? args.splice(oi, 2)[1] : 'storyboards/_out');
const src = args[0];
if (!src) {
	console.error('usage: render.mjs <storyboard folder or file>');
	process.exit(2);
}
const list = loadStoryboards(src);
const name = basename(resolve(src)).replace(/\.json$/, '');
const out = resolve(outRoot, name);
mkdirSync(resolve(out, 'svg'), { recursive: true });

const SCALE = 0.5;
const rem = (px) => `${(px / 16).toFixed(3)}rem`;
const findings = [];
const blocks = [];
for (const { file, sb } of list) {
	const v = validate(sb, file);
	findings.push(...v.errors.map((e) => ['error', e]), ...v.warnings.map((w) => ['warn', w]));
	blocks.push(`<h2 class="scene">${esc(sb.source?.scene)} ${esc(sb.source?.heading ?? '')}</h2>`);
	for (const g of sb.groups ?? []) {
		for (const shot of g.shots ?? []) {
			const { svg, w, h } = shotSvg(shot, sb);
			writeFileSync(resolve(out, 'svg', `${shot.id}.svg`), svg);
			const m = measureShot(shot);
			const pano = shot.format === 'loop360';
			const dispW = W * SCALE, dispH = pano ? (h * dispW) / w : h * SCALE;
			const max = 60;
			blocks.push(`<div class="shot" id="${esc(shot.id)}">
  <div class="img${pano ? ' pano' : ''}" style="width:${rem(dispW)};height:${rem(Math.max(dispH, 40))}">${pano ? `<div class="scroller">` : ''}<img src="svg/${esc(shot.id)}.svg" alt="${esc(textOf(shot.action))}" style="${pano ? `height:${rem(S * SCALE * 0.5)}` : 'width:100%'}">${pano ? '</div>' : ''}</div>
  <div class="rail"><div class="bar"><span class="look" style="width:${((m.lookS / max) * 100).toFixed(1)}%"></span><span class="text" style="width:${((m.textS / max) * 100).toFixed(1)}%"></span><span class="scroll" style="width:${((m.scrollS / max) * 100).toFixed(1)}%"></span></div>
  <div class="meta"><b>${esc(shot.id)}</b> ${m.seconds.toFixed(0)} s · ${pano ? `${m.panoScreens.toFixed(1)} screens wide` : `${m.screens.toFixed(1)} screens`} · ${m.words} w${shot.source?.want ? ` · ${esc(shot.source.want)}` : ''}${shot.status ? ` · ${esc(shot.status)}` : ''}</div>
  <div class="action">${esc(textOf(shot.action))}</div>${shot.note ? `<div class="note">${esc(shot.note)}</div>` : ''}</div>
</div>`);
		}
		if ((g.events ?? []).some((e) => e.type === 'showMessage')) {
			const vn = measureVn(g);
			const lines = (g.events ?? []).filter((e) => ['showMessage', 'addChoice'].includes(e.type)).map((e) => e.type === 'addChoice' ? `<li class="choice">▸ ${esc(textOf(e.params?.label))}</li>` : `<li><b>${esc(e.params?.character ?? '')}</b> ${esc(textOf(e.params?.text))}</li>`);
			blocks.push(`<div class="vn"><b>VN ${esc(g.label)}</b> – ${vn.messages} messages, ${vn.seconds.toFixed(0)} s<ul>${lines.join('')}</ul></div>`);
		}
	}
	for (const n of sb.notes ?? []) blocks.push(`<div class="note">${esc(n)}</div>`);
}
const { scenes, total } = summarize(list);
const html = `<!doctype html><html lang="de"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Storyboard ${esc(name)}</title><style>
:root{--bg:#fafaf7;--fg:#1d1d1b;--muted:#77756e;--look:#c9c4b5;--text:#3b6fb6;--scroll:#9ab89a;--warn:#b5651d;--err:#c0392b;--line:#0001;--card:#fff}
@media (prefers-color-scheme:dark){:root{--bg:#161614;--fg:#eceae4;--muted:#9a978e;--look:#4a473f;--text:#6f9be0;--scroll:#5f7d5f;--warn:#e0a060;--err:#ff6b5c;--line:#fff1;--card:#1f1f1c}}
body{background:var(--bg);color:var(--fg);font:0.875rem/1.45 system-ui,sans-serif;margin:0;padding:1rem;max-width:60rem}
h1{font-size:1.25rem;margin:0 0 0.5rem}h2.scene{font:600 0.8125rem/1.3 ui-monospace,monospace;margin:1.5rem 0 0.25rem;border-top:0.0625rem solid var(--fg);padding-top:0.5rem}
table{border-collapse:collapse;font-size:0.8125rem;margin:0.5rem 0 1rem}td,th{padding:0.125rem 0.5rem;text-align:right;border-bottom:0.0625rem solid var(--line)}td:last-child,th:last-child{text-align:left}
.shot{display:grid;grid-template-columns:${rem(W * SCALE)} 1fr;gap:0.75rem;margin:0}
.img{background:var(--card);overflow:hidden}.img img{display:block}.pano .scroller{overflow-x:auto;height:100%}
.rail{padding:0.25rem 0;font-size:0.75rem}.bar{display:flex;height:0.5rem;background:var(--line);max-width:20rem}
.look{background:var(--look)}.text{background:var(--text)}.scroll{background:var(--scroll)}
.meta{color:var(--muted);margin-top:0.25rem}.action{margin-top:0.25rem;max-width:30rem}.note{color:var(--warn);font-size:0.75rem;max-width:40rem;margin:0.25rem 0}
.vn{margin:0.5rem 0 0.5rem ${rem(W * SCALE + 12)};font-size:0.8125rem}.vn ul{margin:0.25rem 0;padding-left:1rem}.vn .choice{list-style:none;color:var(--text)}
.findings li.error{color:var(--err)}.findings li.warn{color:var(--warn)}.findings{font-size:0.75rem}
@media (max-width:40rem){.shot{grid-template-columns:1fr}.vn{margin-left:0}}
</style></head><body>
<h1>Storyboard ${esc(name)} – first throw</h1>
<p>${total.shots} shots · ${total.screens.toFixed(1)} screens · ${total.words} words · <b>${fmtMin(total.seconds)}</b> careful (target ${TARGET.screens.join('–')} screens, ${TARGET.minutes.join('–')} min). Schematics only – the images come from the Figma selection, the gaps get prompted. Bar: <span style="color:var(--look)">■</span> look <span style="color:var(--text)">■</span> text <span style="color:var(--scroll)">■</span> scroll (60 s = full).</p>
<table><tr><th>scene</th><th>shots</th><th>screens</th><th>words</th><th>careful</th><th></th></tr>
${scenes.map((s) => `<tr><td><a href="#${esc(list.find((l) => l.sb.source?.scene === s.scene)?.sb.groups?.[0]?.shots?.[0]?.id ?? '')}">${esc(s.scene)}</a></td><td>${s.shots}</td><td>${s.screens.toFixed(1)}</td><td>${s.words}</td><td>${fmtMin(s.seconds)}</td><td>${esc(s.heading ?? '')}</td></tr>`).join('')}</table>
${findings.length ? `<details class="findings"><summary>${findings.filter((f) => f[0] === 'error').length} errors, ${findings.filter((f) => f[0] === 'warn').length} warnings</summary><ul>${findings.map(([k, m]) => `<li class="${k}">${esc(m)}</li>`).join('')}</ul></details>` : '<p>No findings.</p>'}
${blocks.join('\n')}
</body></html>`;
writeFileSync(resolve(out, 'index.html'), html);
console.log(`${total.shots} shots, ${total.screens.toFixed(1)} screens, ${fmtMin(total.seconds)} careful → ${resolve(out, 'index.html')}`);
