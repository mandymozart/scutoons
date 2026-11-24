import { describe, it } from 'node:test';
import { expect } from './expect.ts';
import { dateText } from '../src/editor/date-text.ts';

describe('dateText', () => {
	it('keeps an ISO day', () => {
		expect(dateText('2026-11-15')).toBe('2026-11-15');
	});
	it('pads a short ISO day and reads the dotted form', () => {
		expect(dateText(' 2026-1-5 ')).toBe('2026-01-05');
		expect(dateText('5.1.2026')).toBe('2026-01-05');
	});
	it('reads an empty field as unset', () => {
		expect(dateText('  ')).toBe('');
	});
	it('refuses what is not a day', () => {
		expect(dateText('2026-02-30')).toBe(null);
		expect(dateText('2026-13-01')).toBe(null);
		expect(dateText('tomorrow')).toBe(null);
		expect(dateText('2026-11')).toBe(null);
	});
});
