// text-presets.ts: speech / thought are drawn bubbles with a free tail (owner 2026-10-18, DESIGN.md Part 2 "Tails")
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { TEXT_PRESETS, applyTextPreset, matchTextPreset } from '../src/lib/editor/text-presets.ts';

const preset = (id: string) => TEXT_PRESETS.find((p) => p.id === id)!;

test('speech: a drawn bubble with a default tail below its box; thought keeps the tip, only the kind changes', () => {
	const d: Record<string, any> = { id: 'a', rect: { x: 10, y: 10, width: 80, height: 12 }, text: 'Hi', variant: 'caption' };
	applyTextPreset(d, preset('speech'));
	assert.equal(d.variant, 'bubble');
	assert.equal(d.frame, true);
	assert.equal(d.tail.kind, 'speech');
	assert.deepEqual(d.tail.base, { side: 'bottom', at: 0.3 });
	assert.ok(d.tail.tip.y > 22);
	assert.equal(matchTextPreset(d), 'speech');
	d.tail.tip = { x: -30, y: 150 };
	applyTextPreset(d, preset('thought'));
	assert.deepEqual(d.tail, { kind: 'thought', tip: { x: -30, y: 150 }, base: { side: 'bottom', at: 0.3 } });
	assert.equal(matchTextPreset(d), 'thought');
	// another look drops the tail
	applyTextPreset(d, preset('narration'));
	assert.equal(d.tail, undefined);
	assert.equal(matchTextPreset(d), 'narration');
});

test('an old fixed tail becomes a free one where it was', () => {
	const d: Record<string, any> = { rect: { x: 10, y: 50, width: 40, height: 10 }, text: 'Hi', variant: 'bubble', frame: true, font: 'mono', align: 'left', valign: 'middle', size: 4, tail: 'top-right' };
	// an old bubble with a fixed tail counts as speech
	assert.equal(matchTextPreset(d), 'speech');
	applyTextPreset(d, preset('speech'), { left: 10, top: 50, width: 40, height: 10 });
	assert.equal(d.tail.base.side, 'top');
	assert.equal(d.tail.base.at, 0.75);
	assert.ok(d.tail.tip.y < 50);
});
