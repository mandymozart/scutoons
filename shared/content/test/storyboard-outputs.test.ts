// @scutoons/content/storyboard (#119): what the storyboard produces for the generators and the sheets – the gap
// prompts, the Luma prompt, the references' choice, the generation record, the schematic SVG, the shot list.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	airPhrase, characterImage, foldLog, gapPrompt, idOf, isMarkLine, kindOf, lightPhrase, lumaPrompt, markLine, moodOf,
	nextName, parentsOf, parseLines, pickSeed, promptSheet, record, refSet, SEED_MAX, shotlistMarkdown, shotOf, shotRows,
	shotSvg, takenByLog, takeOf, wantCounts, wrap, type RefSource, type Shot, type StoryboardDoc
} from '../src/storyboard/index.ts';

const shot = (extra: Partial<Shot> = {}): Shot => ({ id: 'sb-1-1', format: 'vertical', aspect: '1:3.2', action: { de: 'PLF | im Schnee' }, source: { want: 'generated' }, ...extra });
const sb = (shots: Shot[]): StoryboardDoc => ({
	storyboard: 2, toon: 'catpope', source: { scene: 1, heading: 'EXT. DORF' }, light: { key: 'top-left', source: 'sun', shadow: 'long' },
	cast: [{ ref: 'plf', character: 'catpope', persona: 'human' }, { ref: 'yplf', character: null, new: 'YPLF – junge PLF' }],
	locations: [{ ref: 'village', place: 'mountain-village', time: 'snow' }],
	groups: [{ label: 'scene-1', shots }]
});

test('gap prompts: the format, the safe-format check, the figures and what to attach', () => {
	const doc = sb([]);
	const g = gapPrompt(doc, shot({ location: 'village', air: [[0, 20]], figures: [{ cast: 'plf', x: 0.2, y: 0.8, mood: 'cold' }, { cast: 'yplf' }] }), 's1c3');
	assert.match(g.prompt, /^Very tall vertical image 1:3\.2; composed to be scrolled from top to bottom \(tilt-down\)\./);
	assert.match(g.prompt, /Figures: a person in a long plain white papal robe \(PLF\), cold, left, lower part of the frame/);
	assert.match(g.prompt, /YPLF – junge PLF, centre, middle part of the frame/);
	assert.match(g.prompt, /Location: mountain-village, snow\./);
	assert.match(g.prompt, /Composition: the top 20 % of the frame calm and empty/);
	assert.equal(g.check, "taller than Luma's native 9:16 – generate 9:16 segments with overlap and outpaint, or stitch");
	assert.deepEqual(g.refs, ['catpope / human: the base views + a mood portrait', 'YPLF – junge PLF (no reference yet – Wanted)', 'mountain-village: the Figma selection on the chapter page (closest photo of the place)', 'the schematic as "layout only" (storyboards/_out/s1c3/png/sb-1-1.png)']);
	assert.equal(gapPrompt(doc, shot({ format: 'loop360', aspect: '20:1' }), 'x').check, 'beyond 13 screens – split');
	assert.equal(gapPrompt(doc, shot({ format: 'viewport', aspect: '9:16' }), 'x').prompt.startsWith('Vertical phone frame 9:16.'), true);
	assert.equal(airPhrase([[60, 100]]), 'the bottom 40 % of the frame calm and empty (sky, snow, darkness or a plain field) – space for text');
	assert.equal(lightPhrase({}), 'Light: one extremely hard key light, deep black shadows, no fill.');
	const sheet = promptSheet('s1c3', [sb([shot(), shot({ id: 'sb-1-2', source: { want: 'photo' }, prompt: { en: 'a village' } })])]);
	assert.equal(sheet.count, 1);
	assert.match(sheet.markdown, /## sb-1-1 – vertical 1:3\.2/);
	assert.match(sheet.markdown, /No `prompt\.en` yet/);
	assert.equal(promptSheet('s1c3', [sb([shot(), shot({ id: 'sb-1-2', source: { want: 'photo' } })])], true).count, 2);
});

test('the Luma prompt: the story moment, figures, camera, the one hard light, the empty zones, the references', () => {
	const p = lumaPrompt(sb([]), shot({ view: 'belief', camera: { pos: [1, 2, 3], target: [0, 1, 0], fov: 40 }, figures: [{ cast: 'plf', pose: 'pray', facing: 'away', turn: 30 }], light: { accent: { color: 'red', on: 'die Beeren' }, soft: 0.1 }, air: [[0, 25]] }), { references: { structure: 's.png' } });
	assert.match(p, /^SHOT sb-1-1 – vertical 1:3\.2 \(a tall image, read from top to bottom\)\n/);
	assert.match(p, /Scene 1: EXT\. DORF\./);
	assert.match(p, /Perspective: what the character believes/);
	assert.match(p, /- PLF – Cat Pope in human form.*; hands folded in prayer; back to the camera \(turned 30°\); in the centre middle of the frame, about 30 %/);
	assert.match(p, /Hand-placed camera: at \[1\.0, 2\.0, 3\.0\] looking at \[0\.0, 1\.0, 0\.0\].*lens 40° vertical/);
	assert.match(p, /ONE extremely hard sun key light from the upper left, low and grazing|ONE extremely hard sun key light from the upper left\. Long hard cast shadows\. No fill light/);
	assert.match(p, /The only colour in the frame: red, on die Beeren\./);
	assert.match(p, /- the top 25 % of the frame/);
	assert.match(p, /- Image reference \(strong\): s\.png – the layout/);
	assert.ok(!/\n{3,}/.test(p) && p.endsWith('\n'));
	// a camera position without a target: the old tool threw, this one looks at the origin
	assert.match(lumaPrompt(sb([]), shot({ camera: { pos: [1, 1, 1] } })), /looking at \[0\.0, 0\.0, 0\.0\]/);
});

test("references: the persona's set by framing, one per mood; else the media images by view and mood", () => {
	const src: RefSource = {
		refFiles: (id) => (id === 'catpope/human/default' ? ['full-1.jpg', 'full-2.jpg', 'face-1.jpg', 'notes.txt', 'half-1.png'] : null),
		mediaHas: (rel) => rel === 'characters/catpope-mountaintiger/catpope-mountaintiger_full-neutral.webp' || rel === 'characters/boy/boy_portrait-scared.png'
	};
	assert.equal(refSet('ecu'), 'face');
	assert.equal(refSet('mcu'), 'half');
	assert.equal(refSet('medium'), 'full'); // the old tool's: only mcu / ms / mws are half
	assert.deepEqual(characterImage({ character: 'catpope', persona: 'human' }, 'cu', 'cold', src), { rel: 'refs/catpope/human/default/face-1.jpg', mood: 'sad' });
	const sad = [...'sad'].reduce((h, c) => h + c.charCodeAt(0), 0) % 2;
	assert.equal(characterImage({ character: 'catpope', persona: 'human' }, 'full', 'cold', src)?.rel, `refs/catpope/human/default/full-${sad + 1}.jpg`);
	assert.deepEqual(characterImage({ character: 'catpope', persona: 'mountaintiger' }, 'full', 'hunting', src), { rel: 'characters/catpope-mountaintiger/catpope-mountaintiger_full-neutral.webp', mood: 'neutral' });
	assert.deepEqual(characterImage({ character: 'boy' }, 'cu', 'terrified', src), { rel: 'characters/boy/boy_portrait-scared.png', mood: 'scared' });
	assert.equal(characterImage({ character: null }, 'cu', 'x', src), null);
	assert.equal(characterImage({ character: 'crows' }, 'cu', 'x', { refFiles: () => null }), null);
	assert.equal(moodOf('awe'), 'surprised');
	assert.equal(moodOf('happy'), 'happy');
	assert.equal(moodOf('bored'), 'neutral');
});

test('generation record: names, kinds, parents and takes from the inputs, every field present', () => {
	assert.equal(kindOf('sb-1-1--g2.MP4'), 'video');
	assert.equal(kindOf('sb-1-1--g2.png'), 'image');
	assert.equal(idOf('sb-1-1--g2.png'), 'sb-1-1--g2');
	assert.equal(shotOf('sb-1-1--g2'), 'sb-1-1');
	assert.equal(shotOf('nothing'), null);
	const inputs = [
		{ role: 'control', key: 'out:s1c3/handheld/sb-1-1--t3--grey.mp4' },
		{ role: 'still', key: 'frame:out:s1c3/generated/sb-1-1--g1.mp4#last' },
		{ role: 'style', key: 'boards:style/x.png' }
	];
	assert.deepEqual(parentsOf(inputs), ['sb-1-1--g1']);
	assert.deepEqual(parentsOf(inputs, (k) => (k.startsWith('boards:') ? 'sb-1-1--g0' : null)), ['sb-1-1--g1', 'sb-1-1--g0']);
	assert.equal(takeOf(inputs), 't3');
	const r = record({ id: 'sb-1-1--g2', file: 'sb-1-1--g2.mp4', inputs, generator: 'runway' }, () => 'NOW');
	assert.deepEqual([r.v, r.shot, r.kind, r.take, r.parents, r.status, r.date, r.mark, r.seed], [2, 'sb-1-1', 'video', 't3', ['sb-1-1--g1'], 'succeeded', 'NOW', null, null]);
	assert.ok(!('error' in r) && !('params' in r));
	const failed = record({ id: 'sb-1-1--g3', status: 'failed', error: 'quota', params: { lora: 'x' }, date: null });
	assert.deepEqual([failed.file, failed.kind, failed.error, failed.params, failed.date], [null, 'image', 'quota', { lora: 'x' }, null]);
	for (let i = 0; i < 50; i++) { const s = pickSeed(); assert.ok(Number.isInteger(s) && s >= 0 && s <= SEED_MAX); }
	assert.equal(pickSeed(() => 0.9999999999), SEED_MAX);
});

test('the log: the last mark line per id wins (with the one before), old and broken lines apart; the next free name', () => {
	const text = [
		JSON.stringify(record({ id: 'a--g1', file: 'a--g1.png', date: 'd' })),
		JSON.stringify(markLine('a--g1', 'review', 't1')),
		JSON.stringify(markLine('a--g1', 'rejected', 't2')),
		JSON.stringify(markLine('a--g4', 'review', 't3')),
		JSON.stringify({ shot: 'a', file: 'old.png' }),
		'not json',
		''
	].join('\n');
	const lines = parseLines(text);
	assert.equal(lines.length, 6);
	const log = foldLog(lines);
	assert.equal(log.records.size, 1);
	assert.deepEqual([log.records.get('a--g1')!.mark, log.records.get('a--g1')!.before], ['rejected', { mark: 'review', marked: 't1' }]);
	assert.equal(log.marks.get('a--g4')!.mark, 'review');
	assert.equal(log.earlier.length, 1);
	assert.equal(log.broken, 1);
	assert.ok(isMarkLine(markLine('x')) && !isMarkLine({ v: 2, id: 'x', file: 'y' }));
	const taken = ['a--g2.png', ...takenByLog(lines)];
	assert.deepEqual(nextName('a', taken, 'png'), { id: 'a--g5', file: 'a--g5.png', n: 5 });
	assert.deepEqual(nextName('a', taken, null, 9), { id: 'a--g10', file: null, n: 10 });
	assert.equal(nextName('sb-1-1', []).id, 'sb-1-1--g1');
});

test('schematic: canvas size, escaped text, the move arrow; wrap', () => {
	const doc = sb([]);
	const v = shotSvg(shot({ location: 'village', move: 'climb', figures: [{ cast: 'plf' }], dialog: [{ rect: { x: 5, y: 5 }, text: { de: '<Hallo> & "du"' } }] }), doc);
	assert.equal(v.w, 390);
	assert.equal(v.h, 1248);
	assert.match(v.svg, /fill="#eef2f5"/);
	assert.match(v.svg, /&lt;Hallo&gt; &amp; &quot;du&quot;/);
	assert.match(v.svg, />PLF</);
	assert.match(v.svg, />↑<\/text>\n<\/svg>$/);
	assert.ok(!shotSvg(shot({ dialog: [{ rect: { x: 5, y: 5 }, text: { de: 'weg' } }] }), doc, { noText: true }).svg.includes('weg'));
	const p = shotSvg(shot({ format: 'loop360', aspect: '3:1' }), doc);
	assert.deepEqual([p.w, p.h], [2532, 844]);
	assert.equal((p.svg.match(/stroke-dasharray="3 5"/g) ?? []).length, 6);
	assert.deepEqual(wrap('eins zwei drei vier', 9), ['eins zwei', 'drei vier']);
});

test('shot list: rows per shot, the want counts, a table per place', () => {
	const rows = shotRows([sb([shot({ location: 'village', figures: [{ cast: 'yplf', mood: 'awe' }] }), shot({ id: 'sb-1-2', format: 'loop360', aspect: '3:1', source: undefined, projection: 'polar' })])]);
	assert.deepEqual(rows.map((r) => [r.id, r.place, r.want, r.px, r.framing, r.who]), [
		['sb-1-1', 'mountain-village', 'generated', '390 × 1248', '', 'YPLF – junge PLF (awe)'],
		['sb-1-2', '–', '?', '6.5 screens wide', 'polar', '']
	]);
	assert.deepEqual(wantCounts(rows), [['generated', 1], ['?', 1]]);
	const md = shotlistMarkdown('s1c3', rows);
	assert.match(md, /^# Shot list s1c3 – 2 shots/);
	assert.match(md, /\| generated \| 1 \|/);
	assert.ok(!md.includes('| photo | 0 |'));
	assert.match(md, /## mountain-village \(1\)/);
	assert.match(md, /PLF \/ im Schnee \|$/m);
});
