// Parity (#119): the TypeScript storyboard model gives the old plain-JS tools' results on the real storyboards
// (storyboards/catpope/s1c3 … s1c5) – constants, the grid, measures, validate()'s messages, the render spec, the
// schematic SVG, the Luma prompt, the references' choice, the generation record, the prompt and shot-list sheets.
// It guards against drift while both exist; it goes with the old tools (#128).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync, mkdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import * as sb from '../src/storyboard/index.ts';
import * as oldSpec from '../../../tools/storyboard-render/spec.js';
import * as oldLib from '../../../scripts/storyboard/lib.mjs';
import * as oldSb from '../../../scripts/storyboard/storyboard.mjs';
import * as oldSchematic from '../../../scripts/storyboard/schematic.mjs';
import * as oldRefs from '../../../scripts/storyboard/refs.mjs';
import * as oldLog from '../../../scripts/storyboard/generated-log.mjs';
import { lumaPrompt as oldLuma } from '../../../scripts/storyboard/luma.js';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');
const CHAPTERS = ['s1c3', 's1c4', 's1c5'].map((c) => join(ROOT, 'storyboards/catpope', c));
const scenes = CHAPTERS.flatMap((dir) => readdirSync(dir).filter((f) => /^scene-.*\.json$/.test(f)).map((f) => {
	const file = join(dir, f);
	return { file, doc: JSON.parse(readFileSync(file, 'utf8')) as sb.StoryboardDoc };
}));
const shots = scenes.flatMap(({ doc }) => sb.shotsOf(doc).map(({ shot }) => ({ doc, shot })));
const plain = <T>(v: T): T => JSON.parse(JSON.stringify(v));

test('the real storyboards are there (14 scenes, 100+ shots)', () => {
	assert.equal(scenes.length, 14);
	assert.ok(shots.length > 100, `${shots.length} shots`);
});

test('constants and vocabulary are the old ones', () => {
	for (const k of ['W', 'S', 'UNIT', 'SLICE_ASPECTS'] as const) assert.deepEqual(sb[k], oldSpec[k], k);
	for (const k of ['RHYTHM', 'TARGET', 'SAFE', 'FORMATS', 'PROJECTIONS', 'MOVES', 'SIZES', 'ANGLES', 'LIGHT', 'POSES', 'FACING', 'VIEWS', 'SOURCES', 'STATUS', 'W', 'S', 'UNIT', 'SLICE_ASPECTS'] as const)
		assert.deepEqual(plain(sb[k]), plain(oldLib[k]), k);
	assert.deepEqual([...sb.MOODS], oldRefs.MOODS);
	assert.deepEqual([...sb.MARKS], oldLog.MARKS);
	assert.equal(sb.SEED_MAX, oldLog.SEED_MAX);
	assert.equal(sb.LOG, oldLog.LOG);
});

test('the grid: slicesOf, parseAspect, aspectSlices, shotHeight on every height and aspect', () => {
	for (let h = 0; h <= 4000; h += 1) assert.equal(sb.slicesOf(h), oldSpec.slicesOf(h), `h ${h}`);
	const aspects = ['9:16', '1:1.6', '1:3', '1:3.2', '1:4', '1:4.8', '1:5', '1:6.4', '1:7', '1:8', '3:1', '6:1', '13:1', 'x', '', '1:', '1.5:2.5', null, undefined, 4];
	for (const a of [...aspects, ...shots.map((s) => s.shot.aspect)]) {
		assert.deepEqual(sb.parseAspect(a), oldSpec.parseAspect(a), String(a));
		assert.equal(sb.aspectSlices(a), oldSpec.aspectSlices(a), String(a));
		for (const f of ['viewport', 'vertical', 'loop360', undefined]) assert.equal(sb.shotHeight(a, f), oldLib.shotHeight(a, f), `${a} ${f}`);
	}
});

test('words, readSeconds, textOf, esc on every text', () => {
	for (const { shot } of shots) {
		for (const t of [shot.action, ...(shot.dialog ?? []).map((d) => d.text), shot.note, undefined, 'a – b 12 ?']) {
			assert.equal(sb.words(t as sb.SbText), oldLib.words(t));
			assert.equal(sb.textOf(t as sb.SbText), oldLib.textOf(t));
			assert.equal(sb.textOf(t as sb.SbText, 'en'), oldLib.textOf(t, 'en'));
			assert.equal(sb.esc(t), oldLib.esc(t));
		}
	}
	for (let n = 0; n < 50; n++) assert.equal(sb.readSeconds(n), oldLib.readSeconds(n));
	assert.equal(sb.fmtMin(1234), oldLib.fmtMin(1234));
});

test('measureShot (careful + skim), measureVn, summarize', () => {
	for (const { shot } of shots) {
		assert.deepEqual(sb.measureShot(shot), oldSb.measureShot(shot), shot.id);
		assert.deepEqual(sb.measureShot(shot, sb.RHYTHM.lookPerScreen.skim), oldSb.measureShot(shot, 3), shot.id);
	}
	const msg = (text: string) => ({ type: 'showMessage', params: { text: { de: text } } });
	const vn = { events: [msg('eins zwei'), msg('drei'), { type: 'showChoices' }, ...Array.from({ length: 8 }, () => msg('a b c')), { type: 'joinScene' }, msg('x')] };
	assert.deepEqual(sb.measureVn(vn), oldSb.measureVn(vn));
	const list = scenes.map(({ doc, file }) => ({ sb: doc, file }));
	assert.deepEqual(sb.summarize(list), oldSb.summarize(list));
	assert.deepEqual(sb.shotsOf(scenes[0]!.doc).map((s) => s.shot.id), oldSb.shotsOf(scenes[0]!.doc).map((s: { shot: sb.Shot }) => s.shot.id));
});

test('validate: the same errors and warnings on every scene file (with and without a file name)', () => {
	let warnings = 0;
	for (const { file, doc } of scenes) {
		assert.deepEqual(sb.validate(doc, file), oldSb.validate(doc, file), file);
		assert.deepEqual(sb.validate(doc), oldSb.validate(doc), file);
		warnings += oldSb.validate(doc, file).warnings.length;
	}
	assert.ok(warnings > 0, 'the real files carry warnings, so the messages are compared');
});

test('validate: the same messages on a storyboard that breaks every rule', () => {
	const base = scenes.find((s) => s.doc.source.scene === 28)!.doc;
	const broken = structuredClone(base) as unknown as Record<string, any>;
	broken.storyboard = 1;
	broken.toon = '';
	broken.cast.push({ character: 'x' }, { ref: 'nop', character: 'catpope' });
	broken.light = { key: 'sideways', height: 'mid', source: 'torch', shadow: 'soft', soft: 3, quality: 'diffuse' };
	const g = broken.groups[0];
	g.events = Array.from({ length: 9 }, () => ({ type: 'showMessage', params: { text: 'Hallo du' } }));
	const s = g.shots;
	s.push({ ...s[0] }); // a duplicate id
	s.push({ format: 'vertical' }); // no id
	s.push({ id: 'Shot 9', format: 'film', aspect: '9-16', projection: 'fisheye', move: 'zoom', shot: { size: 'xl', angle: 'odd' }, action: {}, location: 'moon', source: { type: 'ai', want: 'magic' }, status: 'done', review: { status: 'maybe' }, view: 'god', figures: [{ cast: 'ghost', pose: 'fly', facing: 'up', x: 3, y: -0.5 }, { cast: 'nop' }], air: [[50, 20], [-1, 10]], dialog: [{ text: 'x', variant: 'shout', tail: 'side', font: 'serif', rect: { x: 120, y: 5 } }, { text: { de: Array(40).fill('Wort').join(' ') } }] });
	s.push({ id: 'sb-28-90', format: 'viewport', aspect: '1:3', action: { de: 'x' }, review: { status: 'change', note: 'heller', date: '2026-10-10' } });
	s.push({ id: 'sb-28-91', format: 'vertical', aspect: '1:9', action: { de: 'x' }, source: { want: 'photo' } });
	s.push({ id: 'sb-28-92', format: 'vertical', aspect: '1:5', action: { de: 'x' }, source: { want: 'generated' } });
	s.push({ id: 'sb-28-93', format: 'vertical', aspect: '9:16', action: { de: 'x' } });
	s.push({ id: 'sb-28-94', format: 'loop360', aspect: '15:1', action: { de: 'x' }, dialog: [{ text: { de: 'quer' }, rect: { x: 10, y: 10, width: 30 } }] });
	s.push({ id: 'sb-28-95', format: 'loop360', aspect: '4:1', action: { de: 'x' } });
	for (let i = 0; i < 6; i++) s.push({ id: `sb-28-${96 + i}`, format: i < 5 ? 'viewport' : 'vertical', aspect: '1:1.6', action: { de: 'x' }, dialog: Array.from({ length: 5 }, () => ({ text: { de: 'kurz' }, rect: { x: 0, y: 0 } })) });
	for (let i = 0; i < 6; i++) s.push({ id: `sb-28-${110 + i}`, format: 'vertical', aspect: '1:3.2', action: { de: 'x' } });
	broken.groups.push({ shots: [] });
	const doc = broken as unknown as sb.StoryboardDoc;
	const ours = sb.validate(doc, 'storyboards/catpope/s1c3/scene-28.json'), theirs = oldSb.validate(doc, 'storyboards/catpope/s1c3/scene-28.json');
	assert.deepEqual(ours, theirs);
	assert.ok(ours.errors.length > 25 && ours.warnings.length > 20, `${ours.errors.length} errors, ${ours.warnings.length} warnings`);
	const empty = {} as sb.StoryboardDoc;
	assert.deepEqual(sb.validate(empty), oldSb.validate(empty));
});

test('shotSpec (ss 1 and 2) on every shot', () => {
	for (const { doc, shot } of shots) {
		assert.deepEqual(plain(sb.shotSpec(doc, shot)), plain(oldSpec.shotSpec(doc, shot)), shot.id);
		assert.deepEqual(plain(sb.shotSpec(doc, shot, 2)), plain(oldSpec.shotSpec(doc, shot, 2)), shot.id);
	}
});

test('the schematic SVG, byte for byte, on every shot (with and without text)', () => {
	for (const { doc, shot } of shots) {
		assert.deepEqual(sb.shotSvg(shot, doc), oldSchematic.shotSvg(shot, doc), shot.id);
		assert.deepEqual(sb.shotSvg(shot, doc, { noText: true }), oldSchematic.shotSvg(shot, doc, { noText: true }), shot.id);
	}
	const extra = { id: 'sb-1-1', format: 'viewport', aspect: '9:16', projection: 'isometric', move: 'pass', action: { de: 'x' }, figures: [{ cast: 'a', framing: 'cu' }], dialog: [{ variant: 'bubble', rect: { x: 1, y: 2 }, text: { de: 'Ein <b>&</b> "Text"' } }] } as unknown as sb.Shot;
	assert.deepEqual(sb.shotSvg(extra, { cast: [{ ref: 'a', character: null }] }), oldSchematic.shotSvg(extra, { cast: [{ ref: 'a', character: null }] }));
});

test('the Luma prompt on every shot (with references and the script)', () => {
	const ctx = { script: '  25 EXT. DORF\nPLF stapft.  ', references: { structure: 'sb-25-1.png', style: ['a.png', 'b.png'], characters: ['plf.jpg'] } };
	for (const { doc, shot } of shots) {
		assert.equal(sb.lumaPrompt(doc, shot), oldLuma(doc, shot), shot.id);
		assert.equal(sb.lumaPrompt(doc, shot, ctx), oldLuma(doc, shot, ctx), shot.id);
	}
	const doc = scenes[0]!.doc;
	const lit = { ...shots[0]!.shot, camera: { dist: 4, yaw: 10 }, light: { azimuth: 90, soft: 0.1, rim: true, source: 'none' as const } };
	assert.equal(sb.lumaPrompt(doc, lit), oldLuma(doc, lit));
	const lit2 = { ...lit, light: { elevation: 60, soft: 0.1, rim: true, shadow: 'short' as const } };
	assert.equal(sb.lumaPrompt(doc, lit2), oldLuma(doc, lit2));
});

test("the references' choice: the same character image for every figure (the real refs folder and media copy)", () => {
	const refsDir = join(ROOT, 'storyboards/style/refs');
	const media = oldLib.mediaRoot();
	const src: sb.RefSource = {
		refFiles: (id) => (existsSync(join(refsDir, id)) ? readdirSync(join(refsDir, id)) : null),
		...(media ? { mediaHas: (rel: string) => existsSync(join(media, rel)) } : {})
	};
	let found = 0;
	for (const { doc, shot } of shots) {
		const ours = sb.shotCharacters(doc, shot, src), theirs = oldRefs.shotCharacters(doc, shot);
		assert.equal(ours.length, theirs.length);
		ours.forEach((o, i) => {
			const t = theirs[i];
			assert.deepEqual(o.cast, t.cast, shot.id);
			assert.deepEqual(o.image, t.image ? { rel: t.image.rel, mood: t.image.mood } : null, `${shot.id} ${o.fig.cast}`);
			if (o.image) found++;
		});
		for (const f of shot.figures ?? []) for (const fr of ['cu', 'ecu', 'mcu', 'medium', 'full', undefined])
			assert.deepEqual(sb.characterImage({ character: 'catpope', persona: 'human' }, fr, f.mood, src)?.rel ?? null, oldRefs.characterImage({ character: 'catpope', persona: 'human' }, fr, f.mood)?.rel ?? null);
	}
	assert.ok(found > 0, 'some figures have an image');
	for (const m of [...sb.MOODS, 'terrified', 'cold', 'whatever', undefined]) assert.equal(sb.moodOf(m), oldRefs.moodOf(m));
});

test('the generation record: ids, kinds, parents, takes, record(), mark lines, keyOf', () => {
	for (const f of ['sb-25-1--g3.png', 'sb-25-1--g12.mp4', 'x.webm', 'noext', 'a.b.MOV', 'sb-28-2a--g1--up.png']) {
		assert.equal(sb.kindOf(f), oldLog.kindOf(f));
		assert.equal(sb.idOf(f), oldLog.idOf(f));
		assert.equal(sb.shotOf(f), oldLog.shotOf(f));
	}
	const inputs = [
		{ role: 'control', key: 'frame:out:s1c3/handheld/sb-25-1--t2--beauty.mp4#first' },
		{ role: 'still', key: 'out:s1c3/generated/sb-25-1--g2.png' },
		{ role: 'reference', key: 'out:s1c3/generated/sb-25-1--g2.png' },
		{ role: 'keyframe', key: 'out:s1c3/handheld/sb-25-1--t4/depth/f0012.png' },
		{ role: 'style', key: 'boards:style/a.png' },
		{ role: 'character', key: null, name: 'x.jpg' }
	];
	const parentOf = (k: string) => (k.startsWith('boards:') ? 'sb-1-1--g9' : null);
	assert.deepEqual(sb.parentsOf(inputs), oldLog.parentsOf(inputs));
	assert.deepEqual(sb.parentsOf(inputs, parentOf), oldLog.parentsOf(inputs, parentOf));
	assert.equal(sb.takeOf(inputs), oldLog.takeOf(inputs));
	assert.equal(sb.takeOf(inputs.slice(3)), oldLog.takeOf(inputs.slice(3)));
	assert.equal(sb.takeOf(inputs.slice(4)), oldLog.takeOf(inputs.slice(4)));
	const fields = [
		{ id: 'sb-25-1--g3', file: 'sb-25-1--g3.mp4', inputs, generator: 'runway', seed: 7, date: '2026-10-20T10:00:00Z', params: { lora: null } },
		{ id: 'sb-25-1--g4', status: 'failed', error: new Error('boom') as unknown as string, date: null, params: { lora: 'x' }, take: null },
		{ id: 'odd', inputs: [{ role: 'still' }], date: '2026-10-20' }
	];
	for (const f of fields) assert.deepEqual(sb.record(f as sb.RecordFields), oldLog.record(f), f.id);
	assert.deepEqual(sb.markLine('a', 'review', 'now'), oldLog.markLine('a', 'review', 'now'));
	assert.deepEqual(sb.markLine('a'), oldLog.markLine('a'));
	const roots = { out: join(ROOT, 'storyboards/_out'), media: join(ROOT, 'media'), boards: join(ROOT, 'storyboards') };
	for (const f of ['storyboards/_out/s1c3/generated/a.png', 'storyboards/style/x.jpg', 'media/characters/c.webp', 'elsewhere/x.png'])
		assert.equal(sb.keyOf(roots, join(ROOT, f)), oldLog.keyOf(roots, join(ROOT, f)), f);
	assert.equal(sb.keyOf(null, join(ROOT, 'x')), oldLog.keyOf(null, join(ROOT, 'x')));
});

test('the log fold and the next free name: the same as reading the file', () => {
	const out = mkdtempSync(join(tmpdir(), 'sb-log-'));
	try {
		const lines = [
			JSON.stringify(oldLog.record({ id: 'sb-25-1--g1', file: 'sb-25-1--g1.png', date: 'd1' })),
			JSON.stringify({ shot: 'sb-25-1', file: 'old.png' }),
			'{ broken',
			JSON.stringify(oldLog.markLine('sb-25-1--g1', 'review', 't1')),
			JSON.stringify(oldLog.markLine('sb-25-1--g1', 'rejected', 't2')),
			JSON.stringify(oldLog.markLine('sb-25-1--g7', 'review', 't3')),
			JSON.stringify({ v: 2, id: '' }),
			'42',
			JSON.stringify(oldLog.record({ id: 'sb-25-1--g5', status: 'failed', date: 'd2' })),
			JSON.stringify(oldLog.record({ id: 'sb-25-1--g1', file: 'sb-25-1--g1.png', date: 'd3', prompt: 'again' }))
		];
		mkdirSync(join(out, 's1c3', 'generated'), { recursive: true });
		writeFileSync(join(out, 's1c3', oldLog.LOG), lines.join('\n') + '\n');
		writeFileSync(join(out, 's1c3', 'generated', 'sb-25-1--g9.png'), '');
		const text = readFileSync(join(out, 's1c3', oldLog.LOG), 'utf8');
		const parsed = sb.parseLines(text);
		assert.deepEqual(parsed, oldLog.readLines(out, 's1c3'));
		const ours = sb.foldLog(parsed), theirs = oldLog.readLog(out, 's1c3');
		assert.deepEqual([...ours.records], [...theirs.records]);
		assert.deepEqual([...ours.marks], [...theirs.marks]);
		assert.deepEqual(ours.earlier, theirs.earlier);
		assert.equal(ours.broken, theirs.broken);
		const taken = [...readdirSync(join(out, 's1c3', 'generated')), ...sb.takenByLog(parsed)];
		for (const [shot, ext, after] of [['sb-25-1', 'png', 0], ['sb-25-1', null, 12], ['sb-25-2', 'mp4', 0], ['sb.x', 'png', 0]] as const)
			assert.deepEqual(sb.nextName(shot, taken, ext, after), oldLog.nextName(out, 's1c3', shot, ext, after), `${shot} ${after}`);
	} finally {
		rmSync(out, { recursive: true, force: true });
	}
});

test('the prompt sheet and the shot list: the old scripts write the same Markdown', () => {
	const out = mkdtempSync(join(tmpdir(), 'sb-sheets-'));
	try {
		for (const dir of CHAPTERS) {
			const name = dir.split(/[\\/]/).pop()!;
			const docs = oldSb.loadStoryboards(dir).map((x: { sb: sb.StoryboardDoc }) => x.sb) as sb.StoryboardDoc[];
			for (const all of [false, true]) {
				execFileSync(process.execPath, [join(ROOT, 'scripts/storyboard/prompts.mjs'), dir, ...(all ? ['--all'] : []), '--out', out], { cwd: ROOT, stdio: 'pipe' });
				assert.equal(sb.promptSheet(name, docs, all).markdown, readFileSync(join(out, `${name}-prompts.md`), 'utf8'), `${name} all=${all}`);
			}
			execFileSync(process.execPath, [join(ROOT, 'scripts/storyboard/shotlist.mjs'), dir, '--out', out], { cwd: ROOT, stdio: 'pipe' });
			assert.equal(sb.shotlistMarkdown(name, sb.shotRows(docs)), readFileSync(join(out, `${name}-shotlist.md`), 'utf8'), name);
		}
	} finally {
		rmSync(out, { recursive: true, force: true });
	}
});
