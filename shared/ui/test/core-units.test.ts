import { describe, it } from 'node:test';
import { expect } from './expect.ts';
import { coreUnits, unitRefs, type UnitConversions } from '../src/core-units.ts';

// the core's units laid out as ScrubNumber's options: every number comes from the core (a stub here, the wasm in the app)
const asked: string[] = [];
const core: UnitConversions = {
	offered: (dimension, refs) => (dimension === 'time' ? (Number.isNaN(refs[0]) ? ['s', 'ms'] : ['s', 'ms', 'frames']) : []),
	convert: (v, from, to) => (asked.push(`${from}>${to}`), from === to ? v : from === 'ms' && to === 's' ? v / 1000 : to === 'ms' ? v * 1000 : NaN),
	pick: (chosen, fallback) => chosen || fallback,
	word: (u) => u,
	decimals: (u) => (u === 's' ? 2 : 0),
	step: (u) => (u === 's' ? 0.01 : 1)
};

describe('coreUnits', () => {
	it('offers what the core offers, converting through it both ways', () => {
		const u = coreUnits(core, { kind: 'crossfade', dimension: 'time', stored: 'ms' });
		expect(u.options.map((o) => o.id)).toEqual(['s', 'ms']);
		expect(u.fallback).toBe('ms');
		const s = u.options[0];
		expect([s.label, s.step, s.decimals]).toEqual(['s', 0.01, 2]);
		expect(s.toShown(3000)).toBe(3);
		expect(s.toStored(1.5)).toBe(1500);
		expect(asked).toEqual(['ms>s', 's>ms']);
	});

	it('hands the core what is known and leaves the rest NaN', () => {
		expect(Array.from(unitRefs({ fps: 25, pixels: 1920 })).map((v) => (Number.isNaN(v) ? null : v))).toEqual([25, null, null, 1920, null]);
		expect(coreUnits(core, { kind: 'trim', dimension: 'time', stored: 's', refs: { fps: 25 } }).options.map((o) => o.id)).toEqual(['s', 'ms', 'frames']);
	});
});
