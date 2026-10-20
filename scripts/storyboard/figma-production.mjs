// Figma draft → production draft (owner 2026-10-10: "aus den drafts dann production ready slices kopiert, die texte aus
// dem entwurf wieder entfernt, mit den 390x625 … in 1x 2x 3x in die richtigen aufloesungen exportiert … text box
// vorschlaege in den sprachen die verfuegbar sind … ein transfer json, das dann via mcp als draft auf die production
// seite geladen wird"; source: the storyboard draft frame, its `layout` layer on top).
//
//   node scripts/storyboard/figma-production.mjs storyboards/catpope/s1c3 --page "S1C3 - BARGAINING" --frame "s1c3 – storyboard draft 2026-10-09" [--only 1-5] [--folder s1/c3/v624]
//   (--frame takes a node id too – 2369:3211 – then the page is not loaded whole to find it)
//
// Through the self-healing bridge client (lib.mjs figma(), #78): a preflight first (relay – started if not –, the
// plugin's code version, free RAM, the Figma file – a plugin re-run in another file is waited for); a plugin that goes
// away is waited for (one line says what to click); a timeout waits until Figma is free, then retries. Each slice is a
// `region` with partial + layers (only the layers' nodes that reach into the slice are cloned – a whole-draft clone per
// slice was failure D). Resumable: production/run.json says which slice is done, at which standard, from which region;
// a slice counts as done only with that entry AND its three WebPs at the expected size (an old 625 cut is no 624 one) –
// a crashed run goes on where it stopped (--force: all again). A slice that still fails is marked failed (its old files
// removed), the rest goes on; two slices in a row timing out end the run (Figma too slow or stuck), as does Figma busy
// for 30 min (plugin_stuck). chapter.json + uploads.json are written only when EVERY slice is exported at this standard
// (else the old ones are removed and the run ends with exit code 1 and the list). Pacing: a pause every 25 exports and
// after a call slower than twice the median.
// CDN paths never change (RULES "Product rules"; production-upload.mjs skips a path that exists): the slices go to
// <toon>/--folder/, by default s<n>/c<n>/v<SLICE_H> (v624) – never the folder of the chapter's 625 cut. A re-cut of
// changed art needs a new --folder.
//
// Per scene group (compile.mjs's chapter draft – the same layout figma-chapter.mjs drew):
//   vertical    cut at shot boundaries into slices of SLICE_H = 624 (the rest: a shorter slice; a rest < 160 joins the one above)
//   panorama    the strip as 390 × 844 slides
// each exported from the frame with `text` and `guides` hidden, at 1x / 2x / 3x → WebP
// `<toon>/<folder>/s<n>c<n>-slice-NN.webp` (+ `.2x.webp`, `.3x.webp` = imageVariants low / medium / high).
// The texts come from the frame's `text` layer (as the owner edited them: wording + place), become the slice's dialog
// (rect in % of the slice, the preset's style from the chapter draft), German as written plus the other languages of
// the toon from production/translations.json when there (suggestions, marked in `meta`).
// Out: storyboards/_out/<ch>/production/ – chapter.json (the language-neutral chapter: the scene groups replaced by the
// sliced ones and nothing else – no test groups, no thumbnail slice), uploads.json [{ file, path }], slices/*.webp, preview.png. Nothing is uploaded here.

import { existsSync, mkdirSync, readFileSync, unlinkSync, writeFileSync } from 'node:fs';
import { basename, resolve } from 'node:path';
import { ROOT, S, SLICE_H, W, figma, figmaPacer, figmaPreflight, readEntity, runManifest, variantsOk, webpVariants } from './lib.mjs';

const args = process.argv.slice(2);
const take = (f, d) => { const i = args.indexOf(f); return i >= 0 ? args.splice(i, 2)[1] : d; };
const page = take('--page'), frameName = take('--frame'), folderArg = take('--folder');
const only = take('--only', null)?.split('-').map(Number);
// --figma-slices <x>: first build the slices as frames in Figma (the owner's way – a frame per slice holding the
// layers active there, no text), at page x <x>, each at its draft height; then export FROM those frames (re-runs
// replace them; the owner can fix a slice frame in Figma and run with --from-figma-slices to export again)
const sliceX = take('--figma-slices', null);
const fromFrames = args.includes('--from-figma-slices') && args.splice(args.indexOf('--from-figma-slices'), 1);
const force = args.includes('--force') && args.splice(args.indexOf('--force'), 1);
const src = args[0];
if (!src || !page || !frameName) { console.error('usage: figma-production.mjs <storyboard folder> --page "<Figma page>" --frame "<draft frame>" [--only a-b]'); process.exit(2); }
const ch = basename(resolve(src));
const [, sn, cn] = /^s(\d+)c(\d+)$/.exec(ch) ?? [];
const OUT = resolve(ROOT, 'storyboards/_out', ch);
const PROD = resolve(OUT, 'production');
mkdirSync(resolve(PROD, 'slices'), { recursive: true });
mkdirSync(resolve(PROD, 'png'), { recursive: true });
const SLICE = SLICE_H, MIN_REST = 160;
// the CDN folder below the toon: new paths for this cut (the chapter's own folder holds its 625 cut, and uploads never replace)
const dir = (folderArg ?? `s${sn}/c${cn}/v${SLICE}`).replace(/^\/+|\/+$/g, '');
if (dir === `s${sn}/c${cn}`) { console.error(`--folder ${dir}: the chapter's own folder holds its earlier cut (chapters 1–5: 625) – CDN paths never change and the upload skips existing ones; take a new folder (default s${sn}/c${cn}/v${SLICE})`); process.exit(2); }
let expectFile = null; // the Figma file the run started in (preflight) – a plugin re-run elsewhere is waited for
const bridge = (c, a, o) => figma(c, a, { client: `figma-production ${ch}`, expectFile, ...o });

const draft = JSON.parse(readFileSync(resolve(OUT, 'chapter.draft.json'), 'utf8'));
const toon = draft.groups.flatMap((g) => g.slices)[0]?.image ? 'catpope' : 'catpope';
let languages = ['de'];
try { languages = readEntity(`toon:${toon}`)?.languages ?? languages; } catch {}
const tr = existsSync(resolve(PROD, 'translations.json')) ? JSON.parse(readFileSync(resolve(PROD, 'translations.json'), 'utf8')) : {};

const pre = await figmaPreflight();
expectFile = pre.file;
console.log(`Figma: "${pre.file}" (plugin ${pre.version}, ping ${pre.pingMs} ms, ${pre.freeGB} GB RAM free) – slices of ${SLICE} → <toon>/${dir}/`);

// the frame and its texts (by id: only the frame is read; by name: the page once, depth 1 – a page load can take minutes)
let frame;
if (/^\d+:\d+$/.test(frameName)) frame = await bridge('read', { nodeId: frameName, depth: 0 });
else {
	const tree = await bridge('read', { page, depth: 1 }, { timeout: 240_000 });
	frame = tree.children.find((n) => n.name === frameName);
}
if (!frame) throw new Error(`no frame "${frameName}" on "${page}"`);
const layers = await bridge('read', { nodeId: frame.id, depth: 2 });
const texts = (layers.children.find((l) => l.name === 'text')?.children ?? []).filter((t) => t.type === 'TEXT' && (t.text ?? '').trim());

// the chapter draft's layout (as figma-chapter.mjs drew it) and its dialog styles by shot
let y = 0;
const groups = draft.groups.map((g) => {
	const pano = g.renderer?.type === 'horizontal';
	const shots = pano ? [{ id: basename(g.slices[0].image).replace(/-\d+\.png$/, ''), y, h: S, dialog: g.slices.flatMap((s) => s.dialog ?? []) }]
		: g.slices.map((s) => { const o = { id: basename(s.image).replace(/\.\w+$/, ''), y, h: s.height, dialog: s.dialog ?? [] }; y += s.height; return o; });
	if (pano) y += S;
	return { g, pano, shots, own: /^scene-/.test(g.label ?? '') };
});

// a Figma text → a dialog of the slice that holds its top
const byShot = {};
for (const t of texts) { const id = t.name.split(' · ')[0]; (byShot[id] ??= []).push(t); }
function dialogFor(t, slice) {
	const id = t.name.split(' · ')[0];
	const k = byShot[id].indexOf(t);
	const shot = groups.flatMap((g) => g.shots).find((s) => s.id === id);
	const base = shot?.dialog?.[k] ?? {};
	const de = t.text.trim();
	const text = { de };
	for (const l of languages) if (l !== 'de' && tr[de]?.[l]) text[l] = tr[de][l];
	const { rect, text: _t, ...style } = base;
	return { ...style, id: base.id ?? `${ch}-${id}-${k + 1}`, rect: { x: +((t.x - slice.x) / slice.w * 100).toFixed(1), y: +((t.y - slice.y) / slice.h * 100).toFixed(1), width: +(t.w / slice.w * 100).toFixed(1) }, text };
}

// the slices
const slices = [];
for (const G of groups) {
	if (!G.own) continue;
	if (G.pano) {
		const s = G.shots[0], n = G.g.slices.length;
		for (let i = 0; i < n; i++) slices.push({ group: G, x: i * W, y: s.y, w: W, h: S, pano: true });
		continue;
	}
	const top = G.shots[0].y, bottom = G.shots.at(-1).y + G.shots.at(-1).h;
	const cuts = [];
	for (let at = top; at < bottom; at += SLICE) cuts.push([at, Math.min(at + SLICE, bottom)]);
	if (cuts.length > 1 && cuts.at(-1)[1] - cuts.at(-1)[0] < MIN_REST) { const last = cuts.pop(); cuts.at(-1)[1] = last[1]; }
	for (const [a, b] of cuts) slices.push({ group: G, x: 0, y: a, w: W, h: b - a });
}
slices.forEach((s, i) => { s.n = i + 1; s.name = `s${sn}c${cn}-slice-${String(i + 1).padStart(2, '0')}`; });
for (const t of texts) {
	const s = slices.find((s) => t.y >= s.y && t.y < s.y + s.h && t.x + t.w / 2 >= s.x && t.x + t.w / 2 < s.x + s.w + (s.pano ? 0 : 1e6) && (!s.pano || t.x >= s.x - 1));
	if (s) (s.dialog ??= []).push(dialogFor(t, s));
}
console.log(`${ch}: ${slices.length} slices (${slices.filter((s) => s.pano).length} panorama slides), ${texts.length} texts, languages ${languages.join(' + ')}${Object.keys(tr).length ? '' : ' (no translations.json yet – German only)'}`);

// export: 1x / 2x / 3x PNG from Figma → WebP
const todo = slices.filter((s) => !only || (s.n >= only[0] && s.n <= (only[1] ?? only[0])));
const frameIds = {};
if (sliceX != null) {
	for (let i = 0; i < todo.length; i += 10) {
		const part = todo.slice(i, i + 10);
		const made = await bridge('sliceFrames', { frame: frame.id, skip: ['text', 'guides'], slices: part.map((s) => ({ name: s.name, x: s.x, y: s.y, w: s.w, h: s.h, px: Number(sliceX) + s.x, py: frame.y + s.y })) });
		for (const m of made) frameIds[m.name] = m.id;
		process.stdout.write(`slice frames ${Math.min(i + 10, todo.length)}/${todo.length}   `);
	}
	console.log();
}
if (fromFrames && sliceX == null) {
	const pg = await bridge('read', { page, depth: 1 }, { timeout: 240_000 });
	for (const n of pg.children) if (/-slice-\d+$/.test(n.name)) frameIds[n.name] = n.id;
}
// the run manifest: a slice is done when this run (or a crashed one before it) exported it at this standard from the same
// region and its three files have the expected size. From slice frames (--figma-slices / --from-figma-slices) everything
// is exported again, as before – the owner may have fixed a frame (--only N for just that one)
const manifest = runManifest(resolve(PROD, 'run.json'), { standard: SLICE, page, frame: frame.id, folder: dir });
const sourceOf = (s) => (frameIds[s.name] ? `frame ${frameIds[s.name]}` : `region ${frame.id} ${s.x},${s.y} ${s.w}×${s.h}`);
/** the slice's manifest entry when it is exported at this standard (from any source) and its three files are at its size */
const exported = (s) => { const m = manifest.get(s.name); return m?.status === 'done' && m.standard === SLICE && variantsOk(resolve(PROD, 'slices'), s.name, s.w, s.h) ? m : null; };
const pace = figmaPacer();
const failed = [];
let skipped = 0, timedOut = 0;
for (const s of todo) {
	const source = sourceOf(s);
	if (!force && sliceX == null && !fromFrames && exported(s)?.source === source) { skipped++; continue; }
	const t0 = Date.now();
	try {
		const res = frameIds[s.name]
			? await bridge('exportScales', { node: frameIds[s.name], scales: [1, 2, 3] })
			: await bridge('region', { frame: frame.id, x: s.x, y: s.y, w: s.w, h: s.h, hide: ['text', 'guides'], partial: true, layers: true, unclip: s.pano, scales: [1, 2, 3] });
		for (const r of res) writeFileSync(resolve(PROD, 'png', `${s.name}@${r.scale}x.png`), Buffer.from(r.base64, 'base64'));
		webpVariants(PROD, s.name);
		if (!variantsOk(resolve(PROD, 'slices'), s.name, s.w, s.h)) throw Object.assign(new Error(`the export is not ${s.w} × ${s.h} (1x) / ${s.w * 3} × ${s.h * 3} (3x)`), { code: 'size' });
		manifest.set(s.name, { status: 'done', standard: SLICE, source, ms: Date.now() - t0 });
		timedOut = 0;
	} catch (e) {
		// this slice failed for good (after the client's retries) – the rest goes on; anything else (no relay, no plugin
		// for 30 min, Figma stuck, no memory) ends the run: a re-run resumes
		if (!['plugin_error', 'timeout', 'queue_timeout', 'size'].includes(e.code)) throw e;
		manifest.set(s.name, { status: 'failed', standard: SLICE, source, error: e.message });
		// its old files (a 625 cut, a wrong export) must not be taken for this cut later
		for (const suf of ['', '.2x', '.3x']) try { unlinkSync(resolve(PROD, 'slices', `${s.name}${suf}.webp`)); } catch {}
		failed.push(`${s.name}: ${e.message}`);
		if (/timeout/.test(e.code) && ++timedOut >= 2) {
			console.error(`\ntwo slices in a row timed out – Figma is too slow or stuck right now; the run stops (run again, it resumes):\n  ${failed.join('\n  ')}`);
			process.exit(1);
		}
	}
	const paused = await pace(Date.now() - t0);
	process.stdout.write(`\r${s.n}/${slices.length} ${s.name}${paused ? ` (paused: ${paused})` : ''}   `);
}
console.log(skipped ? `\n${skipped} slices already done (run.json) – skipped` : '');

// chapter.json + uploads.json only for a complete cut: every slice (also outside --only) exported at this standard –
// a stale file (the 625 cut, a failed slice) would be uploaded to the new paths and stay there for good
const missing = slices.filter((s) => !exported(s));
if (missing.length) {
	const gone = ['chapter.json', 'uploads.json'].filter((f) => existsSync(resolve(PROD, f)));
	for (const f of gone) unlinkSync(resolve(PROD, f)); // from an older (625) or incomplete run – a complete run writes them
	console.log(`\nchapter.json / uploads.json NOT written: ${missing.length} of ${slices.length} slices are not exported at ${SLICE} yet (${missing.slice(0, 8).map((s) => s.n).join(', ')}${missing.length > 8 ? ' …' : ''})${gone.length ? ` – removed the old ${gone.join(' + ')}` : ''}; run again${only ? ' without --only' : ''} (it resumes)`);
	if (failed.length) console.log(`${failed.length} slice(s) FAILED:\n  ${failed.join('\n  ')}`);
	process.exit(1);
}

// the chapter: ONLY the scene groups as sliced – the stored test groups (mama-forest, mama-biker, weed) and the
// thumbnail placeholder slice go (owner 2026-10-10: "die mama scenen und weed koennten wir loeschen … das war alles nur
// test", "chapter 3 4 5 sollten eigentlich ausser dem thumbnail als ersten slice nichts haben … das der nicht bleibt")
const chapter = { ...draft, groups: [] };
for (const G of groups) {
	if (!G.own) continue;
	const { slices: _s, ...g } = G.g;
	chapter.groups.push({ ...g, slices: slices.filter((s) => s.group === G).map((s) => ({
		image: `${dir}/${s.name}.webp`, width: s.w, height: s.h,
		imageVariants: { low: `${dir}/${s.name}.webp`, medium: `${dir}/${s.name}.2x.webp`, high: `${dir}/${s.name}.3x.webp` },
		...(s.dialog?.length ? { dialog: s.dialog } : {})
	})) });
}
// the last group can't continue – nothing follows it on the page (the creator's check)
if (chapter.groups.length) delete chapter.groups.at(-1).continues;
chapter.meta = { ...(draft.meta ?? {}), production: { from: `Figma "${pre.file}" / "${page}" / "${frameName}"`, date: new Date().toLocaleDateString('sv'), standard: SLICE, folder: dir, suggested: languages.filter((l) => l !== 'de') } };
writeFileSync(resolve(PROD, 'chapter.json'), JSON.stringify(chapter, null, '\t'));
// w / h per file: what the path must hold (an upload can compare an existing path's size instead of skipping it blindly)
const uploads = slices.flatMap((s) => [['', 1], ['.2x', 2], ['.3x', 3]].map(([suf, k]) => ({ file: `slices/${s.name}${suf}.webp`, path: `${toon}/${dir}/${s.name}${suf}.webp`, w: s.w * k, h: s.h * k })));
writeFileSync(resolve(PROD, 'uploads.json'), JSON.stringify(uploads, null, '\t'));
const missingTr = [...new Set(texts.map((t) => t.text.trim()))].filter((de) => languages.some((l) => l !== 'de' && !tr[de]?.[l]));
writeFileSync(resolve(PROD, 'to-translate.json'), JSON.stringify(missingTr, null, '\t'));
console.log(`→ ${PROD}: chapter.json, uploads.json (${uploads.length} files → ${toon}/${dir}/), slices/ – ${missingTr.length} texts without a translation (to-translate.json)`);
