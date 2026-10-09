// Storyboards → one PNG per shot (owner 2026-10-09: images instead of SVG; Sin City black and white, extreme hard
// light, posable figurines). Builds a spec per shot from the storyboard JSON and renders each in headless Edge /
// Chrome with three.js (render.html + render.js). Local only; nothing is sent anywhere.
//
//   node tools/storyboard-render/render-shots.mjs storyboards/catpope/s1c3 [--only sb-28-4,sb-28-7] [--out storyboards/_out]
// → storyboards/_out/<chapter>/frames/<shot id>.png

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { basename, dirname, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { ROOT } from '../../scripts/storyboard/lib.mjs';
import { shotSpec } from './spec.js';
import { loadStoryboards } from '../../scripts/storyboard/storyboard.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);
const take = (f, d) => { const i = args.indexOf(f); return i >= 0 ? args.splice(i, 2)[1] : d; };
const only = take('--only', null)?.split(',');
const outRoot = resolve(ROOT, take('--out', 'storyboards/_out'));
const src = args[0];
if (!src) { console.error('usage: render-shots.mjs <storyboard folder or file> [--only ids]'); process.exit(2); }
const threeDir = [resolve(HERE, 'node_modules/three'), resolve(ROOT, 'node_modules/three')].find((d) => existsSync(resolve(d, 'build/three.module.js')));
if (!threeDir) throw new Error('three is not installed – npm install (the workspace tools/storyboard-render)');
const threeUrl = pathToFileURL(threeDir).href + '/';

const browser = [process.env.CHROME, 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', 'C:/Program Files/Google/Chrome/Application/chrome.exe', '/usr/bin/chromium', '/usr/bin/google-chrome'].filter(Boolean).find((b) => existsSync(b));
if (!browser) throw new Error('no Edge / Chrome found (CHROME=<path>)');

const name = basename(resolve(src)).replace(/\.json$/, '');
const out = resolve(outRoot, name, 'frames');
mkdirSync(out, { recursive: true });
const SS = 2; // supersampling: render at 2×, the screenshot is scaled down → anti-aliased hard edges

const specs = [];
for (const { sb } of loadStoryboards(src)) {
	for (const g of sb.groups ?? []) for (const shot of g.shots ?? []) {
		if (only && !only.includes(shot.id)) continue;
		specs.push(shotSpec(sb, shot, SS)); // the same spec the studio builds (spec.js)
	}
}
const specFile = resolve(out, '_shots.mjs');
writeFileSync(specFile, `export default ${JSON.stringify(specs)};\n`);

const page = pathToFileURL(resolve(HERE, 'render.html')).href;
let n = 0;
for (const s of specs) {
	const file = resolve(out, `${s.id}.png`);
	const url = `${page}?spec=${encodeURIComponent(pathToFileURL(specFile).href)}&id=${encodeURIComponent(s.id)}&three=${encodeURIComponent(threeUrl)}`;
	execFileSync(browser, ['--headless=new', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--allow-file-access-from-files', '--hide-scrollbars', `--force-device-scale-factor=${1 / SS}`, `--window-size=${s.w},${s.h}`, '--virtual-time-budget=20000', `--screenshot=${file}`, url], { stdio: 'ignore', timeout: 120000 });
	n++;
	process.stdout.write(`\r${n}/${specs.length} ${s.id}        `);
}
console.log(`\n${n} frames → ${out}`);
