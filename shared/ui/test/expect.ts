// A small `expect` over node:assert for the editor kit's tests (the matchers they use)
import assert from 'node:assert/strict';

const subset = (actual: unknown, expected: unknown): boolean => {
	if (typeof expected !== 'object' || expected === null) return Object.is(actual, expected);
	if (typeof actual !== 'object' || actual === null) return false;
	return Object.entries(expected).every(([k, v]) => subset((actual as Record<string, unknown>)[k], v));
};

export const expect = (actual: unknown) => ({
	toBe: (expected: unknown) => assert.equal(actual, expected),
	toEqual: (expected: unknown) => assert.deepEqual(actual, expected),
	toMatchObject: (expected: object) => assert.ok(subset(actual, expected), `${JSON.stringify(actual)} does not match ${JSON.stringify(expected)}`),
	toBeNull: () => assert.equal(actual, null),
	toBeUndefined: () => assert.equal(actual, undefined),
	toBeGreaterThanOrEqual: (n: number) => assert.ok((actual as number) >= n, `${actual} < ${n}`),
	toBeLessThanOrEqual: (n: number) => assert.ok((actual as number) <= n, `${actual} > ${n}`),
	toContain: (item: unknown) => assert.ok((actual as unknown[]).includes(item), `${JSON.stringify(actual)} lacks ${JSON.stringify(item)}`),
	not: {
		toBe: (expected: unknown) => assert.notEqual(actual, expected),
		toContain: (item: unknown) => assert.ok(!(actual as unknown[]).includes(item), `${JSON.stringify(actual)} has ${JSON.stringify(item)}`)
	}
});
