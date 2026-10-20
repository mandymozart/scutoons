// Figma draft → production draft (owner 2026-10-10: "aus den drafts dann production ready slices kopiert, die texte aus
// dem entwurf wieder entfernt, mit den 390x625 … in 1x 2x 3x in die richtigen aufloesungen exportiert … text box
// vorschlaege in den sprachen die verfuegbar sind … ein transfer json, das dann via mcp als draft auf die production
// seite geladen wird"; source: the storyboard draft frame, its `layout` layer on top).
//
//   node scripts/storyboard/figma-production.mjs storyboards/catpope/s1c3 --page "S1C3 - BARGAINING" --frame "s1c3 – storyboard draft 2026-10-09" [--only 1-5] [--folder s1/c3/v624] [--out production]
//   (--frame takes a node id too – 2369:3211 – then the page is not loaded whole to find it)
//   --dry: the cut only (per group its pieces, the lines figma-chapter.mjs --guides-only draws) – no Figma, nothing written
//   --out: the folder under _out/<ch>/ (default production) – a trial cut elsewhere leaves production/run.json alone
//   (a slice it lists as done is skipped by the next run, also after the art changed – then --force)
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
// Per scene group (compile.mjs's chapter draft laid out by lib.mjs chapterLayout – the layout figma-chapter.mjs drew,
// its guides = these cuts):
//   vertical    cut every SLICE_H = 624 from the GROUP's top, straight through shots. The last piece (the rest r < 624)
//               is never joined (#108, owner 2026-10-20 "Q2 b and c are fine"): DROPPED when all of it is one colour and
//               no text's top lies in it (the owner ended the group on a slice line), else exported r high and PADDED
//               below to 624 in its bottom row's colour (one colour), else white. A group's only piece is never dropped.
//               Slice numbers count every piece, so a drop leaves a gap (that number is missing) and no later path moves.
//   panorama    the strip as 390 × 844 slides
// each exported from the frame with `text` and `guides` hidden, at 1x / 2x / 3x → WebP; the run lists every pad, every
// drop and every text that lands on no slice
// `<toon>/<folder>/s<n>c<n>-slice-NN.webp` (+ `.2x.webp`, `.3x.webp` = imageVariants low / medium / high).
// The texts come from the frame's `text` layer (as the owner edited them: wording + place), become the slice's dialog
// (rect in % of the slice, the preset's style from the chapter draft), German as written plus the other languages of
// the toon from production/translations.json when there (suggestions, marked in `meta`).
// Out: storyboards/_out/<ch>/production/ – chapter.json (the language-neutral chapter: the scene groups replaced by the
// sliced ones and nothing else – no test groups, no thumbnail slice), uploads.json [{ file, path }], slices/*.webp, preview.png. Nothing is uploaded here.

import { cpSync, existsSync, mkdirSync, readdirSync, readFileSync, unlinkSync, writeFileSync } from 'node:fs';
import { basename, resolve } from 'node:path';
import { ROOT, S, SLICE_H, W, chapterLayout, figma, figmaPacer, figmaPreflight, hexOf, pieceLook, readEntity, runManifest, sliceLines, variantsOk, webpVariants } from './lib.mjs';

const args = process.argv.slice(2);
const take = (f, d) => { const i = args.indexOf(f); return i >= 0 ? args.splice(i, 2)[1] : d; };
const page = take('--page'), frameName = take('--frame'), folderArg = take('--folder'), outDir = take('--out', 'production');
const dry = args.includes('--dry') && args.splice(args.indexOf('--dry'), 1);
const only = take('--only', null)?.split('-').map(Number);
// --figma-slices <x>: first build the slices as frames in Figma (the owner's way – a frame per slice holding the
// layers active there, no text), at page x <x>, each at its draft height; then export FROM those frames (re-runs
// replace them; the owner can fix a slice frame in Figma and run with --from-figma-slices to export again)
const sliceX = take('--figma-slices', null);
const fromFrames = args.includes('--from-figma-slices') && args.splice(args.indexOf('--from-figma-slices'), 1);
const force = args.includes('--force') && args.splice(args.indexOf('--force'), 1);
const src = args[0];
if (!src || ((!page || !frameName) && !dry)) { console.error('usage: figma-production.mjs <storyboard folder> --page "<Figma page>" --frame "<draft frame>" [--only a-b] [--dry]'); process.exit(2); }
const ch = basename(resolve(src));
const [, sn, cn] = /^s(\d+)c(\d+)$/.exec(ch) ?? [];
const OUT = resolve(ROOT, 'storyboards/_out', ch);
const PROD = resolve(OUT, outDir);
const SLICE = SLICE_H;
// the CDN folder below the toon: new paths for this cut (the chapter's own folder holds its 625 cut, and uploads never replace)
const dir = (folderArg ?? `s${sn}/c${cn}/v${SLICE}`).replace(/^\/+|\/+$/g, '');
if (dir === `s${sn}/c${cn}`) { console.error(`--folder ${dir}: the chapter's own folder holds its earlier cut (chapters 1–5: 625) – CDN paths never change and the upload skips existing ones; take a new folder (default s${sn}/c${cn}/v${SLICE})`); process.exit(2); }
let expectFile = null; // the Figma file the run started in (preflight) – a plugin re-run elsewhere is waited for
const bridge = (c, a, o) => figma(c, a, { client: `figma-production ${ch}`, expectFile, ...o });

const draft = JSON.parse(readFileSync(resolve(OUT, 'chapter.draft.json'), 'utf8'));
const toon = draft.groups.flatMap((g) => g.slices)[0]?.image ? 'catpope' : 'catpope';

// the chapter draft's layout (as figma-chapter.mjs drew it) and the cut: a vertical scene group every SLICE from its own
// top, its last piece (cut < SLICE) padded to SLICE or dropped (`end`); a band as its slides
const layout = chapterLayout(draft);
const slices = [];
for (const G of layout) {
	if (!G.own) continue;
	if (G.pano) {
		for (let i = 0; i < G.slides; i++) slices.push({ group: G, x: i * W, y: G.top, w: W, h: S, cut: S, pano: true });
		continue;
	}
	G.pieces.forEach(([a, b], i) => {
		const end = i === G.pieces.length - 1 && b - a < SLICE ? { only: G.pieces.length === 1 } : null;
		slices.push({ group: G, x: 0, y: a, w: W, cut: b - a, h: end ? SLICE : b - a, end }); // h: the slice; cut: what Figma exports
	});
}
slices.forEach((s, i) => { s.n = i + 1; s.name = `s${sn}c${cn}-slice-${String(i + 1).padStart(2, '0')}`; });
if (dry) {
	for (const G of layout.filter((G) => G.own)) {
		const mine = slices.filter((s) => s.group === G);
		console.log(G.pano
			? `${G.label.padEnd(16)} y ${G.top}–${G.top + G.h}  band: ${mine.length} slides of ${W} × ${S} (${mine[0].n}–${mine.at(-1).n})`
			: `${G.label.padEnd(16)} y ${G.top}–${G.top + G.h}  ${Math.floor(G.h / SLICE)} × ${SLICE}${G.rest ? ` + ${G.rest} (slice ${mine.at(-1).n}: dropped if one colour without a text, else padded +${SLICE - G.rest})` : ''}  slices ${mine[0].n}–${mine.at(-1).n}`);
	}
	// where the cut runs: every piece's bottom, a band's top and bottom – figma-chapter.mjs draws a guide on each (sliceLines)
	const cuts = [...new Set(slices.flatMap((s) => (s.pano ? [s.y, s.y + s.cut] : [s.y + s.cut])))].sort((a, b) => a - b);
	const drawn = new Set(sliceLines(layout).guides.filter((g) => g.kind !== 'screen').map((g) => g.y));
	const off = [...cuts.filter((y) => !drawn.has(y)), ...[...drawn].filter((y) => !cuts.includes(y))];
	console.log(`${slices.length} slices; the cut runs at ${cuts.join(' ')}${off.length ? `\nTHE GUIDES DIFFER at ${off.join(' ')}` : '\n(the guides figma-chapter.mjs draws are the same lines)'}`);
	process.exit(0);
}
mkdirSync(resolve(PROD, 'slices'), { recursive: true });
mkdirSync(resolve(PROD, 'png'), { recursive: true });

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

// a Figma text → a dialog of the slice that holds its top (in % of the slice as uploaded: a padded one is 624 high)
const byShot = {};
for (const t of texts) { const id = t.name.split(' · ')[0]; (byShot[id] ??= []).push(t); }
function dialogFor(t, slice) {
	const id = t.name.split(' · ')[0];
	const k = byShot[id].indexOf(t);
	const shot = layout.flatMap((g) => g.shots).find((s) => s.id === id);
	const base = shot?.dialog?.[k] ?? {};
	const de = t.text.trim();
	const text = { de };
	for (const l of languages) if (l !== 'de' && tr[de]?.[l]) text[l] = tr[de][l];
	const { rect, text: _t, ...style } = base;
	return { ...style, id: base.id ?? `${ch}-${id}-${k + 1}`, rect: { x: +((t.x - slice.x) / slice.w * 100).toFixed(1), y: +((t.y - slice.y) / slice.h * 100).toFixed(1), width: +(t.w / slice.w * 100).toFixed(1) }, text };
}

// texts → slices; a text of a scene group that lands on no slice is listed (it would be lost)
const lost = [];
for (const t of texts) {
	const s = slices.find((s) => t.y >= s.y && t.y < s.y + s.cut && t.x + t.w / 2 >= s.x && t.x + t.w / 2 < s.x + s.w + (s.pano ? 0 : 1e6) && (!s.pano || t.x >= s.x - 1));
	if (s) (s.dialog ??= []).push(dialogFor(t, s));
	else if (!layout.some((G) => !G.own && t.y >= G.top && t.y < G.top + G.h)) lost.push(`${t.name} (${t.id}) at x ${t.x}, y ${t.y}: "${t.text.trim().slice(0, 40)}"`);
}
console.log(`${ch}: ${slices.length} slices (${slices.filter((s) => s.pano).length} panorama slides, ${slices.filter((s) => s.end).length} group ends to pad or drop), ${texts.length} texts, languages ${languages.join(' + ')}${Object.keys(tr).length ? '' : ' (no translations.json yet – German only)'}`);
if (lost.length) console.log(`${lost.length} text(s) on NO slice – not in the cut:\n  ${lost.join('\n  ')}`);

// export: 1x / 2x / 3x PNG from Figma → WebP
const todo = slices.filter((s) => !only || (s.n >= only[0] && s.n <= (only[1] ?? only[0])));
const frameIds = {};
if (sliceX != null) {
	for (let i = 0; i < todo.length; i += 10) {
		const part = todo.slice(i, i + 10);
		const made = await bridge('sliceFrames', { frame: frame.id, skip: ['text', 'guides'], slices: part.map((s) => ({ name: s.name, x: s.x, y: s.y, w: s.w, h: s.cut, px: Number(sliceX) + s.x, py: frame.y + s.y })) });
		for (const m of made) frameIds[m.name] = m.id;
		process.stdout.write(`slice frames ${Math.min(i + 10, todo.length)}/${todo.length}   `);
	}
	console.log();
}
if (fromFrames && sliceX == null) {
	const pg = await bridge('read', { page, depth: 1 }, { timeout: 240_000 });
	for (const n of pg.children) if (/-slice-\d+$/.test(n.name)) frameIds[n.name] = n.id;
}
// an earlier cut at another standard (chapters 3–5: 625, no run.json) holds the only lossless masters of the published
// slices – copied once to production-<standard>/ before this cut overwrites them (#78)
const prevStandard = readdirSync(resolve(PROD, 'png')).length ? (JSON.parse(existsSync(resolve(PROD, 'run.json')) ? readFileSync(resolve(PROD, 'run.json'), 'utf8') : '{}').standard ?? 625) : SLICE;
if (prevStandard !== SLICE && !existsSync(resolve(OUT, `production-${prevStandard}`))) {
	cpSync(PROD, resolve(OUT, `production-${prevStandard}`), { recursive: true });
	console.log(`the ${prevStandard} cut copied to ${ch}/production-${prevStandard}/ (its lossless masters) before this ${SLICE} cut`);
}
// the run manifest: a slice is done when this run (or a crashed one before it) exported it at this standard from the same
// region and its three files have the expected size. From slice frames (--figma-slices / --from-figma-slices) everything
// is exported again, as before – the owner may have fixed a frame (--only N for just that one)
const manifest = runManifest(resolve(PROD, 'run.json'), { standard: SLICE, page, frame: frame.id, folder: dir });
const sourceOf = (s) => (frameIds[s.name] ? `frame ${frameIds[s.name]}` : `region ${frame.id} ${s.x},${s.y} ${s.w}×${s.cut}`);
/** the slice's manifest entry when it is exported at this standard (from any source) and its three files are at its size */
const exported = (s) => { const m = manifest.get(s.name); return m?.status === 'done' && m.standard === SLICE && variantsOk(resolve(PROD, 'slices'), s.name, s.w, s.h) ? m : null; };
/** exported, or a group end dropped at this standard */
const dropped = (s) => { const m = manifest.get(s.name); return m?.status === 'dropped' && m.standard === SLICE ? m : null; };
const settled = (s) => exported(s) ?? dropped(s);
const removeFiles = (s, png) => {
	for (const suf of ['', '.2x', '.3x']) try { unlinkSync(resolve(PROD, 'slices', `${s.name}${suf}.webp`)); } catch {}
	if (png) for (const k of [1, 2, 3]) try { unlinkSync(resolve(PROD, 'png', `${s.name}@${k}x.png`)); } catch {}
};
const pace = figmaPacer();
const failed = [];
let skipped = 0, timedOut = 0;
for (const s of todo) {
	const source = sourceOf(s);
	if (!force && sliceX == null && !fromFrames && settled(s)?.source === source) { skipped++; continue; }
	const t0 = Date.now();
	try {
		const res = frameIds[s.name]
			? await bridge('exportScales', { node: frameIds[s.name], scales: [1, 2, 3] })
			: await bridge('region', { frame: frame.id, x: s.x, y: s.y, w: s.w, h: s.cut, hide: ['text', 'guides'], partial: true, layers: true, unclip: s.pano, scales: [1, 2, 3] });
		for (const r of res) writeFileSync(resolve(PROD, 'png', `${s.name}@${r.scale}x.png`), Buffer.from(r.base64, 'base64'));
		// a group's end (#108): one colour and no text's top in it → dropped; else padded to SLICE in its bottom row's colour
		let end = null;
		if (s.end) {
			const look = pieceLook(PROD, s.name);
			if (look.flat && !s.dialog?.length && !s.end.only) {
				removeFiles(s, true);
				manifest.set(s.name, { status: 'dropped', standard: SLICE, source, rest: s.cut, colour: hexOf(look.rgb), ms: Date.now() - t0 });
				timedOut = 0;
				const paused = await pace(Date.now() - t0);
				process.stdout.write(`\r${s.n}/${slices.length} ${s.name} dropped${paused ? ` (paused: ${paused})` : ''}   `);
				continue;
			}
			end = { rest: s.cut, pad: SLICE - s.cut, colour: hexOf(look.bottom ?? [255, 255, 255]), why: look.flat ? (s.dialog?.length ? 'one colour, but a text' : 'one colour, but the group\'s only piece') : look.bottom ? 'bottom row one colour' : 'bottom row not one colour → white' };
			webpVariants(PROD, s.name, { h: SLICE, rgb: look.bottom ?? [255, 255, 255] });
		} else webpVariants(PROD, s.name);
		if (!variantsOk(resolve(PROD, 'slices'), s.name, s.w, s.h)) throw Object.assign(new Error(`the export is not ${s.w} × ${s.h} (1x) / ${s.w * 3} × ${s.h * 3} (3x)`), { code: 'size' });
		manifest.set(s.name, { status: 'done', standard: SLICE, source, ...(end ? { end } : {}), ms: Date.now() - t0 });
		timedOut = 0;
	} catch (e) {
		// this slice failed for good (after the client's retries) – the rest goes on; anything else (no relay, no plugin
		// for 30 min, Figma stuck, no memory) ends the run: a re-run resumes
		if (!['plugin_error', 'timeout', 'queue_timeout', 'size'].includes(e.code)) throw e;
		manifest.set(s.name, { status: 'failed', standard: SLICE, source, error: e.message });
		// its old files (a 625 cut, a wrong export) must not be taken for this cut later
		removeFiles(s, false);
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
// every group end of the cut so far (this run and earlier ones): padded or dropped
const ends = slices.filter((s) => s.end).map((s) => ({ s, m: settled(s) })).filter((e) => e.m);
for (const { s, m } of ends) {
	console.log(m.status === 'dropped'
		? `drop  ${s.name}  ${s.group.label}: its last ${m.rest} px are one colour (${m.colour}) without a text – not in the chapter (the number stays free)`
		: `pad   ${s.name}  ${s.group.label}: ${m.end.rest} px + ${m.end.pad} px of ${m.end.colour} below (${m.end.why})`);
}
if (lost.length) console.log(`${lost.length} text(s) on NO slice (listed above) – move them onto the art, or they are not in the chapter`);

// chapter.json + uploads.json only for a complete cut: every slice (also outside --only) exported at this standard –
// a stale file (the 625 cut, a failed slice) would be uploaded to the new paths and stay there for good
const missing = slices.filter((s) => !settled(s));
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
const kept = slices.filter((s) => !dropped(s));
const chapter = { ...draft, groups: [] };
for (const G of layout) {
	if (!G.own) continue;
	const { slices: _s, ...g } = G.g;
	chapter.groups.push({ ...g, slices: kept.filter((s) => s.group === G).map((s) => ({
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
const uploads = kept.flatMap((s) => [['', 1], ['.2x', 2], ['.3x', 3]].map(([suf, k]) => ({ file: `slices/${s.name}${suf}.webp`, path: `${toon}/${dir}/${s.name}${suf}.webp`, w: s.w * k, h: s.h * k })));
writeFileSync(resolve(PROD, 'uploads.json'), JSON.stringify(uploads, null, '\t'));
const missingTr = [...new Set(texts.map((t) => t.text.trim()))].filter((de) => languages.some((l) => l !== 'de' && !tr[de]?.[l]));
writeFileSync(resolve(PROD, 'to-translate.json'), JSON.stringify(missingTr, null, '\t'));
console.log(`→ ${PROD}: chapter.json, uploads.json (${uploads.length} files → ${toon}/${dir}/), slices/ – ${missingTr.length} texts without a translation (to-translate.json)`);
