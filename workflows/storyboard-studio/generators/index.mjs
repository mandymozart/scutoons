// The studio's generator routes (api.mjs spreads them in): the registry (comfy-local · runway · luma-manual ·
// comfyui-remote, + fake with STUDIO_FAKE_GENERATOR=1), the inputs a shot offers, the estimate, start / cancel, the jobs
// and the session's spend (in USD) + the euro rate the UI shows them in (rate.mjs). The interface: jobs.mjs. README
// "Generators".
//
// Inputs travel as keys, never as paths: "out:<ch>/clean/<shot>.png" (storyboards/_out), "boards:<…>"
// (storyboards/ – the approved images, the style folder), "media:<…>" (the media root) – resolved and checked here.

import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { dirname, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createManager } from './jobs.mjs';
import { createRunway } from './runway.mjs';
import { createComfyLocal } from './comfy-local.mjs';
import { createLumaManual } from './luma-manual.mjs';
import { createComfyRemote, createFake } from './others.mjs';
import { createRate } from './rate.mjs';
import { approvedOf } from '../clips.mjs';
import { idOf, kindOf } from '../../../scripts/storyboard/generated-log.mjs';

const PASS_ORDER = ['grey', 'beauty', 'depth', 'normals'];
/** the euro rate's cache – gitignored (workflows/storyboard-studio/.gitignore) */
const RATE_FILE = resolve(dirname(fileURLToPath(import.meta.url)), '../.cache/eur-usd.json');

export function generatorRoutes({ root, out, boards, media, findShot, shotCharacters, styleFiles, env = process.env, rate = createRate({ cacheFile: RATE_FILE, env }) }) {
	const roots = { out, boards, media };
	const generators = [createComfyLocal({ root, env }), createRunway({ root, env }), createLumaManual(), createComfyRemote()];
	if (env.STUDIO_FAKE_GENERATOR === '1') generators.push(createFake({ read: (f) => readFileSync(f) }));
	// the approved image (or video) as an input is a copy of a generated file: its parent is that file (approve.mjs
	// `from`, clips.mjs approvedOf) – a clip made from the chosen image hangs under it (VARIANTS-CANVAS §3.7)
	function parentOf(key) {
		const m = /^boards:dataset\/files\/([a-z0-9-]+)\/([\w-]+)\/chosen\.\w+$/.exec(key);
		if (!m) return null;
		const gd = resolve(boards, '_out', m[1], 'generated');
		const files = existsSync(gd) ? readdirSync(gd).filter((f) => f.startsWith(`${m[2]}--`)) : [];
		const hit = [...approvedOf({ boards, ch: m[1], shot: m[2], files })].find((f) => kindOf(f) === kindOf(key));
		return hit ? idOf(hit) : null;
	}
	const manager = createManager({ generators, out, parentOf });
	// the first fetch starts with the studio (in the background), so the first page load already has today's rate
	rate.get();

	const keyOf = (rootName, file) => `${rootName}:${relative(roots[rootName], file).replaceAll('\\', '/')}`;
	function fileOf(key) {
		if (!key) return null;
		const m = /^(out|boards|media):(.+)$/.exec(String(key));
		const base = m && roots[m[1]];
		if (!base) throw new Error(`bad input ${key}`);
		const file = resolve(base, m[2]);
		if (!file.startsWith(base + sep) || !existsSync(file) || statSync(file).isDirectory()) throw new Error(`no input ${key}`);
		return file;
	}
	const urlOf = (key) => { const [r, rel] = key.split(/:(.*)/s); return r === 'out' ? `/out/${rel}` : r === 'media' ? `/media/${rel}` : rel.startsWith('style/') ? `/style/${rel.slice(6)}` : null; };
	const item = (rootName, file, label) => { const key = keyOf(rootName, file); return { key, label, url: urlOf(key) }; };

	/** what a shot offers as inputs (the UI pre-fills from it) */
	function inputs({ toon, ch, id }) {
		const hit = findShot(toon, ch, id);
		const shot = hit.group.shots[hit.index];
		const stills = [];
		const chosen = resolve(boards, 'dataset/files', ch, id);
		if (existsSync(chosen)) for (const f of readdirSync(chosen).filter((x) => /^chosen\.(png|jpe?g|webp)$/i.test(x))) stills.push(item('boards', resolve(chosen, f), 'The approved image (chosen)'));
		for (const [dir, label] of [['clean', 'The render (no texts)'], ['frames', 'The saved frame']]) {
			const f = resolve(out, ch, dir, `${id}.png`);
			if (existsSync(f)) stills.push(item('out', f, label));
		}
		const gd = resolve(out, ch, 'generated');
		const gen = existsSync(gd) ? readdirSync(gd).filter((f) => f.startsWith(`${id}--`)) : [];
		for (const f of gen.filter((x) => /\.(png|jpe?g|webp)$/i.test(x)).reverse()) stills.push(item('out', resolve(gd, f), `Generated ${f.slice(id.length + 2)}`));
		const hd = resolve(out, ch, 'handheld');
		const controls = existsSync(hd)
			? readdirSync(hd).map((f) => [f, new RegExp(`^${id}--([\\w-]+?)--(\\w+)\\.mp4$`).exec(f)]).filter(([, m]) => m)
				.sort((a, b) => b[1][1].localeCompare(a[1][1]) || PASS_ORDER.indexOf(a[1][2]) - PASS_ORDER.indexOf(b[1][2]))
				.map(([f, m]) => item('out', resolve(hd, f), `Take ${m[1]} · ${m[2]}`))
			: [];
		const characters = shotCharacters(hit.sb, shot).filter((c) => c.image).map((c) => {
			const inMedia = media && c.image.file.startsWith(media + sep);
			return item(inMedia ? 'media' : 'boards', c.image.file, `${c.fig.cast} (${c.image.mood})`);
		});
		const style = styleFiles().map((f) => item('boards', resolve(boards, 'style', f), f));
		return { stills, controls, characters, style, prompt: shot.prompt?.en ?? '', aspect: shot.aspect ?? '9:16', duration: 5 };
	}

	/** the client's job → the generator's input (keys resolved to files) */
	function jobOf(body) {
		const j = body.job ?? body;
		return {
			generator: j.generator, model: j.model, toon: j.toon, ch: j.ch, shot: j.shot,
			prompt: j.prompt ?? '', duration: j.duration, aspect: j.aspect, seed: j.seed ?? null,
			still: j.still ?? null, control: j.control ?? null, characters: j.characters ?? [], style: j.style ?? [],
			stillFile: fileOf(j.still), controlFile: fileOf(j.control), characterFiles: (j.characters ?? []).map(fileOf), styleFiles: (j.style ?? []).map(fileOf)
		};
	}

	return {
		manager,
		routes: {
			'GET /api/generators': async () => ({ generators: await manager.list(), spend: manager.spend(), rate: rate.get() }),
			'GET /api/generate/inputs': inputs,
			'POST /api/generate/estimate': (body) => manager.estimate(jobOf(body)),
			'POST /api/generate': (body) => manager.start(jobOf(body), { confirm: body.confirm }),
			'GET /api/generate/jobs': ({ ch, id }) => ({ jobs: manager.jobs({ ch, shot: id }), spend: manager.spend(), rate: rate.get() }),
			'POST /api/generate/cancel': ({ job }) => manager.cancel(job)
		}
	};
}
