// A code typed by hand on the membership screen / in the camera (components/camera/typed-code.ts):
//   npm test -w viewer
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { parseTypedCode } from '../src/lib/components/camera/typed-code.ts';

const link = (url: string) => new URL(url).searchParams.get('c');

test('a typed code: case kept, spaces dropped, only letters, digits, - and _', () => {
	assert.equal(parseTypedCode(' Ab3 xY9 ', link), 'Ab3xY9');
	assert.equal(parseTypedCode('ab3xy9', link), 'ab3xy9');
	assert.equal(parseTypedCode('A-b_3', link), 'A-b_3');
	assert.equal(parseTypedCode('', link), null);
	assert.equal(parseTypedCode('not a code!', link), null);
	assert.equal(parseTypedCode('x'.repeat(33), link), null);
});

test('a pasted link goes through the scanner parser', () => {
	assert.equal(parseTypedCode('https://qr.scutoons.com/?c=Ab3xY9', link), 'Ab3xY9');
	assert.equal(parseTypedCode('https://qr.scutoons.com/', link), null);
});
