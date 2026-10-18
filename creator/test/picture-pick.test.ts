// picture-pick.ts (owner 2026-10-18): the eyedropper's fallback reads a pixel of the slice picture
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { sampleAt } from '../src/lib/editor/picture-pick.ts';

test('sampleAt: fractions of the picture → that pixel as #rrggbb, clamped at the edges', () => {
	// 2 × 2: red, green / blue, white (RGBA)
	const pixels = { width: 2, height: 2, data: [255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255] };
	assert.equal(sampleAt(pixels, 0.1, 0.1), '#ff0000');
	assert.equal(sampleAt(pixels, 0.9, 0.1), '#00ff00');
	assert.equal(sampleAt(pixels, 0.1, 0.9), '#0000ff');
	assert.equal(sampleAt(pixels, 1, 1), '#ffffff');
	assert.equal(sampleAt(pixels, -0.5, 2), '#0000ff');
});
