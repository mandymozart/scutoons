// The storyboard renderer moved into the creator (src/lib/storyboard/render, #124): the scene of a shot built in node
// (three's scene graph needs no WebGL; the procedural figurines – no model files): the preset and the hand camera,
// each figure placed where its x / y / h say (the 3D view's inverse gives them back), facing / turn, a deterministic
// set, everything freed by dispose; the text presets.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import type { BufferGeometry } from 'three';
import { shotSpec, type Shot, type StoryboardDoc } from '@scutoons/content/storyboard';
import { buildScene, shotCamera } from '../src/lib/storyboard/render/scene.ts';
import { presetOf, presetStyle } from '../src/lib/storyboard/render/text-presets.ts';
import { rng } from '../src/lib/storyboard/render/sets.ts';
import { seedOf } from '../src/lib/storyboard/render/texts.ts';
import { D, screenPlace, wrapDeg } from '../src/lib/editor/storyboard-3d-place.ts';

const scene: Pick<StoryboardDoc, 'cast' | 'locations' | 'light'> = {
	cast: [
		{ ref: 'plf', character: 'catpope', persona: 'human' },
		{ ref: 'tiger', character: 'catpope', persona: 'mountaintiger' }
	],
	locations: [{ ref: 'wald', place: 'mountain-forest', time: 'day' }],
	light: { key: 'top-left', height: 'high' }
};
const shot = (over: Partial<Shot> = {}): Shot => ({
	id: 'sb-28-2',
	format: 'viewport',
	aspect: '1:1.6',
	shot: { size: 'full', angle: 'high' },
	location: 'wald',
	action: { de: 'PLF steht im Schnee' },
	figures: [
		{ cast: 'plf', x: 0.35, y: 0.55, h: 0.3, facing: 'three-quarter-left' },
		{ cast: 'tiger', x: 0.7, y: 0.6, h: 0.2, turn: 30 }
	],
	...over
});
const near = (a: number, b: number, eps = 1e-6) => assert.ok(Math.abs(a - b) < eps, `${a} ≉ ${b}`);

test('the preset camera of size + angle, the hand camera', () => {
	const spec = shotSpec(scene, shot(), 1);
	const { camera, dist } = shotCamera(spec);
	assert.equal(dist, 6);
	assert.deepEqual(camera.position.toArray().map((n) => +n.toFixed(6)), [0, 5, 6]);
	const hand = shotCamera(shotSpec(scene, shot({ camera: { pos: [2, 1.5, 4], target: [0, 1, 0], fov: 40 } }), 1));
	assert.deepEqual(hand.camera.position.toArray(), [2, 1.5, 4]);
	assert.equal((hand.camera as { fov: number }).fov, 40);
});

test('every figure stands where its screen place says – the 3D view gives the same place back', async () => {
	const spec = shotSpec(scene, shot(), 1);
	const built = await buildScene(spec, { shadowSize: 256 });
	assert.equal(built.figs.length, 2);
	const f = { W: spec.w, H: spec.h, K: built.K };
	spec.figures.forEach((fig, i) => {
		const m = built.figs[i];
		assert.ok(m.place, 'placed');
		const back = screenPlace(built.camera, f, m.place!.pos, m.place!, m.root.rotation.y);
		near(back.x, fig.x!);
		near(back.y, fig.y!);
		near(back.h, fig.h!);
	});
	// the facing / turn against the camera
	near(screenPlace(built.camera, f, built.figs[0].place!.pos, built.figs[0].place!, built.figs[0].root.rotation.y).turn, -40, 1e-6);
	near(wrapDeg((built.figs[1].root.rotation.y - Math.atan2(built.camera.position.x - built.figs[1].place!.pos.x, built.camera.position.z - built.figs[1].place!.pos.z)) / D), 30, 1e-6);
	// a high camera grounds a standing figure: its feet on the snow
	assert.equal(built.figs[0].place!.grounded, true);
	near(built.figs[0].place!.pos.y, 0);
	assert.equal(built.skyWhite, true);
	assert.ok(built.light.key, 'one hard key light');
	built.dispose();
});

test('the set is deterministic per shot id, and dispose frees every geometry', async () => {
	const count = async () => {
		const built = await buildScene(shotSpec(scene, shot(), 1), { shadowSize: 256 });
		const geos = new Set<BufferGeometry>();
		built.scene.traverse((o) => {
			const g = (o as { geometry?: BufferGeometry }).geometry;
			if (g) geos.add(g);
		});
		let freed = 0;
		for (const g of geos) g.addEventListener('dispose', () => freed++);
		built.dispose();
		return { n: geos.size, freed };
	};
	const a = await count(),
		b = await count();
	assert.ok(a.n > 100, `a forest: ${a.n} geometries`);
	assert.equal(a.n, b.n);
	assert.equal(a.freed, a.n);
	const r1 = rng('sb-28-2'),
		r2 = rng('sb-28-2');
	assert.deepEqual([r1(), r1(), r1()], [r2(), r2(), r2()]);
	assert.equal(seedOf('sb-28-2'), [...'sb-28-2'].reduce((s, c) => s + c.charCodeAt(0), 0) % 97);
});

test('a floating figure stays in the air (ground: false) and a jump is airborne', async () => {
	const built = await buildScene(shotSpec(scene, shot({ figures: [{ cast: 'plf', x: 0.5, y: 0.5, h: 0.3, ground: false }, { cast: 'plf', x: 0.3, y: 0.5, h: 0.3, pose: 'jump' }] }), 1));
	assert.equal(built.figs[0].place!.grounded, false);
	assert.equal(built.figs[1].place!.free, true);
	built.dispose();
});

test('the texts in the SCUTOONS presets', () => {
	assert.equal(presetOf({ variant: 'narration', text: '25 EXT. WALD' }), 'scene');
	assert.equal(presetOf({ variant: 'narration', text: 'Später.' }), 'narration');
	assert.equal(presetOf({ variant: 'bubble' }), 'speech-bubble');
	assert.equal(presetOf({ font: 'lettering' }), 'onomatopoeia');
	assert.equal(presetOf({ font: 'lettering', title: true }), 'title');
	assert.equal(presetOf({}), 'story');
	assert.equal(presetStyle('scene').uppercase, true);
	assert.equal(presetStyle('onomatopoeia').rotation, -7);
	assert.equal(presetStyle('title', { size: 20 }).size, 20);
	assert.equal(presetStyle('speech-bubble').tail, 'bottom-left');
});
