// clip.ts (BACKLOG 10.39): the clipboard's JSON, fresh ids on paste, media references across toons, settings only
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	applyBoxLook,
	applyGroupSettings,
	applyItemSettings,
	applyPieceLook,
	applySampleSettings,
	applySliceEventSettings,
	applyVnSettings,
	clipPlacements,
	clipText,
	countPlacements,
	foreignFiles,
	listAt,
	makeClip,
	mergeSounds,
	newPlacementId,
	offsetRect,
	prepareGroup,
	preparePiece,
	prepareSlices,
	readClip,
	remapSounds,
	soundsOf
} from '../src/lib/editor/clip.ts';

test('a clip round-trips through the clipboard text; anything else is not a clip', () => {
	const entry = { id: 'a', rect: { x: 1, y: 2, width: 3 } };
	const clip = makeClip('dialog', 'catpope', [entry], { chapter: 1 });
	entry.rect.x = 99;
	assert.equal(clip.entries[0].rect.x, 1, 'a deep copy');
	assert.deepEqual(readClip(clipText(clip)), clip);
	assert.equal(readClip('hello'), null);
	assert.equal(readClip('{"marker":"other"}'), null);
	assert.equal(readClip('{"marker":"scutoons/creator-clip","kind":"nope","toon":"x","entries":[]}'), null);
});

test('newPlacementId: <item>-<7 random>, never a taken one', () => {
	const id = newPlacementId('cucumber');
	assert.match(id, /^cucumber-[a-z0-9]{7}$/);
	assert.notEqual(newPlacementId('cucumber', new Set([id])), id);
});

test('prepareSlices: media reference the source toon, fresh box / event / placement ids, showTextBox follows its box', () => {
	const clip = makeClip('slices', 'catpope', []);
	const slices = [
		{
			image: 's1/c1/a.webp',
			imageVariants: { medium: 's1/c1/a.2x.webp' },
			video: { src: 's1/c1/v.mp4', variants: { low: 's1/c1/v.480.mp4' } },
			dialog: [{ id: 's1c1-3-1', rect: { x: 0, y: 0, width: 1 } }],
			events: [{ id: 'eold', type: 'showTextBox', params: { dialog: 's1c1-3-1' } }],
			items: [{ id: 'coin-1', item: 'coin', events: [{ id: 'eitem', type: 'hideMessage' }] }]
		}
	];
	let n = 0;
	const out = prepareSlices(clip, slices, { toon: 'toptoy', nextBoxId: () => `s1c2-9-${++n}` });
	const s = out[0];
	assert.equal(s.image, '../catpope/s1/c1/a.webp');
	assert.equal(s.imageVariants.medium, '../catpope/s1/c1/a.2x.webp');
	assert.equal(s.video.src, '../catpope/s1/c1/v.mp4');
	assert.equal(s.video.variants.low, '../catpope/s1/c1/v.480.mp4');
	assert.equal(s.dialog[0].id, 's1c2-9-1');
	assert.equal(s.events[0].params.dialog, 's1c2-9-1', 'showTextBox follows its box');
	assert.equal(s.events[0].id, undefined, 'event ids go (the edit assigns fresh ones)');
	assert.equal(s.items[0].events[0].id, undefined);
	assert.match(s.items[0].id, /^coin-[a-z0-9]{7}$/);
	assert.equal(slices[0].image, 's1/c1/a.webp', 'the clip itself is untouched');
	assert.equal(foreignFiles(clip.entries.length ? clip.entries : slices, undefined, 'catpope', 'toptoy'), 4);
	assert.equal(foreignFiles(slices, undefined, 'catpope', 'catpope'), 0);
});

test('prepareGroup: unique label, script ids dropped, the script\'s showTextBox follows', () => {
	const clip = makeClip('groups', 'catpope', []);
	const group = { label: 'scene-1', events: [{ id: 'e1', type: 'showTextBox', params: { dialog: 'd1' } }], slices: [{ image: 'a.webp', dialog: [{ id: 'd1' }] }] };
	const labels = new Set(['scene-1', 'scene-1-2']);
	const g = prepareGroup(clip, group, { toon: 'catpope', nextBoxId: () => 'new-1' }, labels);
	assert.equal(g.label, 'scene-1-3');
	assert.equal(g.events[0].id, undefined);
	assert.equal(g.events[0].params.dialog, 'new-1');
	assert.equal(g.slices[0].image, 'a.webp', 'same toon: paths unchanged');
});

test('sounds: the used library entries travel; a paste adds the missing ones (rebased), clashing ids get a new one', () => {
	const audio = { sampleLibrary: [{ id: 'door', path: 's1/c1/door.mp3' }, { id: 'rain', path: 's1/c1/rain.mp3' }], music: { tracks: [{ id: 'theme', src: 's1/theme.mp3' }] } };
	const slices = [{ audio: { audio: { samples: [{ id: 'door' }] }, music: { tracks: [{ id: 'theme', volume: 1 }] } } }];
	const sounds = soundsOf(slices, audio);
	assert.deepEqual(sounds.samples?.map((s) => s.id), ['door']);
	assert.deepEqual(sounds.tracks?.map((s) => s.id), ['theme']);
	const target: any = { audio: { sampleLibrary: [{ id: 'door', path: 's1/c3/other-door.mp3' }] } };
	const map = mergeSounds(target, sounds, 'catpope', 'toptoy');
	assert.deepEqual(target.audio.sampleLibrary[1], { id: 'door_2', path: '../catpope/s1/c1/door.mp3' });
	assert.deepEqual(target.audio.music.tracks[0], { id: 'theme', src: '../catpope/s1/theme.mp3' });
	const pasted: any = JSON.parse(JSON.stringify(slices[0]));
	remapSounds(pasted, map);
	assert.equal(pasted.audio.audio.samples[0].id, 'door_2');
	// the same file again: no new entry
	const again = mergeSounds(target, sounds, 'catpope', 'toptoy');
	assert.equal(target.audio.sampleLibrary.length, 2);
	assert.equal(again.samples.get('door'), 'door_2');
});

test('settings only (Shift+V)', () => {
	const box: any = { id: 'b', rect: { x: 1 }, text: { de: 'x' }, layer: 2, variant: 'caption', font: 'mono' };
	applyBoxLook(box, { id: 'a', rect: { x: 9 }, text: { de: 'y' }, variant: 'bubble', tail: { x: 1 } });
	assert.deepEqual(box, { id: 'b', rect: { x: 1 }, text: { de: 'x' }, layer: 2, variant: 'bubble', tail: { x: 1 } });

	const item: any = { id: 'coin-x', item: 'coin', rect: { x: 1 }, action: 'none' };
	applyItemSettings(item, { id: 'coin-y', item: 'coin', rect: { x: 5 }, action: 'events', events: [{ id: 'e1', type: 'hideMessage' }] });
	assert.equal(item.id, 'coin-x');
	assert.equal(item.action, 'events');
	assert.equal(item.events[0].id, undefined);

	const sample: any = { id: 'door', trigger: { type: 'click', rect: { x: 1 } }, volume: 0.6 };
	applySampleSettings(sample, { id: 'rain', trigger: { type: 'intersect', mode: 'poly', rect: { x: 9 } }, volume: 1 });
	assert.deepEqual(sample, { id: 'rain', trigger: { type: 'intersect', mode: 'poly', rect: { x: 1 } }, volume: 1 });

	const event: any = { id: 'e1', type: 'showMessage', params: {}, trigger: { type: 'click', rect: { x: 1 } } };
	applySliceEventSettings(event, { id: 'e2', type: 'jumpTo', params: { label: 'a' }, if: 'x', trigger: { type: 'click', rect: { x: 9 } } });
	assert.deepEqual(event, { id: 'e1', type: 'jumpTo', params: { label: 'a' }, if: 'x', trigger: { type: 'click', rect: { x: 1 } } });

	const vn: any = { id: 'e1', type: 'showMessage', params: { text: { de: 'a' } } };
	assert.equal(applyVnSettings(vn, [{ type: 'jumpTo' }]), false);
	assert.equal(applyVnSettings(vn, [{ type: 'showMessage', params: { text: { de: 'b' } }, if: 'y' }]), true);
	assert.deepEqual(vn, { id: 'e1', type: 'showMessage', params: { text: { de: 'b' } }, if: 'y' });

	const group: any = { label: 'a', slices: [1], renderer: { type: 'vertical' } };
	applyGroupSettings(group, { label: 'b', renderer: { type: 'horizontal' }, continues: true });
	assert.deepEqual(group, { label: 'a', slices: [1], renderer: { type: 'horizontal' }, continues: true });
});

test('placements are counted per catalog item (a chapter, a scene, a clip)', () => {
	const chapter = { groups: [{ slices: [{ items: [{ item: 'coin' }, { item: 'coin' }] }, { items: [{ item: 'key' }] }] }, { ref: { src: 'x' } }] };
	assert.deepEqual([...countPlacements(chapter)], [['coin', 2], ['key', 1]]);
	assert.deepEqual([...countPlacements({ slices: [{ items: [{ item: 'coin' }] }] })], [['coin', 1]]);
	assert.deepEqual([...clipPlacements(makeClip('item', 't', [{ item: 'coin' }]))], [['coin', 1]]);
	assert.deepEqual([...clipPlacements(makeClip('groups', 't', [{ slices: [{ items: [{ item: 'key' }] }] }]))], [['key', 1]]);
});

// a stand-in for a Svelte $state document: what is assigned is stored as ANOTHER object (Svelte stores a proxy of it)
// – an `a.b ??= []` expression is the raw value, so a push into it bypasses the document (owner 2026-10-17)
const stateLike = (target: any): any =>
	new Proxy(target, {
		get: (t, key) => (t[key] && typeof t[key] === 'object' ? stateLike(t[key]) : t[key]),
		set: (t, key, value) => ((t[key] = Array.isArray(value) ? [...value] : value && typeof value === 'object' ? { ...value } : value), true)
	});

test('listAt: a pasted entry lands in the document even when the slice had no such list yet', () => {
	const slice = stateLike({ image: 'a.webp' });
	const old: any[] = (slice.items ??= []);
	old.push({ id: 'lost' });
	assert.deepEqual(slice.items, [], 'the `??=` value is not the stored list');
	listAt(slice, ['dialog']).push({ id: 'd1' });
	listAt(slice, ['audio', 'audio', 'samples']).push({ id: 's1' });
	listAt(slice, ['dialog']).push({ id: 'd2' });
	assert.deepEqual(slice.dialog.map((d: any) => d.id), ['d1', 'd2']);
	assert.deepEqual(slice.audio.audio.samples, [{ id: 's1' }]);
});

test('collage pieces: a paste gets a new id, an offset rect and its file referenced across toons; Shift+V copies the look only', () => {
	const piece = { id: 'piece-aaaaaaa', src: 'vectors/disc.svg', ratio: 1, rect: { x: 10, y: 20, width: 30 }, anchor: 'c', preset: 'negative', style: { opacity: 0.5 } };
	const clip = makeClip('piece', 'catpope', [piece]);
	assert.equal(readClip(clipText(clip))?.kind, 'piece', 'a piece clip reads back');
	const taken = new Set(['piece-aaaaaaa']);
	const pasted = preparePiece(clip.entries[0], 'catpope', 'toptoy', [piece], taken)!;
	assert.match(pasted.id, /^piece-[a-z0-9]{7}$/);
	assert.notEqual(pasted.id, piece.id);
	assert.ok(taken.has(pasted.id), 'the new id is taken now');
	assert.deepEqual(pasted.rect, { x: 12, y: 22, width: 30 }, 'on top of the original: one step right / down');
	assert.equal(pasted.src, '../catpope/vectors/disc.svg', 'referenced from toptoy, never copied');
	assert.equal(pasted.preset, 'negative');
	assert.equal(clip.entries[0].src, 'vectors/disc.svg', 'the clip itself is untouched');
	// back in catpope: the toon's own path; a character picture stays the root's
	assert.equal(preparePiece({ ...piece, src: '../catpope/vectors/disc.svg' }, 'toptoy', 'catpope', [], new Set())!.src, 'vectors/disc.svg');
	assert.equal(preparePiece({ ...piece, src: '../characters/catpope/catpope_portrait-neutral.webp' }, 'catpope', 'toptoy', [], new Set())!.src, '../characters/catpope/catpope_portrait-neutral.webp');
	assert.deepEqual(offsetRect({ x: 1, y: 1, width: 5 }, []), { x: 1, y: 1, width: 5 });
	assert.equal(preparePiece('nope', 'catpope', 'toptoy', [], new Set()), null);
	// the toast counts its file (pieces as entries, and pieces inside slices); a pasted slice's pieces are rebased too
	assert.equal(foreignFiles(clip.entries, undefined, 'catpope', 'toptoy', 'piece'), 1);
	assert.equal(foreignFiles([{ image: 'a.webp', pieces: [piece, { ...piece, src: 'vectors/star.svg' }] }], undefined, 'catpope', 'toptoy'), 3);
	const slices = prepareSlices(makeClip('slices', 'catpope', []), [{ image: 'a.webp', pieces: [piece] }], { toon: 'toptoy', nextBoxId: () => 'x' });
	assert.equal(slices[0].pieces[0].src, '../catpope/vectors/disc.svg');

	// Shift+V: preset + own style come over; file, place, figure and id stay
	const target: any = { id: 'piece-b', src: '../characters/catpope/a.webp', ratio: 0.5, rect: { x: 5, y: 5, width: 10 }, rotation: 20, figure: { id: 'catpope', view: 'portrait' }, preset: 'ink' };
	applyPieceLook(target, piece);
	assert.deepEqual(target, { id: 'piece-b', src: '../characters/catpope/a.webp', ratio: 0.5, rect: { x: 5, y: 5, width: 10 }, rotation: 20, figure: { id: 'catpope', view: 'portrait' }, preset: 'negative', style: { opacity: 0.5 } });
	target.style.opacity = 1;
	assert.equal(piece.style.opacity, 0.5, 'a copy');
	applyPieceLook(target, { id: 'p', src: 'b.svg', ratio: 1, rect: { x: 0, y: 0, width: 1 } });
	assert.equal(target.preset, undefined, 'as drawn: no look, no own style');
	assert.equal(target.style, undefined);
});
