import assert from 'node:assert/strict';
import { test } from 'node:test';
import { frameOf, frameOptions, windowAction } from '../src/frame.ts';

test('Windows and Linux: no system title bar, the page draws the buttons; resize and shadow stay', () => {
	for (const platform of ['win32', 'linux']) {
		assert.equal(frameOf(platform), 'own');
		assert.deepEqual(frameOptions(platform), { frame: false, thickFrame: true, hasShadow: true });
	}
});

test("macOS: the system's traffic lights in a hidden, inset title bar", () => {
	assert.equal(frameOf('darwin'), 'inset');
	assert.deepEqual(frameOptions('darwin'), { titleBarStyle: 'hiddenInset', hasShadow: true });
});

test('the page may only minimize, maximize / restore and close its window', () => {
	assert.equal(windowAction('minimize'), 'minimize');
	assert.equal(windowAction('maximize'), 'maximize');
	assert.equal(windowAction('close'), 'close');
	for (const raw of ['quit', 'setBounds', '', null, 1, { action: 'close' }]) assert.equal(windowAction(raw), null);
});
