// slice-edges.ts (agents/research/TEXT-ACROSS-SLICES.md §7): which slice edges are open, which the layout's window cuts,
// and the validator's warnings – nothing for an open edge, one per cut edge, one per collision.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { dialogCollisions, groupAxis, sliceEdges, validateFile } from '../src/index.ts';

const slice = (extra: object = {}) => ({ image: 'a.webp', width: 390, height: 625, ...extra });
const group = (renderer: object, slices: object[]) => ({ label: 'g', renderer, slices });

test('sliceEdges: vertical cuts at the column and the group ends, a strip at the screen and its ends, the band only at the screen', () => {
	const v = group({ type: 'vertical' }, [slice(), slice(), slice()]);
	assert.equal(groupAxis(v), 'vertical');
	assert.deepEqual(sliceEdges(v, 0), { top: 'group-end', bottom: 'open', left: 'column', right: 'column' });
	assert.deepEqual(sliceEdges(v, 1), { top: 'open', bottom: 'open', left: 'column', right: 'column' });
	assert.deepEqual(sliceEdges(v, 2), { top: 'open', bottom: 'group-end', left: 'column', right: 'column' });
	const s = group({ type: 'horizontal' }, [slice(), slice()]);
	assert.equal(groupAxis(s), 'strip');
	assert.deepEqual(sliceEdges(s, 0), { top: 'screen', bottom: 'screen', left: 'strip-end', right: 'open' });
	assert.deepEqual(sliceEdges(s, 1), { top: 'screen', bottom: 'screen', left: 'open', right: 'strip-end' });
	const e = group({ type: 'horizontal', params: { endless: true } }, [slice(), slice()]);
	assert.equal(groupAxis(e), 'endless');
	assert.deepEqual(sliceEdges(e, 0), { top: 'screen', bottom: 'screen', left: 'open', right: 'open' });
});

test('validator: nothing for an open edge, a warning per cut edge, a collision names the later box', () => {
	const warnings = (g: object) =>
		validateFile('chapter', { title: 'x', groups: [g] }, 'de', { file: 'x' } as never)
			.filter((p) => p.level === 'warning' && p.path.includes('slices'))
			.map((p) => `${p.path.slice(2).join('.')}: ${p.message}`);
	const box = (y: number, h: number, text = 'Hallo', x = 10) => ({ rect: { x, y, width: 60, height: h }, text, fit: 'fixed' });
	// vertical: a box crossing from slice 1 into slice 2 – open, no warning
	assert.deepEqual(warnings(group({ type: 'vertical' }, [slice({ dialog: [box(90, 20)] }), slice()])), []);
	// past the first slice's top and past the side: cut
	const cut = warnings(group({ type: 'vertical' }, [slice({ dialog: [box(-10, 20), box(10, 10, 'x', 60)] }), slice()]));
	assert.equal(cut.length, 2);
	assert.match(cut[0], /^slices\.0\.dialog\.0\.rect: .*above the group's first slice/);
	assert.match(cut[1], /^slices\.0\.dialog\.1\.rect: .*right side – the reader's column cuts it/);
	// a collision: slice 1's box reaches into slice 2 where slice 2 has a box
	const hit = warnings(group({ type: 'vertical' }, [slice({ dialog: [box(90, 20, 'Alle warteten')] }), slice({ dialog: [box(2, 10, 'Wo bist du?!')] })]));
	assert.deepEqual(hit, ["slices.0.dialog.0.rect: covered by 'Wo bist du?!' on slice 2 – the later slice paints on top (move one, or anchor this box on slice 2)"]);
	// a strip: across a seam open, past the screen's bottom and the strip's end cut
	const strip = warnings(group({ type: 'horizontal' }, [slice({ dialog: [box(10, 10, 'a', 80)] }), slice({ dialog: [box(95, 10, 'b', 80)] })]));
	assert.equal(strip.length, 2);
	assert.match(strip[0], /^slices\.1\.dialog\.0\.rect: .*strip's end/);
	assert.match(strip[1], /^slices\.1\.dialog\.0\.rect: .*bottom – a horizontal group is one screen high/);
	// the band: left / right open (the wrap seam too)
	assert.deepEqual(warnings(group({ type: 'horizontal', params: { endless: true } }, [slice({ dialog: [box(10, 10, 'a', -20)] }), slice({ dialog: [box(10, 10, 'b', 80)] })])), []);
	// an old-style click area across a seam: only its own slice's part reacts
	const area = warnings(group({ type: 'vertical' }, [slice({ events: [{ id: 'e1', type: 'showMessage', params: { text: 'x' }, trigger: { type: 'click', rect: { x: 0, y: 90, width: 50, height: 20 } } }] }), slice()]));
	assert.ok(area.some((w) => /only the part on its own slice reacts/.test(w)), area.join('\n'));
});

test('dialogCollisions: in a strip along x; boxes without a height never collide', () => {
	const s = group({ type: 'horizontal' }, [slice({ dialog: [{ rect: { x: 80, y: 10, width: 40, height: 10 }, text: 'A', fit: 'fixed' }] }), slice({ dialog: [{ rect: { x: 5, y: 12, width: 30, height: 10 }, text: 'B', fit: 'fixed' }] })]);
	assert.deepEqual(dialogCollisions(s), [{ slice: 0, dialog: 0, bySlice: 1, byLabel: 'B' }]);
	const auto = group({ type: 'vertical' }, [slice({ dialog: [{ rect: { x: 10, y: 95, width: 60 }, text: 'A' }] }), slice({ dialog: [{ rect: { x: 10, y: 0, width: 60 }, text: 'B' }] })]);
	assert.deepEqual(dialogCollisions(auto), []);
});
