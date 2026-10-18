// version-note.ts: the version note's suggestion (owner 2026-10-18, DESIGN.md "Version notes") – a deterministic
// summary of what changed since the stored version, in the UI language (the German words here, like de.json's).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { chapterChanges, groupChanges, metaChanges, numberRange, summarizeChanges, type GroupRef, type NoteContext, type NoteWords } from '../src/lib/editor/version-note.ts';

const groupName = (g: GroupRef) => ('scene' in g ? `Szene ${g.scene}` : 'label' in g ? `Gruppe ${g.label}` : `Gruppe ${g.number}`);
const one: Record<string, string> = { textbox: 'Textbox', item: 'Item', piece: 'Collage-Stück', tail: 'Zeiger', slice: 'Slice', group: 'Gruppe', event: 'Event', sound: 'Sound', entry: 'Eintrag' };
const many: Record<string, string> = { textbox: 'Textboxen', item: 'Items', piece: 'Collage-Stücke', tail: 'Zeiger', slice: 'Slices', group: 'Gruppen', event: 'Events', sound: 'Sounds', entry: 'Einträge' };
const verbs: Record<string, string> = { reordered: 'umsortiert', added: 'hinzugefügt', removed: 'entfernt', moved: 'verschoben', recoloured: 'eingefärbt', set: 'gesetzt', changed: 'geändert' };
const de: NoteWords = {
	subject: (kind, count) => (count === 1 ? one[kind] : `${count} ${many[kind]}`),
	verb: (action, kind) => (kind === 'item' && action === 'added' ? 'platziert' : (verbs[action] ?? action)),
	phrase: (subject, verb, place) => `${subject}${place} ${verb}`,
	onSlice: (n) => `auf Slice ${n}`,
	inGroup: (g) => `in ${groupName(g)}`,
	group: groupName,
	slice: (n) => `Slice ${n}`,
	newSlices: (range, count) => (count === 1 ? `Neuer Slice ${range}` : `Neue Slices ${range}`),
	removedSlices: (range, count) => (count === 1 ? `Slice ${range} entfernt` : `Slices ${range} entfernt`),
	translated: (langs) => `Texte (${langs.join(', ')}) übersetzt`,
	plural: (kind) => many[kind],
	field: (key) => ({ title: 'Titel', description: 'Beschreibung', release: 'Erscheinungsdatum' })[key] ?? null,
	settings: 'Einstellungen',
	created: 'Neu angelegt'
};
const words = de;

const L = { source: 'de', languages: ['de', 'en'] } as const;
const ctx: NoteContext = { languages: L, itemName: (id) => ({ camera: 'Kamera' })[id] ?? id };
const clone = <T>(v: T): T => JSON.parse(JSON.stringify(v));
const box = (id: string, de: string, x = 10, en = '') => ({ id, rect: { x, y: 10, width: 50 }, text: en ? { de, en } : { de } });
const slice = (image: string, extra: Record<string, unknown> = {}): Record<string, any> => ({ image, width: 390, height: 625, ...extra });
const scene = (n: number): { label: string; slices: Record<string, any>[] } => ({ label: 'scene-17-24', slices: Array.from({ length: n }, (_, i) => slice(`s${i + 1}.webp`, { dialog: [box(`b${i}`, `Text ${i}`)] })) });

test('one text box moved and its tail set on one slice: the slice in front, the box by its first words', () => {
	const before = scene(14);
	before.slices[11].dialog = [box('a', 'Na, servus, wie geht es dir?')];
	const after = clone(before);
	(after.slices[11].dialog[0] as any).rect.x = 30;
	(after.slices[11].dialog[0] as any).tail = { kind: 'speech', tip: { x: 20, y: 90 }, base: { side: 'bottom', at: 0.3 } };
	const note = summarizeChanges(groupChanges(before, after, { scene: 'scene-17-24' }, ctx), words);
	assert.equal(note, "Slice 12: Textbox 'Na, servus…' verschoben, Zeiger gesetzt");
});

test('three text boxes changed on different slices of one scene (the page has several groups): counted, the scene named', () => {
	const before = scene(6);
	const after = clone(before);
	for (const i of [1, 3, 4]) (after.slices[i].dialog[0] as any).text.de = 'Neu';
	const note = summarizeChanges(groupChanges(before, after, { scene: 'scene-17-24' }, ctx), words, { multipleGroups: true });
	assert.equal(note, '3 Textboxen in Szene scene-17-24 geändert');
});

test('an item placed on a slice: named by the catalog, the slice named', () => {
	const before = scene(5);
	const after = clone(before);
	(after.slices[3] as any).items = [{ id: 'camera-1', item: 'camera', rect: { x: 10, y: 10, width: 20 } }];
	assert.equal(summarizeChanges(groupChanges(before, after, undefined, ctx), words), "Item 'Kamera' auf Slice 4 platziert");
});

test('a collage piece recoloured (only its look): "eingefärbt", by its file name', () => {
	const piece = { id: 'piece-abcdefg', src: 'collage/brush-grey-22.webp', ratio: 1, rect: { x: 5, y: 5, width: 30 } };
	const before = { slices: [slice('a.webp'), slice('b.webp', { pieces: [piece] })] };
	const after = clone(before);
	(after.slices[1] as any).pieces[0].preset = 'ink';
	assert.equal(summarizeChanges(groupChanges(before, after, undefined, ctx), words), "Collage-Stück 'brush-grey-22' auf Slice 2 eingefärbt");
	// moved and recoloured = changed; only the order = reordered
	(after.slices[1] as any).pieces[0].rect.x = 50;
	assert.equal(summarizeChanges(groupChanges(before, after, undefined, ctx), words), "Collage-Stück 'brush-grey-22' auf Slice 2 geändert");
	const two = { slices: [slice('a.webp', { pieces: [piece, { ...piece, id: 'piece-2' }] })] };
	const swapped = clone(two);
	swapped.slices[0].pieces.reverse();
	assert.equal(summarizeChanges(groupChanges(two, swapped, undefined, ctx), words), 'Collage-Stücke auf Slice 1 umsortiert');
});

test('new slices at the end: one range; removed ones too; a replaced picture is the slice changed', () => {
	const before = { slices: Array.from({ length: 39 }, (_, i) => slice(`p${i}.webp`)) };
	const after = clone(before);
	for (let i = 0; i < 5; i++) after.slices.push(slice(''));
	assert.equal(summarizeChanges(groupChanges(before, after, undefined, ctx), words), 'Neue Slices 40–44');
	const fewer = clone(before);
	fewer.slices.splice(2, 3);
	assert.equal(summarizeChanges(groupChanges(before, fewer, undefined, ctx), words), 'Slices 3–5 entfernt');
	const replaced = clone(before);
	replaced.slices[6].image = 'new.webp';
	assert.equal(summarizeChanges(groupChanges(before, replaced, undefined, ctx), words), 'Slice 7 geändert');
	// a slice inserted in the middle: the others keep their content (no false "changed")
	const inserted = clone(before);
	inserted.slices.splice(10, 0, slice('fresh.webp'));
	assert.equal(summarizeChanges(groupChanges(before, inserted, undefined, ctx), words), 'Neuer Slice 11');
	assert.equal(numberRange([5, 1, 2, 3, 9]), '1–3, 5, 9');
});

test('only the English texts changed: "Texte (en) übersetzt", not an edit of the boxes', () => {
	const before = scene(3);
	const after = clone(before);
	for (const s of after.slices) (s.dialog[0] as any).text.en = 'Translated';
	assert.equal(summarizeChanges(groupChanges(before, after, { scene: 'x' }, ctx), words, { multipleGroups: true }), 'Texte (en) übersetzt');
});

test('a chapter: its settings by name, a group added, the order of groups, the sound', () => {
	const before = { title: { de: 'Alt' }, description: { de: 'D' }, groups: [{ label: 'a', slices: [] }, { ref: { src: 'chapter1/scene-2.json' } }], audio: { music: 'a.mp3' } };
	const after = clone(before) as any;
	after.title.de = 'Neu';
	after.release = { de: '2026-11-01' };
	assert.equal(summarizeChanges(chapterChanges(before, after, ctx), words), 'Titel, Erscheinungsdatum geändert');
	after.groups.reverse();
	after.groups.push({ label: 'c', slices: [] });
	after.audio.music = 'b.mp3';
	assert.equal(summarizeChanges(chapterChanges(before, after, ctx), words), "Gruppe 'c' hinzugefügt, Gruppen umsortiert, Sound geändert, …");
	after.audio.music = 'a.mp3';
	assert.equal(summarizeChanges(chapterChanges(before, after, ctx), words), "Gruppe 'c' hinzugefügt, Gruppen umsortiert, Titel, Erscheinungsdatum geändert");
	assert.equal(summarizeChanges(chapterChanges(null, after, ctx), words), 'Neu angelegt');
});

test('slice events and a text box added, all on one slice of a chapter with several groups: the group and slice in front', () => {
	const before = { title: { de: 'T' }, groups: [{ label: 'intro', slices: [slice('a.webp')] }, { label: 'main', slices: [slice('b.webp'), slice('c.webp')] }] };
	const after = clone(before) as any;
	after.groups[1].slices[1].events = [{ id: 'e1', type: 'showMessage' }, { id: 'e2', type: 'fx' }];
	after.groups[1].slices[1].dialog = [box('n', '')];
	assert.equal(summarizeChanges(chapterChanges(before, after, ctx), words, { multipleGroups: true }), 'Gruppe main, Slice 2: Textbox hinzugefügt, 2 Events hinzugefügt');
});

test('too long: whole phrases up to 90 characters, then "…"', () => {
	const before = scene(8);
	const after = clone(before);
	after.slices[0].dialog.push(box('x', 'Ein sehr langer Text'));
	(after.slices[1] as any).items = [{ id: 'i', item: 'camera', rect: { x: 1, y: 1, width: 5 } }];
	(after.slices[2] as any).pieces = [{ id: 'p', src: 'a/b.webp', ratio: 1, rect: { x: 1, y: 1, width: 5 } }];
	(after.slices[3] as any).audio = { sfx: 'x' };
	(after.slices[4] as any).events = [{ id: 'e' }];
	after.slices.splice(6, 1);
	(after as any).label = 'neu';
	const note = summarizeChanges(groupChanges(before, after, { scene: 'scene-17-24' }, ctx), words, { multipleGroups: true });
	assert.ok(note.length <= 90, note);
	assert.ok(note.endsWith(', …'), note);
	assert.ok(note.startsWith("Szene scene-17-24: Textbox 'Ein sehr l…' auf Slice 1 hinzugefügt, "), note);
	assert.equal(summarizeChanges([], words), '');
});

test('meta pages: the cast by entry, credits entries by name, toon settings by field, a translation', () => {
	const cast = { seasons: { '1': [{ character: 'pope' }, { character: 'kira' }] } };
	const cast2 = clone(cast);
	cast2.seasons['1'].push({ character: 'max' });
	assert.equal(summarizeChanges(metaChanges(cast, cast2, { languages: null }), words), "Eintrag 'max' hinzugefügt");
	const credits = { people: [{ id: 'p1', name: 'Anna' }, { id: 'p2', name: 'Ben' }, { id: 'p3', name: 'Cem' }] };
	const credits2 = clone(credits);
	credits2.people[0].name = 'Anna B.';
	credits2.people[2].name = 'Cem K.';
	credits2.people.splice(1, 1);
	assert.equal(summarizeChanges(metaChanges(credits, credits2, { languages: null }), words), "Eintrag 'Ben' entfernt, 2 Einträge geändert");
	const toon = { title: { de: 'Catpope', en: 'Catpope' }, description: { de: 'x', en: 'y' }, colour: '#fff' };
	const toon2 = clone(toon);
	toon2.description.en = 'z';
	assert.equal(summarizeChanges(metaChanges(toon, toon2, { languages: L }), words), 'Texte (en) übersetzt');
	toon2.title.de = 'Cat';
	toon2.colour = '#000';
	assert.equal(summarizeChanges(metaChanges(toon, toon2, { languages: L }), words), 'Texte (en) übersetzt, Einstellungen geändert');
	assert.equal(summarizeChanges(metaChanges(null, toon2, { languages: L }), words), 'Neu angelegt');
});
