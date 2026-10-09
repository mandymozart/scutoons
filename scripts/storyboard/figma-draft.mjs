// The first throw as Figma draft frames (STORYBOARD.md §9 "The first throw lands in Figma"): per scene a frame
// "s1c3 / scene-25 – draft", 390 wide, on a page of its own, holding
//   storyboard  (locked) the schematic shots as SVG at their y – hide it to judge the layout alone
//   guides      (locked, hidden) a line every screen (844) and every slice (625)
//   text        the dialog as real TEXT layers (Track A's convention: text becomes live text boxes later)
//   layout      empty, on top – the owner's work, never touched by a re-run
// and the rhythm rail as text beside the frame (x > 390, never exported).
//
// Writes per scene a spec (JSON) for the SCUTOONS Figma bridge (`node tools/figma-bridge/cli.mjs draft <spec>`) and the
// same as a Plugin-API script for Figma's MCP (`use_figma`, ≤ 45,000 characters a part). Both run applyDraft from
// tools/figma-bridge/plugin/code.js – one implementation. Nothing is sent from here.
//
//   node scripts/storyboard/figma-draft.mjs storyboards/catpope/s1c3 [--page "S1C3 · storyboard (test)"] [--out storyboards/_out]

import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { basename, resolve } from 'node:path';
import { ROOT, S, SLICE_H, W, textOf } from './lib.mjs';
import { shotSvg } from './schematic.mjs';
import { loadStoryboards, measureShot } from './storyboard.mjs';

const args = process.argv.slice(2);
const take = (flag, dflt) => { const i = args.indexOf(flag); return i >= 0 ? args.splice(i, 2)[1] : dflt; };
const outRoot = resolve(ROOT, take('--out', 'storyboards/_out'));
const src = args[0];
if (!src) { console.error('usage: figma-draft.mjs <storyboard folder>'); process.exit(2); }
const name = basename(resolve(src));
const PAGE = take('--page', `${name.toUpperCase()} · storyboard (test)`);
const LIMIT = 45000;
const STEP = W + 260; // scene frames side by side, room for the rail

const pluginCode = readFileSync(resolve(ROOT, 'tools/figma-bridge/plugin/code.js'), 'utf8');
const applier = /\/\/ <applyDraft>[^\n]*\n([\s\S]*?)\/\/ <\/applyDraft>/.exec(pluginCode)?.[1];
if (!applier) throw new Error('applyDraft not found in tools/figma-bridge/plugin/code.js');

/** the scene's spec: frame, guides, rail, shots (svg + live text) */
export function sceneSpec(sb, x) {
	const shots = (sb.groups ?? []).flatMap((g) => g.shots ?? []);
	let y = 0;
	const placed = shots.map((shot) => {
		const { svg, w, h } = shotSvg(shot, sb, { noText: true });
		const pano = shot.format === 'loop360';
		const band = pano ? S : h;
		const scale = pano ? W / w : 1;
		const top = pano ? y + Math.round((S - h * scale) / 2) : y;
		const texts = (shot.dialog ?? []).map((d) => ({
			x: +(((d.rect?.x ?? 5) / 100) * W).toFixed(1), y: +(y + ((d.rect?.y ?? 5) / 100) * band).toFixed(1), w: +(((d.rect?.width ?? 60) / 100) * W).toFixed(1),
			text: textOf(d.text), lettering: d.font === 'lettering', variant: d.variant ?? 'narration'
		}));
		const m = measureShot(shot);
		const p = { id: shot.id, svg, x: 0, y: top, scale: +scale.toFixed(5), texts, rail: { y, label: `${shot.id}  ${m.seconds.toFixed(0)} s · ${m.words} w · ${shot.source?.want ?? ''}` } };
		y += band;
		return p;
	});
	const guides = [];
	for (let gy = S; gy < y; gy += S) guides.push({ y: gy, kind: 'screen' });
	for (let gy = SLICE_H; gy < y; gy += SLICE_H) guides.push({ y: gy, kind: 'slice' });
	return {
		page: PAGE,
		frame: { name: `${name} / scene-${sb.source?.scene} – draft`, x, w: W, h: y },
		guides,
		rail: placed.map((p) => p.rail),
		shots: placed.map(({ rail, ...p }) => p)
	};
}

/** split a spec so each use_figma part stays under the limit (part 1 carries guides + rail) */
function parts(spec) {
	const base = { page: spec.page, frame: spec.frame };
	const out = [];
	let cur = { ...base, guides: spec.guides, rail: spec.rail, shots: [] };
	for (const s of spec.shots) {
		if (cur.shots.length && JSON.stringify(cur).length + JSON.stringify(s).length + applier.length > LIMIT) {
			out.push(cur);
			cur = { ...base, shots: [] };
		}
		cur.shots.push(s);
	}
	out.push(cur);
	return out;
}

if (process.argv[1]?.endsWith('figma-draft.mjs')) {
	const list = loadStoryboards(src);
	const out = resolve(outRoot, name, 'figma');
	mkdirSync(out, { recursive: true });
	let x = 0;
	for (const { sb } of list) {
		const scene = sb.source?.scene;
		const spec = sceneSpec(sb, x);
		writeFileSync(resolve(out, `scene-${scene}.spec.json`), JSON.stringify(spec));
		parts(spec).forEach((p, i) => {
			const code = `${applier}\nreturn await applyDraft(${JSON.stringify(p)});\n`;
			writeFileSync(resolve(out, `scene-${scene}.part${i + 1}.js`), code);
			console.log(`scene-${scene}.part${i + 1}.js  ${code.length} chars  ${p.shots.length} shots${code.length > 50000 ? '  OVER 50,000' : ''}`);
		});
		x += STEP;
	}
	console.log(`\npage "${PAGE}". Bridge: node tools/figma-bridge/cli.mjs draft ${out.replace(/\\/g, '/')}/scene-*.spec.json\nuse_figma: run the scene-*.part*.js in order. Nothing was sent.`);
}
